//! Real production Store, parent sessions, migrations and Axum assembly.
use super::Fixture;
use crate::esk_asset::platform::game_access::test_support;
use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

fn tls(f: &Fixture) -> Router {
    crate::node_endpoint_transport::asset_access::test_routes("https://main.example.test")
        .with_state(Arc::clone(&f.state))
}
async fn send(
    router: &Router,
    path: &str,
    token: Option<&str>,
    service: Option<&str>,
    origin: Option<&str>,
    body: String,
) -> (StatusCode, Value) {
    let mut r = Request::builder()
        .method("POST")
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(t) = token {
        r = r.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    if let Some(s) = service {
        r = r.header("x-esk-game-service", s);
    }
    if let Some(o) = origin {
        r = r.header(header::ORIGIN, o);
    }
    let response = router
        .clone()
        .oneshot(r.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(response.headers()[header::REFERRER_POLICY], "no-referrer");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 32 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
fn authorize() -> String {
    json!({"schema":"esk.game.access.authorize.v1","client_id":"esk-game.web",
        "redirect_uri":"https://game.example.test/api/account/callback","state":"s".repeat(43),
        "code_challenge":"E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM","code_challenge_method":"S256",
        "scopes":["play","inventory_read","redeem"],"expires_in_seconds":900,
        "explicit_consent":true,"confirmation":"授权此游戏使用我的主账号及所选权限"}).to_string()
}
async fn login(router: &Router, f: &Fixture) -> Value {
    let (s, code) = send(
        router,
        "/api/me/game-access/authorize",
        Some(&f.user_token),
        None,
        Some("https://main.example.test"),
        authorize(),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{code}");
    let body=json!({"schema":"esk.game.access.exchange.v1","grant_type":"authorization_code","client_id":"esk-game.web",
        "redirect_uri":code["redirect_uri"],"state":code["state"],"code":code["code"],
        "code_verifier":"dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"}).to_string();
    let (s, t) = send(
        router,
        "/api/game-access/v1/token",
        None,
        Some(&test_support::service()),
        None,
        body,
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{t}");
    t
}
fn challenge(token: &str, nonce: usize) -> String {
    use sha2::{Digest, Sha256};
    json!({"domain":"esk.game.session.challenge.v1","main_issuer":"synthetic-main","audience":"esk-game",
        "stage":"platform_recorded","nonce":format!("{nonce:064x}"),"credential_digest":hex::encode(Sha256::digest(token.as_bytes())),
        "action":{"kind":"authenticate"}}).to_string()
}

#[tokio::test]
async fn game_access_real_user_login_observe_revoke_and_parent_logout() {
    let _policy = test_support::enable();
    let f = Fixture::new();
    let router = tls(&f);
    let t = login(&router, &f).await;
    let token = t["access_token"].as_str().unwrap();
    let (s, o) = send(
        &router,
        "/api/game-access/v1/observe",
        Some(token),
        Some(&test_support::service()),
        None,
        challenge(token, 1),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(o["grant"]["main_user_id"], f.user_id);
    assert!(o["grant"]["main_session_id"].is_string());
    assert!(!o.to_string().contains(&f.user_token));
    let path = format!(
        "/api/me/game-access/grants/{}/revoke",
        t["grant_id"].as_str().unwrap()
    );
    let revoke = json!({"schema":"esk.game.access.revoke.v1","expected_revision":"1"}).to_string();
    let (s, _) = send(
        &router,
        &path,
        Some(&f.other_token),
        None,
        Some("https://main.example.test"),
        revoke.clone(),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _) = send(
        &router,
        &path,
        Some(&f.user_token),
        None,
        Some("https://main.example.test"),
        revoke,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (s, _) = send(
        &router,
        "/api/game-access/v1/observe",
        Some(token),
        Some(&test_support::service()),
        None,
        challenge(token, 2),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let t = login(&router, &f).await;
    let token = t["access_token"].as_str().unwrap();
    f.state
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE sessions SET revoked_at='revoked' WHERE user_id=?1",
            [&f.user_id],
        )
        .unwrap();
    let (s, _) = send(
        &router,
        "/api/game-access/v1/observe",
        Some(token),
        Some(&test_support::service()),
        None,
        challenge(token, 3),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    drop(router);
    f.cleanup();
}

#[tokio::test]
async fn game_access_transport_origin_and_static_owner_are_not_authority() {
    let _policy = test_support::enable();
    let f = Fixture::new();
    let (s, _) = send(
        &f.router,
        "/api/me/game-access/authorize",
        Some(&f.user_token),
        None,
        Some("https://main.example.test"),
        "not json".into(),
    )
    .await;
    assert_eq!(s, StatusCode::UPGRADE_REQUIRED);
    let router = tls(&f);
    let (s, _) = send(
        &router,
        "/api/me/game-access/authorize",
        Some(&f.user_token),
        None,
        Some("https://evil.example"),
        authorize(),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    for token in [
        None,
        Some("synthetic-static-owner-not-a-session"),
        Some("aat_fake"),
    ] {
        let (s, _) = send(
            &router,
            "/api/me/game-access/authorize",
            token,
            None,
            Some("https://main.example.test"),
            authorize(),
        )
        .await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
    }
    drop(router);
    f.cleanup();
}

#[tokio::test]
async fn game_access_requires_explicit_config_service_and_strict_fields() {
    let f = Fixture::new();
    let router = tls(&f);
    {
        let _policy = test_support::disabled();
        let (s, o) = send(
            &router,
            "/api/me/game-access/authorize",
            Some(&f.user_token),
            None,
            Some("https://main.example.test"),
            authorize(),
        )
        .await;
        assert_eq!(s, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(o["error"], "game_access_disabled");
    }
    let _policy = test_support::enable();
    let raw = authorize().replace(
        "\"explicit_consent\":true",
        "\"explicit_consent\":true,\"explicit_consent\":false",
    );
    let (s, _) = send(
        &router,
        "/api/me/game-access/authorize",
        Some(&f.user_token),
        None,
        Some("https://main.example.test"),
        raw,
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let t = login(&router, &f).await;
    let token = t["access_token"].as_str().unwrap();
    let (s, _) = send(
        &router,
        "/api/game-access/v1/observe",
        Some(token),
        None,
        None,
        challenge(token, 1),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _) = send(
        &router,
        "/api/game-access/v1/observe?debug=1",
        Some(token),
        Some(&test_support::service()),
        None,
        challenge(token, 1),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    drop(router);
    f.cleanup();
}

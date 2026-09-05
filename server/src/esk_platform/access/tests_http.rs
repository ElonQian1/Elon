//! Included beneath the existing platform HTTP fixture; no new account/test-state framework.
use super::Fixture;
use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
    Router,
};
use rusqlite::params;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

use crate::esk_asset::platform::access::*;

const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";

fn tls_router(fixture: &Fixture) -> Router {
    crate::node_endpoint_transport::asset_access::test_routes("https://main.example.test")
        .with_state(Arc::clone(&fixture.state))
}

async fn send(
    router: &Router,
    method: &str,
    path: &str,
    token: Option<&str>,
    client: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    if let Some(client) = client {
        request = request.header(CLIENT_HEADER, client);
    }
    let response = router
        .clone()
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(response.headers()[header::PRAGMA], "no-cache");
    assert_eq!(response.headers()[header::REFERRER_POLICY], "no-referrer");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn token(router: &Router, fixture: &Fixture) -> String {
    let (status, code) = send(
        router,
        "POST",
        "/api/me/asset-access/authorize",
        Some(&fixture.user_token),
        None,
        json!({
            "schema":AUTHORIZE_SCHEMA,"client_id":"quant.android",
            "redirect_uri":"com.elon.quant:/asset-access/callback","state":"s".repeat(32),
            "code_challenge":challenge(VERIFIER).unwrap(),"code_challenge_method":"S256",
            "scopes":["esk.summary.read"],"expires_in":3600,"explicit_consent":true,
            "confirmation":AUTHORIZE_CONFIRMATION,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(code["schema"], "yilong.asset_access.authorization_code.v1");
    let (status, value) = send(
        router,
        "POST",
        "/api/asset-access/token",
        None,
        None,
        json!({
            "schema":TOKEN_SCHEMA,"grant_type":"authorization_code","client_id":"quant.android",
            "redirect_uri":"com.elon.quant:/asset-access/callback","state":"s".repeat(32),
            "code":code["code"],"code_verifier":VERIFIER,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(value["schema"], "yilong.asset_access.token.v1");
    assert_eq!(value["expires_at"], code["expires_at"]);
    assert!(value.get("refresh_token").is_none());
    assert!(!value.to_string().contains(&fixture.user_token));
    value["access_token"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn forged_proxy_headers_on_http_fail_before_body_parsing_and_remain_private() {
    let fixture = Fixture::new();
    let router = routes().with_state(Arc::clone(&fixture.state));
    for (name, value) in [
        ("forwarded", "proto=https;host=main.example.test"),
        ("x-forwarded-proto", "https"),
        ("x-forwarded-host", "main.example.test"),
    ] {
        let request = Request::builder()
            .method("POST")
            .uri("/api/me/asset-access/authorize")
            .header(name, value)
            .header(header::CONTENT_TYPE, "application/json")
            .header(
                header::AUTHORIZATION,
                format!("Bearer {}", fixture.user_token),
            )
            .body(Body::from("not-json-synthetic-sensitive-body"))
            .unwrap();
        let response = router.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UPGRADE_REQUIRED);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert_eq!(response.headers()[header::PRAGMA], "no-cache");
        assert!(response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_none());
        let body = to_bytes(response.into_body(), 4096).await.unwrap();
        assert!(!String::from_utf8_lossy(&body).contains("not-json-synthetic-sensitive-body"));
    }
    drop(router);
    fixture.cleanup();
}

#[tokio::test]
async fn tls_assembly_authorizes_once_reads_minimal_identity_and_revokes_only_its_grant() {
    let fixture = Fixture::new();
    let router = tls_router(&fixture);
    let bearer = token(&router, &fixture).await;
    let (status, identity) = send(
        &router,
        "GET",
        "/api/asset-access/me",
        Some(&bearer),
        Some("quant.android"),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(identity["schema"], "yilong.asset_access.identity.v1");
    for key in [
        "user_id",
        "account",
        "email",
        "role",
        "nickname",
        "avatar_data_url",
    ] {
        assert!(identity.get(key).is_none());
    }
    assert_ne!(identity["subject"], fixture.user_id);
    let (status, _) = send(
        &router,
        "GET",
        "/api/asset-access/me",
        Some(&bearer),
        Some("quant.web"),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let revoke = json!({"schema":REVOKE_SCHEMA,"confirmation":REVOKE_CONFIRMATION});
    for _ in 0..2 {
        let (status, value) = send(
            &router,
            "POST",
            "/api/asset-access/revoke",
            Some(&bearer),
            Some("quant.android"),
            revoke.clone(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(value["revoked"], true);
    }
    let (status, _) = send(
        &router,
        "GET",
        "/api/asset-access/me",
        Some(&bearer),
        Some("quant.android"),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(fixture
        .state
        .store
        .authenticate_token(&fixture.user_token)
        .is_ok());
    drop(router);
    fixture.cleanup();
}

#[tokio::test]
async fn grant_directory_get_does_not_touch_session_last_seen_or_authorization_rows() {
    let fixture = Fixture::new();
    let router = tls_router(&fixture);
    let _bearer = token(&router, &fixture).await;
    let before = {
        let conn = fixture.state.store.conn().unwrap();
        conn.execute(
            "UPDATE sessions SET last_seen_at='2000-01-01T00:00:00Z' WHERE user_id=?1",
            params![fixture.user_id],
        )
        .unwrap();
        conn.query_row(
            "SELECT last_seen_at,expires_at,(SELECT COUNT(*) FROM asset_access_audit)
            FROM sessions WHERE user_id=?1",
            params![fixture.user_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .unwrap()
    };
    let (status, value) = send(
        &router,
        "GET",
        "/api/me/asset-access/grants",
        Some(&fixture.user_token),
        None,
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(value["grants"].as_array().unwrap().len(), 1);
    let after = fixture
        .state
        .store
        .conn()
        .unwrap()
        .query_row(
            "SELECT last_seen_at,expires_at,(SELECT COUNT(*) FROM asset_access_audit)
        FROM sessions WHERE user_id=?1",
            params![fixture.user_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(before, after);
    drop(router);
    fixture.cleanup();
}

#[tokio::test]
async fn restricted_token_never_enters_formal_asset_write_or_master_auth() {
    let fixture = Fixture::new();
    let router = tls_router(&fixture);
    let bearer = token(&router, &fixture).await;
    for path in [
        "/api/me/assets/esk/platform/sellback-requests",
        "/api/admin/assets/esk/platform-allocations/prepare",
    ] {
        let (status, _) = send(
            &fixture.router,
            "POST",
            path,
            Some(&bearer),
            None,
            json!({}),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    assert!(fixture.state.store.authenticate_token(&bearer).is_err());
    drop(router);
    fixture.cleanup();
}

#[tokio::test]
async fn foreign_origin_cannot_use_tls_marker_to_submit_authorization() {
    let fixture = Fixture::new();
    let router = tls_router(&fixture);
    let request = Request::builder()
        .method("POST")
        .uri("/api/asset-access/token")
        .header(header::ORIGIN, "https://other.example.test")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("malformed-json"))
        .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert!(response
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .is_none());
    drop(response);
    drop(router);
    fixture.cleanup();
}

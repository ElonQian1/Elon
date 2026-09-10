//! Browser routes use production password sessions and the same grant authority.
use super::{
    game_access_http_tests::{authorize, send},
    Fixture,
};
use crate::esk_asset::platform::game_access::test_support;
use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
    Router,
};
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;

fn browser(f: &Fixture) -> Router {
    crate::node_endpoint_transport::asset_access::test_routes("https://main.example.test")
        .merge(
            crate::node_endpoint_transport::asset_access::browser_routes(
                "https://main.example.test",
                &f.state.data_dir,
            ),
        )
        .with_state(Arc::clone(&f.state))
}

#[tokio::test]
async fn game_access_browser_password_login_issues_original_main_session() {
    let _policy = test_support::enable();
    let f = Fixture::new();
    let router = browser(&f);
    let body = json!({"account":"holder@example.test","password":"secret1"}).to_string();
    for (origin, expected) in [
        (None, StatusCode::FORBIDDEN),
        (Some("https://evil.test"), StatusCode::FORBIDDEN),
    ] {
        let (status, _) = send(
            &router,
            "/api/game-access/v1/login",
            None,
            None,
            origin,
            body.clone(),
        )
        .await;
        assert_eq!(status, expected);
    }
    let (status, session) = send(
        &router,
        "/api/game-access/v1/login",
        None,
        None,
        Some("https://main.example.test"),
        body,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{session}");
    assert_eq!(session["user"]["id"], f.user_id);
    let token = session["token"].as_str().unwrap();
    assert_eq!(
        f.state.store.authenticate_token(token).unwrap().id,
        f.user_id
    );
    let (status, grant) = send(
        &router,
        "/api/me/game-access/authorize",
        Some(token),
        None,
        Some("https://main.example.test"),
        authorize(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{grant}");
    for account in ["holder@example.test", "missing@example.test"] {
        let (status, value) = send(
            &router,
            "/api/game-access/v1/login",
            None,
            None,
            Some("https://main.example.test"),
            json!({"account":account,"password":"wrong"}).to_string(),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(value["error"], "game_login_failed");
    }
    drop(router);
    f.cleanup();
}

#[tokio::test]
async fn game_access_browser_scope_disabled_strict_body_and_static_assets() {
    let f = Fixture::new();
    let router = browser(&f);
    {
        let _policy = test_support::disabled();
        let (status, value) = send(
            &router,
            "/api/game-access/v1/login",
            None,
            None,
            Some("https://main.example.test"),
            "{}".into(),
        )
        .await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(value["error"], "game_access_disabled");
    }
    let _policy = test_support::enable();
    for body in [
        r#"{"account":"holder@example.test","password":"secret1","remember_device":true}"#
            .to_owned(),
        "x".repeat(4097),
    ] {
        let (status, _) = send(
            &router,
            "/api/game-access/v1/login",
            None,
            None,
            Some("https://main.example.test"),
            body,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    let dist = f.state.data_dir.join("pc-next-dist");
    std::fs::create_dir_all(dist.join("assets")).unwrap();
    std::fs::write(
        dist.join("index.html"),
        "<!doctype html><title>Game consent fixture</title>",
    )
    .unwrap();
    std::fs::write(dist.join("assets/test.js"), "export const fixture = true;").unwrap();
    for path in [
        "/pc/game-access?request=synthetic",
        "/pc/game-login",
        "/pc/assets/test.js",
    ] {
        let response = router
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert_eq!(response.headers()[header::REFERRER_POLICY], "no-referrer");
        assert!(response.headers()[header::CONTENT_SECURITY_POLICY]
            .to_str()
            .unwrap()
            .contains("frame-ancestors 'none'"));
        assert!(!to_bytes(response.into_body(), 4096)
            .await
            .unwrap()
            .is_empty());
    }
    // Scope does not accidentally mount legacy main APIs or the node owner's login protocol.
    for path in ["/api/auth/login", "/api/auth/register", "/api/me", "/pc/ai"] {
        let response = router
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    let raw = crate::esk_asset::platform::game_access::browser::routes(&f.state.data_dir)
        .with_state(Arc::clone(&f.state));
    let (status, _) = send(
        &raw,
        "/api/game-access/v1/login",
        None,
        None,
        Some("https://main.example.test"),
        "{}".into(),
    )
    .await;
    assert_eq!(status, StatusCode::UPGRADE_REQUIRED);
    drop(raw);
    drop(router);
    f.cleanup();
}

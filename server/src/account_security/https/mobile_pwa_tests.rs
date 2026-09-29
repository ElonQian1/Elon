use super::*;
use axum::{
    body::{to_bytes, Body, Bytes},
    extract::DefaultBodyLimit,
    http::{Request, StatusCode},
    routing::{get, post},
};
use tower::ServiceExt;

fn browser() -> Router {
    Router::new()
        .route("/web", get(|| async { "browser" }))
        .route("/api/me/groups", get(|| async { StatusCode::UNAUTHORIZED }))
        .route(
            "/api/me",
            get(|| async { "must not override account policy" }),
        )
        .route(
            "/upload",
            post(|bytes: Bytes| async move { bytes.len().to_string() }),
        )
        .layer(DefaultBodyLimit::max(12 * 1024 * 1024))
}

#[test]
fn opt_in_requires_tls_and_rejects_ambiguous_values() {
    assert!(!enabled(None, false).unwrap());
    assert!(!enabled(Some("false"), true).unwrap());
    assert!(enabled(Some("true"), true).unwrap());
    assert!(enabled(Some("true"), false).is_err());
    assert!(enabled(Some("yes"), true).is_err());
}

#[tokio::test]
async fn browser_routes_are_absent_until_enabled() {
    let app = attach(Router::new(), browser(), false);
    let response = app
        .oneshot(Request::builder().uri("/web").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(!response.headers().contains_key("x-elon-pwa-transport"));
}

#[tokio::test]
async fn enabled_router_preserves_authentication_and_account_precedence() {
    let account = super::super::policy::protect(
        Router::new().route("/api/me", get(|| async { StatusCode::UNAUTHORIZED })),
    );
    let app = attach(account, browser(), true);
    for (path, status, marked) in [
        ("/web", StatusCode::OK, true),
        ("/api/me/groups", StatusCode::UNAUTHORIZED, true),
        ("/api/me", StatusCode::UNAUTHORIZED, false),
        ("/api/me?token=probe", StatusCode::NOT_FOUND, false),
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), status, "{path}");
        assert_eq!(
            response.headers().contains_key("x-elon-pwa-transport"),
            marked,
            "{path}"
        );
    }
}

#[tokio::test]
async fn uploads_keep_browser_body_limits_instead_of_account_limits() {
    let app = attach(
        super::super::policy::protect(Router::new()),
        browser(),
        true,
    );
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/upload")
                .body(Body::from(vec![0; 32768]))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(response.into_body(), 100).await.unwrap().as_ref(),
        b"32768"
    );
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/upload")
                .body(Body::from(vec![0; 12 * 1024 * 1024 + 1]))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

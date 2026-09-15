use super::*;
use axum::{body::Body, Extension};
use tower::ServiceExt;

#[tokio::test]
async fn square_http_guard_requires_bearer_and_real_socket_context() {
    let app = Router::new()
        .route("/probe", get(|| async { "ok" }).post(|| async { "ok" }))
        .layer(middleware::from_fn(guard));
    for (method, bearer, peer, expected) in [
        ("GET", false, false, StatusCode::UNAUTHORIZED),
        ("POST", true, false, StatusCode::SERVICE_UNAVAILABLE),
        ("POST", true, true, StatusCode::OK),
        ("GET", true, false, StatusCode::OK),
    ] {
        let mut request = axum::http::Request::builder()
            .uri("/probe")
            .method(method)
            .header("x-forwarded-proto", "https");
        if bearer {
            request = request.header("authorization", "Bearer fixture-token");
        }
        let app = if peer {
            app.clone().layer(Extension(ConnectInfo(
                "192.0.2.199:1000".parse::<SocketAddr>().unwrap(),
            )))
        } else {
            app.clone()
        };
        let response = app
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        if expected == StatusCode::OK {
            assert_eq!(response.headers()["cache-control"], "private, no-store");
        }
    }
}

#[test]
fn square_cors_does_not_accept_unrelated_origins() {
    assert!(allowed_origin("https://43.139.149.158:8443", None));
    assert!(allowed_origin("http://127.0.0.1:7799", None));
    assert!(!allowed_origin("http://127.0.0.1:9000", None));
    assert!(!allowed_origin(
        "https://43.139.149.158.evil.test:8443",
        None
    ));
    assert!(!allowed_origin("null", None));
}

#[tokio::test]
async fn square_page_serves_only_its_secure_assets_and_no_secrets() {
    let app = super::super::square_page::routes();
    let r = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/square")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(r.headers()["cache-control"], "no-store");
    assert!(r.headers()["content-security-policy"]
        .to_str()
        .unwrap()
        .contains("frame-ancestors 'none'"));
    let r = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/admin/users")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
}

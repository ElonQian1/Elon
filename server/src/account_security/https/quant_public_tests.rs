use super::*;
use axum::{body::Body, http::Method, routing::get};
use tower::ServiceExt;

fn request(method: Method, uri: &str) -> Request {
    Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

#[test]
fn public_https_requires_explicit_valid_tls_configuration() {
    assert!(!enabled(None, false).unwrap());
    assert!(!enabled(Some("false"), true).unwrap());
    assert!(enabled(Some("true"), true).unwrap());
    assert!(enabled(Some("true"), false).is_err());
    for value in ["TRUE", "1", "", " true", "false "] {
        assert!(enabled(Some(value), true).is_err());
    }
}

#[tokio::test]
async fn public_attachment_does_not_widen_account_or_private_api_surface() {
    let account =
        super::super::policy::protect(Router::new().route("/api/me", get(|| async { "account" })));
    let app = attach(account, Path::new("missing-quant-test-dist"), true);
    assert_eq!(
        app.clone()
            .oneshot(request(Method::GET, "/api/me"))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    for uri in [
        "/api/me?token=test",
        "/api/nodes",
        "/mcp",
        "/ws",
        "/quant/api/v1/grid-center/robots",
        "/quant/api/v1/participants",
        "/quant/api/v1/paper/orders",
    ] {
        assert_eq!(
            app.clone()
                .oneshot(request(Method::GET, uri))
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND,
            "{uri}"
        );
    }
    assert_eq!(
        app.clone()
            .oneshot(request(Method::POST, "/quant/api/v1/markets/spot/overview"))
            .await
            .unwrap()
            .status(),
        StatusCode::METHOD_NOT_ALLOWED
    );
    assert_eq!(
        app.clone()
            .oneshot(request(Method::GET, "/quant"))
            .await
            .unwrap()
            .status(),
        StatusCode::PERMANENT_REDIRECT
    );
    let disabled = attach(
        super::super::policy::protect(Router::new()),
        Path::new("missing-quant-test-dist"),
        false,
    );
    assert_eq!(
        disabled
            .oneshot(request(Method::GET, "/quant"))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn public_middleware_strips_credentials_and_bounds_requests() {
    let app = Router::new()
        .route(
            "/quant/",
            get(|headers: axum::http::HeaderMap| async move {
                assert!(!headers.contains_key(header::AUTHORIZATION));
                assert!(!headers.contains_key(header::COOKIE));
                ([(header::SET_COOKIE, "must-not-persist=1")], "public")
            }),
        )
        .layer(middleware::from_fn(public_request));
    let mut valid = request(Method::GET, "/quant/?app_section=market");
    valid.headers_mut().insert(
        header::AUTHORIZATION,
        HeaderValue::from_static("Bearer test"),
    );
    valid
        .headers_mut()
        .insert(header::COOKIE, HeaderValue::from_static("test=1"));
    let response = app.clone().oneshot(valid).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(!response.headers().contains_key(header::SET_COOKIE));
    assert_eq!(
        response.headers()["x-yilong-quant-transport"],
        "https-public-v1"
    );
    assert_eq!(
        response.headers()[header::STRICT_TRANSPORT_SECURITY],
        "max-age=15552000"
    );
    let oversized = format!("/quant/?{}", "x".repeat(2049));
    assert_eq!(
        app.oneshot(request(Method::GET, &oversized))
            .await
            .unwrap()
            .status(),
        StatusCode::URI_TOO_LONG
    );
}

#[tokio::test]
async fn public_router_serves_existing_dist_with_original_headers() {
    let root = std::env::temp_dir().join(format!("elon-quant-https-{}", uuid::Uuid::new_v4()));
    let dist = root.join("quant-http-preview-dist");
    std::fs::create_dir_all(dist.join("assets")).unwrap();
    std::fs::write(dist.join("index.html"), "<html>quant-test</html>").unwrap();
    std::fs::write(dist.join("assets/test.js"), "export const test = true").unwrap();
    let app = attach(super::super::policy::protect(Router::new()), &root, true);
    let page = app
        .clone()
        .oneshot(request(Method::GET, "/quant/?app_section=market"))
        .await
        .unwrap();
    assert_eq!(page.status(), StatusCode::OK);
    assert_eq!(
        page.headers()[header::CACHE_CONTROL],
        "no-store, no-cache, must-revalidate"
    );
    assert!(page.headers()[header::CONTENT_SECURITY_POLICY]
        .to_str()
        .unwrap()
        .contains("form-action 'none'"));
    let asset = app
        .clone()
        .oneshot(request(Method::GET, "/quant/assets/test.js"))
        .await
        .unwrap();
    assert_eq!(asset.status(), StatusCode::OK);
    assert_eq!(
        asset.headers()[header::CACHE_CONTROL],
        "public, max-age=31536000, immutable"
    );
    for uri in [
        "/quant/assets/../index.html",
        "/quant/assets/%2e%2e/index.html",
        "/quant/assets//test.js",
    ] {
        assert_eq!(
            app.clone()
                .oneshot(request(Method::GET, uri))
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND,
            "{uri}"
        );
    }
    std::fs::remove_file(dist.join("assets/test.js")).unwrap();
    std::fs::remove_file(dist.join("index.html")).unwrap();
    std::fs::remove_dir(dist.join("assets")).unwrap();
    std::fs::remove_dir(dist).unwrap();
    std::fs::remove_dir(root).unwrap();
}

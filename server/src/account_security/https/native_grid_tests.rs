//! Regression for the account TLS ingress that serves production port 8443.
use super::*;
use axum::{body::Body, routing::get};
use tower::ServiceExt;

#[tokio::test]
async fn native_grid_issuer_reaches_handler_only_on_exact_get_and_post() {
    const PATH: &str = "/api/me/quant/native-grid/access-grants";
    let app = protect(Router::new().route(
        PATH,
        get(|| async { StatusCode::UNAUTHORIZED }).post(|| async { StatusCode::UNAUTHORIZED }),
    ));
    for (method, path, expected) in [
        ("GET", PATH.to_owned(), StatusCode::UNAUTHORIZED),
        ("POST", PATH.to_owned(), StatusCode::UNAUTHORIZED),
        ("PUT", PATH.to_owned(), StatusCode::NOT_FOUND),
        ("DELETE", PATH.to_owned(), StatusCode::NOT_FOUND),
        ("OPTIONS", PATH.to_owned(), StatusCode::NOT_FOUND),
        (
            "GET",
            format!("{PATH}?token=synthetic"),
            StatusCode::NOT_FOUND,
        ),
        ("POST", format!("{PATH}/extra"), StatusCode::NOT_FOUND),
        ("GET", format!("{PATH}/"), StatusCode::NOT_FOUND),
        (
            "GET",
            PATH.replace("native-grid", "native%2Dgrid"),
            StatusCode::NOT_FOUND,
        ),
    ] {
        let request = Request::builder()
            .method(method)
            .uri(&path)
            .extension(ConnectInfo(
                "192.0.2.83:1234".parse::<SocketAddr>().unwrap(),
            ))
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), expected, "{method} {path}");
        if expected == StatusCode::UNAUTHORIZED {
            assert_eq!(response.headers()["cache-control"], "no-store");
            assert_eq!(response.headers()["referrer-policy"], "no-referrer");
        }
    }
}

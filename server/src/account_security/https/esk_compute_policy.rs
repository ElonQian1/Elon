//! Exact account-TLS entry; other account endpoints retain their existing policy.
use axum::{
    extract::Request,
    http::{Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    Router,
};

pub(super) fn origin(public_url: &str) -> String {
    // The configured main project asset origin, also used by native clients.
    // Never derive TLS authority from Host, Origin or forwarding headers.
    if matches!(
        public_url,
        "http://43.139.149.158:8080" | "http://43.139.149.158:8080/"
    ) {
        "https://43.139.149.158:8443".into()
    } else {
        public_url.into()
    }
}

pub(super) fn protect(app: Router) -> Router {
    app.layer(middleware::from_fn(guard))
}

fn allowed(method: &Method, path: &str, query: Option<&str>) -> bool {
    match (method, path) {
        (&Method::GET, "/pc/esk-compute") => query.is_none(),
        (&Method::POST, "/api/esk-compute-center/login") => query.is_none(),
        (&Method::GET, "/api/me/esk-compute-center") => query.is_none_or(|query| {
            query.strip_prefix("page=").is_some_and(|page| {
                !page.starts_with('0')
                    && page.len() <= 4
                    && page.bytes().all(|b| b.is_ascii_digit())
                    && page.parse::<usize>().is_ok_and(|v| (1..=1000).contains(&v))
            })
        }),
        _ => false,
    }
}

async fn guard(request: Request, next: Next) -> Response {
    let mut response = if !allowed(
        request.method(),
        request.uri().path(),
        request.uri().query(),
    ) {
        StatusCode::NOT_FOUND.into_response()
    } else if let Some(response) = super::policy::throttle(&request) {
        response
    } else {
        next.run(request).await
    };
    response
        .headers_mut()
        .insert("cache-control", "no-store".parse().unwrap());
    response
        .headers_mut()
        .insert("referrer-policy", "no-referrer".parse().unwrap());
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        extract::ConnectInfo,
        routing::{get, post},
    };
    use std::net::SocketAddr;
    use tower::ServiceExt;

    #[tokio::test]
    async fn exact_account_center_routes_reach_auth_and_reject_query_credentials() {
        let app = protect(
            Router::new()
                .route("/pc/esk-compute", get(|| async { StatusCode::OK }))
                .route(
                    "/api/me/esk-compute-center",
                    get(|| async { StatusCode::UNAUTHORIZED }),
                )
                .route(
                    "/api/esk-compute-center/login",
                    post(|| async { StatusCode::UNAUTHORIZED }),
                ),
        );
        for (method, path, expected) in [
            ("GET", "/pc/esk-compute", StatusCode::OK),
            (
                "GET",
                "/api/me/esk-compute-center",
                StatusCode::UNAUTHORIZED,
            ),
            (
                "GET",
                "/api/me/esk-compute-center?page=1",
                StatusCode::UNAUTHORIZED,
            ),
            (
                "GET",
                "/api/me/esk-compute-center?page=1000",
                StatusCode::UNAUTHORIZED,
            ),
            (
                "GET",
                "/api/me/esk-compute-center?token=synthetic",
                StatusCode::NOT_FOUND,
            ),
            (
                "GET",
                "/api/me/esk-compute-center?page=1&page=2",
                StatusCode::NOT_FOUND,
            ),
            (
                "GET",
                "/api/me/esk-compute-center?page=01",
                StatusCode::NOT_FOUND,
            ),
            (
                "GET",
                "/api/me/esk-compute-center?page=1001",
                StatusCode::NOT_FOUND,
            ),
            (
                "POST",
                "/api/esk-compute-center/login",
                StatusCode::UNAUTHORIZED,
            ),
            (
                "POST",
                "/api/esk-compute-center/login?token=synthetic",
                StatusCode::NOT_FOUND,
            ),
            ("PUT", "/api/me/esk-compute-center", StatusCode::NOT_FOUND),
            (
                "GET",
                "/api/me/esk-compute-center/extra",
                StatusCode::NOT_FOUND,
            ),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(path)
                        .extension(ConnectInfo(
                            "192.0.2.123:1234".parse::<SocketAddr>().unwrap(),
                        ))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), expected, "{method} {path}");
            assert_eq!(response.headers()["cache-control"], "no-store");
        }
    }

    #[tokio::test]
    async fn login_requires_socket_identity_before_authentication() {
        let app = protect(Router::new().route(
            "/api/esk-compute-center/login",
            post(|| async { StatusCode::OK }),
        ));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/esk-compute-center/login")
                    .header("x-forwarded-for", "192.0.2.123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn origin_is_bound_to_configuration_not_caller_headers() {
        assert_eq!(
            origin("http://43.139.149.158:8080"),
            "https://43.139.149.158:8443"
        );
        assert_eq!(
            origin("https://main.example.test"),
            "https://main.example.test"
        );
        assert_eq!(
            origin("http://evil.example.test"),
            "http://evil.example.test"
        );
    }
}

use axum::{
    extract::{ConnectInfo, DefaultBodyLimit, Request},
    http::{Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    Router,
};
use std::{net::SocketAddr, time::Duration};

use crate::auth_request_guard::check_rate_limit;

pub(super) fn allowed(method: &Method, path: &str) -> bool {
    match method {
        &Method::GET => matches!(
            path,
            "/health"
                | "/api/me"
                | "/api/auth/security"
                | "/api/me/asset-access/grants"
                | "/api/asset-access/me"
                | "/api/asset-access/grids"
        ),
        &Method::POST => {
            matches!(
                path,
                "/api/auth/login"
                    | "/api/auth/register"
                    | "/api/auth/logout"
                    | "/api/auth/password/recover"
                    | "/api/auth/recovery-codes/rotate"
                    | "/api/me/asset-access/authorize"
                    | "/api/asset-access/token"
                    | "/api/asset-access/revoke"
                    | "/api/node/private-read-projections"
            ) || owner_grant_revoke_path(path)
        }
        &Method::PUT => path == "/api/auth/password",
        _ => false,
    }
}

fn owner_grant_revoke_path(path: &str) -> bool {
    // Restrict the dynamic route at ingress; the handler still validates ownership.
    path.strip_prefix("/api/me/asset-access/grants/aag_")
        .and_then(|id| id.strip_suffix("/revoke"))
        .is_some_and(|id| {
            id.len() == 32
                && id
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        })
}

pub(super) fn protect(app: Router) -> Router {
    app.layer(DefaultBodyLimit::max(16 * 1024))
        .layer(middleware::from_fn(guard))
}

async fn guard(request: Request, next: Next) -> Response {
    if !allowed(request.method(), request.uri().path()) || request.uri().query().is_some() {
        return StatusCode::NOT_FOUND.into_response();
    }
    let mut response = match throttle(&request) {
        Some(response) => response,
        None => next.run(request).await,
    };
    response
        .headers_mut()
        .insert("cache-control", "no-store".parse().unwrap());
    response
        .headers_mut()
        .insert("referrer-policy", "no-referrer".parse().unwrap());
    response
}

fn throttle(request: &Request) -> Option<Response> {
    if request.method() == Method::GET {
        return None;
    }
    // Use the socket peer, never a caller-controlled forwarding header.
    let Some(ConnectInfo(peer)) = request.extensions().get::<ConnectInfo<SocketAddr>>() else {
        return Some(StatusCode::SERVICE_UNAVAILABLE.into_response());
    };
    let window = Duration::from_secs(60);
    let result = check_rate_limit("account_https_peer", &peer.ip().to_string(), 60, window)
        .and_then(|_| check_rate_limit("account_https_global", "all", 240, window));
    result.err().map(|error| {
        let mut response = StatusCode::TOO_MANY_REQUESTS.into_response();
        response
            .headers_mut()
            .insert("retry-after", error.retry_after_seconds.into());
        response
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, routing::post, Json};
    use tower::ServiceExt;

    #[test]
    fn writes_require_socket_identity_and_cannot_spoof_limit_key() {
        let mut request = Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            throttle(&request).unwrap().status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        request.extensions_mut().insert(ConnectInfo(
            "192.0.2.71:1234".parse::<SocketAddr>().unwrap(),
        ));
        for _ in 0..60 {
            assert!(throttle(&request).is_none());
        }
        request
            .headers_mut()
            .insert("x-forwarded-for", "192.0.2.72".parse().unwrap());
        let response = throttle(&request).unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(response.headers().contains_key("retry-after"));
        *request.method_mut() = Method::GET;
        assert!(throttle(&request).is_none());
    }

    #[test]
    fn only_account_routes_are_published() {
        assert!(allowed(&Method::POST, "/api/auth/login"));
        assert!(allowed(&Method::PUT, "/api/auth/password"));
        assert!(allowed(&Method::POST, "/api/node/private-read-projections"));
        assert!(allowed(&Method::GET, "/api/asset-access/grids"));
        assert!(allowed(&Method::POST, "/api/asset-access/revoke"));
        assert!(!allowed(&Method::POST, "/api/asset-access/grids"));
        assert!(!allowed(&Method::GET, "/api/node/private-read-projections"));
        assert!(!allowed(&Method::GET, "/quant/api/v1/grid-center/robots"));
        for path in [
            "/api/admin/users",
            "/api/nodes",
            "/mcp",
            "/ws",
            "/api/auth/login/extra",
            "/api/auth/sessions",
            "/api/auth/password/recovery/start",
        ] {
            assert!(!allowed(&Method::GET, path));
            assert!(!allowed(&Method::POST, path));
        }
    }

    #[tokio::test]
    async fn account_https_rejects_query_tokens_and_oversize_bodies() {
        let app = protect(Router::new().route(
            "/api/auth/login",
            post(|Json(_): Json<serde_json::Value>| async { StatusCode::OK }),
        ));
        let request = |uri, body| {
            Request::builder()
                .extension(ConnectInfo(
                    "127.0.0.1:12345".parse::<SocketAddr>().unwrap(),
                ))
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap()
        };
        assert_eq!(
            app.clone()
                .oneshot(request("/api/auth/login?token=secret", "{}".to_owned()))
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            app.clone()
                .oneshot(request(
                    "/api/auth/login",
                    format!("{{\"x\":\"{}\"}}", "x".repeat(17000))
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::PAYLOAD_TOO_LARGE
        );
        let response = app
            .oneshot(request("/api/auth/login", "{}".to_owned()))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "no-store");
    }

    #[tokio::test]
    async fn owner_revocation_reaches_handler_only_on_the_exact_post_route() {
        let app = protect(Router::new().route(
            "/api/me/asset-access/grants/:grant_id/revoke",
            post(|| async { StatusCode::NO_CONTENT }),
        ));
        let valid = "/api/me/asset-access/grants/aag_0123456789abcdef0123456789abcdef/revoke";
        for (method, path, expected) in [
            ("POST", valid.to_owned(), StatusCode::NO_CONTENT),
            ("GET", valid.to_owned(), StatusCode::NOT_FOUND),
            (
                "POST",
                format!("{valid}?token=secret"),
                StatusCode::NOT_FOUND,
            ),
            ("POST", format!("{valid}/extra"), StatusCode::NOT_FOUND),
            (
                "POST",
                valid.replace("aag_", "aag_%30"),
                StatusCode::NOT_FOUND,
            ),
            (
                "POST",
                valid.replace("abcdef", "ABCDEF"),
                StatusCode::NOT_FOUND,
            ),
            (
                "POST",
                valid.replace("aag_", "aag_/"),
                StatusCode::NOT_FOUND,
            ),
        ] {
            let request = Request::builder()
                .method(method)
                .uri(path)
                .extension(ConnectInfo(
                    "192.0.2.76:1234".parse::<SocketAddr>().unwrap(),
                ))
                .body(Body::empty())
                .unwrap();
            assert_eq!(
                app.clone().oneshot(request).await.unwrap().status(),
                expected
            );
        }
    }

    #[tokio::test]
    async fn larger_private_snapshot_limit_does_not_expand_account_limits() {
        let handler = || post(|Json(_): Json<serde_json::Value>| async { StatusCode::OK });
        let app = protect(
            Router::new().route("/api/auth/login", handler()).merge(
                Router::new()
                    .route("/api/node/private-read-projections", handler())
                    .layer(DefaultBodyLimit::max(256 * 1024)),
            ),
        );
        for (path, length, expected) in [
            ("/api/auth/login", 20_000, StatusCode::PAYLOAD_TOO_LARGE),
            ("/api/node/private-read-projections", 20_000, StatusCode::OK),
            (
                "/api/node/private-read-projections",
                262_144,
                StatusCode::PAYLOAD_TOO_LARGE,
            ),
        ] {
            let request = Request::builder()
                .extension(ConnectInfo(
                    "192.0.2.75:1234".parse::<SocketAddr>().unwrap(),
                ))
                .method("POST")
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(format!("{{\"x\":\"{}\"}}", "a".repeat(length))))
                .unwrap();
            assert_eq!(
                app.clone().oneshot(request).await.unwrap().status(),
                expected
            );
        }
    }
}

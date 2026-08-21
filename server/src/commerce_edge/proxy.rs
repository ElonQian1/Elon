use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use axum::{
    body::{to_bytes, Body},
    extract::State,
    http::{header, HeaderMap, HeaderName, HeaderValue, Request, Response, StatusCode},
    routing::get,
    Router,
};
use futures::StreamExt;
use serde_json::json;

use super::{
    config::EdgeConfig,
    routes::{RouteRegistry, RouteRejection},
};

#[derive(Clone)]
pub(crate) struct EdgeState {
    registry: Arc<RouteRegistry>,
    client: reqwest::Client,
    request_timeout: Duration,
    max_request_body_bytes: usize,
    max_response_body_bytes: usize,
}

pub(crate) fn build_client(config: &EdgeConfig) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(config.connect_timeout())
        .timeout(config.request_timeout())
        .build()
        .context("COMMERCE_EDGE_HTTP_CLIENT_BUILD_FAILED")
}

pub(crate) fn build_router(
    config: &EdgeConfig,
    registry: Arc<RouteRegistry>,
    client: reqwest::Client,
) -> Router {
    let state = EdgeState {
        registry,
        client,
        request_timeout: config.request_timeout(),
        max_request_body_bytes: config.max_request_body_bytes(),
        max_response_body_bytes: config.max_response_body_bytes(),
    };
    Router::new()
        .route("/health", get(edge_health))
        .fallback(proxy_request)
        .with_state(state)
}

async fn edge_health(State(state): State<EdgeState>, headers: HeaderMap) -> Response<Body> {
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok());
    if let Err(rejection) = state.registry.snapshot().validate_host(host) {
        return route_rejection(rejection);
    }
    json_response(
        StatusCode::OK,
        json!({
            "schema":"yilong.commerce-edge.health.v1",
            "status":"ok",
            "active_routes":state.registry.snapshot().enabled_routes()
        }),
    )
}

async fn proxy_request(State(state): State<EdgeState>, request: Request<Body>) -> Response<Body> {
    let host_header = request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let method = request.method().clone();
    let uri = request.uri().clone();
    let target = match state
        .registry
        .snapshot()
        .resolve(host_header.as_deref(), &method, &uri)
    {
        Ok(target) => target,
        Err(rejection) => return route_rejection(rejection),
    };
    let request_headers = sanitize_request_headers(request.headers(), host_header.as_deref());
    let request_body = match to_bytes(request.into_body(), state.max_request_body_bytes).await {
        Ok(body) => body,
        Err(_) => return error_response(StatusCode::PAYLOAD_TOO_LARGE, "request_body_too_large"),
    };

    let outbound = state
        .client
        .request(method, target.upstream_url())
        .headers(request_headers)
        .body(request_body);
    let upstream = match tokio::time::timeout(state.request_timeout, outbound.send()).await {
        Ok(Ok(response)) => response,
        Ok(Err(_)) => return error_response(StatusCode::BAD_GATEWAY, "upstream_unavailable"),
        Err(_) => return error_response(StatusCode::GATEWAY_TIMEOUT, "upstream_timeout"),
    };
    if upstream.status().is_redirection() {
        return error_response(StatusCode::BAD_GATEWAY, "upstream_redirect_rejected");
    }

    let status = upstream.status();
    let response_headers = sanitize_response_headers(upstream.headers());
    let mut stream = upstream.bytes_stream();
    let mut response_body = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(_) => return error_response(StatusCode::BAD_GATEWAY, "upstream_body_failed"),
        };
        if response_body.len().saturating_add(chunk.len()) > state.max_response_body_bytes {
            return error_response(StatusCode::BAD_GATEWAY, "upstream_response_body_too_large");
        }
        response_body.extend_from_slice(&chunk);
    }

    let mut response = Response::builder().status(status);
    for (name, value) in response_headers {
        if let Some(name) = name {
            response = response.header(name, value);
        }
    }
    response
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(Body::from(response_body))
        .unwrap_or_else(|_| {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "response_build_failed")
        })
}

fn sanitize_request_headers(headers: &HeaderMap, host: Option<&str>) -> HeaderMap {
    let mut clean = HeaderMap::new();
    for name in [
        header::CONTENT_TYPE,
        header::ACCEPT,
        header::USER_AGENT,
        HeaderName::from_static("x-request-id"),
        HeaderName::from_static("x-yilong-runtime-key-id"),
        HeaderName::from_static("x-yilong-runtime-timestamp"),
        HeaderName::from_static("x-yilong-runtime-signature"),
    ] {
        if let Some(value) = headers.get(&name) {
            clean.insert(name, value.clone());
        }
    }
    clean.insert(
        HeaderName::from_static("x-forwarded-proto"),
        HeaderValue::from_static("https"),
    );
    if let Some(host) = host.and_then(|value| HeaderValue::from_str(value).ok()) {
        clean.insert(HeaderName::from_static("x-forwarded-host"), host);
    }
    clean
}

fn sanitize_response_headers(headers: &HeaderMap) -> HeaderMap {
    let mut clean = HeaderMap::new();
    for name in [
        header::CONTENT_TYPE,
        header::CACHE_CONTROL,
        header::ETAG,
        header::VARY,
        header::RETRY_AFTER,
    ] {
        if let Some(value) = headers.get(&name) {
            clean.insert(name, value.clone());
        }
    }
    clean
}

fn route_rejection(rejection: RouteRejection) -> Response<Body> {
    match rejection {
        RouteRejection::HostMissing => {
            error_response(StatusCode::BAD_REQUEST, "host_header_required")
        }
        RouteRejection::HostForbidden => {
            error_response(StatusCode::MISDIRECTED_REQUEST, "host_forbidden")
        }
        RouteRejection::QueryForbidden => {
            error_response(StatusCode::BAD_REQUEST, "query_not_supported")
        }
        RouteRejection::NotFound => error_response(StatusCode::NOT_FOUND, "route_not_found"),
        RouteRejection::MethodNotAllowed => {
            error_response(StatusCode::METHOD_NOT_ALLOWED, "method_not_allowed")
        }
    }
}

fn error_response(status: StatusCode, code: &str) -> Response<Body> {
    json_response(
        status,
        json!({"schema":"yilong.commerce-edge.error.v1","error_code":code}),
    )
}

fn json_response(status: StatusCode, value: serde_json::Value) -> Response<Body> {
    let body = serde_json::to_vec(&value).unwrap_or_else(|_| b"{}".to_vec());
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CACHE_CONTROL, "no-store")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(Body::from(body))
        .expect("static commerce edge response")
}

#[cfg(test)]
mod tests {
    use axum::{
        extract::State as AxumState,
        http::HeaderMap as AxumHeaderMap,
        response::IntoResponse,
        routing::{any, get as axum_get, post},
    };
    use tower::ServiceExt;

    use super::*;
    use crate::commerce_edge::{config::EdgeConfig, routes::RouteTable};

    #[derive(Clone)]
    struct SeenRequest {
        headers: HeaderMap,
        body: Vec<u8>,
    }

    #[derive(Clone)]
    struct SeenRequestSlot(Arc<std::sync::Mutex<Option<SeenRequest>>>);

    #[tokio::test]
    async fn proxy_preserves_runtime_signature_and_blocks_admin_paths() {
        let seen = SeenRequestSlot(Arc::new(std::sync::Mutex::new(None)));
        let upstream = Router::new()
            .route("/health", axum_get(|| async { "ok" }))
            .route(
                "/commerce/v1/invoke",
                post(
                    |AxumState(seen): AxumState<SeenRequestSlot>,
                     headers: AxumHeaderMap,
                     body: axum::body::Bytes| async move {
                        *seen.0.lock().unwrap() = Some(SeenRequest {
                            headers,
                            body: body.to_vec(),
                        });
                        (
                            StatusCode::OK,
                            axum::Json(json!({"schema":"merchant_runtime.result.v1"})),
                        )
                            .into_response()
                    },
                ),
            )
            .fallback(any(|| async { StatusCode::IM_A_TEAPOT }))
            .with_state(seen.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream_addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, upstream).await.unwrap() });

        let config = config_for(upstream_addr);
        let registry = Arc::new(RouteRegistry::new(
            RouteTable::from_config(&config).unwrap(),
        ));
        let app = build_router(&config, registry, build_client(&config).unwrap());
        let request = Request::builder()
            .method("POST")
            .uri("/merchants/coffee-a/commerce/v1/invoke")
            .header("host", "commerce.example.com")
            .header("cookie", "must-not-pass=1")
            .header("x-forwarded-for", "203.0.113.9")
            .header("x-yilong-runtime-key-id", "KEY_A")
            .header("x-yilong-runtime-timestamp", "123")
            .header("x-yilong-runtime-signature", "v1=abc")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"hello":"world"}"#))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let seen = seen.0.lock().unwrap();
        let forwarded = seen.as_ref().unwrap();
        assert_eq!(
            forwarded
                .headers
                .get("x-yilong-runtime-signature")
                .unwrap()
                .to_str()
                .unwrap(),
            "v1=abc"
        );
        assert_eq!(forwarded.body, br#"{"hello":"world"}"#);
        assert!(forwarded.headers.get("cookie").is_none());
        assert!(forwarded.headers.get("x-forwarded-for").is_none());
        assert_eq!(forwarded.headers.get("x-forwarded-proto").unwrap(), "https");
        drop(seen);

        let admin = Request::builder()
            .uri("/merchants/coffee-a/api/admin/stores")
            .header("host", "commerce.example.com")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(admin).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let wrong_host = Request::builder()
            .uri("/health")
            .header("host", "other.example.com")
            .body(Body::empty())
            .unwrap();
        let response = build_router(
            &config,
            Arc::new(RouteRegistry::new(
                RouteTable::from_config(&config).unwrap(),
            )),
            build_client(&config).unwrap(),
        )
        .oneshot(wrong_host)
        .await
        .unwrap();
        assert_eq!(response.status(), StatusCode::MISDIRECTED_REQUEST);
    }

    fn config_for(upstream_addr: std::net::SocketAddr) -> EdgeConfig {
        let cert = std::env::temp_dir()
            .join("edge-cert.pem")
            .to_string_lossy()
            .replace('\\', "\\\\");
        let key = std::env::temp_dir()
            .join("edge-key.pem")
            .to_string_lossy()
            .replace('\\', "\\\\");
        let value = format!(
            r#"{{"schema":"yilong.commerce-edge.v1","listen_addr":"127.0.0.1:18443","certificate_chain_path":"{cert}","private_key_path":"{key}","public_hosts":["commerce.example.com"],"routes":[{{"instance_id":"coffee-a","public_base_path":"/merchants/coffee-a","upstream_addr":"{upstream_addr}"}}]}}"#
        );
        EdgeConfig::parse(value.as_bytes()).unwrap()
    }
}

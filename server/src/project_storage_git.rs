use axum::{
    body::Body,
    extract::{Path, Request, State},
    http::{HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use homecli_proto::AgentToServer;
use std::sync::Arc;

use crate::types::AppState;

const DEFAULT_BODY_LIMIT: usize = 128 * 1024 * 1024;

/// Relay Git smart-HTTP requests to a storage PC node through the existing WS tunnel.
///
/// Public clone URLs look like:
/// `/api/storage-git/<node_id>/<token>/projects/<user>/<project>.git`.
/// The token is validated by the storage PC against the target bare repo.
pub async fn storage_git_handler(
    State(state): State<Arc<AppState>>,
    Path((node_id, token, sub_path)): Path<(String, String, String)>,
    req: Request,
) -> Response {
    if !valid_token_shape(&token) {
        return (StatusCode::UNAUTHORIZED, "invalid storage git token").into_response();
    }
    let method = req.method().to_string();
    let query = req
        .uri()
        .query()
        .map(|query| format!("?{query}"))
        .unwrap_or_default();
    let local_path = format!("/storage/git/{token}/{sub_path}{query}");
    let headers = relay_headers(req.headers());
    let body_bytes = match axum::body::to_bytes(req.into_body(), request_body_limit()).await {
        Ok(body) => body,
        Err(err) => return (StatusCode::PAYLOAD_TOO_LARGE, err.to_string()).into_response(),
    };
    let body_b64 = (!body_bytes.is_empty()).then(|| B64.encode(&body_bytes));

    match state
        .agent_manager
        .dispatch_http(&node_id, method, local_path, headers, body_b64)
        .await
    {
        Ok(AgentToServer::HttpResponse {
            status,
            headers,
            body_b64,
            ..
        }) => relay_response(status, headers, body_b64),
        Ok(AgentToServer::HttpError { message, .. }) => (
            StatusCode::BAD_GATEWAY,
            format!("storage PC git relay error: {message}"),
        )
            .into_response(),
        Ok(other) => (
            StatusCode::BAD_GATEWAY,
            format!("unexpected storage git relay response: {other:?}"),
        )
            .into_response(),
        Err(err) => (
            StatusCode::BAD_GATEWAY,
            format!("storage PC node is offline or timed out: {err}"),
        )
            .into_response(),
    }
}

fn relay_headers(headers: &axum::http::HeaderMap) -> Vec<(String, String)> {
    let skip_headers = [
        "host",
        "connection",
        "transfer-encoding",
        "upgrade",
        "keep-alive",
        "proxy-connection",
    ];
    headers
        .iter()
        .filter(|(name, _)| !skip_headers.contains(&name.as_str()))
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.to_string(), value.to_string()))
        })
        .collect()
}

fn relay_response(
    status: u16,
    headers: Vec<(String, String)>,
    body_b64: Option<String>,
) -> Response {
    let status_code = StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY);
    let body = body_b64
        .as_deref()
        .and_then(|body| B64.decode(body).ok())
        .unwrap_or_default();
    let mut builder = Response::builder().status(status_code);
    for (name, value) in headers {
        if hop_by_hop_header(&name) {
            continue;
        }
        if let (Ok(name), Ok(value)) = (name.parse::<HeaderName>(), value.parse::<HeaderValue>()) {
            builder = builder.header(name, value);
        }
    }
    builder
        .body(Body::from(body))
        .unwrap_or_else(|err| (StatusCode::BAD_GATEWAY, err.to_string()).into_response())
}

fn valid_token_shape(token: &str) -> bool {
    token.len() >= 32
        && token
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
}

fn request_body_limit() -> usize {
    std::env::var("ELON_STORAGE_GIT_RELAY_BODY_LIMIT_MB")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .and_then(|mb| mb.checked_mul(1024 * 1024))
        .unwrap_or(DEFAULT_BODY_LIMIT)
}

fn hop_by_hop_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection" | "transfer-encoding" | "upgrade" | "keep-alive" | "proxy-connection"
    )
}

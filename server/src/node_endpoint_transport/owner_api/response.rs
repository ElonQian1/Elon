use axum::{
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use crate::store::NodeEndpointOwnerCredentialMutationDelivery;

#[derive(Serialize)]
struct CredentialBindingResponse {
    credential_id: String,
    agent_id: String,
    install_id: String,
    credential_revision: u64,
    credential_digest: String,
    status: String,
}

#[derive(Serialize)]
struct OwnerCredentialMutationResponse {
    credential: CredentialBindingResponse,
    consumption_id: String,
    consumption_digest: String,
    replayed: bool,
    result_is_current: bool,
    secret_visible_once: bool,
    endpoint_secret: Option<String>,
    error_code: Option<&'static str>,
}

pub(super) fn success(
    delivery: NodeEndpointOwnerCredentialMutationDelivery,
    requested_secret: bool,
) -> Response {
    let replay_lost_secret = requested_secret && delivery.replayed();
    let status = if replay_lost_secret {
        StatusCode::CONFLICT
    } else {
        StatusCode::OK
    };
    let binding = delivery.committed();
    let body = OwnerCredentialMutationResponse {
        credential: CredentialBindingResponse {
            credential_id: binding.credential_id().to_string(),
            agent_id: binding.agent_id().to_string(),
            install_id: binding.install_id().to_string(),
            credential_revision: binding.credential_revision(),
            credential_digest: binding.credential_digest().to_string(),
            status: binding.status().to_string(),
        },
        consumption_id: delivery.consumption_id().to_string(),
        consumption_digest: delivery.consumption_digest().to_string(),
        replayed: delivery.replayed(),
        result_is_current: delivery.result_is_current(),
        secret_visible_once: delivery.secret().is_some(),
        endpoint_secret: delivery.secret().map(str::to_string),
        error_code: replay_lost_secret.then_some("NODE_ENDPOINT_SECRET_NOT_REPLAYABLE"),
    };
    (status, sensitive_headers(), Json(body)).into_response()
}

pub(super) fn error(status: StatusCode, code: &'static str) -> Response {
    (
        status,
        sensitive_headers(),
        Json(serde_json::json!({ "error_code": code })),
    )
        .into_response()
}

pub(super) fn rate_limited(retry_after_seconds: u64) -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        sensitive_headers(),
        [(
            axum::http::header::RETRY_AFTER,
            retry_after_seconds.to_string(),
        )],
        Json(serde_json::json!({
            "error_code": "NODE_ENDPOINT_OWNER_CREDENTIAL_RATE_LIMITED"
        })),
    )
        .into_response()
}

fn sensitive_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers
}

//! Authenticated administrator API for V252 renewable sandbox evidence.

use std::sync::Arc;

use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};

use crate::{
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

use super::external_pool_adapter_sandbox_reattestation_service::{
    self as service, RecordSandboxReattestationBody, RevokeSandboxReattestationBody,
    SandboxReattestationChallengeBody, SandboxReattestationServiceError,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/sandbox-reattestations/challenge",
            post(challenge),
        )
        .route(
            "/api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/sandbox-reattestations",
            post(record),
        )
        .route(
            "/api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/sandbox-reattestations/currentness",
            get(currentness),
        )
        .route(
            "/api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/sandbox-reattestations/:reattestation_receipt_id/revoke",
            post(revoke),
        )
}

async fn challenge(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(registry_release_id): Path<String>,
    payload: Result<Json<SandboxReattestationChallengeBody>, JsonRejection>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    let body = match json_body(payload) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match service::challenge_for_admin(&state.store, &registry_release_id, body) {
        Ok(output) => (StatusCode::CREATED, Json(output)).into_response(),
        Err(error) => error_response(error),
    }
}

async fn record(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(registry_release_id): Path<String>,
    payload: Result<Json<RecordSandboxReattestationBody>, JsonRejection>,
) -> Response {
    let actor = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match service::record_for_admin(&state.store, &actor, &registry_release_id, body) {
        Ok(output) => replay_response(output),
        Err(error) => error_response(error),
    }
}

async fn currentness(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(registry_release_id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    match service::currentness_for_admin(&state.store, &registry_release_id) {
        Ok(output) => Json(output).into_response(),
        Err(error) => error_response(error),
    }
}

async fn revoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((registry_release_id, reattestation_receipt_id)): Path<(String, String)>,
    payload: Result<Json<RevokeSandboxReattestationBody>, JsonRejection>,
) -> Response {
    let actor = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match service::revoke_for_admin(
        &state.store,
        &actor,
        &registry_release_id,
        &reattestation_receipt_id,
        body,
    ) {
        Ok(output) => replay_response(output),
        Err(error) => error_response(error),
    }
}

fn replay_response(output: serde_json::Value) -> Response {
    let status = if output
        .get("replayed")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };
    (status, Json(output)).into_response()
}

fn json_body<T>(payload: Result<Json<T>, JsonRejection>) -> Result<T, Response> {
    payload
        .map(|Json(value)| value)
        .map_err(|error| json_error(StatusCode::UNPROCESSABLE_ENTITY, error))
}

fn error_response(error: SandboxReattestationServiceError) -> Response {
    let status = match error {
        SandboxReattestationServiceError::NotFound => StatusCode::NOT_FOUND,
        SandboxReattestationServiceError::Invalid(_) => StatusCode::BAD_REQUEST,
        SandboxReattestationServiceError::Conflict(_) => StatusCode::CONFLICT,
    };
    json_error(status, error)
}

fn platform_admin(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "only platform administrators can manage Adapter sandbox re-attestations",
        ));
    }
    Ok(user.id)
}

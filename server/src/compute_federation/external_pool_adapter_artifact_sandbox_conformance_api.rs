//! Authenticated administrator API for signed dynamic sandbox conformance evidence.

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

use super::external_pool_adapter_artifact_sandbox_conformance_service::{
    self as service, RecordSandboxConformanceBody, SandboxConformanceChallengeBody,
    SandboxConformanceServiceError,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/admin/compute/external-pool-adapter-release-admissions/:admission_id/sandbox-conformance/challenge",
            post(challenge),
        )
        .route(
            "/api/admin/compute/external-pool-adapter-release-admissions/:admission_id/sandbox-conformance",
            get(currentness).post(record),
        )
}

async fn challenge(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(admission_id): Path<String>,
    payload: Result<Json<SandboxConformanceChallengeBody>, JsonRejection>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    let body = match json_body(payload) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match service::challenge_for_admin(&state.store, &admission_id, body) {
        Ok(output) => Json(output).into_response(),
        Err(error) => error_response(error),
    }
}

async fn record(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(admission_id): Path<String>,
    payload: Result<Json<RecordSandboxConformanceBody>, JsonRejection>,
) -> Response {
    let actor = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match service::record_for_admin(&state.store, &actor, &admission_id, body) {
        Ok(output) => {
            let status = if output.replayed {
                StatusCode::OK
            } else {
                StatusCode::CREATED
            };
            (status, Json(output)).into_response()
        }
        Err(error) => error_response(error),
    }
}

async fn currentness(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(admission_id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    match service::currentness_for_admin(&state.store, &admission_id) {
        Ok(output) => Json(output).into_response(),
        Err(error) => error_response(error),
    }
}

fn json_body<T>(payload: Result<Json<T>, JsonRejection>) -> Result<T, Response> {
    payload
        .map(|Json(value)| value)
        .map_err(|error| json_error(StatusCode::BAD_REQUEST, error))
}

fn error_response(error: SandboxConformanceServiceError) -> Response {
    let status = match error {
        SandboxConformanceServiceError::NotFound => StatusCode::NOT_FOUND,
        SandboxConformanceServiceError::Invalid(_) => StatusCode::BAD_REQUEST,
        SandboxConformanceServiceError::Conflict(_) => StatusCode::CONFLICT,
    };
    json_error(status, error)
}

fn platform_admin(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "只有平台管理员可以验证 external-pool Adapter 沙箱符合性",
        ));
    }
    Ok(user.id)
}

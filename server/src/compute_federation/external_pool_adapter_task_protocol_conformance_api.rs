//! Platform-administrator HTTP surface for V272 task-protocol conformance.

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

use super::{
    external_pool_adapter_task_protocol_conformance_service::{
        self as service, TaskProtocolConformanceServiceError,
    },
    external_pool_adapter_task_protocol_conformance_service_validation::{
        CreateTaskProtocolConformanceRunBody, RevokeTaskProtocolConformanceRunBody,
    },
};

const RUNS_PATH: &str = "/api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/task-protocol-conformance-runs";
const CURRENTNESS_PATH: &str = "/api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/task-protocol-conformance-runs/currentness";
const REVOCATION_PATH: &str = "/api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/task-protocol-conformance-runs/:run_receipt_id/revoke";

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(RUNS_PATH, post(create))
        .route(CURRENTNESS_PATH, get(currentness))
        .route(REVOCATION_PATH, post(revoke))
}

async fn create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(registry_release_id): Path<String>,
    payload: Result<Json<CreateTaskProtocolConformanceRunBody>, JsonRejection>,
) -> Response {
    let actor = match platform_admin(&state, &headers) {
        Ok(actor) => actor,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(body) => body,
        Err(response) => return response,
    };
    write_response(service::create(state, actor, &registry_release_id, body).await)
}

async fn currentness(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(registry_release_id): Path<String>,
) -> Response {
    let actor = match platform_admin(&state, &headers) {
        Ok(actor) => actor,
        Err(response) => return response,
    };
    read_response(service::currentness(&state, &actor, &registry_release_id))
}

async fn revoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((registry_release_id, run_receipt_id)): Path<(String, String)>,
    payload: Result<Json<RevokeTaskProtocolConformanceRunBody>, JsonRejection>,
) -> Response {
    let actor = match platform_admin(&state, &headers) {
        Ok(actor) => actor,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(body) => body,
        Err(response) => return response,
    };
    write_response(service::revoke(
        &state,
        &actor,
        &registry_release_id,
        &run_receipt_id,
        body,
    ))
}

fn platform_admin(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    if user.id == "local-owner" || !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "only durable platform administrators can manage task-protocol conformance runs",
        ));
    }
    Ok(user.id)
}

fn json_body<T>(payload: Result<Json<T>, JsonRejection>) -> Result<T, Response> {
    payload
        .map(|Json(value)| value)
        .map_err(|error| json_error(StatusCode::UNPROCESSABLE_ENTITY, error))
}

fn write_response(
    result: Result<serde_json::Value, TaskProtocolConformanceServiceError>,
) -> Response {
    match result {
        Ok(output) => {
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
        Err(error) => error_response(error),
    }
}

fn read_response(
    result: Result<serde_json::Value, TaskProtocolConformanceServiceError>,
) -> Response {
    match result {
        Ok(output) => Json(output).into_response(),
        Err(error) => error_response(error),
    }
}

fn error_response(error: TaskProtocolConformanceServiceError) -> Response {
    let status = match &error {
        TaskProtocolConformanceServiceError::NotFound => StatusCode::NOT_FOUND,
        TaskProtocolConformanceServiceError::Invalid(_) => StatusCode::BAD_REQUEST,
        TaskProtocolConformanceServiceError::Conflict(_) => StatusCode::CONFLICT,
        TaskProtocolConformanceServiceError::Unavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        TaskProtocolConformanceServiceError::Task(_)
        | TaskProtocolConformanceServiceError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    json_error(status, error)
}

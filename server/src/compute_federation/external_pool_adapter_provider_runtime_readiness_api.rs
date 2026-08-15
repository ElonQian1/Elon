//! Exact owner/admin HTTP surface for V270 Provider runtime-readiness receipts.

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
    external_pool_adapter_provider_runtime_readiness::{
        CreateProviderRuntimeReadinessReceiptBody, RevokeProviderRuntimeReadinessReceiptBody,
    },
    external_pool_adapter_provider_runtime_readiness_service::{
        self as service, ProviderRuntimeReadinessActor, ProviderRuntimeReadinessServiceError,
    },
};

const OWNER_BINDINGS: &str = "/api/me/compute/external-pool-provider-bindings";
const ADMIN_BINDINGS: &str = "/api/admin/compute/external-pool-provider-bindings";
const READINESS_ROOT: &str = "/:provider_binding_id/activation-candidates/:candidate_id/runtime-launch-profiles/:profile_id/upstream-transport-targets/:target_id/supervisor-session-policy-companions/:companion_id/provider-runtime-readiness-receipts";

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            &format!("{ADMIN_BINDINGS}{READINESS_ROOT}"),
            post(admin_create),
        )
        .route(
            &format!("{ADMIN_BINDINGS}{READINESS_ROOT}/:readiness_receipt_id/currentness"),
            get(admin_currentness),
        )
        .route(
            &format!("{ADMIN_BINDINGS}{READINESS_ROOT}/:readiness_receipt_id/revocation"),
            post(admin_revoke),
        )
        .route(
            &format!("{OWNER_BINDINGS}{READINESS_ROOT}/:readiness_receipt_id/currentness"),
            get(owner_currentness),
        )
        .route(
            &format!("{OWNER_BINDINGS}{READINESS_ROOT}/:readiness_receipt_id/revocation"),
            post(owner_revoke),
        )
}

type ReadinessRootPath = (String, String, String, String, String);
type ReadinessReceiptPath = (String, String, String, String, String, String);

async fn admin_create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((binding, candidate, profile, target, companion)): Path<ReadinessRootPath>,
    payload: Result<Json<CreateProviderRuntimeReadinessReceiptBody>, JsonRejection>,
) -> Response {
    let actor = match admin_actor(&state, &headers) {
        Ok(actor) => actor,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(body) => body,
        Err(response) => return response,
    };
    trigger_response(
        service::create(
            state.clone(),
            actor,
            [&binding, &candidate, &profile, &target, &companion],
            body,
        )
        .await,
    )
}

async fn admin_currentness(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((binding, candidate, profile, target, companion, receipt)): Path<ReadinessReceiptPath>,
) -> Response {
    dispatch_currentness(
        &state,
        admin_actor(&state, &headers),
        [&binding, &candidate, &profile, &target, &companion],
        &receipt,
    )
}

async fn admin_revoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((binding, candidate, profile, target, companion, receipt)): Path<ReadinessReceiptPath>,
    payload: Result<Json<RevokeProviderRuntimeReadinessReceiptBody>, JsonRejection>,
) -> Response {
    dispatch_revoke(
        &state,
        admin_actor(&state, &headers),
        [&binding, &candidate, &profile, &target, &companion],
        &receipt,
        payload,
    )
}

async fn owner_currentness(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((binding, candidate, profile, target, companion, receipt)): Path<ReadinessReceiptPath>,
) -> Response {
    dispatch_currentness(
        &state,
        owner_actor(&state, &headers),
        [&binding, &candidate, &profile, &target, &companion],
        &receipt,
    )
}

async fn owner_revoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((binding, candidate, profile, target, companion, receipt)): Path<ReadinessReceiptPath>,
    payload: Result<Json<RevokeProviderRuntimeReadinessReceiptBody>, JsonRejection>,
) -> Response {
    dispatch_revoke(
        &state,
        owner_actor(&state, &headers),
        [&binding, &candidate, &profile, &target, &companion],
        &receipt,
        payload,
    )
}

fn dispatch_currentness(
    state: &AppState,
    actor: Result<ProviderRuntimeReadinessActor, Response>,
    path: [&str; 5],
    readiness_receipt_id: &str,
) -> Response {
    match actor {
        Ok(actor) => read_response(service::currentness(
            state,
            actor,
            path,
            readiness_receipt_id,
        )),
        Err(response) => response,
    }
}

fn dispatch_revoke(
    state: &AppState,
    actor: Result<ProviderRuntimeReadinessActor, Response>,
    path: [&str; 5],
    readiness_receipt_id: &str,
    payload: Result<Json<RevokeProviderRuntimeReadinessReceiptBody>, JsonRejection>,
) -> Response {
    let actor = match actor {
        Ok(actor) => actor,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(body) => body,
        Err(response) => return response,
    };
    write_response(service::revoke(
        state,
        actor,
        path,
        readiness_receipt_id,
        body,
    ))
}

fn owner_actor(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<ProviderRuntimeReadinessActor, Response> {
    auth_from_headers(state, headers)
        .map(|user| ProviderRuntimeReadinessActor::ProviderOwner(user.id))
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))
}

fn admin_actor(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<ProviderRuntimeReadinessActor, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "only platform administrators can trigger Provider runtime readiness",
        ));
    }
    Ok(ProviderRuntimeReadinessActor::PlatformAdmin(user.id))
}

fn json_body<T>(payload: Result<Json<T>, JsonRejection>) -> Result<T, Response> {
    payload
        .map(|Json(value)| value)
        .map_err(|error| json_error(StatusCode::UNPROCESSABLE_ENTITY, error))
}

fn trigger_response(
    result: Result<serde_json::Value, ProviderRuntimeReadinessServiceError>,
) -> Response {
    write_response_with_availability(result, true)
}

fn write_response(
    result: Result<serde_json::Value, ProviderRuntimeReadinessServiceError>,
) -> Response {
    write_response_with_availability(result, false)
}

fn write_response_with_availability(
    result: Result<serde_json::Value, ProviderRuntimeReadinessServiceError>,
    trigger: bool,
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
        Err(error) => error_response(error, trigger),
    }
}

fn read_response(
    result: Result<serde_json::Value, ProviderRuntimeReadinessServiceError>,
) -> Response {
    match result {
        Ok(output) => Json(output).into_response(),
        Err(error) => error_response(error, false),
    }
}

fn error_response(error: ProviderRuntimeReadinessServiceError, trigger: bool) -> Response {
    let status = match &error {
        ProviderRuntimeReadinessServiceError::NotFound => StatusCode::NOT_FOUND,
        ProviderRuntimeReadinessServiceError::Forbidden => StatusCode::FORBIDDEN,
        ProviderRuntimeReadinessServiceError::Invalid(_) => StatusCode::BAD_REQUEST,
        ProviderRuntimeReadinessServiceError::Conflict(_) => StatusCode::CONFLICT,
        ProviderRuntimeReadinessServiceError::Unavailable(_) if trigger => {
            StatusCode::SERVICE_UNAVAILABLE
        }
        ProviderRuntimeReadinessServiceError::Unavailable(_)
        | ProviderRuntimeReadinessServiceError::Task(_)
        | ProviderRuntimeReadinessServiceError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    json_error(status, error)
}

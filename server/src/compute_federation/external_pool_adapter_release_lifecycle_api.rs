//! Authenticated administrator API for Adapter release admission currentness.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};

use crate::{
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

use super::external_pool_adapter_release_lifecycle_service::{
    self as service, CreateExternalPoolAdapterReleaseAdmissionTerminalBody,
    ExternalPoolAdapterReleaseAdmissionLifecycleServiceError,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/admin/compute/external-pool-adapter-release-admissions/:admission_id/terminal",
            post(create_terminal),
        )
        .route(
            "/api/admin/compute/external-pool-adapter-release-admissions/:admission_id/currentness",
            get(get_currentness),
        )
}

async fn create_terminal(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(admission_id): Path<String>,
    Json(body): Json<CreateExternalPoolAdapterReleaseAdmissionTerminalBody>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    lifecycle_write_response(service::create_terminal_for_admin(
        &state.store,
        &admin_user_id,
        &admission_id,
        body,
    ))
}

async fn get_currentness(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(admission_id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    lifecycle_read_response(service::currentness_for_admin(&state.store, &admission_id))
}

fn lifecycle_write_response(
    result: Result<
        crate::store::ExternalPoolAdapterReleaseAdmissionTerminalWriteReceipt,
        ExternalPoolAdapterReleaseAdmissionLifecycleServiceError,
    >,
) -> Response {
    match result {
        Ok(receipt) => {
            let status = if receipt.replayed {
                StatusCode::OK
            } else {
                StatusCode::CREATED
            };
            (status, Json(receipt)).into_response()
        }
        Err(error) => lifecycle_error_response(error),
    }
}

fn lifecycle_read_response(
    result: Result<
        crate::store::ExternalPoolAdapterReleaseAdmissionCurrentnessReceipt,
        ExternalPoolAdapterReleaseAdmissionLifecycleServiceError,
    >,
) -> Response {
    match result {
        Ok(receipt) => Json(receipt).into_response(),
        Err(error) => lifecycle_error_response(error),
    }
}

fn lifecycle_error_response(
    error: ExternalPoolAdapterReleaseAdmissionLifecycleServiceError,
) -> Response {
    let status = match &error {
        ExternalPoolAdapterReleaseAdmissionLifecycleServiceError::NotFound => StatusCode::NOT_FOUND,
        ExternalPoolAdapterReleaseAdmissionLifecycleServiceError::Invalid(_) => {
            StatusCode::BAD_REQUEST
        }
        ExternalPoolAdapterReleaseAdmissionLifecycleServiceError::Conflict(_) => {
            StatusCode::CONFLICT
        }
    };
    json_error(status, error)
}

fn platform_admin(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "只有平台管理员可以管理 external-pool Adapter release admission 生命周期",
        ));
    }
    Ok(user.id)
}

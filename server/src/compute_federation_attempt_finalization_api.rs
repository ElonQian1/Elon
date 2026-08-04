use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;

use crate::{
    compute_federation_attempt_finalization_service::{self, FinalizeComputeAttemptBody},
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/admin/compute/attempt-leases/:lease_id/trusted-finalization",
            post(finalize_attempt),
        )
        .route(
            "/api/me/compute/attempt-leases/:lease_id/trusted-finalization",
            get(get_finalization),
        )
}

async fn finalize_attempt(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(lease_id): Path<String>,
    Json(body): Json<FinalizeComputeAttemptBody>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    finalization_response(
        compute_federation_attempt_finalization_service::finalize_for_platform_admin(
            &state.store,
            &admin_user_id,
            &lease_id,
            body,
        ),
    )
}

async fn get_finalization(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(lease_id): Path<String>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    finalization_response(
        compute_federation_attempt_finalization_service::get_for_attempt_participant(
            &state.store,
            &user_id,
            &lease_id,
        ),
    )
}

fn authenticated_user(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    auth_from_headers(state, headers)
        .map(|user| user.id)
        .map_err(|_| json_error(StatusCode::UNAUTHORIZED, "未登录"))
}

fn platform_admin(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "只有平台管理员可以应用算力 Attempt 可信终态",
        ));
    }
    Ok(user.id)
}

fn finalization_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":format!("{error:#}")})),
        )
            .into_response(),
    }
}

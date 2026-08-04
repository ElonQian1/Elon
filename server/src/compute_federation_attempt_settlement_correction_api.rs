use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;

use crate::{
    compute_federation_attempt_settlement_correction_service::{
        self, CorrectComputeAttemptSettlementBody,
    },
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/admin/compute/attempt-leases/:lease_id/settlement-correction",
            get(get_admin_correction).post(correct_settlement),
        )
        .route(
            "/api/me/compute/attempt-leases/:lease_id/settlement-correction",
            get(get_participant_correction),
        )
}

async fn correct_settlement(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(lease_id): Path<String>,
    Json(body): Json<CorrectComputeAttemptSettlementBody>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    correction_response(
        compute_federation_attempt_settlement_correction_service::correct_for_platform_admin(
            &state.store,
            &admin_user_id,
            &lease_id,
            body,
        ),
    )
}

async fn get_participant_correction(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(lease_id): Path<String>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    correction_response(
        compute_federation_attempt_settlement_correction_service::get_for_attempt_participant(
            &state.store,
            &user_id,
            &lease_id,
        ),
    )
}

async fn get_admin_correction(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(lease_id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    correction_response(
        compute_federation_attempt_settlement_correction_service::get_for_platform_admin(
            &state.store,
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
            "只有平台管理员可以纠正 accepted 算力结算挑战",
        ));
    }
    Ok(user.id)
}

fn correction_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":format!("{error:#}")})),
        )
            .into_response(),
    }
}

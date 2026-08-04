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
    compute_federation_settlement_withdrawal_terminal_service::{
        self, AdminTerminalizeComputeSettlementWithdrawalBody,
        CancelMyComputeSettlementWithdrawalBody,
    },
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/me/compute/providers/:provider_id/settlement-withdrawals/:withdrawal_id/cancellation",
            axum::routing::post(cancel_request),
        )
        .route(
            "/api/me/compute/providers/:provider_id/settlement-withdrawals/:withdrawal_id/terminal",
            get(get_owner_terminal),
        )
        .route(
            "/api/admin/compute/settlement-withdrawals/:withdrawal_id",
            get(get_admin_request),
        )
        .route(
            "/api/admin/compute/settlement-withdrawals/:withdrawal_id/terminal",
            get(get_admin_terminal).post(admin_terminalize),
        )
}

async fn cancel_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_id, withdrawal_id)): Path<(String, String)>,
    Json(body): Json<CancelMyComputeSettlementWithdrawalBody>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    terminal_response(
        compute_federation_settlement_withdrawal_terminal_service::cancel_for_provider_owner(
            &state.store,
            &user_id,
            &provider_id,
            &withdrawal_id,
            body,
        ),
    )
}

async fn get_owner_terminal(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_id, withdrawal_id)): Path<(String, String)>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    terminal_response(
        compute_federation_settlement_withdrawal_terminal_service::get_for_provider_owner(
            &state.store,
            &user_id,
            &provider_id,
            &withdrawal_id,
        ),
    )
}

async fn get_admin_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(withdrawal_id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    terminal_response(
        compute_federation_settlement_withdrawal_terminal_service::get_request_for_platform_admin(
            &state.store,
            &withdrawal_id,
        ),
    )
}

async fn get_admin_terminal(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(withdrawal_id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    terminal_response(
        compute_federation_settlement_withdrawal_terminal_service::get_terminal_for_platform_admin(
            &state.store,
            &withdrawal_id,
        ),
    )
}

async fn admin_terminalize(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(withdrawal_id): Path<String>,
    Json(body): Json<AdminTerminalizeComputeSettlementWithdrawalBody>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    terminal_response(
        compute_federation_settlement_withdrawal_terminal_service::terminalize_for_platform_admin(
            &state.store,
            &admin_user_id,
            &withdrawal_id,
            body,
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
            "只有平台管理员可以处理算力结算提现终态",
        ));
    }
    Ok(user.id)
}

fn terminal_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":format!("{error:#}")})),
        )
            .into_response(),
    }
}

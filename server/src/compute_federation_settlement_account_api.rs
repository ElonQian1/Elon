use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    compute_federation_settlement_account_service,
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/me/compute/providers/:provider_id/settlement-account",
            get(get_account),
        )
        .route(
            "/api/me/compute/providers/:provider_id/settlement-withdrawal-queue",
            get(list_owner_withdrawal_queue),
        )
        .route(
            "/api/admin/compute/settlement-withdrawals",
            get(list_withdrawal_queue),
        )
        .route(
            "/api/admin/compute/settlement-account",
            get(get_platform_account),
        )
}

#[derive(Debug, Deserialize)]
struct QueueQuery {
    #[serde(default = "default_status")]
    status: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

async fn get_account(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    account_response(
        compute_federation_settlement_account_service::get_for_provider_owner(
            &state.store,
            &user_id,
            &provider_id,
        ),
    )
}

async fn list_owner_withdrawal_queue(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
    Query(query): Query<QueueQuery>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    account_response(
        compute_federation_settlement_account_service::list_withdrawal_queue_for_provider_owner(
            &state.store,
            &user_id,
            &provider_id,
            &query.status,
            query.limit,
        ),
    )
}

async fn list_withdrawal_queue(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<QueueQuery>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    account_response(
        compute_federation_settlement_account_service::list_withdrawal_queue_for_platform_admin(
            &state.store,
            &query.status,
            query.limit,
        ),
    )
}

async fn get_platform_account(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    account_response(
        compute_federation_settlement_account_service::get_for_platform_admin(&state.store),
    )
}

fn authenticated_user(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    auth_from_headers(state, headers)
        .map(|user| user.id)
        .map_err(|_| json_error(StatusCode::UNAUTHORIZED, "未登录"))
}

fn platform_admin(state: &AppState, headers: &HeaderMap) -> Result<(), Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "只有平台管理员可以读取算力结算账户和提现队列",
        ));
    }
    Ok(())
}

fn account_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":format!("{error:#}")})),
        )
            .into_response(),
    }
}

fn default_status() -> String {
    "pending".to_string()
}

fn default_limit() -> usize {
    50
}

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
    compute_federation_settlement_withdrawal_request_service::{
        self, CreateMyComputeSettlementWithdrawalBody,
    },
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/me/compute/providers/:provider_id/settlement-withdrawals",
            get(list_requests).post(create_request),
        )
        .route(
            "/api/me/compute/providers/:provider_id/settlement-withdrawals/:withdrawal_id",
            get(get_request),
        )
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    #[serde(default = "default_limit")]
    limit: usize,
}

async fn create_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
    Json(body): Json<CreateMyComputeSettlementWithdrawalBody>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    withdrawal_response(
        compute_federation_settlement_withdrawal_request_service::create_for_provider_owner(
            &state.store,
            &user_id,
            &provider_id,
            body,
        ),
    )
}

async fn get_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_id, withdrawal_id)): Path<(String, String)>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    withdrawal_response(
        compute_federation_settlement_withdrawal_request_service::get_for_provider_owner(
            &state.store,
            &user_id,
            &provider_id,
            &withdrawal_id,
        ),
    )
}

async fn list_requests(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
    Query(query): Query<ListQuery>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    withdrawal_response(
        compute_federation_settlement_withdrawal_request_service::list_for_provider_owner(
            &state.store,
            &user_id,
            &provider_id,
            query.limit,
        ),
    )
}

fn authenticated_user(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    auth_from_headers(state, headers)
        .map(|user| user.id)
        .map_err(|_| json_error(StatusCode::UNAUTHORIZED, "未登录"))
}

fn withdrawal_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":format!("{error:#}")})),
        )
            .into_response(),
    }
}

fn default_limit() -> usize {
    20
}

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde_json::json;

use crate::{
    compute_federation_capacity_supply_service::{self, AddMyComputeCapacitySupplyRequest},
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new().route(
        "/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/supply",
        post(add_supply),
    )
}

async fn add_supply(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_id, pool_id)): Path<(String, String)>,
    Json(request): Json<AddMyComputeCapacitySupplyRequest>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    supply_response(compute_federation_capacity_supply_service::add_for_user(
        &state.store,
        &user_id,
        &provider_id,
        &pool_id,
        request,
    ))
}

fn authenticated_user(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    auth_from_headers(state, headers)
        .map(|user| user.id)
        .map_err(|_| json_error(StatusCode::UNAUTHORIZED, "未登录"))
}

fn supply_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":format!("{error:#}")})),
        )
            .into_response(),
    }
}

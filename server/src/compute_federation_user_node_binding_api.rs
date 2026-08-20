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
    compute_federation_user_node_binding_service::{self, BindMyUserNodeProviderRequest},
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new().route(
        "/api/me/compute/providers/:provider_id/node-binding",
        get(get_binding).post(bind_provider),
    )
}

async fn bind_provider(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
    Json(request): Json<BindMyUserNodeProviderRequest>,
) -> Response {
    let owner_user_id = match authenticated_user(&state, &headers) {
        Ok(owner_user_id) => owner_user_id,
        Err(response) => return response,
    };
    binding_response(compute_federation_user_node_binding_service::bind_for_user(
        &state.store,
        &owner_user_id,
        &provider_id,
        request,
    ))
}

async fn get_binding(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
) -> Response {
    let owner_user_id = match authenticated_user(&state, &headers) {
        Ok(owner_user_id) => owner_user_id,
        Err(response) => return response,
    };
    binding_response(compute_federation_user_node_binding_service::get_for_user(
        &state.store,
        &owner_user_id,
        &provider_id,
    ))
}

fn authenticated_user(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    auth_from_headers(state, headers)
        .map(|user| user.id)
        .map_err(|_| json_error(StatusCode::UNAUTHORIZED, "未登录"))
}

fn binding_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":format!("{error:#}")})),
        )
            .into_response(),
    }
}

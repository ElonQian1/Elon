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
    compute_federation_provider_service::{self, CreateMyComputeProviderRequest},
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/me/compute/providers",
            get(list_providers).post(create_provider),
        )
        .route("/api/me/compute/providers/:provider_id", get(get_provider))
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    #[serde(default = "default_limit")]
    limit: usize,
}

async fn create_provider(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateMyComputeProviderRequest>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    provider_response(compute_federation_provider_service::create_for_user(
        &state.store,
        &user_id,
        request,
    ))
}

async fn get_provider(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    provider_response(compute_federation_provider_service::get_for_user(
        &state.store,
        &user_id,
        &provider_id,
    ))
}

async fn list_providers(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    provider_response(compute_federation_provider_service::list_for_user(
        &state.store,
        &user_id,
        query.limit,
    ))
}

fn authenticated_user(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    auth_from_headers(state, headers)
        .map(|user| user.id)
        .map_err(|_| json_error(StatusCode::UNAUTHORIZED, "未登录"))
}

fn provider_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
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

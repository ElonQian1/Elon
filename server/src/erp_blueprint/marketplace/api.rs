use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

use super::service::CreateMarketplaceInstanceRequest;

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new().route(
        "/api/store/projects/:project_id/erp-instances",
        post(create_marketplace_instance),
    )
}

async fn create_marketplace_instance(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<CreateMarketplaceInstanceRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    match super::create_instance(&state.store, &project_id, &user.id, request) {
        Ok(result) => Json(result).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}

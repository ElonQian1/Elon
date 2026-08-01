//! HTTP management surface for merchant-controlled developer App blocks.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};

use crate::{
    open_commerce_app_block_model::BlockOpenCommerceAppRequest,
    open_commerce_app_block_service,
    open_commerce_model::normalize_app_id,
    project_auth::{auth_from_headers, json_error, project_access},
    types::AppState,
};

const DEFAULT_HTTP_APP_ID: &str = "pc-web";

struct ProjectCaller {
    user_id: String,
    role: String,
    app_id: String,
}

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/open-commerce/app-blocks",
            get(list_blocks).put(block_app),
        )
        .route(
            "/api/projects/:project_id/open-commerce/app-blocks/:block_id/unblock",
            post(unblock_app),
        )
}

async fn list_blocks(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    if let Err(response) = project_caller(&state, &headers, &project_id) {
        return response;
    }
    service_response(open_commerce_app_block_service::list_blocks(
        &state.store,
        &project_id,
    ))
}

async fn block_app(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<BlockOpenCommerceAppRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_app_block_service::block_app(
        &state.store,
        &project_id,
        &caller.user_id,
        &caller.app_id,
        &caller.role,
        request,
    ))
}

async fn unblock_app(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, block_id)): Path<(String, String)>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_app_block_service::unblock_app(
        &state.store,
        &project_id,
        &block_id,
        &caller.user_id,
        &caller.app_id,
        &caller.role,
    ))
}

fn project_caller(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
) -> Result<ProjectCaller, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    let access = project_access(state, &user.id, project_id)
        .map_err(|error| json_error(StatusCode::FORBIDDEN, error))?;
    let raw_app_id = headers
        .get("x-elon-app-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or(DEFAULT_HTTP_APP_ID);
    let app_id =
        normalize_app_id(raw_app_id).map_err(|error| json_error(StatusCode::BAD_REQUEST, error))?;
    Ok(ProjectCaller {
        user_id: user.id,
        role: access.role,
        app_id,
    })
}

fn service_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => {
            let message = format!("{error:#}");
            let status = if message.contains("权限") {
                StatusCode::FORBIDDEN
            } else if message.contains("不存在") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_REQUEST
            };
            json_error(status, message)
        }
    }
}

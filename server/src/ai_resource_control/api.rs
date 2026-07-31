use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
    Json, Router,
};

use crate::{
    project_auth::{auth_from_headers, can_edit, json_error, project_access},
    types::AppState,
};

use super::{
    model::{AiRoutePreviewRequest, UpdateAiResourcePolicy},
    service,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/ai-resources/overview",
            get(get_overview),
        )
        .route(
            "/api/projects/:project_id/ai-resources/policy",
            patch(update_policy),
        )
        .route(
            "/api/projects/:project_id/ai-resources/preview",
            post(preview_route),
        )
}

async fn get_overview(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let (user_id, _) = match project_caller(&state, &headers, &project_id) {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    service_response(service::overview(&state, &project_id, &user_id).await)
}

async fn update_policy(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<UpdateAiResourcePolicy>,
) -> Response {
    let (user_id, role) = match project_caller(&state, &headers, &project_id) {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    if !can_edit(&role) {
        return json_error(StatusCode::FORBIDDEN, "只有项目编辑者可以修改 AI 资源策略");
    }
    service_response(service::update_policy(
        &state,
        &project_id,
        &user_id,
        request,
    ))
}

async fn preview_route(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<AiRoutePreviewRequest>,
) -> Response {
    let (user_id, _) = match project_caller(&state, &headers, &project_id) {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    service_response(service::preview(&state, &project_id, &user_id, request).await)
}

fn project_caller(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
) -> Result<(String, String), Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    let access = project_access(state, &user.id, project_id)
        .map_err(|error| json_error(StatusCode::FORBIDDEN, error))?;
    Ok((user.id, access.role))
}

fn service_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

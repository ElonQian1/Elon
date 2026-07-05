use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{patch, post},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, can_edit, json_error, project_access},
    types::AppState,
};

#[derive(Deserialize)]
pub struct CreateProjectChannelRequest {
    pub name: String,
    pub category_id: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateProjectChannelRequest {
    pub name: String,
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/channels",
            post(create_project_channel),
        )
        .route(
            "/api/projects/:project_id/channels/:channel_id",
            patch(rename_project_channel).delete(delete_project_channel),
        )
}

pub async fn create_project_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(req): Json<CreateProjectChannelRequest>,
) -> Response {
    let (user_id, project_id) = match editable_project(&state, &headers, &project_id) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    match state.store.create_project_channel(
        &user_id,
        &project_id,
        &req.name,
        req.category_id.as_deref(),
    ) {
        Ok(channel) => Json(serde_json::json!({ "channel": channel })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn rename_project_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, channel_id)): Path<(String, String)>,
    Json(req): Json<UpdateProjectChannelRequest>,
) -> Response {
    let (user_id, project_id) = match editable_project(&state, &headers, &project_id) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    match state
        .store
        .rename_project_channel(&user_id, &project_id, &channel_id, &req.name)
    {
        Ok(channel) => Json(serde_json::json!({ "channel": channel })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn delete_project_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, channel_id)): Path<(String, String)>,
) -> Response {
    let (_user_id, project_id) = match editable_project(&state, &headers, &project_id) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    match state.store.delete_project_channel(&project_id, &channel_id) {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

fn editable_project(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    project_id: &str,
) -> Result<(String, String), Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|e| json_error(StatusCode::UNAUTHORIZED, e.to_string()))?;
    let project = project_access(state, &user.id, project_id)
        .map_err(|e| json_error(StatusCode::FORBIDDEN, e.to_string()))?;
    if !can_edit(&project.role) {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "当前成员角色不能管理频道",
        ));
    }
    Ok((user.id, project.id))
}

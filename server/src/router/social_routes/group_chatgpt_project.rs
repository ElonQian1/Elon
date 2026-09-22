use crate::store::social_ai_messages::group_project::ProjectAction;
use crate::{
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;

pub(super) async fn action(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(group): Path<String>,
    Json(request): Json<ProjectAction>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "请先登录"),
    };
    match state
        .store
        .group_chatgpt_project_action(&user.id, &group, &request)
    {
        Ok(binding) => ([("cache-control", "private, no-store")], Json(binding)).into_response(),
        Err(e) => json_error(StatusCode::CONFLICT, e.to_string()),
    }
}

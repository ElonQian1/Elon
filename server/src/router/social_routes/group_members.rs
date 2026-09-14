use crate::{
    project_auth::{auth_from_headers, json_error},
    store::SOCIAL_AI_USER_ID,
    types::AppState,
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;

pub(super) async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(group_id): Path<String>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    match state
        .store
        .list_friend_group_mention_members(&user.id, &group_id)
    {
        Ok(members) => Json(serde_json::json!({
            "members": members,
            "ai_members": [{ "id": SOCIAL_AI_USER_ID, "display_name": "EL" }],
        }))
        .into_response(),
        Err(error) => json_error(StatusCode::FORBIDDEN, error.to_string()),
    }
}

pub(super) async fn script() -> impl IntoResponse {
    (
        [
            ("content-type", "application/javascript; charset=utf-8"),
            ("cache-control", "no-cache"),
        ],
        include_str!("../../assets/group_mentions.js"),
    )
}

pub(super) async fn styles() -> impl IntoResponse {
    (
        [
            ("content-type", "text/css; charset=utf-8"),
            ("cache-control", "no-cache"),
        ],
        include_str!("../../assets/group_mentions.css"),
    )
}

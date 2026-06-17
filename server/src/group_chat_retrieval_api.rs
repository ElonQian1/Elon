//! Group chat message retrieval endpoints for AI context building.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, json_error},
    store::GroupChatRetrievalInput,
    types::AppState,
};

#[derive(Deserialize)]
pub struct SearchGroupChatMessagesRequest {
    pub query: Option<String>,
    pub sender: Option<String>,
    pub message_ids: Option<Vec<String>>,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub limit: Option<i64>,
}

pub async fn search_group_chat_messages(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(group_id): Path<String>,
    Json(req): Json<SearchGroupChatMessagesRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let input = GroupChatRetrievalInput {
        query: clean_optional(req.query),
        sender: clean_optional(req.sender),
        message_ids: req.message_ids.unwrap_or_default(),
        start_at: clean_optional(req.start_at),
        end_at: clean_optional(req.end_at),
        limit: req.limit.unwrap_or(40),
    };
    match state
        .store
        .search_group_chat_messages(&user.id, &group_id, &input)
    {
        Ok(retrieval) => Json(json!({ "retrieval": retrieval })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

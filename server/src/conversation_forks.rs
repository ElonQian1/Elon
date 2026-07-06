use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    conversation_router::{resolve_system_conversation_route, ConversationEntryKind},
    project_auth::{auth_from_headers, can_edit, json_error, project_access},
    project_events,
    types::AppState,
};

#[derive(Deserialize)]
pub struct ForkConversationRequest {
    #[serde(alias = "messageId")]
    pub message_id: String,
    #[serde(default, alias = "conversationId", alias = "newConversationId")]
    pub new_conversation_id: Option<String>,
    pub title: Option<String>,
}

#[derive(Serialize)]
struct ForkConversationResponse {
    conversation_id: String,
    source_conversation_id: String,
    source_message_id: String,
    title: Option<String>,
    copied_message_count: usize,
}

pub async fn fork_ai_chat_conversation(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(conversation_id): Path<String>,
    Json(req): Json<ForkConversationRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let route = match resolve_system_conversation_route(
        &state.store,
        &user.id,
        ConversationEntryKind::ChatMemory,
    ) {
        Ok(route) => route,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    fork_response(
        &state,
        &route.project_id,
        &user.id,
        &conversation_id,
        req,
    )
}

pub async fn fork_project_member_conversation(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, member_user_id, conversation_id)): Path<(String, String, String)>,
    Json(req): Json<ForkConversationRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match project_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    if member_user_id != user.id {
        return json_error(StatusCode::FORBIDDEN, "只能分叉自己的项目会话");
    }
    if !can_edit(&project.role) {
        return json_error(StatusCode::FORBIDDEN, "当前用户没有修改项目的权限");
    }
    let result = match fork_result(&state, &project_id, &user.id, &conversation_id, req) {
        Ok(result) => result,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, message),
    };
    project_events::publish_message_updated(
        state.as_ref(),
        &project_id,
        None,
        Some(&result.conversation_id),
        None,
        "conversation_fork",
    );
    Json(result).into_response()
}

fn fork_response(
    state: &AppState,
    project_id: &str,
    user_id: &str,
    conversation_id: &str,
    req: ForkConversationRequest,
) -> Response {
    match fork_result(state, project_id, user_id, conversation_id, req) {
        Ok(result) => Json(result).into_response(),
        Err(message) => json_error(StatusCode::BAD_REQUEST, message),
    }
}

fn fork_result(
    state: &AppState,
    project_id: &str,
    user_id: &str,
    conversation_id: &str,
    req: ForkConversationRequest,
) -> Result<ForkConversationResponse, String> {
    state.store.fork_conversation_at_message(
        project_id,
        user_id,
        conversation_id,
        &req.message_id,
        req.new_conversation_id.as_deref(),
        req.title.as_deref(),
    )
    .map(|result| ForkConversationResponse {
        conversation_id: result.conversation_id,
        source_conversation_id: result.source_conversation_id,
        source_message_id: result.source_message_id,
        title: result.title,
        copied_message_count: result.copied_message_count,
    })
    .map_err(|err| err.to_string())
}

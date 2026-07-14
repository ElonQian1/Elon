use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, json_error, project_access},
    project_events,
    types::AppState,
};

use super::{
    ensure_project_member_can_speak, MemberConversationQuery, SendChannelMessageRequest,
    UpdateMemberConversationVisibilityRequest,
};

pub async fn list_member_conversations(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, member_user_id)): Path<(String, String)>,
    Query(query): Query<MemberConversationQuery>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    if let Err(e) = project_access(&state, &user.id, &project_id) {
        return json_error(StatusCode::FORBIDDEN, e.to_string());
    }
    match state.store.list_project_member_conversations(
        &user.id,
        &project_id,
        &member_user_id,
        query.limit.unwrap_or(50),
    ) {
        Ok(conversations) => {
            Json(serde_json::json!({ "conversations": conversations })).into_response()
        }
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn list_member_conversation_messages(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, member_user_id, conversation_id)): Path<(String, String, String)>,
    Query(query): Query<MemberConversationQuery>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    if let Err(e) = project_access(&state, &user.id, &project_id) {
        return json_error(StatusCode::FORBIDDEN, e.to_string());
    }
    match state.store.list_project_member_conversation_messages(
        &user.id,
        &project_id,
        &member_user_id,
        &conversation_id,
        query.limit.unwrap_or(120),
    ) {
        Ok(messages) => Json(serde_json::json!({ "messages": messages })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn send_member_conversation_message(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, member_user_id, conversation_id)): Path<(String, String, String)>,
    Json(req): Json<SendChannelMessageRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    if let Err(e) = project_access(&state, &user.id, &project_id) {
        return json_error(StatusCode::FORBIDDEN, e.to_string());
    }
    if let Err(response) = ensure_project_member_can_speak(&state, &project_id, &user.id) {
        return response;
    }
    match state
        .store
        .insert_project_member_conversation_discussion_message(
            &user.id,
            &project_id,
            &member_user_id,
            &conversation_id,
            &req.content,
        ) {
        Ok(message) => {
            project_events::publish_message_updated(
                state.as_ref(),
                &project_id,
                None,
                Some(&conversation_id),
                None,
                "discussion",
            );
            Json(serde_json::json!({ "message": message })).into_response()
        }
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn update_member_conversation_visibility(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, conversation_id)): Path<(String, String)>,
    Json(req): Json<UpdateMemberConversationVisibilityRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    if let Err(e) = project_access(&state, &user.id, &project_id) {
        return json_error(StatusCode::FORBIDDEN, e.to_string());
    }
    match state.store.update_project_member_conversation_visibility(
        &user.id,
        &project_id,
        &conversation_id,
        req.is_public,
    ) {
        Ok(conversation) => {
            Json(serde_json::json!({ "conversation": conversation })).into_response()
        }
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

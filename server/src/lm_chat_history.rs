use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::{collections::HashMap, sync::Arc};

use crate::{
    conversation_router::{resolve_system_conversation_route, ConversationEntryKind},
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

pub(crate) async fn list_ai_chat_conversations(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let route = match resolve_system_conversation_route(
        &state.store,
        &user.id,
        ConversationEntryKind::ChatMemory,
    ) {
        Ok(route) => route,
        Err(e) => {
            tracing::warn!("确保普通聊天归档项目失败 user={}: {e}", user.id);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "创建聊天归档项目失败");
        }
    };
    let limit = query
        .get("limit")
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(50);
    match state
        .store
        .list_user_conversations(&route.project_id, &user.id, limit)
    {
        Ok(conversations) => Json(json!({
            "conversations": conversations,
            "project_id": route.project_id,
            "project_name": route.project_name,
            "scope": route.entry_key,
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub(crate) async fn list_ai_chat_conversation_messages(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(conversation_id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let route = match resolve_system_conversation_route(
        &state.store,
        &user.id,
        ConversationEntryKind::ChatMemory,
    ) {
        Ok(route) => route,
        Err(e) => {
            tracing::warn!("确保普通聊天归档项目失败 user={}: {e}", user.id);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "创建聊天归档项目失败");
        }
    };
    let limit = query
        .get("limit")
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(120);
    match state.store.list_user_conversation_messages(
        &route.project_id,
        &user.id,
        &conversation_id,
        limit,
    ) {
        Ok(messages) => Json(json!({
            "messages": messages,
            "conversation_id": conversation_id,
            "project_id": route.project_id,
            "scope": route.entry_key,
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::NOT_FOUND, e.to_string()),
    }
}

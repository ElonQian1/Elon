use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::{collections::HashMap, sync::Arc};

use crate::{
    project_auth::{auth_from_headers, json_error},
    project_docs_channel,
    store::{ProjectAccess, CHANNEL_PERMISSION_SEND, CHANNEL_PERMISSION_VIEW},
    types::AppState,
};

use super::{
    ensure_project_member_can_speak, ensure_user_project_for_space, project_member_can_use_channel,
    project_space_access, publish_channel_message_updated, query_limit, ChannelMessagesQuery,
    SendChannelMessageRequest, DOCS_CHANNEL_KIND,
};

pub async fn list_user_project_channel_messages(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((user_id, project_id, channel_id)): Path<(String, String, String)>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let (user, project) = match ensure_user_project_for_space(
        &state,
        &headers,
        &user_id,
        &project_id,
        query.get("title").map(String::as_str),
    ) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    list_channel_messages_response(
        state,
        user.id,
        project,
        channel_id,
        query_limit(&query, 120),
    )
    .await
}

pub async fn list_channel_messages(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, channel_id)): Path<(String, String)>,
    Query(query): Query<ChannelMessagesQuery>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match project_space_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    list_channel_messages_response(
        state,
        user.id,
        project,
        channel_id,
        query.limit.unwrap_or(120),
    )
    .await
}

async fn list_channel_messages_response(
    state: Arc<AppState>,
    user_id: String,
    project: ProjectAccess,
    channel_id: String,
    limit: i64,
) -> Response {
    let channel_kind = match state
        .store
        .get_project_channel_kind(&project.id, &channel_id)
    {
        Ok(kind) => kind,
        Err(e) => return json_error(StatusCode::NOT_FOUND, e.to_string()),
    };
    if !project_member_can_use_channel(
        &state,
        &project.id,
        &channel_id,
        &user_id,
        CHANNEL_PERMISSION_VIEW,
    ) {
        return json_error(StatusCode::FORBIDDEN, "当前角色无权查看该频道");
    }
    if channel_kind == DOCS_CHANNEL_KIND {
        let messages =
            project_docs_channel::load_project_doc_messages(state, &user_id, &project, &channel_id)
                .await;
        return Json(serde_json::json!({ "messages": messages })).into_response();
    }
    match state
        .store
        .list_project_channel_messages(&user_id, &project.id, &channel_id, limit)
    {
        Ok(messages) => Json(serde_json::json!({ "messages": messages })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn send_user_project_channel_message(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((user_id, project_id, channel_id)): Path<(String, String, String)>,
    Query(query): Query<HashMap<String, String>>,
    Json(req): Json<SendChannelMessageRequest>,
) -> Response {
    let (user, project) = match ensure_user_project_for_space(
        &state,
        &headers,
        &user_id,
        &project_id,
        query.get("title").map(String::as_str),
    ) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    send_channel_message_response(state, user.id, project, channel_id, req)
}

pub async fn send_channel_message(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, channel_id)): Path<(String, String)>,
    Json(req): Json<SendChannelMessageRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match project_space_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    send_channel_message_response(state, user.id, project, channel_id, req)
}

pub async fn recall_user_project_channel_message(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((user_id, project_id, channel_id, message_id)): Path<(String, String, String, String)>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let (user, project) = match ensure_user_project_for_space(
        &state,
        &headers,
        &user_id,
        &project_id,
        query.get("title").map(String::as_str),
    ) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    recall_channel_message_response(state, user.id, project, channel_id, message_id)
}

pub async fn recall_channel_message(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, channel_id, message_id)): Path<(String, String, String)>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match project_space_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    recall_channel_message_response(state, user.id, project, channel_id, message_id)
}

fn recall_channel_message_response(
    state: Arc<AppState>,
    user_id: String,
    project: ProjectAccess,
    channel_id: String,
    message_id: String,
) -> Response {
    let channel_kind = match state
        .store
        .get_project_channel_kind(&project.id, &channel_id)
    {
        Ok(kind) => kind,
        Err(e) => return json_error(StatusCode::NOT_FOUND, e.to_string()),
    };
    if !project_member_can_use_channel(
        &state,
        &project.id,
        &channel_id,
        &user_id,
        CHANNEL_PERMISSION_VIEW,
    ) {
        return json_error(StatusCode::FORBIDDEN, "当前角色无权查看该频道");
    }
    if channel_kind == DOCS_CHANNEL_KIND {
        return json_error(StatusCode::BAD_REQUEST, "文档频道消息不能撤回");
    }
    match state.store.recall_project_channel_message(
        &user_id,
        &project.id,
        &channel_id,
        &message_id,
    ) {
        Ok(()) => {
            publish_channel_message_updated(
                state.as_ref(),
                &project.id,
                &channel_id,
                None,
                None,
                "recall",
            );
            Json(serde_json::json!({ "ok": true })).into_response()
        }
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

fn send_channel_message_response(
    state: Arc<AppState>,
    user_id: String,
    project: ProjectAccess,
    channel_id: String,
    req: SendChannelMessageRequest,
) -> Response {
    let channel_kind = match state
        .store
        .get_project_channel_kind(&project.id, &channel_id)
    {
        Ok(kind) => kind,
        Err(e) => return json_error(StatusCode::NOT_FOUND, e.to_string()),
    };
    if !project_member_can_use_channel(
        &state,
        &project.id,
        &channel_id,
        &user_id,
        CHANNEL_PERMISSION_VIEW,
    ) {
        return json_error(StatusCode::FORBIDDEN, "当前角色无权查看该频道");
    }
    if channel_kind == DOCS_CHANNEL_KIND {
        return json_error(StatusCode::BAD_REQUEST, "文档频道是固定只读频道，不能发帖");
    }
    if !project_member_can_use_channel(
        &state,
        &project.id,
        &channel_id,
        &user_id,
        CHANNEL_PERMISSION_SEND,
    ) {
        return json_error(StatusCode::FORBIDDEN, "当前角色无权在该频道发言");
    }
    if let Err(response) = ensure_project_member_can_speak(&state, &project.id, &user_id) {
        return response;
    }
    let message_kind = if channel_kind == "suggestions" {
        "suggestion"
    } else {
        "text"
    };
    match state.store.insert_project_channel_message(
        &project.id,
        &channel_id,
        Some(&user_id),
        message_kind,
        &req.content,
        None,
        req.reply_to_message_id.as_deref(),
    ) {
        Ok(message) => {
            publish_channel_message_updated(
                state.as_ref(),
                &project.id,
                &channel_id,
                None,
                None,
                message_kind,
            );
            Json(serde_json::json!({ "message": message })).into_response()
        }
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

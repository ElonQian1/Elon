//! 好友关系 & 点对点消息 API
//!
//! 路由（在 router.rs 注册）：
//!   GET  /api/me/friends                         → 好友列表
//!   POST /api/me/friends                         → 通过手机号添加好友
//!   GET  /api/me/friends/search                  → 按手机号搜索用户
//!   GET  /api/me/friends/:friend_id/messages     → 获取与好友的消息记录
//!   POST /api/me/friends/:friend_id/messages     → 发送消息给好友

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, json_error},
    project_ws_protocol::ProjectAttachmentRef,
    types::AppState,
};

#[derive(Deserialize)]
pub struct FriendSearchQuery {
    pub phone: Option<String>,
    pub query: Option<String>,
    pub search_type: Option<String>,
}

#[derive(Deserialize)]
pub struct AddFriendRequest {
    pub phone: Option<String>,
    pub query: Option<String>,
    pub search_type: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateFriendGroupRequest {
    pub name: Option<String>,
    pub member_ids: Vec<String>,
}

#[derive(Deserialize)]
pub struct FriendMessagesQuery {
    pub after: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct SendFriendMessageRequest {
    pub content: String,
    pub attachments: Option<Vec<ProjectAttachmentRef>>,
}

#[derive(Deserialize)]
pub struct FriendGroupMessagesQuery {
    pub after: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct SendFriendGroupMessageRequest {
    pub content: String,
    pub attachments: Option<Vec<ProjectAttachmentRef>>,
}

pub async fn list_friends(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    match state.store.list_friends(&user.id) {
        Ok(friends) => Json(serde_json::json!({ "friends": friends })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn list_friend_groups(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    match state.store.list_friend_groups(&user.id) {
        Ok(groups) => Json(serde_json::json!({ "groups": groups })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn create_friend_group(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<CreateFriendGroupRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    match state
        .store
        .create_friend_group(&user.id, req.name.as_deref(), &req.member_ids)
    {
        Ok(group) => Json(serde_json::json!({ "group": group })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn search_friend_by_phone(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<FriendSearchQuery>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let search_text = friend_search_text(query.phone.as_deref(), query.query.as_deref());
    match state
        .store
        .search_friend(&user.id, query.search_type.as_deref(), &search_text)
    {
        Ok(Some(result)) => Json(serde_json::json!({
            "found": true,
            "user": result.user,
            "already_friend": result.already_friend,
            "is_self": result.is_self,
        }))
        .into_response(),
        Ok(None) => Json(serde_json::json!({ "found": false })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn add_friend_by_phone(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<AddFriendRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let search_text = friend_search_text(req.phone.as_deref(), req.query.as_deref());
    match state
        .store
        .add_friend(&user.id, req.search_type.as_deref(), &search_text)
    {
        Ok(result) => Json(serde_json::json!({
            "friend": result.friend,
            "already_friend": result.already_friend,
        }))
        .into_response(),
        Err(e) => {
            let message = e.to_string();
            let status = if message.contains("未找到") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_REQUEST
            };
            json_error(status, message)
        }
    }
}

pub async fn list_friend_messages(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(friend_id): Path<String>,
    Query(query): Query<FriendMessagesQuery>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    match state.store.list_friend_messages(
        &user.id,
        &friend_id,
        query.after.as_deref(),
        query.limit.unwrap_or(80),
    ) {
        Ok(messages) => Json(serde_json::json!({ "messages": messages })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn send_friend_message(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(friend_id): Path<String>,
    Json(req): Json<SendFriendMessageRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    match state.store.send_friend_message(
        &user.id,
        &friend_id,
        &req.content,
        req.attachments.as_deref(),
    ) {
        Ok(message) => {
            crate::friend_events::publish_friend_message(&message);
            crate::social_ai::spawn_friend_reply(
                state.clone(),
                user.id.clone(),
                friend_id.clone(),
                req.content.clone(),
            );
            Json(serde_json::json!({ "message": message })).into_response()
        }
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn delete_friend_message(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((friend_id, message_id)): Path<(String, String)>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    match state
        .store
        .delete_friend_message(&user.id, &friend_id, &message_id)
    {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn list_friend_group_messages(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(group_id): Path<String>,
    Query(query): Query<FriendGroupMessagesQuery>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    match state.store.list_friend_group_messages(
        &user.id,
        &group_id,
        query.after.as_deref(),
        query.limit.unwrap_or(120),
    ) {
        Ok(messages) => Json(serde_json::json!({ "messages": messages })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn send_friend_group_message(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(group_id): Path<String>,
    Json(req): Json<SendFriendGroupMessageRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    match state.store.send_friend_group_message(
        &user.id,
        &group_id,
        &req.content,
        req.attachments.as_deref(),
    ) {
        Ok(message) => {
            if let Ok(recipient_user_ids) = state.store.friend_group_member_ids(&user.id, &group_id)
            {
                crate::friend_events::publish_group_message(&message, recipient_user_ids);
            }
            crate::social_ai::spawn_group_reply(
                state.clone(),
                user.id.clone(),
                group_id.clone(),
                req.content.clone(),
            );
            Json(serde_json::json!({ "message": message })).into_response()
        }
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn delete_friend_group_message(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((group_id, message_id)): Path<(String, String)>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    match state
        .store
        .delete_friend_group_message(&user.id, &group_id, &message_id)
    {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

fn friend_search_text(phone: Option<&str>, query: Option<&str>) -> String {
    query
        .or(phone)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_default()
        .to_string()
}

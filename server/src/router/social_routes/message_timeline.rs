use crate::{
    project_auth::{auth_from_headers, json_error},
    store::friend_messages::timeline::TimelineRequest,
    types::AppState,
};
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/me/message-timeline", get(read))
        .route("/api/me/message-timeline/read", post(mark_read))
        .route("/api/me/message-timeline/window", post(recover_window))
}

async fn resolve(
    state: &AppState,
    owner: &str,
    mut request: TimelineRequest,
) -> anyhow::Result<TimelineRequest> {
    if request.kind == "ai" && request.project.is_empty() {
        request.project = crate::conversation_router::resolve_system_conversation_route(
            &state.store,
            owner,
            crate::conversation_router::ConversationEntryKind::ChatMemory,
        )?
        .project_id;
    }
    Ok(request)
}

async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(request): Query<TimelineRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "请重新登录"),
    };
    let request = match resolve(&state, &user.id, request).await {
        Ok(r) => r,
        Err(_) => return json_error(StatusCode::NOT_FOUND, "会话不可用"),
    };
    match state.store.read_message_timeline(&user.id, &request) {
        Ok(page) => ([("cache-control", "no-store")], Json(page)).into_response(),
        Err(e) => failure(e),
    }
}

#[derive(Deserialize)]
struct ReadReceipt {
    #[serde(flatten)]
    request: TimelineRequest,
    message_id: String,
}

#[derive(Deserialize)]
struct WindowRequest {
    #[serde(flatten)]
    request: TimelineRequest,
    message_ids: Vec<String>,
}

async fn recover_window(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(window): Json<WindowRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "请重新登录"),
    };
    let request = match resolve(&state, &user.id, window.request).await {
        Ok(r) => r,
        Err(_) => return json_error(StatusCode::NOT_FOUND, "会话不可用"),
    };
    match state
        .store
        .recover_message_timeline_window(&user.id, &request, &window.message_ids)
    {
        Ok(page) => ([("cache-control", "no-store")], Json(page)).into_response(),
        Err(e) => failure(e),
    }
}

async fn mark_read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(receipt): Json<ReadReceipt>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "请重新登录"),
    };
    let request = match resolve(&state, &user.id, receipt.request).await {
        Ok(r) => r,
        Err(_) => return json_error(StatusCode::NOT_FOUND, "会话不可用"),
    };
    match state
        .store
        .mark_timeline_read(&user.id, &request, &receipt.message_id)
    {
        Ok(()) => Json(serde_json::json!({"ok":true})).into_response(),
        Err(e) => failure(e),
    }
}

fn failure(error: anyhow::Error) -> Response {
    let text = error.to_string();
    if text == "timeline_access_denied" {
        json_error(StatusCode::FORBIDDEN, "无法访问此会话")
    } else if text.contains("cursor")
        || text.starts_with("invalid_")
        || text == "choose_history_or_sync"
    {
        json_error(StatusCode::BAD_REQUEST, "消息位置已失效，请重新打开会话")
    } else {
        tracing::warn!("message timeline read failed: {error}");
        json_error(StatusCode::INTERNAL_SERVER_ERROR, "消息同步失败，请重试")
    }
}

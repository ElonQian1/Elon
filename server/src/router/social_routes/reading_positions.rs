use crate::{
    project_auth::{auth_from_headers, json_error},
    store::friend_messages::timeline::{
        reading::{Command, Scope},
        v2,
    },
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
        .route("/api/me/reading-bookmarks", get(list).post(write))
        .route("/api/me/message-timeline/v2", get(timeline))
        .route("/api/me/message-timeline/v2/window", post(window))
        .route("/api/me/reading-capabilities", get(capabilities))
}
async fn capabilities(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if auth_from_headers(&state, &headers).is_err() {
        return json_error(StatusCode::UNAUTHORIZED, "请重新登录");
    }
    Json(serde_json::json!({"schema":"elon.reading_positions.v1","timeline_around":true,"timeline_after":true,"reading_bookmarks":true,"sources":["friend","group","ai","channel"],"page_size":50,"max_page_size":100})).into_response()
}
#[derive(Deserialize)]
struct List {
    #[serde(flatten)]
    scope: Scope,
    #[serde(default)]
    after: String,
}
async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(r): Query<List>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(v) => v,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "请重新登录"),
    };
    respond(state.store.reading_list(&user.id, &r.scope, &r.after))
}
async fn write(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(c): Json<Command>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(v) => v,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "请重新登录"),
    };
    respond(state.store.reading_command(&user.id, &c))
}
async fn timeline(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(r): Query<v2::Request>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(v) => v,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "请重新登录"),
    };
    respond(state.store.read_message_timeline_v2(&user.id, &r))
}
#[derive(Deserialize)]
struct Window {
    #[serde(flatten)]
    scope: Scope,
    message_ids: Vec<String>,
}
async fn window(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(r): Json<Window>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(v) => v,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "请重新登录"),
    };
    let result = state
        .store
        .recover_message_timeline_window(&user.id, &r.scope.request(), &r.message_ids)
        .and_then(|page| state.store.decorate_timeline_v2(&user.id, &r.scope, page));
    respond(result)
}
fn respond(result: anyhow::Result<serde_json::Value>) -> Response {
    match result {
        Ok(value) => ([("cache-control", "no-store")], Json(value)).into_response(),
        Err(e) => {
            let code = e.to_string();
            let (status, message) = match code.as_str() {
                "timeline_access_denied" => (StatusCode::FORBIDDEN, "无法访问此会话"),
                "reading_unavailable"
                | "reading_message_unavailable"
                | "reading_position_unavailable" => (StatusCode::NOT_FOUND, "书签或消息已不可用"),
                "reading_revision_conflict" | "reading_id_exists" => {
                    (StatusCode::CONFLICT, "书签已变化，请刷新后重试")
                }
                "reading_limit" => (
                    StatusCode::TOO_MANY_REQUESTS,
                    "保存数量已达上限，请处理已有书签或同步冲突",
                ),
                v if v.starts_with("invalid_") || v.contains("cursor") => {
                    (StatusCode::BAD_REQUEST, "阅读位置无效，请重新定位")
                }
                _ => {
                    tracing::warn!("reading positions storage failure");
                    (StatusCode::INTERNAL_SERVER_ERROR, "书签同步失败，请重试")
                }
            };
            (status,Json(serde_json::json!({"error":message,"code":if status==StatusCode::INTERNAL_SERVER_ERROR{"reading_storage_error"}else{&code}}))).into_response()
        }
    }
}

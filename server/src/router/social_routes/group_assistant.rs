use crate::{
    project_auth::{auth_from_headers, json_error},
    store::articles::group_assistant::model::{Share, Sync},
    types::AppState,
};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/assets/group_assistant.js", get(script))
        .route("/assets/group_assistant.css", get(styles))
        .route("/api/me/ai-assistant", get(owned))
        .route("/api/me/groups/:group/ai-assistant", get(list).post(share))
        .route(
            "/api/me/groups/:group/ai-assistant/:id",
            get(updates).post(sync).delete(revoke),
        )
}
async fn script() -> impl IntoResponse {
    (
        [
            ("content-type", "application/javascript; charset=utf-8"),
            ("cache-control", "no-cache"),
        ],
        include_str!("../../assets/group_assistant.js"),
    )
}
async fn styles() -> impl IntoResponse {
    (
        [
            ("content-type", "text/css; charset=utf-8"),
            ("cache-control", "no-cache"),
        ],
        include_str!("../../assets/group_assistant.css"),
    )
}
async fn owned(State(s): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let user = match auth_from_headers(&s, &headers) {
        Ok(u) => u,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "请先登录"),
    };
    response(s.store.group_assistant_owned(&user.id))
}
fn response(value: anyhow::Result<serde_json::Value>) -> Response {
    match value {
        Ok(v) => ([("cache-control", "private, no-store")], Json(v)).into_response(),
        Err(_) => json_error(
            StatusCode::CONFLICT,
            "关注事项不可用、权限已变更或内容不符合分享范围，请刷新后重试",
        ),
    }
}
async fn list(
    State(s): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(group): Path<String>,
) -> Response {
    let user = match auth_from_headers(&s, &headers) {
        Ok(u) => u,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "请先登录"),
    };
    response(s.store.group_assistant_list(&user.id, &group))
}
async fn share(
    State(s): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(group): Path<String>,
    Json(body): Json<Share>,
) -> Response {
    let user = match auth_from_headers(&s, &headers) {
        Ok(u) => u,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "请先登录"),
    };
    response(s.store.group_assistant_share(&user.id, &group, &body))
}
#[derive(Deserialize, Default)]
struct Page {
    #[serde(default)]
    before: i64,
}
async fn updates(
    State(s): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((group, id)): Path<(String, String)>,
    Query(page): Query<Page>,
) -> Response {
    let user = match auth_from_headers(&s, &headers) {
        Ok(u) => u,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "请先登录"),
    };
    response(
        s.store
            .group_assistant_updates(&user.id, &group, &id, page.before.max(0)),
    )
}
async fn sync(
    State(s): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((group, id)): Path<(String, String)>,
    Json(body): Json<Sync>,
) -> Response {
    let user = match auth_from_headers(&s, &headers) {
        Ok(u) => u,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "请先登录"),
    };
    response(s.store.group_assistant_sync(&user.id, &group, &id, &body))
}
async fn revoke(
    State(s): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((group, id)): Path<(String, String)>,
) -> Response {
    let user = match auth_from_headers(&s, &headers) {
        Ok(u) => u,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "请先登录"),
    };
    response(s.store.group_assistant_revoke(&user.id, &group, &id))
}

use crate::{
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Sharing {
    allow_continue: bool,
    version: i64,
}

pub(super) async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((group, message)): Path<(String, String)>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(v) => v,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "请先登录"),
    };
    match state
        .store
        .group_ai_reply_sources(&user.id, &group, &message)
    {
        Ok(value) => ([("cache-control", "private, no-store")], Json(value)).into_response(),
        Err(_) => json_error(StatusCode::NOT_FOUND, "来源记录不可用，或你已不在群中"),
    }
}

pub(super) async fn share(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((group, message)): Path<(String, String)>,
    Json(body): Json<Sharing>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(v) => v,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "请先登录"),
    };
    match state.store.set_group_ai_continuation(
        &user.id,
        &group,
        &message,
        body.allow_continue,
        body.version,
    ) {
        Ok(value) => ([("cache-control", "private, no-store")], Json(value)).into_response(),
        Err(_) => json_error(
            StatusCode::CONFLICT,
            "无法更新分享权限；仅发起人可操作，请刷新后重试",
        ),
    }
}

//! POST /api/agent-balloon/ensure — 为当前用户自动创建"手机控制"项目空间
//!
//! 幂等：同一用户多次调用只创建一次，返回相同的 project_id。
//! 这个项目空间是悬浮球会话历史和脚本历史的容器，不需要代码 workspace。

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

const BALLOON_PROJECT_NAME: &str = "手机控制";

/// POST /api/agent-balloon/ensure
///
/// 确保当前用户有一个名为"手机控制"的专属项目空间。
/// 返回 { "project_id": "...", "created": true/false }
pub async fn ensure_balloon_project(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };

    match state
        .store
        .ensure_balloon_project_for_user(&user.id, BALLOON_PROJECT_NAME)
    {
        Ok((project_id, created)) => Json(json!({
            "project_id": project_id,
            "created": created,
        }))
        .into_response(),
        Err(e) => json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("创建手机控制项目失败: {e}"),
        ),
    }
}

use std::sync::Arc;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::NodeRuntime;

pub(super) async fn bound_credentials(
    runtime: &Arc<NodeRuntime>,
) -> Result<crate::Credentials, Response> {
    runtime.creds().await.ok_or_else(|| {
        json_error(
            StatusCode::UNAUTHORIZED,
            "本机节点尚未绑定账号，不能创建或读取离线任务。",
        )
    })
}

pub(super) fn local_identity_is_valid(value: &str, max_chars: usize) -> bool {
    let value = value.trim();
    !value.is_empty() && value.chars().count() <= max_chars && !value.chars().any(char::is_control)
}

pub(super) fn json_error(status: StatusCode, message: impl Into<String>) -> Response {
    (
        status,
        Json(json!({ "ok": false, "error": message.into() })),
    )
        .into_response()
}

pub(super) fn internal_error(error: anyhow::Error) -> Response {
    json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

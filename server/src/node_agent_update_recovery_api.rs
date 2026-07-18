//! Local-admin HTTP control for reviewed update recovery continuation.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde_json::json;

use crate::NodeRuntime;

pub(crate) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new().route(
        "/api/local-tasks/:task_id/update-recovery/resume",
        post(resume_update_recovery),
    )
}

async fn resume_update_recovery(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(task_id): Path<String>,
) -> Response {
    let Some(creds) = runtime.creds().await else {
        return json_error(StatusCode::UNAUTHORIZED, "本机节点尚未绑定账号。");
    };
    match runtime
        .local_tasks
        .get_for_owner(&creds.owner_user_id, task_id.trim())
    {
        Ok(Some(_)) => {}
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "本机任务不存在。"),
        Err(error) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    }
    match crate::node_agent_update_resume::resume_reviewed(runtime, task_id.trim()).await {
        Ok(receipt) => Json(json!({
            "ok": true,
            "task_id": task_id,
            "protocol": crate::node_agent_update_recovery::UPDATE_RECOVERY_PROTOCOL,
            "update_recovery": receipt,
        }))
        .into_response(),
        Err(error) => json_error(StatusCode::CONFLICT, error.to_string()),
    }
}

fn json_error(status: StatusCode, error: impl Into<String>) -> Response {
    (status, Json(json!({"ok": false, "error": error.into()}))).into_response()
}

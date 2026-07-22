//! Local-admin HTTP control for reviewed update recovery continuation.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;

use crate::NodeRuntime;

pub(crate) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route("/api/update-recovery", get(update_recovery_page))
        .route(
            "/api/update-recovery/reconcile",
            post(reconcile_update_gate),
        )
        .route(
            "/api/local-tasks/:task_id/update-recovery/resume",
            post(resume_update_recovery),
        )
}

async fn reconcile_update_gate(State(runtime): State<Arc<NodeRuntime>>) -> Response {
    match crate::node_agent_update_gate_reconcile::reconcile(runtime).await {
        Ok(payload) => Json(payload).into_response(),
        Err(error) => json_error(StatusCode::CONFLICT, error.to_string()),
    }
}

#[derive(Debug, Default, Deserialize)]
struct RecoveryPageQuery {
    #[serde(default)]
    cursor: usize,
    #[serde(default = "default_page_limit")]
    limit: usize,
    #[serde(default)]
    include_events: bool,
}

fn default_page_limit() -> usize {
    20
}

async fn update_recovery_page(
    State(runtime): State<Arc<NodeRuntime>>,
    Query(query): Query<RecoveryPageQuery>,
) -> Response {
    match runtime.update_recovery.status_page_payload(
        query.cursor,
        query.limit,
        query.include_events,
    ) {
        Ok(payload) => Json(payload).into_response(),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    }
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

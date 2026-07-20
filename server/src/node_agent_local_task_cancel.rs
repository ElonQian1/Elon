use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use homecli_proto::CancelRequestAudit;
use serde::Deserialize;
use serde_json::json;

use crate::NodeRuntime;

#[derive(Debug, Default, Deserialize)]
pub(super) struct CancelLocalTaskRequest {
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    requested_at_ms: Option<u128>,
}

pub(super) async fn cancel_task(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(task_id): Path<String>,
    request: Option<Json<CancelLocalTaskRequest>>,
) -> Response {
    let creds = match super::bound_credentials(&runtime).await {
        Ok(creds) => creds,
        Err(response) => return response,
    };
    let record = match runtime
        .local_tasks
        .get_for_owner(&creds.owner_user_id, task_id.trim())
    {
        Ok(Some(record)) => record,
        Ok(None) => return super::json_error(StatusCode::NOT_FOUND, "本机任务不存在。"),
        Err(error) => return super::internal_error(error),
    };
    if record.status != "running" {
        return Json(json!({
            "ok": true,
            "task_id": record.task_id,
            "status": record.status,
            "message": "任务已经结束，无需重复停止。",
        }))
        .into_response();
    }
    let request = request.map(|Json(value)| value).unwrap_or_default();
    let mut audit = CancelRequestAudit::now(
        creds.owner_user_id.clone(),
        request.source.as_deref().unwrap_or("pc_ui"),
        request.reason.as_deref().unwrap_or("user_requested"),
    );
    if request.requested_at_ms.is_some() {
        audit.requested_at_ms = request.requested_at_ms;
    }
    let signaled = runtime
        .cancel_cli_prompt_with_audit(&record.task_id, &audit)
        .await;
    let durable_intent = runtime
        .task_journal
        .snapshot(&record.task_id, 0, 1)
        .ok()
        .and_then(|snapshot| snapshot.record)
        .is_some_and(|journal| {
            matches!(
                journal.status.as_str(),
                "cancel_requested" | "canceled" | "failed" | "done"
            )
        });
    if !signaled && !durable_intent {
        return super::json_error(
            StatusCode::CONFLICT,
            "任务记录仍在运行，但当前进程没有可停止的控制句柄。",
        );
    }
    if let Err(error) = runtime
        .local_tasks
        .mark_canceled(&creds.owner_user_id, &record.task_id)
    {
        return super::internal_error(error);
    }
    Json(json!({
        "ok": true,
        "task_id": record.task_id,
        "status": "cancel_requested",
        "cancel": audit,
    }))
    .into_response()
}

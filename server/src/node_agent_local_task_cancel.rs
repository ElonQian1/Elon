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
    if matches!(
        record.status.as_str(),
        "done" | "failed" | "canceled" | "finished"
    ) {
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
    match runtime
        .cancel_cli_prompt_with_audit_result(&record.task_id, &audit)
        .await
    {
        Ok(crate::node_agent_cancel_saga::CancelDispatchOutcome::Dispatched {
            action_id,
            target_kind,
            ..
        }) => Json(json!({
            "ok": true,
            "task_id": record.task_id,
            "status": "cancel_requested",
            "action_id": action_id,
            "side_effect": target_kind,
            "cancel": audit,
        }))
        .into_response(),
        Ok(crate::node_agent_cancel_saga::CancelDispatchOutcome::AlreadyCommitted {
            action_id,
        }) => Json(json!({
            "ok": true,
            "task_id": record.task_id,
            "status": "cancel_requested",
            "action_id": action_id,
            "cancel": audit,
        }))
        .into_response(),
        Ok(crate::node_agent_cancel_saga::CancelDispatchOutcome::Pending { action_id }) => (
            StatusCode::CONFLICT,
            Json(json!({
                "ok": false,
                "task_id": record.task_id,
                "status": "cancel_requested",
                "action_id": action_id,
                "recoverable": true,
                "error": "取消意图已持久化，但尚未向匹配的存活执行器提交副作用。",
            })),
        )
            .into_response(),
        Ok(crate::node_agent_cancel_saga::CancelDispatchOutcome::Terminal { status }) => {
            Json(json!({
                "ok": true,
                "task_id": record.task_id,
                "status": status,
                "message": "任务已经结束，无需重复停止。",
            }))
            .into_response()
        }
        Ok(crate::node_agent_cancel_saga::CancelDispatchOutcome::NotFound) => super::json_error(
            StatusCode::CONFLICT,
            "任务记录仍未终结，但当前没有可验证身份的控制句柄。",
        ),
        Err(error) => super::internal_error(error),
    }
}

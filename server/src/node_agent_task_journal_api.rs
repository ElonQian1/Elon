// server/src/node_agent_task_journal_api.rs

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::{
    node_agent_task_approval_snapshot::TaskApprovalStateSnapshot,
    node_agent_task_journal::{TaskJournalEventView, TaskJournalRecord},
    node_agent_task_resume::{
        task_attach_state_with_sidecar, task_resume_contract_with_journal_approvals,
        TaskAttachState, TaskResumeContract,
    },
    NodeRuntime,
};

#[derive(Debug, Deserialize)]
struct JournalQuery {
    since: Option<usize>,
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
struct LocalTaskJournalListResponse {
    ok: bool,
    records: Vec<TaskJournalRecord>,
}

#[derive(Debug, Serialize)]
struct LocalTaskJournalResponse {
    ok: bool,
    task_id: String,
    record: Option<TaskJournalRecord>,
    events: Vec<TaskJournalEventView>,
    last_event_seq: usize,
    has_more: bool,
    attach: TaskAttachState,
    resume: TaskResumeContract,
    approval_state: TaskApprovalStateSnapshot,
    runtime: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct LocalTaskCancelResponse {
    ok: bool,
    task_id: String,
    status: &'static str,
    message: &'static str,
}

pub(crate) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route("/api/task-journal", get(list_task_journal))
        .route(
            "/api/task-journal/:task_id",
            get(get_task_journal).post(cancel_task_journal),
        )
}

async fn list_task_journal(
    State(runtime): State<Arc<NodeRuntime>>,
    Query(query): Query<JournalQuery>,
) -> Response {
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    match runtime.task_journal.latest_records(limit) {
        Ok(records) => Json(LocalTaskJournalListResponse { ok: true, records }).into_response(),
        Err(error) => internal_error(error),
    }
}

async fn get_task_journal(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(task_id): Path<String>,
    Query(query): Query<JournalQuery>,
) -> Response {
    let task_id = task_id.trim().to_string();
    if let Err(message) = validate_task_id(&task_id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "ok": false,
                "error": message,
            })),
        )
            .into_response();
    }

    let since = query.since.unwrap_or(0);
    let limit = query.limit.unwrap_or(100);
    let active = runtime.active_cli_prompt_view(&task_id).await;
    let sidecar = runtime
        .cli_sidecars
        .session_for_task(&task_id)
        .unwrap_or_else(|error| {
            tracing::warn!("读取 CLI sidecar 会话失败: {error}");
            None
        });
    match runtime.task_journal.snapshot(&task_id, since, limit) {
        Ok(snapshot) => {
            // 本地 API 只暴露进程恢复所需的最小字段；prompt/API key 从未写入 journal。
            let attach = task_attach_state_with_sidecar(snapshot.record.as_ref(), active, sidecar);
            let resume = task_resume_contract_with_journal_approvals(&attach, &snapshot.approvals);
            let task_status = snapshot
                .record
                .as_ref()
                .map(|record| record.status.as_str());
            let approval_state = snapshot.approvals.resolve_runtime_state_for_task_status(
                resume.active_approval_ids(),
                resume.can_approve_tools(),
                task_status,
            );
            let mut runtime_status =
                crate::node_agent_task_journal::runtime_status_payload(snapshot.record.as_ref());
            if approval_state.actionable_count > 0 {
                runtime_status["phase"] = serde_json::Value::String("approval".to_string());
            }
            Json(LocalTaskJournalResponse {
                ok: true,
                task_id: snapshot.task_id,
                record: snapshot.record,
                events: snapshot.events,
                last_event_seq: snapshot.last_event_seq,
                has_more: snapshot.has_more,
                attach,
                resume,
                approval_state,
                runtime: runtime_status,
            })
            .into_response()
        }
        Err(error) => internal_error(error),
    }
}

async fn cancel_task_journal(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(task_id): Path<String>,
) -> Response {
    let task_id = task_id.trim().to_string();
    if let Err(message) = validate_task_id(&task_id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "ok": false,
                "error": message,
            })),
        )
            .into_response();
    }

    let audit = homecli_proto::CancelRequestAudit::now(
        "local_admin",
        "task_journal_api",
        "operator_requested",
    );
    if runtime.cancel_cli_prompt_with_audit(&task_id, &audit).await {
        return Json(LocalTaskCancelResponse {
            ok: true,
            task_id,
            status: "cancel_requested",
            message: "已向本机运行控制句柄发送停止信号。",
        })
        .into_response();
    }

    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "ok": false,
            "task_id": task_id,
            "error": "任务不在运行中或当前节点已没有控制句柄。",
        })),
    )
        .into_response()
}

fn validate_task_id(task_id: &str) -> Result<(), &'static str> {
    if task_id.is_empty() {
        return Err("任务 ID 不能为空。");
    }
    if task_id.len() > 160 {
        return Err("任务 ID 过长。");
    }
    if task_id
        .chars()
        .any(|ch| ch.is_control() || ch == '/' || ch == '\\')
    {
        return Err("任务 ID 包含非法字符。");
    }
    Ok(())
}

fn internal_error(error: anyhow::Error) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({
            "ok": false,
            "error": error.to_string(),
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::validate_task_id;

    #[test]
    fn rejects_unsafe_task_ids() {
        assert!(validate_task_id("task-1").is_ok());
        assert!(validate_task_id("").is_err());
        assert!(validate_task_id("task/1").is_err());
        assert!(validate_task_id("task\\1").is_err());
    }
}

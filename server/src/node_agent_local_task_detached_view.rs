//! A bounded fail-closed view when a live control surface outlives its local row.

use anyhow::Result;
use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

use crate::{
    node_agent_local_task_supervision::{load_supervision_state, SUPERVISION_PROTOCOL},
    NodeRuntime,
};

pub(crate) async fn response_or_not_found(
    runtime: &NodeRuntime,
    task_id: &str,
    since: usize,
    limit: usize,
    expected_cursor_epoch: Option<&str>,
) -> axum::response::Response {
    match response_if_recoverable(runtime, task_id, since, limit, expected_cursor_epoch).await {
        Ok(Some(response)) => response,
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "ok": false,
                "error": "本机任务不存在。",
            })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "ok": false,
                "error": error.to_string(),
            })),
        )
            .into_response(),
    }
}

async fn response_if_recoverable(
    runtime: &NodeRuntime,
    task_id: &str,
    since: usize,
    limit: usize,
    expected_cursor_epoch: Option<&str>,
) -> Result<Option<axum::response::Response>> {
    let active = runtime.active_cli_prompt_view(task_id).await;
    let sidecar = runtime.cli_sidecars.session_for_task(task_id)?;
    let live_sidecar = sidecar
        .as_ref()
        .is_some_and(|session| session.is_live_at(crate::node_agent_cli_sidecar::now_ms()));
    if active.is_none() && !live_sidecar {
        return Ok(None);
    }
    let snapshot = runtime.task_journal.snapshot_with_epoch(
        task_id,
        since,
        limit.clamp(1, 200),
        expected_cursor_epoch,
    )?;
    let Some(journal_record) = snapshot.record.as_ref() else {
        return Ok(None);
    };
    let supervision = load_supervision_state(&runtime.task_journal, task_id)?;
    if supervision
        .contract()
        .is_none_or(|contract| contract.protocol != SUPERVISION_PROTOCOL)
    {
        return Ok(None);
    }
    let attach = crate::node_agent_task_resume::task_attach_state_with_sidecar(
        Some(journal_record),
        active,
        sidecar,
    );
    let resume = crate::node_agent_task_resume::task_resume_contract_with_journal_approvals(
        &attach,
        &snapshot.approvals,
    );
    let runtime_status =
        crate::node_agent_task_journal::runtime_status_payload(Some(journal_record));
    Ok(Some(
        Json(json!({
            "ok": true,
            "record": null,
            "detached": {
                "state": "recoverable_detached",
                "recoverable": true,
                "durable_local_row_missing": true,
                "ownership_status": "unresolved_fail_closed",
                "project_id": null,
                "workspace_path": null,
                "message": "活动控制句柄或 sidecar 仍存在，但 durable local task row 缺失；未凭 sidecar 猜测 owner/project/workspace。可继续检查 journal，并通过 journal cancel fallback 审计取消。",
                "cancel_fallback": format!("/api/task-journal/{task_id}"),
            },
            "events": snapshot.events,
            "last_event_seq": snapshot.last_event_seq,
            "has_more": snapshot.has_more,
            "cursor_epoch": snapshot.cursor_epoch,
            "requested_cursor_epoch": snapshot.requested_cursor_epoch,
            "previous_cursor_epoch": snapshot.previous_cursor_epoch,
            "cursor_reset": snapshot.cursor_reset,
            "requested_cursor": snapshot.requested_cursor,
            "old_cursor": snapshot.old_cursor,
            "new_cursor": snapshot.new_cursor,
            "resume_cursor": snapshot.resume_cursor,
            "sidecar_update_epoch": snapshot.sidecar_update_epoch,
            "runtime": runtime_status,
            "performance_timing": crate::node_agent_task_performance_timing::payload(&runtime.task_journal, Some(journal_record)),
            "attach": attach,
            "resume": resume,
            "supervision": supervision,
            "update_recovery": runtime.update_recovery.receipt_for_task(task_id)?,
        }))
        .into_response(),
    ))
}

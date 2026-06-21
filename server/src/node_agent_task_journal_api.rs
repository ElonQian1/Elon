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
    node_agent_task_journal::{TaskJournalEventView, TaskJournalRecord},
    NodeRuntime,
};

#[derive(Debug, Deserialize)]
struct JournalQuery {
    since: Option<usize>,
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
struct LocalTaskAttachState {
    status: &'static str,
    live: bool,
    can_reconnect: bool,
    continue_mode: &'static str,
    source: &'static str,
    reason: &'static str,
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
    attach: LocalTaskAttachState,
}

pub(crate) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route("/api/task-journal", get(list_task_journal))
        .route("/api/task-journal/:task_id", get(get_task_journal))
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
    let active = runtime
        .active_cli_prompts
        .read()
        .await
        .contains_key(&task_id);
    match runtime.task_journal.snapshot(&task_id, since, limit) {
        Ok(snapshot) => {
            // 本地 API 只暴露进程恢复所需的最小字段；prompt/API key 从未写入 journal。
            let attach = local_attach_state(snapshot.record.as_ref(), active);
            Json(LocalTaskJournalResponse {
                ok: true,
                task_id: snapshot.task_id,
                record: snapshot.record,
                events: snapshot.events,
                last_event_seq: snapshot.last_event_seq,
                has_more: snapshot.has_more,
                attach,
            })
            .into_response()
        }
        Err(error) => internal_error(error),
    }
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

fn local_attach_state(record: Option<&TaskJournalRecord>, active: bool) -> LocalTaskAttachState {
    if active {
        return LocalTaskAttachState {
            status: "live",
            live: true,
            can_reconnect: true,
            continue_mode: "reconnect_original_process",
            source: "local_journal",
            reason: "本机节点仍持有该任务的运行控制句柄，可以重连原进程现场。",
        };
    }
    match record.map(|record| record.status.as_str()) {
        Some("running" | "cancel_requested") => LocalTaskAttachState {
            status: "detached",
            live: false,
            can_reconnect: false,
            continue_mode: "snapshot_continue",
            source: "local_journal",
            reason: "本机 journal 显示任务未终态，但当前节点已没有运行句柄，只能基于快照继续。",
        },
        Some(_) => LocalTaskAttachState {
            status: "terminal",
            live: false,
            can_reconnect: false,
            continue_mode: "snapshot_continue",
            source: "local_journal",
            reason: "本机进程已经结束，只能基于任务快照继续新一轮处理。",
        },
        None => LocalTaskAttachState {
            status: "missing",
            live: false,
            can_reconnect: false,
            continue_mode: "snapshot_continue",
            source: "local_journal",
            reason: "本机没有该任务的 journal 记录，前端只能使用云端任务快照。",
        },
    }
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
    use super::{local_attach_state, validate_task_id};
    use crate::node_agent_task_journal::TaskJournalRecord;

    fn record(status: &str) -> TaskJournalRecord {
        TaskJournalRecord {
            req_id: "task-1".to_string(),
            cli_name: "codex".to_string(),
            cwd: Some("D:/demo".to_string()),
            runtime_permission: Some("project_write".to_string()),
            status: status.to_string(),
            started_at_ms: 1,
            updated_at_ms: 2,
            cancel_requested_at_ms: None,
        }
    }

    #[test]
    fn rejects_unsafe_task_ids() {
        assert!(validate_task_id("task-1").is_ok());
        assert!(validate_task_id("").is_err());
        assert!(validate_task_id("task/1").is_err());
        assert!(validate_task_id("task\\1").is_err());
    }

    #[test]
    fn attach_state_distinguishes_live_detached_and_snapshot() {
        let running = record("running");
        let finished = record("finished");
        let live = local_attach_state(Some(&running), true);
        assert_eq!(live.status, "live");
        assert!(live.can_reconnect);
        assert_eq!(live.continue_mode, "reconnect_original_process");
        assert_eq!(local_attach_state(Some(&running), false).status, "detached");
        assert_eq!(
            local_attach_state(Some(&finished), false).status,
            "terminal"
        );
        assert_eq!(local_attach_state(None, false).status, "missing");
    }
}

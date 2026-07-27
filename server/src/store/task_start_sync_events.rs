use anyhow::Result;
use rusqlite::params;

use super::{clean_optional, new_id, PcLocalTaskStartApply};

pub(super) fn insert_dispatch_event(
    tx: &rusqlite::Transaction<'_>,
    task_id: &str,
    input: &PcLocalTaskStartApply<'_>,
    timestamp: &str,
) -> Result<bool> {
    let event = serde_json::json!({
        "type": "pc_dispatch_started",
        "origin": "local_offline",
        "pc_req_id": input.request_id,
        "task_id": task_id,
        "agent_id": input.node_id,
        "cli": input.cli,
        "workspace_path": input.workspace_path,
    })
    .to_string();
    Ok(tx.execute(
        "INSERT INTO task_events (id, task_id, event_json, created_at)
         SELECT ?1, ?2, ?3, ?4
          WHERE NOT EXISTS (
            SELECT 1 FROM task_events
             WHERE task_id = ?2 AND instr(event_json, '\"type\":\"pc_dispatch_started\"') > 0
          )",
        params![new_id("tev"), task_id, event, timestamp],
    )? > 0)
}

pub(super) fn insert_revision_event(
    tx: &rusqlite::Transaction<'_>,
    task_id: &str,
    input: &PcLocalTaskStartApply<'_>,
    timestamp: &str,
) -> Result<bool> {
    let event = serde_json::json!({
        "type": "local_task_synced",
        "origin": "local_offline",
        "pc_req_id": input.request_id,
        "revision": input.revision,
        "status": input.status,
        "codex_thread_id": clean_optional(input.codex_session_id),
    })
    .to_string();
    let marker = format!(
        "\"revision\":{}",
        serde_json::to_string(input.revision).unwrap_or_default()
    );
    Ok(tx.execute(
        "INSERT INTO task_events (id, task_id, event_json, created_at)
         SELECT ?1, ?2, ?3, ?4
          WHERE NOT EXISTS (
            SELECT 1 FROM task_events
             WHERE task_id = ?2 AND instr(event_json, ?5) > 0
          )",
        params![new_id("tev"), task_id, event, timestamp, marker],
    )? > 0)
}

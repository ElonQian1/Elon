use anyhow::Result;
use serde_json::json;

use super::{
    dispatch::{default_dispatch_schema, TaskDispatchProgress},
    now_ms, with_task_journal_io_lock, TaskJournal, TaskJournalRecord, TaskJournalStart,
};

impl TaskJournal {
    pub(crate) fn record_started(&self, start: TaskJournalStart<'_>) -> Result<()> {
        self.record_started_inner(start, false).map(|_| ())
    }

    pub(crate) fn record_started_if_absent(&self, start: TaskJournalStart<'_>) -> Result<bool> {
        self.record_started_inner(start, true)
    }

    fn record_started_inner(
        &self,
        start: TaskJournalStart<'_>,
        only_if_absent: bool,
    ) -> Result<bool> {
        with_task_journal_io_lock(|| {
            let now = now_ms();
            let mut registry = self.load_registry()?;
            if only_if_absent && registry.contains_key(start.req_id) {
                return Ok(false);
            }
            let record = TaskJournalRecord {
                req_id: start.req_id.to_string(),
                cli_name: start.cli_name.to_string(),
                route: start.route.map(str::to_string),
                run_handle_id: start.run_handle_id.map(str::to_string),
                cwd: start.cwd.map(str::to_string),
                runtime_permission: start.runtime_permission.map(str::to_string),
                os_pid: None,
                process_started_at_ms: None,
                process_identity: None,
                codex_session_id: None,
                codex_session_scope_key: None,
                codex_session_updated_at_ms: None,
                status: "running".to_string(),
                phase: "dispatch".to_string(),
                current_command: None,
                last_progress_ms: Some(now),
                heartbeat_at_ms: Some(now),
                timeout_policy: None,
                dispatch: Some(TaskDispatchProgress {
                    schema: default_dispatch_schema(),
                    stage: "persisted".to_string(),
                    stage_started_at_ms: now,
                    stages: Vec::new(),
                    failure: None,
                }),
                started_at_ms: now,
                updated_at_ms: now,
                cancel_requested_at_ms: None,
                cancel_intent: None,
            };
            registry.insert(start.req_id.to_string(), record);
            self.save_registry(&registry)?;
            self.append_event(json!({
                "type": "started",
                "req_id": start.req_id,
                "cli": start.cli_name,
                "route": start.route,
                "run_handle_id": start.run_handle_id,
                "cwd": start.cwd,
                "runtime_permission": start.runtime_permission,
                "at_ms": now
            }))?;
            Ok(true)
        })
    }
}

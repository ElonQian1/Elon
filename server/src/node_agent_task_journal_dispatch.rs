use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{
    is_terminal_status, now_ms, with_task_journal_io_lock, TaskJournal, TaskJournalRecord,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct TaskDispatchProgress {
    #[serde(default = "default_dispatch_schema")]
    pub schema: String,
    #[serde(default)]
    pub stage: String,
    #[serde(default)]
    pub stage_started_at_ms: u128,
    #[serde(default)]
    pub stages: Vec<TaskDispatchStageTiming>,
    #[serde(default)]
    pub failure: Option<TaskDispatchFailure>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct TaskDispatchStageTiming {
    pub stage: String,
    pub started_at_ms: u128,
    pub finished_at_ms: u128,
    pub duration_ms: u128,
    pub outcome: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct TaskDispatchFailure {
    pub stage: String,
    pub code: String,
    pub message: String,
    pub at_ms: u128,
}

impl TaskJournal {
    pub(crate) fn record_dispatch_stage(&self, req_id: &str, stage: &str) -> Result<()> {
        with_task_journal_io_lock(|| {
            let now = now_ms();
            let mut registry = self.load_registry()?;
            if let Some(record) = registry.get_mut(req_id) {
                if is_terminal_status(&record.status) {
                    return Ok(());
                }
                advance_dispatch_record(record, stage, now);
                record.phase = "dispatch".to_string();
                record.last_progress_ms = Some(now);
                record.heartbeat_at_ms = Some(now);
                record.updated_at_ms = now;
            }
            self.save_registry(&registry)?;
            self.append_event(json!({
                "type": "dispatch_stage",
                "req_id": req_id,
                "stage": stage,
                "at_ms": now,
            }))
        })
    }

    pub(crate) fn record_dispatch_failure(
        &self,
        req_id: &str,
        stage: &str,
        code: &str,
        message: &str,
    ) -> Result<()> {
        with_task_journal_io_lock(|| {
            let now = now_ms();
            let message = crate::node_agent_cli_redaction::redact_text(message);
            let message = message.chars().take(2_000).collect::<String>();
            let mut registry = self.load_registry()?;
            if let Some(record) = registry.get_mut(req_id) {
                if let Some(dispatch) = record.dispatch.as_mut() {
                    finish_dispatch_stage(dispatch, now, "failed");
                    dispatch.failure = Some(TaskDispatchFailure {
                        stage: stage.to_string(),
                        code: code.to_string(),
                        message: message.clone(),
                        at_ms: now,
                    });
                }
                record.phase = "failed".to_string();
                record.current_command = None;
                record.heartbeat_at_ms = Some(now);
                record.updated_at_ms = now;
            }
            self.save_registry(&registry)?;
            self.append_event(json!({
                "type": "dispatch_failure",
                "req_id": req_id,
                "stage": stage,
                "code": code,
                "message": message,
                "at_ms": now,
            }))
        })
    }
}

pub(super) fn default_dispatch_schema() -> String {
    "elon.cli_dispatch_progress.v1".to_string()
}

pub(super) fn advance_dispatch_record(record: &mut TaskJournalRecord, stage: &str, now: u128) {
    let Some(dispatch) = record.dispatch.as_mut() else {
        return;
    };
    if dispatch.stage == stage {
        return;
    }
    finish_dispatch_stage(dispatch, now, "completed");
    dispatch.stage = stage.to_string();
    dispatch.stage_started_at_ms = now;
}

fn finish_dispatch_stage(dispatch: &mut TaskDispatchProgress, now: u128, outcome: &str) {
    if dispatch.stage.is_empty() || dispatch.stage_started_at_ms == 0 {
        return;
    }
    dispatch.stages.push(TaskDispatchStageTiming {
        stage: dispatch.stage.clone(),
        started_at_ms: dispatch.stage_started_at_ms,
        finished_at_ms: now,
        duration_ms: now.saturating_sub(dispatch.stage_started_at_ms),
        outcome: outcome.to_string(),
    });
    if dispatch.stages.len() > 24 {
        let remove = dispatch.stages.len() - 24;
        dispatch.stages.drain(0..remove);
    }
}

//! Read-only terminal conflict checks used before any completion-side writes.

use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Write},
};

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::TaskJournal;
use crate::{
    node_agent_task_journal_events::{
        is_completed_terminal_status, normalize_finish_error, normalize_finish_status,
    },
    node_agent_task_journal_lock::with_task_journal_io_lock,
};

impl TaskJournal {
    pub(crate) fn preflight_reconciled_finished_with_outcome(
        &self,
        req_id: &str,
        event_id: &str,
        status: &str,
        error: Option<&str>,
    ) -> Result<()> {
        with_task_journal_io_lock(|| {
            let requested = normalize_finish_status(status, error);
            let registry = self.load_registry()?;
            if let Some(record) = registry.get(req_id) {
                anyhow::ensure!(
                    !is_completed_terminal_status(&record.status) || record.status == requested,
                    "task journal terminal status conflicts with durable completion"
                );
            }
            self.assert_reconciled_event(event_id, req_id, requested, error)?;
            Ok(())
        })
    }

    /// Registry replacement and JSONL append are separate durable files. The
    /// completion outbox is the transaction log, so replay must fill whichever
    /// side of this boundary was missing without duplicating the exact event.
    pub(crate) fn record_reconciled_finished_with_outcome(
        &self,
        req_id: &str,
        event_id: &str,
        status: &str,
        error: Option<&str>,
    ) -> Result<()> {
        with_task_journal_io_lock(|| {
            let now = super::now_ms();
            let requested = normalize_finish_status(status, error);
            let mut registry = self.load_registry()?;
            if let Some(record) = registry.get_mut(req_id) {
                anyhow::ensure!(
                    !is_completed_terminal_status(&record.status) || record.status == requested,
                    "task journal terminal status conflicts with durable completion"
                );
                if !is_completed_terminal_status(&record.status) {
                    record.status = requested.to_string();
                    record.phase = super::terminal_runtime_phase(requested).to_string();
                    record.current_command = None;
                    record.heartbeat_at_ms = Some(now);
                    record.updated_at_ms = now;
                    self.save_registry(&registry)?;
                }
            }
            if self.assert_reconciled_event(event_id, req_id, requested, error)? {
                return Ok(());
            }
            let mut event = json!({
                "type": "finished",
                "req_id": req_id,
                "status": requested,
                "completion_event_id": event_id,
                "at_ms": now,
            });
            if let Some(error) = normalize_finish_error(error) {
                event["error"] = Value::String(error);
            }
            self.append_reconciled_event_durable(&event)
        })
    }

    fn append_reconciled_event_durable(&self, event: &Value) -> Result<()> {
        self.ensure_dir()?;
        let path = self.events_path();
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("open reconciled task journal {}", path.display()))?;
        writeln!(file, "{}", serde_json::to_string(event)?)
            .with_context(|| format!("write reconciled task journal {}", path.display()))?;
        file.sync_all()
            .with_context(|| format!("sync reconciled task journal {}", path.display()))
    }

    fn assert_reconciled_event(
        &self,
        event_id: &str,
        req_id: &str,
        status: &str,
        error: Option<&str>,
    ) -> Result<bool> {
        let path = self.events_path();
        if !path.exists() {
            return Ok(false);
        }
        let expected_error = normalize_finish_error(error);
        let mut matched = false;
        for line in BufReader::new(File::open(&path)?).lines() {
            let line = line.with_context(|| format!("read task journal {}", path.display()))?;
            let Ok(event) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            if event.get("completion_event_id").and_then(Value::as_str) != Some(event_id) {
                continue;
            }
            anyhow::ensure!(
                event.get("type").and_then(Value::as_str) == Some("finished")
                    && event.get("req_id").and_then(Value::as_str) == Some(req_id)
                    && event.get("status").and_then(Value::as_str) == Some(status)
                    && event.get("error").and_then(Value::as_str) == expected_error.as_deref(),
                "task journal completion event binding conflicts with durable completion"
            );
            anyhow::ensure!(
                !matched,
                "task journal contains a duplicate completion event"
            );
            matched = true;
        }
        Ok(matched)
    }
}

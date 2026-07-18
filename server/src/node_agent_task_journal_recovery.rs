use super::*;

use crate::node_agent_task_journal_events::is_completed_terminal_status;

impl TaskJournal {
    pub(crate) fn record_recovery_running(
        &self,
        req_id: &str,
        phase: &str,
        current_command: Option<&str>,
        reason: &str,
    ) -> Result<bool> {
        with_task_journal_io_lock(|| {
            let now = now_ms();
            let mut registry = self.load_registry()?;
            let Some(record) = registry.get_mut(req_id) else {
                return Ok(false);
            };
            if is_completed_terminal_status(&record.status)
                || !matches!(
                    record.status.as_str(),
                    "running" | "recovering" | "reattaching"
                )
            {
                return Ok(false);
            }
            record.status = "running".to_string();
            record.phase = normalize_runtime_phase(phase).to_string();
            record.current_command = current_command
                .map(crate::node_agent_cli_output_aggregate::sanitize_command)
                .filter(|command| !command.is_empty());
            record.last_progress_ms = Some(now);
            record.heartbeat_at_ms = Some(now);
            record.updated_at_ms = now;
            let effective_phase = record.phase.clone();
            self.save_registry(&registry)?;
            self.append_event(json!({
                "type": "recovery_running",
                "req_id": req_id,
                "status": "running",
                "phase": effective_phase,
                "reason": reason,
                "at_ms": now,
            }))?;
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(self.events_path())?
                .sync_all()?;
            Ok(true)
        })
    }
}

use anyhow::Result;
use homecli_proto::CancelRequestAudit;
use serde_json::{json, Value};

use super::{is_terminal_status, now_ms, with_task_journal_io_lock, TaskJournal};

impl TaskJournal {
    pub(crate) fn record_cancel_requested(&self, req_id: &str) -> Result<()> {
        self.record_cancel_requested_with_audit(req_id, &CancelRequestAudit::default())
    }

    pub(crate) fn record_cancel_requested_with_audit(
        &self,
        req_id: &str,
        audit: &CancelRequestAudit,
    ) -> Result<()> {
        with_task_journal_io_lock(|| {
            let now = audit.requested_at_ms.unwrap_or_else(now_ms);
            let mut registry = self.load_registry()?;
            let mut current_status = None;
            let mut ignored = false;
            let mut registry_changed = false;
            if let Some(record) = registry.get_mut(req_id) {
                if is_terminal_status(&record.status) {
                    ignored = true;
                    current_status = Some(record.status.clone());
                } else {
                    record.status = "cancel_requested".to_string();
                    record.phase = "finalizing".to_string();
                    record.updated_at_ms = now;
                    record.cancel_requested_at_ms = Some(now);
                    current_status = Some(record.status.clone());
                    registry_changed = true;
                }
            }
            if registry_changed {
                self.save_registry(&registry)?;
            }
            let mut event = json!({
                "type": "cancel_requested",
                "req_id": req_id,
                "requested_by": audit.requested_by,
                "source": audit.source,
                "reason": audit.reason,
                "requested_at_ms": now,
                "interruption_source": audit.interruption_source,
                "at_ms": now
            });
            if let Some(status) = current_status {
                event["status"] = Value::String(status);
            }
            if ignored {
                event["ignored"] = Value::Bool(true);
                event["ignored_reason"] = Value::String("task_already_terminal".to_string());
            }
            self.append_event(event)
        })
    }
}

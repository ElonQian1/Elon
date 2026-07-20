use anyhow::{bail, Result};
use homecli_proto::CancelRequestAudit;
use serde_json::json;

use super::{
    is_completed_terminal_status, now_ms, with_task_journal_io_lock, CancelIntentRecord,
    CancelIntentTarget, CancelSideEffectCommit, PersistCancelIntentOutcome, TaskJournal,
};

impl TaskJournal {
    pub(crate) fn record_cancel_requested(&self, req_id: &str) -> Result<()> {
        self.record_cancel_requested_with_audit(req_id, &CancelRequestAudit::default())
    }

    pub(crate) fn record_cancel_requested_with_audit(
        &self,
        req_id: &str,
        audit: &CancelRequestAudit,
    ) -> Result<()> {
        let outcome = self.record_cancel_intent(
            req_id,
            CancelIntentTarget {
                run_handle_id: Some(req_id.to_string()),
                active_started_at_ms: None,
                sidecar_session_id: None,
            },
            audit,
        )?;
        if let PersistCancelIntentOutcome::Terminal(status) = outcome {
            let now = audit.requested_at_ms.unwrap_or_else(now_ms);
            self.append_event(json!({
                "type": "cancel_requested",
                "req_id": req_id,
                "requested_by": audit.requested_by,
                "source": audit.source,
                "reason": audit.reason,
                "requested_at_ms": now,
                "interruption_source": audit.interruption_source,
                "status": status,
                "ignored": true,
                "ignored_reason": "task_already_terminal",
                "at_ms": now,
            }))?;
        }
        Ok(())
    }

    pub(crate) fn record_cancel_intent(
        &self,
        req_id: &str,
        target: CancelIntentTarget,
        audit: &CancelRequestAudit,
    ) -> Result<PersistCancelIntentOutcome> {
        with_task_journal_io_lock(|| {
            let now = audit.requested_at_ms.unwrap_or_else(now_ms);
            let mut durable_audit = audit.clone();
            durable_audit.requested_at_ms = Some(now);
            let mut registry = self.load_registry()?;
            let Some(record) = registry.get_mut(req_id) else {
                return Ok(PersistCancelIntentOutcome::Missing);
            };
            if is_completed_terminal_status(&record.status) {
                return Ok(PersistCancelIntentOutcome::Terminal(record.status.clone()));
            }
            if let Some(intent) = record.cancel_intent.clone() {
                return Ok(if intent.side_effect.is_some() {
                    PersistCancelIntentOutcome::Committed(intent)
                } else {
                    PersistCancelIntentOutcome::Pending(intent)
                });
            }

            if target.run_handle_id.is_none() && target.sidecar_session_id.is_none() {
                bail!("cancel intent requires an active handle or sidecar identity");
            }
            if let (Some(expected), Some(actual)) = (
                record.run_handle_id.as_deref(),
                target.run_handle_id.as_deref(),
            ) {
                if expected != actual {
                    bail!("cancel intent run handle does not match task journal identity");
                }
            }
            let intent = CancelIntentRecord {
                action_id: format!("cancel-{}", uuid::Uuid::new_v4().simple()),
                task_id: req_id.to_string(),
                task_started_at_ms: record.started_at_ms,
                run_handle_id: target.run_handle_id,
                active_started_at_ms: target.active_started_at_ms,
                sidecar_session_id: target.sidecar_session_id,
                audit: durable_audit.clone(),
                created_at_ms: now,
                side_effect: None,
            };
            record.status = "cancel_requested".to_string();
            record.phase = "finalizing".to_string();
            record.updated_at_ms = now;
            record.cancel_requested_at_ms = Some(now);
            record.cancel_intent = Some(intent.clone());
            self.save_registry(&registry)?;
            self.append_event(json!({
                "type": "cancel_requested",
                "req_id": req_id,
                "action_id": intent.action_id,
                "task_started_at_ms": intent.task_started_at_ms,
                "run_handle_id": intent.run_handle_id,
                "active_started_at_ms": intent.active_started_at_ms,
                "sidecar_session_id": intent.sidecar_session_id,
                "requested_by": durable_audit.requested_by,
                "source": durable_audit.source,
                "reason": durable_audit.reason,
                "requested_at_ms": now,
                "interruption_source": durable_audit.interruption_source,
                "status": "cancel_requested",
                "at_ms": now
            }))?;
            Ok(PersistCancelIntentOutcome::Pending(intent))
        })
    }

    #[cfg(test)]
    pub(crate) fn cancel_intents(&self) -> Result<Vec<CancelIntentRecord>> {
        with_task_journal_io_lock(|| {
            Ok(self
                .load_registry()?
                .into_values()
                .filter_map(|record| record.cancel_intent)
                .collect())
        })
    }

    pub(crate) fn pending_cancel_intents(&self) -> Result<Vec<CancelIntentRecord>> {
        with_task_journal_io_lock(|| {
            Ok(self
                .load_registry()?
                .into_values()
                .filter(|record| !is_completed_terminal_status(&record.status))
                .filter_map(|record| record.cancel_intent)
                .filter(|intent| intent.side_effect.is_none())
                .collect())
        })
    }

    pub(crate) fn cancel_intent_for_reconcile(
        &self,
        task_id: &str,
        action_id: &str,
    ) -> Result<PersistCancelIntentOutcome> {
        with_task_journal_io_lock(|| {
            let registry = self.load_registry()?;
            let Some(record) = registry.get(task_id) else {
                return Ok(PersistCancelIntentOutcome::Missing);
            };
            if is_completed_terminal_status(&record.status) {
                return Ok(PersistCancelIntentOutcome::Terminal(record.status.clone()));
            }
            let Some(intent) = record.cancel_intent.clone() else {
                return Ok(PersistCancelIntentOutcome::Missing);
            };
            if intent.action_id != action_id
                || intent.task_id != record.req_id
                || intent.task_started_at_ms != record.started_at_ms
            {
                return Ok(PersistCancelIntentOutcome::Missing);
            }
            Ok(if intent.side_effect.is_some() {
                PersistCancelIntentOutcome::Committed(intent)
            } else {
                PersistCancelIntentOutcome::Pending(intent)
            })
        })
    }

    pub(crate) fn commit_cancel_side_effect(
        &self,
        task_id: &str,
        action_id: &str,
        target_kind: &str,
        target_id: &str,
    ) -> Result<bool> {
        with_task_journal_io_lock(|| {
            let now = now_ms();
            let mut registry = self.load_registry()?;
            let Some(record) = registry.get_mut(task_id) else {
                return Ok(false);
            };
            if is_completed_terminal_status(&record.status) {
                return Ok(false);
            }
            let Some(intent) = record.cancel_intent.as_mut() else {
                return Ok(false);
            };
            if intent.action_id != action_id
                || intent.task_id != record.req_id
                || intent.task_started_at_ms != record.started_at_ms
            {
                return Ok(false);
            }
            if intent.side_effect.is_some() {
                return Ok(true);
            }
            intent.side_effect = Some(CancelSideEffectCommit {
                target_kind: target_kind.to_string(),
                target_id: target_id.to_string(),
                committed_at_ms: now,
            });
            record.updated_at_ms = now;
            self.save_registry(&registry)?;
            self.append_event(json!({
                "type": "cancel_side_effect_committed",
                "req_id": task_id,
                "action_id": action_id,
                "target_kind": target_kind,
                "target_id": target_id,
                "at_ms": now,
            }))?;
            Ok(true)
        })
    }
}

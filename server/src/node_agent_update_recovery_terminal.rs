//! Idempotent completion reconciliation for restart recovery receipts.

use anyhow::{bail, Result};

use crate::node_agent_update_recovery::{UpdateRecoveryState, UpdateRecoveryStore};

impl UpdateRecoveryStore {
    pub(crate) fn reconcile_terminal_completion(
        &self,
        task_id: &str,
        event_id: &str,
        status: &str,
        finished_at_ms: u128,
        success: bool,
    ) -> Result<bool> {
        let mut ledger = self.load()?;
        let Some(receipt) = ledger
            .receipts
            .iter_mut()
            .filter(|receipt| receipt.active_task_id() == task_id)
            .max_by_key(|receipt| receipt.updated_at_ms)
        else {
            return Ok(false);
        };
        if let Some(current) = receipt.completion_event_id.as_deref() {
            if current != event_id {
                bail!("recovery receipt already binds a different completion event");
            }
        } else {
            receipt.completion_event_id = Some(event_id.to_string());
        }
        receipt.terminal_task_status = Some(status.to_string());
        receipt.terminal_finished_at_ms = Some(finished_at_ms);
        if !receipt.state.is_terminal() {
            receipt.transition(
                if success {
                    UpdateRecoveryState::Verified
                } else {
                    UpdateRecoveryState::Failed
                },
                Some(if success {
                    "durable recovered task completion reconciled"
                } else {
                    "durable recovered task failure reconciled"
                }),
            )?;
        }
        self.save(&ledger)?;
        Ok(true)
    }
}

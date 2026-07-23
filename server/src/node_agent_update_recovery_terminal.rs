//! Strict read-only preflight and idempotent terminal recovery binding.

use anyhow::{bail, Result};

use crate::node_agent_update_recovery::{
    UpdateRecoveryReceipt, UpdateRecoveryState, UpdateRecoveryStore,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ExpectedRecovery {
    NotApplicable,
    Required,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TerminalRecoveryDisposition {
    NotApplicable,
    Reconciled,
}

impl UpdateRecoveryStore {
    pub(crate) fn terminal_receipt_for_task(
        &self,
        task_id: &str,
    ) -> Result<Option<UpdateRecoveryReceipt>> {
        let matches = self
            .load()?
            .receipts
            .into_iter()
            .filter(|receipt| receipt.active_task_id() == task_id && !receipt.is_superseded())
            .collect::<Vec<_>>();
        match matches.len() {
            0 => Ok(None),
            1 => Ok(matches.into_iter().next()),
            _ => bail!("multiple update recovery receipts target the same active task"),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn preflight_terminal_completion(
        &self,
        expected: ExpectedRecovery,
        task_id: &str,
        event_id: &str,
        status: &str,
        finished_at_ms: u128,
        success: bool,
        outcome: Option<&str>,
    ) -> Result<TerminalRecoveryDisposition> {
        let Some(receipt) = self.terminal_receipt_for_task(task_id)? else {
            return match expected {
                ExpectedRecovery::NotApplicable => Ok(TerminalRecoveryDisposition::NotApplicable),
                ExpectedRecovery::Required => {
                    bail!("expected recovery task has no durable recovery receipt")
                }
            };
        };
        anyhow::ensure!(
            expected == ExpectedRecovery::Required,
            "ordinary terminal task unexpectedly matches a recovery receipt"
        );
        validate_receipt_binding(&receipt, event_id, status, finished_at_ms, success, outcome)?;
        Ok(TerminalRecoveryDisposition::Reconciled)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn reconcile_terminal_completion(
        &self,
        expected: ExpectedRecovery,
        task_id: &str,
        event_id: &str,
        status: &str,
        finished_at_ms: u128,
        success: bool,
        outcome: Option<&str>,
    ) -> Result<TerminalRecoveryDisposition> {
        let _guard = crate::node_agent_update_recovery::ledger_mutation_guard();
        let mut ledger = self.load()?;
        let matches = ledger
            .receipts
            .iter_mut()
            .filter(|receipt| receipt.active_task_id() == task_id && !receipt.is_superseded())
            .collect::<Vec<_>>();
        if matches.is_empty() {
            return match expected {
                ExpectedRecovery::NotApplicable => Ok(TerminalRecoveryDisposition::NotApplicable),
                ExpectedRecovery::Required => {
                    bail!("expected recovery task has no durable recovery receipt")
                }
            };
        }
        anyhow::ensure!(
            expected == ExpectedRecovery::Required,
            "ordinary terminal task unexpectedly matches a recovery receipt"
        );
        anyhow::ensure!(
            matches.len() == 1,
            "multiple update recovery receipts target the same active task"
        );
        let receipt = matches.into_iter().next().expect("one recovery receipt");
        validate_receipt_binding(receipt, event_id, status, finished_at_ms, success, outcome)?;
        if receipt.completion_event_id.is_none() {
            receipt.completion_event_id = Some(event_id.to_string());
            receipt.terminal_task_status = Some(status.to_string());
            receipt.terminal_finished_at_ms = Some(finished_at_ms);
            receipt.terminal_success = Some(success);
            receipt.terminal_outcome = outcome.map(str::to_string);
        }
        let expected_state = terminal_state(success);
        if receipt.state.is_terminal() {
            anyhow::ensure!(
                receipt.state == expected_state,
                "recovery terminal state conflicts with completion"
            );
        } else {
            receipt.transition(
                expected_state,
                Some(if success {
                    "trusted recovered task success reconciled"
                } else {
                    "trusted recovered task business failure reconciled"
                }),
            )?;
        }
        let identity = (receipt.update_id.clone(), receipt.original_task_id.clone());
        self.save(&ledger)?;

        let durable = self.load()?;
        let matches = durable
            .receipts
            .iter()
            .filter(|receipt| {
                receipt.update_id == identity.0 && receipt.original_task_id == identity.1
            })
            .collect::<Vec<_>>();
        anyhow::ensure!(
            matches.len() == 1,
            "recovery terminal binding is not uniquely durable"
        );
        validate_bound_receipt(
            matches[0],
            event_id,
            status,
            finished_at_ms,
            success,
            outcome,
        )?;
        Ok(TerminalRecoveryDisposition::Reconciled)
    }
}

fn validate_receipt_binding(
    receipt: &UpdateRecoveryReceipt,
    event_id: &str,
    status: &str,
    finished_at_ms: u128,
    success: bool,
    outcome: Option<&str>,
) -> Result<()> {
    anyhow::ensure!(
        receipt.allows_local_reconcile(),
        "recovery receipt protocol or transport is not trusted"
    );
    if receipt.completion_event_id.is_some() {
        validate_bound_receipt(receipt, event_id, status, finished_at_ms, success, outcome)?;
    } else {
        anyhow::ensure!(
            receipt.terminal_task_status.is_none()
                && receipt.terminal_finished_at_ms.is_none()
                && receipt.terminal_success.is_none()
                && receipt.terminal_outcome.is_none(),
            "recovery receipt contains a partial terminal binding"
        );
    }
    if receipt.state.is_terminal() {
        anyhow::ensure!(
            receipt.state == terminal_state(success),
            "recovery terminal state conflicts with completion"
        );
    } else {
        anyhow::ensure!(
            receipt.state.can_transition_to(terminal_state(success)),
            "recovery state cannot accept this terminal completion"
        );
    }
    Ok(())
}

fn validate_bound_receipt(
    receipt: &UpdateRecoveryReceipt,
    event_id: &str,
    status: &str,
    finished_at_ms: u128,
    success: bool,
    outcome: Option<&str>,
) -> Result<()> {
    anyhow::ensure!(
        receipt.completion_event_id.as_deref() == Some(event_id)
            && receipt.terminal_task_status.as_deref() == Some(status)
            && receipt.terminal_finished_at_ms == Some(finished_at_ms)
            && receipt.terminal_success == Some(success)
            && receipt.terminal_outcome.as_deref() == outcome
            && receipt.state == terminal_state(success),
        "same recovery completion conflicts with event, status, success, finished time, outcome, or state"
    );
    Ok(())
}

fn terminal_state(success: bool) -> UpdateRecoveryState {
    if success {
        UpdateRecoveryState::Verified
    } else {
        UpdateRecoveryState::Failed
    }
}

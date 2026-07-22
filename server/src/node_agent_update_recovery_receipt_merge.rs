use anyhow::{bail, Context, Result};

use super::UpdateRecoveryReceipt;

pub(super) fn canonical_terminal_receipt(
    matches: &[UpdateRecoveryReceipt],
) -> Result<UpdateRecoveryReceipt> {
    let first = matches.first().context("recovery receipt set is empty")?;
    if matches.iter().any(|receipt| {
        !receipt.state.is_terminal()
            || receipt.root_task_id != first.root_task_id
            || receipt.original_task_id != first.original_task_id
            || receipt.state != first.state
    }) {
        bail!("conflicting update recovery receipts target the same task")
    }
    ensure_optional_terminal_fact_agrees(
        matches
            .iter()
            .map(|receipt| receipt.terminal_task_status.as_ref()),
        "terminal_task_status",
    )?;
    ensure_optional_terminal_fact_agrees(
        matches
            .iter()
            .map(|receipt| receipt.terminal_finished_at_ms.as_ref()),
        "terminal_finished_at_ms",
    )?;
    ensure_optional_terminal_fact_agrees(
        matches
            .iter()
            .map(|receipt| receipt.terminal_success.as_ref()),
        "terminal_success",
    )?;
    ensure_optional_terminal_fact_agrees(
        matches
            .iter()
            .map(|receipt| receipt.terminal_outcome.as_ref()),
        "terminal_outcome",
    )?;
    ensure_optional_terminal_fact_agrees(
        matches
            .iter()
            .map(|receipt| receipt.completion_event_id.as_ref()),
        "completion_event_id",
    )?;

    let mut canonical = matches
        .iter()
        .max_by_key(|receipt| (terminal_fact_score(receipt), receipt.updated_at_ms))
        .expect("non-empty receipt set")
        .clone();
    for receipt in matches {
        canonical.terminal_task_status = canonical
            .terminal_task_status
            .or_else(|| receipt.terminal_task_status.clone());
        canonical.terminal_finished_at_ms = canonical
            .terminal_finished_at_ms
            .or(receipt.terminal_finished_at_ms);
        canonical.terminal_success = canonical.terminal_success.or(receipt.terminal_success);
        canonical.terminal_outcome = canonical
            .terminal_outcome
            .or_else(|| receipt.terminal_outcome.clone());
        canonical.completion_event_id = canonical
            .completion_event_id
            .or_else(|| receipt.completion_event_id.clone());
    }
    Ok(canonical)
}

fn ensure_optional_terminal_fact_agrees<'a, T>(
    values: impl Iterator<Item = Option<&'a T>>,
    label: &str,
) -> Result<()>
where
    T: Eq + 'a,
{
    let mut observed: Option<&T> = None;
    for value in values.flatten() {
        if observed.is_some_and(|current| current != value) {
            bail!("conflicting update recovery receipts disagree on {label}")
        }
        observed = Some(value);
    }
    Ok(())
}

fn terminal_fact_score(receipt: &UpdateRecoveryReceipt) -> usize {
    usize::from(receipt.terminal_task_status.is_some())
        + usize::from(receipt.terminal_finished_at_ms.is_some())
        + usize::from(receipt.terminal_success.is_some())
        + usize::from(receipt.terminal_outcome.is_some())
        + usize::from(receipt.completion_event_id.is_some())
}

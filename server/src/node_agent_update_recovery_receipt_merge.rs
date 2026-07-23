use anyhow::{bail, Context, Result};

use super::{UpdateRecoveryReceipt, UpdateRecoveryState, LEGACY_SNAPSHOT_APPLYING_REASON};

pub(super) fn canonical_terminal_receipt(
    matches: &[UpdateRecoveryReceipt],
) -> Result<UpdateRecoveryReceipt> {
    canonical_or_conflict(matches, merge_compatible_terminal_receipts)
}

pub(super) fn canonical_legacy_snapshot_receipt(
    matches: &[UpdateRecoveryReceipt],
) -> Result<UpdateRecoveryReceipt> {
    canonical_or_conflict(matches, merge_compatible_legacy_snapshot_receipts)
}

fn canonical_or_conflict(
    matches: &[UpdateRecoveryReceipt],
    merge: fn(&[UpdateRecoveryReceipt]) -> Result<UpdateRecoveryReceipt>,
) -> Result<UpdateRecoveryReceipt> {
    match merge(matches) {
        Ok(receipt) => Ok(receipt),
        Err(error) => {
            let mut conservative = matches
                .iter()
                .max_by_key(|receipt| receipt.updated_at_ms)
                .context("recovery receipt set is empty")?
                .clone();
            conservative.conflict_detected = true;
            conservative.conflict_count = matches.len();
            conservative.conflict_reason = Some(error.to_string());
            Ok(conservative)
        }
    }
}

fn merge_compatible_terminal_receipts(
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

fn merge_compatible_legacy_snapshot_receipts(
    matches: &[UpdateRecoveryReceipt],
) -> Result<UpdateRecoveryReceipt> {
    let first = matches.first().context("recovery receipt set is empty")?;
    ensure_legacy_snapshot_receipt(first)?;
    for receipt in matches.iter().skip(1) {
        ensure_legacy_snapshot_receipt(receipt)?;
        if !legacy_snapshot_facts_match(first, receipt) {
            bail!("conflicting legacy snapshot recovery receipts target the same task")
        }
    }
    Ok(matches
        .iter()
        .max_by_key(|receipt| receipt.updated_at_ms)
        .expect("non-empty receipt set")
        .clone())
}

fn ensure_legacy_snapshot_receipt(receipt: &UpdateRecoveryReceipt) -> Result<()> {
    if !receipt.update_id.starts_with("legacy-sidecar-")
        || receipt.is_superseded()
        || receipt.conflict_detected
        || receipt.state != UpdateRecoveryState::Applying
        || receipt.state_reason.as_deref() != Some(LEGACY_SNAPSHOT_APPLYING_REASON)
        || receipt
            .sidecar_session_id
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        || receipt.completion_event_id.is_some()
        || receipt.terminal_task_status.is_some()
        || receipt.terminal_finished_at_ms.is_some()
        || receipt.terminal_success.is_some()
        || receipt.terminal_outcome.is_some()
        || receipt.final_reason.is_some()
        || receipt.final_review.is_some()
        || receipt.superseded_by_release.is_some()
        || receipt.supersede_evidence.is_some()
        || receipt.superseded_at_ms.is_some()
        || receipt.conflict_count != 0
        || receipt.conflict_reason.is_some()
        || !receipt.recovery_policy.allow_snapshot_continue
        || !receipt.workspace.isolated
        || receipt.workspace.git_status_clean != Some(true)
        || !receipt.safety.evidence_complete
        || !receipt.safety.pending_approval_ids.is_empty()
        || receipt.safety.non_repeatable_action.is_some()
    {
        bail!("recovery receipt is not an eligible preserved legacy snapshot")
    }
    Ok(())
}

fn legacy_snapshot_facts_match(
    left: &UpdateRecoveryReceipt,
    right: &UpdateRecoveryReceipt,
) -> bool {
    left.schema_version == right.schema_version
        && left.protocol == right.protocol
        && left.root_task_id == right.root_task_id
        && left.parent_task_id == right.parent_task_id
        && left.original_task_id == right.original_task_id
        && left.resume_task_id == right.resume_task_id
        && left.codex_session_id == right.codex_session_id
        && left.codex_session_scope == right.codex_session_scope
        && left.sidecar_session_id == right.sidecar_session_id
        && left.journal_cursor == right.journal_cursor
        && left.sidecar_output_offset == right.sidecar_output_offset
        && left.sidecar_output_sequence == right.sidecar_output_sequence
        && left.expected_downtime_ms == right.expected_downtime_ms
        && left.workspace.base_workspace_path == right.workspace.base_workspace_path
        && left.workspace.workspace_path == right.workspace.workspace_path
        && left.workspace.isolated == right.workspace.isolated
        && left.workspace.branch == right.workspace.branch
        && left.workspace.git_head == right.workspace.git_head
        && left.workspace.git_status_clean == right.workspace.git_status_clean
        && left.transport.kind == right.transport.kind
        && left.transport.protocol == right.transport.protocol
        && left.transport.capabilities == right.transport.capabilities
        && left.transport.auth_mode == right.transport.auth_mode
        && left.transport.lease_id == right.transport.lease_id
        && left.transport.replay_from_cursor == right.transport.replay_from_cursor
        && left.recovery_policy.mode == right.recovery_policy.mode
        && left.recovery_policy.allow_snapshot_continue
            == right.recovery_policy.allow_snapshot_continue
        && left.safety == right.safety
        && left.resume_strategy == right.resume_strategy
        && left.completion_event_id == right.completion_event_id
        && left.terminal_task_status == right.terminal_task_status
        && left.terminal_success == right.terminal_success
        && left.terminal_outcome == right.terminal_outcome
        && left.state == right.state
        && left.state_reason == right.state_reason
        && left.final_reason == right.final_reason
        && semantic_events_match(&left.events, &right.events)
}

fn semantic_events_match(
    left: &[super::UpdateRecoveryEvent],
    right: &[super::UpdateRecoveryEvent],
) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left.sequence == right.sequence
                && left.state == right.state
                && left.reason == right.reason
        })
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

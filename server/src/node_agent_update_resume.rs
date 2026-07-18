//! Explicit continuation of a safety-paused update recovery after Desktop review.

use std::{path::Path, sync::Arc};

use anyhow::{bail, Context, Result};

use crate::{
    node_agent_update_checkpoint::fingerprint_workspace,
    node_agent_update_recovery::{UpdateRecoveryReceipt, UpdateRecoveryState},
    NodeRuntime,
};

pub(crate) async fn resume_reviewed(
    runtime: Arc<NodeRuntime>,
    task_id: &str,
) -> Result<UpdateRecoveryReceipt> {
    let receipt = runtime
        .update_recovery
        .receipt_for_task(task_id)?
        .context("task has no durable update recovery receipt")?;
    if !matches!(
        receipt.state,
        UpdateRecoveryState::Paused
            | UpdateRecoveryState::ApprovalRequired
            | UpdateRecoveryState::Conflict
            | UpdateRecoveryState::Timeout
    ) {
        bail!("update recovery is not waiting for an explicit resume");
    }
    if receipt
        .final_review
        .as_ref()
        .map(|review| review.verdict.as_str())
        != Some("accepted")
    {
        bail!("Desktop final review must be accepted before update recovery can resume");
    }
    if receipt.transport.kind != "local_loopback"
        || !receipt.transport.supports("update_recovery_v1")
        || !receipt.transport.replay_from_cursor
    {
        bail!("remote v1 update recovery is retained but unverified; resume fails closed");
    }
    let active_task_id = receipt.active_task_id().to_string();
    let snapshot = runtime.task_journal.snapshot(&active_task_id, 0, 200)?;
    if snapshot.approvals.pending_count > 0 {
        bail!("pending tool approval must be decided before update recovery can resume");
    }
    let fingerprint = fingerprint_workspace(Path::new(&receipt.workspace.workspace_path));
    if !fingerprint.has_sufficient_identity() {
        bail!("current workspace does not provide sufficient Git recovery evidence");
    }
    if fingerprint != receipt.workspace {
        bail!("workspace Git identity changed after checkpoint; explicit resume is unsafe");
    }
    if !runtime
        .active_cli_prompt_views_for_workspace(Path::new(&receipt.workspace.workspace_path))
        .await
        .is_empty()
    {
        bail!("workspace is occupied by an active foreground task");
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    if runtime
        .cli_sidecars
        .all_sessions()?
        .into_iter()
        .any(|session| {
            session.task_id != active_task_id
                && session.is_live_at(now)
                && session.cwd.as_deref().is_some_and(|cwd| {
                    paths_match(Path::new(cwd), Path::new(&receipt.workspace.workspace_path))
                })
        })
    {
        bail!("workspace is occupied by another live sidecar");
    }
    runtime
        .update_recovery
        .update(&receipt.update_id, &receipt.original_task_id, |current| {
            current.safety.pending_approval_ids.clear();
            current.safety.non_repeatable_action = None;
            current.transition(
                UpdateRecoveryState::RuntimeOnline,
                Some("Desktop accepted recovery evidence and requested resume"),
            )?;
            Ok(())
        })?;
    crate::node_agent_update_reconcile::reconcile_startup(runtime.clone()).await;
    runtime
        .update_recovery
        .receipt_for_task(task_id)?
        .context("recovery receipt disappeared after resume")
}

fn paths_match(left: &Path, right: &Path) -> bool {
    let left = std::fs::canonicalize(left).ok();
    let right = std::fs::canonicalize(right).ok();
    match (left, right) {
        (Some(left), Some(right)) if cfg!(windows) => left
            .to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy()),
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

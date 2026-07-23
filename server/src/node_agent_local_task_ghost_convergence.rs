//! Conservative convergence for historical running rows whose execution ownership is gone.

use std::{collections::HashSet, path::Path};

use anyhow::Result;
use tracing::info;

use crate::{
    node_agent_local_task_store::LocalTaskRecord,
    node_agent_update_recovery::UpdateRecoveryReceipt, NodeRuntime,
};

pub(super) fn converge_verified_history(
    runtime: &NodeRuntime,
    task: &LocalTaskRecord,
    active_workspace: &Path,
    cutoff: i64,
) -> Result<bool> {
    let Some(head) = verified_history_head(runtime, task, active_workspace)? else {
        return Ok(false);
    };
    let changed = runtime
        .local_tasks
        .mark_one_stale_without_runtime(&task.task_id, cutoff)?;
    if changed {
        info!(
            task_id = %task.task_id,
            git_head = head,
            "converged ghost running history from terminal journal, merged commit, missing workspace, and no execution owner"
        );
    }
    Ok(changed)
}

pub(crate) fn receipt_conflict_is_audit_only(
    runtime: &NodeRuntime,
    task: &LocalTaskRecord,
    active_workspace: &Path,
) -> Result<bool> {
    Ok(verified_history_head(runtime, task, active_workspace)?.is_some())
}

fn verified_history_head(
    runtime: &NodeRuntime,
    task: &LocalTaskRecord,
    active_workspace: &Path,
) -> Result<Option<String>> {
    if active_workspace.exists() {
        return Ok(None);
    }
    let snapshot = runtime.task_journal.snapshot(&task.task_id, 0, 10_000)?;
    let Some(record) = snapshot.record.as_ref() else {
        return Ok(None);
    };
    if !journal_is_terminal(&record.status)
        || snapshot.has_more
        || !snapshot.approvals.pending_approval_ids().is_empty()
        || crate::node_agent_update_checkpoint::incomplete_non_repeatable_action(&snapshot.events)
            .is_some()
    {
        return Ok(None);
    }
    let receipts = runtime.update_recovery.receipts_for_task(&task.task_id)?;
    let Some(head) = single_audited_head(&receipts) else {
        return Ok(None);
    };
    let Some(base) = receipts.iter().find_map(|receipt| {
        receipt
            .workspace
            .base_workspace_path
            .as_deref()
            .map(Path::new)
            .filter(|path| path.is_dir())
    }) else {
        return Ok(None);
    };
    if crate::node_agent_update_checkpoint::git_output(
        base,
        &["merge-base", "--is-ancestor", head, "origin/main"],
    )
    .is_none()
    {
        return Ok(None);
    }
    Ok(Some(head.to_string()))
}

fn journal_is_terminal(status: &str) -> bool {
    matches!(
        status,
        "finished" | "done" | "failed" | "canceled" | "resume_required"
    )
}

fn single_audited_head(receipts: &[UpdateRecoveryReceipt]) -> Option<&str> {
    if receipts.is_empty() {
        return None;
    }
    let heads = receipts
        .iter()
        .filter_map(|receipt| {
            receipt
                .workspace
                .git_head
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
        .collect::<HashSet<_>>();
    (heads.len() == 1).then(|| *heads.iter().next().expect("one head"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conflicting_receipts_need_one_shared_nonempty_commit() {
        let mut first = UpdateRecoveryReceipt::planned("a", "root", "task");
        let mut second = UpdateRecoveryReceipt::planned("b", "root", "task");
        first.workspace.git_head = Some("abc".into());
        second.workspace.git_head = Some("abc".into());
        assert_eq!(
            single_audited_head(&[first.clone(), second.clone()]),
            Some("abc")
        );
        second.workspace.git_head = Some("def".into());
        assert_eq!(single_audited_head(&[first, second]), None);
    }

    #[test]
    fn only_terminal_journal_can_prove_ghost_history() {
        assert!(journal_is_terminal("finished"));
        assert!(!journal_is_terminal("running"));
    }
}

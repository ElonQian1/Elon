//! Narrow compatibility gate for a supervised task whose durable startup HEAD
//! predates its own platform-observed terminal commit.

use std::path::Path;

use anyhow::{anyhow, Context, Result};

use super::{git_output, same_path, validate_snapshot_continue_head};
use crate::{
    git_command_error::git_command,
    node_agent_local_task_store::LocalTaskRecord,
    node_agent_local_task_supervision::SupervisionContract,
    node_agent_update_recovery::{UpdateRecoveryReceipt, UpdateRecoveryState},
};

#[allow(clippy::too_many_arguments)]
pub(super) fn validate(
    contract: &SupervisionContract,
    parent: &LocalTaskRecord,
    receipt: Option<&UpdateRecoveryReceipt>,
    base: &Path,
    active: &Path,
    expected_branch: &str,
    recorded_head: &str,
    current_head: &str,
) -> Result<()> {
    let receipt =
        receipt.ok_or_else(|| anyhow!("缺少可验证的终态更新恢复回执，拒绝接受 HEAD 前进。"))?;
    anyhow::ensure!(
        receipt.allows_local_reconcile() && !receipt.is_superseded() && !receipt.conflict_detected,
        "更新恢复回执不可信、已被取代或存在冲突，拒绝接受 HEAD 前进。"
    );
    anyhow::ensure!(
        receipt.active_task_id() == parent.task_id,
        "更新恢复回执不属于当前父任务。"
    );
    let expected_root = contract
        .root_task_id
        .as_deref()
        .unwrap_or(parent.task_id.as_str());
    anyhow::ensure!(
        receipt.root_task_id == expected_root,
        "更新恢复回执的监督 root identity 不一致。"
    );
    validate_terminal_binding(parent, receipt)?;
    validate_receipt_workspace(receipt, base, active, expected_branch, current_head)?;
    anyhow::ensure!(
        git_output(
            active,
            &["status", "--porcelain=v1", "--untracked-files=all"]
        )?
        .is_empty(),
        "父任务活动 worktree 非 clean，拒绝接受 HEAD 前进。"
    );
    validate_forward_landed_commit(base, recorded_head, current_head)
}

fn validate_terminal_binding(
    parent: &LocalTaskRecord,
    receipt: &UpdateRecoveryReceipt,
) -> Result<()> {
    let completion_event_id = parent
        .completion_event_id
        .as_deref()
        .ok_or_else(|| anyhow!("父任务缺少平台终态 completion event。"))?;
    let finished_at_ms = parent
        .finished_at_ms
        .and_then(|value| u128::try_from(value).ok())
        .ok_or_else(|| anyhow!("父任务缺少有效终态时间。"))?;
    anyhow::ensure!(
        receipt.completion_event_id.as_deref() == Some(completion_event_id)
            && receipt.terminal_task_status.as_deref() == Some(parent.status.as_str())
            && receipt.terminal_finished_at_ms == Some(finished_at_ms)
            && matches!(
                (receipt.state, receipt.terminal_success),
                (UpdateRecoveryState::Verified, Some(true))
                    | (UpdateRecoveryState::Failed, Some(false))
            ),
        "更新恢复回执没有与父任务终态原子绑定。"
    );
    anyhow::ensure!(
        receipt.safety.evidence_complete
            && receipt.safety.pending_approval_ids.is_empty()
            && receipt.safety.non_repeatable_action.is_none(),
        "更新恢复回执缺少完整、可重复的安全证据。"
    );
    Ok(())
}

fn validate_receipt_workspace(
    receipt: &UpdateRecoveryReceipt,
    base: &Path,
    active: &Path,
    expected_branch: &str,
    current_head: &str,
) -> Result<()> {
    let workspace = &receipt.workspace;
    anyhow::ensure!(
        workspace.isolated
            && workspace.has_sufficient_identity()
            && workspace.git_status_clean == Some(true),
        "更新恢复回执没有证明 clean 的隔离 worktree。"
    );
    let receipt_base = workspace
        .base_workspace_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(Path::new)
        .ok_or_else(|| anyhow!("更新恢复回执缺少基础仓库路径。"))?;
    anyhow::ensure!(
        same_path(receipt_base, base)
            && same_path(Path::new(&workspace.workspace_path), active)
            && workspace.branch.as_deref() == Some(expected_branch)
            && workspace
                .git_head
                .as_deref()
                .is_some_and(|head| head.eq_ignore_ascii_case(current_head)),
        "更新恢复回执的路径、分支或 HEAD 与当前 worktree 不一致。"
    );
    Ok(())
}

fn validate_forward_landed_commit(
    base: &Path,
    recorded_head: &str,
    current_head: &str,
) -> Result<()> {
    let recorded_commit = git_output(
        base,
        &[
            "rev-parse",
            "--verify",
            &format!("{recorded_head}^{{commit}}"),
        ],
    )?;
    anyhow::ensure!(
        recorded_commit.eq_ignore_ascii_case(recorded_head),
        "父任务记录的 git_head 不是完整 commit。"
    );
    let ancestry = git_command()
        .args(["merge-base", "--is-ancestor", recorded_head, current_head])
        .current_dir(base)
        .status()
        .context("验证父任务合法 HEAD 前进谱系失败")?;
    anyhow::ensure!(
        ancestry.success(),
        "当前 HEAD 不是记录 HEAD 的后继，疑似回退或历史篡改。"
    );
    validate_snapshot_continue_head(base, current_head)
        .context("当前 HEAD 缺少 origin/main 终态提交证据")
}

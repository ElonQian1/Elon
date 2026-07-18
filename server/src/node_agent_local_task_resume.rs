//! Admission rules for reusing a terminated supervised task's isolated worktree.
//!
//! This is deliberately narrower than Route A full-access grants. A resume may
//! reuse only the exact worktree recorded by the node for its parent task, while
//! authorization remains anchored to the parent's already-authorized base repo.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use elon_pc_dev_runtime::safe_path_part;

use crate::{
    git_command_error::{git_command, git_failure_message, git_spawn_context},
    node_agent_local_task_store::LocalTaskRecord,
    node_agent_local_task_supervision::{SupervisionContract, SUPERVISION_PROTOCOL},
    pc_workspace_provisioner::ConversationWorkspaceResult,
};

#[path = "node_agent_local_task_resume_recovery.rs"]
mod recovery;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ResumeWorkspaceMode {
    Inspect,
    Acquire,
}

#[derive(Debug)]
pub(crate) struct ResolvedResumeWorkspace {
    pub authorized_workspace_path: String,
    pub inherited_workspace: ConversationWorkspaceResult,
    pub derivation: String,
    pub git_head: String,
    pub requires_recreation: bool,
}

pub(crate) fn resolve_resume_workspace(
    contract: &SupervisionContract,
    parent: &LocalTaskRecord,
    journal_record: Option<&crate::node_agent_task_journal::TaskJournalRecord>,
    requested_project_id: &str,
    requested_workspace_path: &str,
    receipt: Option<&crate::node_agent_update_recovery::UpdateRecoveryReceipt>,
    mode: ResumeWorkspaceMode,
) -> Result<ResolvedResumeWorkspace> {
    match resolve_existing_resume_workspace(
        contract,
        parent,
        journal_record,
        requested_project_id,
        requested_workspace_path,
        mode == ResumeWorkspaceMode::Acquire,
    ) {
        Ok(workspace) => Ok(workspace),
        Err(existing_error) => {
            let Some(receipt) = receipt else {
                return Err(existing_error);
            };
            crate::node_agent_local_task_resume_rebuild::resolve_recycled_resume_workspace(
                contract,
                parent,
                requested_project_id,
                requested_workspace_path,
                receipt,
                mode,
            )
            .with_context(|| format!("existing worktree unavailable: {existing_error}"))
        }
    }
}

pub(crate) fn validate_resume_workspace(
    contract: &SupervisionContract,
    parent: &LocalTaskRecord,
    journal_record: Option<&crate::node_agent_task_journal::TaskJournalRecord>,
    requested_project_id: &str,
    requested_workspace_path: &str,
) -> Result<ResolvedResumeWorkspace> {
    resolve_existing_resume_workspace(
        contract,
        parent,
        journal_record,
        requested_project_id,
        requested_workspace_path,
        true,
    )
}

pub(crate) fn inspect_resume_workspace(
    contract: &SupervisionContract,
    parent: &LocalTaskRecord,
    journal_record: Option<&crate::node_agent_task_journal::TaskJournalRecord>,
    requested_project_id: &str,
    requested_workspace_path: &str,
) -> Result<ResolvedResumeWorkspace> {
    resolve_existing_resume_workspace(
        contract,
        parent,
        journal_record,
        requested_project_id,
        requested_workspace_path,
        false,
    )
}

fn resolve_existing_resume_workspace(
    contract: &SupervisionContract,
    parent: &LocalTaskRecord,
    journal_record: Option<&crate::node_agent_task_journal::TaskJournalRecord>,
    requested_project_id: &str,
    requested_workspace_path: &str,
    repair_missing_git_metadata: bool,
) -> Result<ResolvedResumeWorkspace> {
    if contract.protocol != SUPERVISION_PROTOCOL || contract.task_role != "resume_original" {
        bail!("只有当前监督协议的 resume_original 可以继承父任务工作区。");
    }
    let parent_task_id = contract
        .parent_task_id
        .as_deref()
        .ok_or_else(|| anyhow!("resume_original 缺少 parent_task_id，已拒绝继承工作区。"))?;
    if parent_task_id != parent.task_id {
        bail!("resume_original 的 parent_task_id 与父任务记录不一致。");
    }
    let supervision_root_task_id = contract.root_task_id.as_deref().unwrap_or(parent_task_id);
    if !crate::node_agent_full_access::project_ids_equivalent(
        requested_project_id,
        &parent.project_id,
    ) {
        bail!("resume_original 不能跨项目继承父任务工作区。");
    }
    if !parent_is_terminal(parent) {
        bail!("父任务仍在运行或没有可靠终态，不能继承其工作区。");
    }

    let project_part = safe_path_part(&parent.project_id, "project", 80);
    let conversation_part = safe_path_part(&parent.conversation_id, "conversation", 80);
    let expected_branch = format!("ai/session/{project_part}/{conversation_part}");
    let authorized_base = canonical_directory(Path::new(&parent.workspace_path), "父任务授权根")?;
    let (recorded_base, active, status_branch, recorded_head, mut derivation) =
        if let Some(status) = parent.workspace_status.as_ref() {
            if status.get("isolated").and_then(serde_json::Value::as_bool) != Some(true) {
                bail!("父任务工作区不是平台生成的隔离 worktree。");
            }
            let status_base = required_status_path(status, "base_workspace_path")?;
            let status_active = required_status_path(status, "active_workspace_path")?;
            let status_branch = status
                .get("branch")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| anyhow!("父任务 workspace_status 缺少隔离分支。"))?;
            (
                canonical_directory(Path::new(status_base), "父任务记录的基础工作区")?,
                canonical_directory(Path::new(status_active), "父任务隔离 worktree")?,
                status_branch.to_string(),
                status
                    .get("git_head")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string),
                "workspace_status".to_string(),
            )
        } else {
            let journal = journal_record
                .filter(|record| record.req_id == parent.task_id)
                .ok_or_else(|| anyhow!("父任务缺少 workspace_status 和可验证的 started.cwd。"))?;
            let started_cwd = journal
                .cwd
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| anyhow!("父任务最新 started 记录缺少 cwd。"))?;
            (
                authorized_base.clone(),
                canonical_directory(Path::new(started_cwd), "父任务 started.cwd")?,
                expected_branch.clone(),
                None,
                "legacy_started_cwd_git_registry".to_string(),
            )
        };
    if !same_path(&authorized_base, &recorded_base) {
        bail!("父任务 workspace_status 的基础工作区与原授权根不一致。");
    }
    if same_path(&authorized_base, &active) {
        bail!("父任务活动工作区不是独立 worktree。");
    }

    let requested = canonical_directory(Path::new(requested_workspace_path), "续跑请求工作区")?;
    if !same_path(&requested, &authorized_base) && !same_path(&requested, &active) {
        bail!("续跑请求只能引用父任务原授权根或其已记录的隔离 worktree。");
    }

    validate_platform_worktree_shape(&active, &project_part, &conversation_part)?;
    if status_branch != expected_branch {
        bail!("父任务隔离分支不符合平台生成规则。");
    }
    let git_head = match validate_git_worktree_identity(&authorized_base, &active, &expected_branch)
    {
        Ok(head) => {
            if recorded_head
                .as_deref()
                .is_some_and(|recorded| !recorded.eq_ignore_ascii_case(&head))
            {
                bail!("父任务记录的 git_head 与活动 worktree 当前 HEAD 不一致。");
            }
            head
        }
        Err(identity_error) if !recovery::is_git_worktree(&active) => {
            let recovered = recovery::inspect_or_repair(
                &authorized_base,
                &active,
                &expected_branch,
                recorded_head.as_deref(),
                supervision_root_task_id,
                repair_missing_git_metadata,
            )
            .with_context(|| {
                format!(
                    "父任务活动目录缺少 Git worktree 注册；自动恢复校验失败（原始错误: {identity_error:#}）"
                )
            })?;
            derivation = recovered.derivation;
            recovered.git_head
        }
        Err(error) => return Err(error),
    };

    if recovery::is_git_worktree(&active) {
        let expected =
            crate::node_agent_supervision_worktree_lease::lease_reason(supervision_root_task_id)?;
        let actual = crate::node_agent_supervision_worktree_lease::worktree_lock_reason(
            &authorized_base,
            &active,
        )?;
        anyhow::ensure!(
            actual.as_deref() == Some(expected.as_str()),
            "父任务 worktree root lease 身份不匹配：expected {expected}, actual {}",
            actual.as_deref().unwrap_or("<unlocked>")
        );
    }

    let requires_recreation =
        !repair_missing_git_metadata && derivation.contains("_recovery_ready_");
    Ok(ResolvedResumeWorkspace {
        authorized_workspace_path: display_path(&authorized_base),
        inherited_workspace: ConversationWorkspaceResult {
            base_workspace_path: Some(display_path(&authorized_base)),
            workspace_path: display_path(&active),
            isolated: true,
            branch: Some(expected_branch),
            supervision_root_task_id: Some(supervision_root_task_id.to_string()),
        },
        derivation,
        git_head,
        requires_recreation,
    })
}

fn parent_is_terminal(parent: &LocalTaskRecord) -> bool {
    parent.finished_at_ms.is_some()
        && matches!(
            parent.status.as_str(),
            "done" | "failed" | "canceled" | "interrupted" | "resume_required"
        )
}

fn required_status_path<'a>(status: &'a serde_json::Value, field: &str) -> Result<&'a str> {
    status
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("父任务 workspace_status 缺少 {field}。"))
}

fn canonical_directory(path: &Path, label: &str) -> Result<PathBuf> {
    let canonical = std::fs::canonicalize(path)
        .with_context(|| format!("{label}不存在或不可访问: {}", path.display()))?;
    if !canonical.is_dir() {
        bail!("{label}不是目录: {}", canonical.display());
    }
    Ok(canonical)
}

fn validate_platform_worktree_shape(
    active: &Path,
    project_part: &str,
    conversation_part: &str,
) -> Result<()> {
    let conversation = active
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow!("父任务隔离 worktree 路径无效。"))?;
    let project = active
        .parent()
        .and_then(Path::file_name)
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow!("父任务隔离 worktree 缺少项目目录。"))?;
    let marker = active
        .parent()
        .and_then(Path::parent)
        .and_then(Path::file_name)
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow!("父任务隔离 worktree 缺少平台目录标记。"))?;
    if !same_component(conversation, conversation_part)
        || !same_component(project, project_part)
        || !same_component(marker, "conversation-worktrees")
    {
        bail!("父任务活动路径不是该项目、该会话的平台生成 worktree。");
    }
    Ok(())
}

fn validate_git_worktree_identity(
    base: &Path,
    active: &Path,
    expected_branch: &str,
) -> Result<String> {
    let base_common = git_resolved_path(base, &["rev-parse", "--git-common-dir"])?;
    let active_common = git_resolved_path(active, &["rev-parse", "--git-common-dir"])?;
    if !same_path(&base_common, &active_common) {
        bail!("父任务活动路径不属于原授权 Git 仓库。");
    }

    let top_level = git_resolved_path(active, &["rev-parse", "--show-toplevel"])?;
    if !same_path(&top_level, active) {
        bail!("父任务活动路径不是隔离 worktree 根目录。");
    }
    let branch = git_output(active, &["branch", "--show-current"])?;
    if branch.trim() != expected_branch {
        bail!("父任务活动 worktree 当前分支与平台记录不一致。");
    }
    if branch.trim() == "main" {
        bail!("禁止在 main 工作区续跑父任务。");
    }
    let head = git_output(active, &["rev-parse", "--verify", "HEAD^{commit}"])?;
    if head.trim().is_empty() {
        bail!("父任务活动 worktree 缺少可验证 HEAD。");
    }
    let worktree_list = git_output(base, &["worktree", "list", "--porcelain"])?;
    let expected_ref = format!("refs/heads/{expected_branch}");
    let registered = worktree_list.split("\n\n").any(|entry| {
        let mut path_matches = false;
        let mut registered_head = None;
        let mut registered_branch = None;
        for line in entry.lines() {
            if let Some(path) = line.strip_prefix("worktree ") {
                path_matches = std::fs::canonicalize(path)
                    .ok()
                    .is_some_and(|path| same_path(&path, active));
            } else if let Some(value) = line.strip_prefix("HEAD ") {
                registered_head = Some(value.trim());
            } else if let Some(value) = line.strip_prefix("branch ") {
                registered_branch = Some(value.trim());
            }
        }
        path_matches
            && registered_head == Some(head.trim())
            && registered_branch == Some(expected_ref.as_str())
    });
    if !registered {
        bail!("父任务活动路径、分支或 HEAD 与 Git worktree 注册表不一致。");
    }
    Ok(head)
}

fn git_resolved_path(cwd: &Path, args: &[&str]) -> Result<PathBuf> {
    let raw = git_output(cwd, args)?;
    let path = PathBuf::from(raw.trim());
    let path = if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    };
    canonical_directory(&path, "Git 工作区身份路径")
}

fn git_output(cwd: &Path, args: &[&str]) -> Result<String> {
    let output = git_command()
        .args(args)
        .current_dir(cwd)
        .output()
        .with_context(|| git_spawn_context(args))?;
    if !output.status.success() {
        bail!(git_failure_message(cwd, args, &output));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn same_path(left: &Path, right: &Path) -> bool {
    if cfg!(windows) {
        display_path(left).eq_ignore_ascii_case(&display_path(right))
    } else {
        left == right
    }
}

fn same_component(left: &str, right: &str) -> bool {
    if cfg!(windows) {
        left.eq_ignore_ascii_case(right)
    } else {
        left == right
    }
}

#[cfg(test)]
#[path = "node_agent_local_task_resume_tests.rs"]
mod tests;

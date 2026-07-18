//! Recreate a safely recycled supervised worktree from a trusted local receipt.
//!
//! The receipt supplies the immutable commit identity. Path, branch, task and
//! transport identities are independently re-derived before `git worktree add`.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use elon_pc_dev_runtime::safe_path_part;

use crate::{
    git_command_error::{git_command, git_failure_message, git_spawn_context},
    node_agent_local_task_resume::{
        recorded_platform_workspace_identity, validate_resume_workspace, ResolvedResumeWorkspace,
        ResumeWorkspaceMode,
    },
    node_agent_local_task_store::LocalTaskRecord,
    node_agent_local_task_supervision::{SupervisionContract, SUPERVISION_PROTOCOL},
    node_agent_update_recovery::{
        UpdateRecoveryReceipt, UPDATE_RECOVERY_PROTOCOL, UPDATE_RECOVERY_SCHEMA_VERSION,
    },
    pc_workspace_provisioner::ConversationWorkspaceResult,
};

pub(crate) fn resolve_recycled_resume_workspace(
    contract: &SupervisionContract,
    parent: &LocalTaskRecord,
    parent_contract: Option<&SupervisionContract>,
    requested_project_id: &str,
    requested_workspace_path: &str,
    receipt: &UpdateRecoveryReceipt,
    mode: ResumeWorkspaceMode,
) -> Result<ResolvedResumeWorkspace> {
    let identity = validate_receipt_identity(contract, parent, receipt)?;
    let supervision_root_task_id = contract
        .root_task_id
        .as_deref()
        .unwrap_or(parent.task_id.as_str());
    anyhow::ensure!(
        requested_project_id == parent.project_id,
        "recycled resume cannot cross projects"
    );
    let base = canonical_directory(Path::new(&parent.workspace_path), "authorized base repo")?;
    let (recorded_base, active, status_branch) = recorded_workspace(parent)?;
    let recorded_base = canonical_directory(&recorded_base, "recorded base repo")?;
    anyhow::ensure!(
        same_path(&base, &recorded_base),
        "recorded base repo drifted"
    );
    anyhow::ensure!(
        same_path(
            &absolute_path(Path::new(&receipt.workspace.workspace_path))?,
            &active
        ),
        "receipt workspace path does not match the parent record"
    );
    anyhow::ensure!(
        receipt.workspace.git_status_clean == Some(true),
        "receipt does not prove a clean parent worktree"
    );
    anyhow::ensure!(
        receipt.workspace.has_sufficient_identity(),
        "receipt workspace identity is incomplete"
    );
    anyhow::ensure!(
        !active.exists(),
        "recorded worktree still exists and must pass live validation"
    );

    let project_part = safe_path_part(&parent.project_id, "project", 80);
    let parent_conversation_part = safe_path_part(&parent.conversation_id, "conversation", 80);
    let platform_identity = recorded_platform_workspace_identity(
        contract,
        parent_contract,
        parent,
        &project_part,
        &parent_conversation_part,
        &status_branch,
    )?;
    let expected_branch = platform_identity.branch;
    validate_platform_path(&active, &project_part, &platform_identity.conversation_part)?;
    anyhow::ensure!(
        expected_branch != "main",
        "refusing to recreate main as a resume worktree"
    );

    let requested = absolute_path(Path::new(requested_workspace_path))?;
    let requested = std::fs::canonicalize(&requested).unwrap_or(requested);
    anyhow::ensure!(
        same_path(&requested, &base) || same_path(&requested, &active),
        "resume requested an arbitrary path"
    );
    validate_base_repo(&base)?;
    let head = receipt
        .workspace
        .git_head
        .as_deref()
        .map(str::trim)
        .filter(|value| valid_commit_id(value))
        .context("receipt commit identity is invalid")?;

    if mode == ResumeWorkspaceMode::Acquire {
        run_git(&base, &["fetch", "origin", "main:refs/remotes/origin/main"])?;
    }
    validate_commit_landed(&base, head)?;
    validate_unoccupied(&base, &active, &expected_branch)?;

    if mode == ResumeWorkspaceMode::Inspect {
        return Ok(ResolvedResumeWorkspace {
            authorized_workspace_path: display_path(&base),
            inherited_workspace: ConversationWorkspaceResult {
                base_workspace_path: Some(display_path(&base)),
                workspace_path: display_path(&active),
                isolated: true,
                branch: Some(expected_branch),
                supervision_root_task_id: Some(supervision_root_task_id.to_string()),
            },
            derivation: "platform_receipt_commit_rebuild_available".to_string(),
            git_head: head.to_string(),
            requires_recreation: true,
            lease_migration: None,
            resume_admission: None,
        });
    }

    recreate_worktree(
        &base,
        &active,
        &expected_branch,
        head,
        supervision_root_task_id,
    )?;
    let validated = validate_resume_workspace(
        contract,
        parent,
        parent_contract,
        None,
        requested_project_id,
        requested_workspace_path,
    );
    match validated {
        Ok(mut workspace) => {
            workspace.derivation = "platform_receipt_commit_rebuilt".to_string();
            workspace.git_head = identity;
            Ok(workspace)
        }
        Err(error) => {
            let _ = run_git(
                &base,
                &["worktree", "remove", "--force", &path_arg(&active)],
            );
            Err(error).context("recreated worktree failed final identity validation")
        }
    }
}

fn validate_receipt_identity(
    contract: &SupervisionContract,
    parent: &LocalTaskRecord,
    receipt: &UpdateRecoveryReceipt,
) -> Result<String> {
    anyhow::ensure!(
        contract.protocol == SUPERVISION_PROTOCOL && contract.task_role == "resume_original",
        "unsupported supervision contract"
    );
    anyhow::ensure!(
        contract.parent_task_id.as_deref() == Some(parent.task_id.as_str()),
        "resume parent identity mismatch"
    );
    anyhow::ensure!(
        receipt.protocol == UPDATE_RECOVERY_PROTOCOL
            && receipt.schema_version == UPDATE_RECOVERY_SCHEMA_VERSION,
        "unsupported recovery receipt version"
    );
    anyhow::ensure!(
        receipt.original_task_id == parent.task_id
            || receipt.resume_task_id.as_deref() == Some(parent.task_id.as_str()),
        "recovery receipt belongs to another task"
    );
    let expected_root = contract
        .root_task_id
        .as_deref()
        .unwrap_or(parent.task_id.as_str());
    anyhow::ensure!(
        receipt.root_task_id == expected_root,
        "recovery receipt root identity mismatch"
    );
    anyhow::ensure!(
        receipt.transport.allows_local_resume_rebuild(),
        "recovery transport is not trusted for local worktree reconstruction"
    );
    Ok(receipt.workspace.git_head.clone().unwrap_or_default())
}

fn recorded_workspace(parent: &LocalTaskRecord) -> Result<(PathBuf, PathBuf, String)> {
    let status = parent
        .workspace_status
        .as_ref()
        .context("parent lacks platform workspace status")?;
    anyhow::ensure!(
        status.get("isolated").and_then(serde_json::Value::as_bool) == Some(true),
        "parent was not executed in an isolated worktree"
    );
    let required = |field: &str| -> Result<&str> {
        status
            .get(field)
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .with_context(|| format!("parent workspace status lacks {field}"))
    };
    Ok((
        absolute_path(Path::new(required("base_workspace_path")?))?,
        absolute_path(Path::new(required("active_workspace_path")?))?,
        required("branch")?.to_string(),
    ))
}

fn validate_base_repo(base: &Path) -> Result<()> {
    let top = canonical_directory(
        Path::new(git_output(base, &["rev-parse", "--show-toplevel"])?.trim()),
        "base repository top level",
    )?;
    anyhow::ensure!(same_path(base, &top), "authorized base is not a Git root");
    anyhow::ensure!(
        git_output(base, &["status", "--porcelain=v1", "--untracked-files=all"])?
            .trim()
            .is_empty(),
        "authorized base repository is dirty"
    );
    Ok(())
}

fn validate_commit_landed(base: &Path, head: &str) -> Result<()> {
    run_git(base, &["cat-file", "-e", &format!("{head}^{{commit}}")])?;
    let output = git_command()
        .args([
            "merge-base",
            "--is-ancestor",
            head,
            "refs/remotes/origin/main",
        ])
        .current_dir(base)
        .output()
        .context("check receipt commit against origin/main")?;
    anyhow::ensure!(
        output.status.success(),
        "receipt commit is not contained in origin/main"
    );
    Ok(())
}

fn validate_unoccupied(base: &Path, active: &Path, branch: &str) -> Result<()> {
    let listing = git_output(base, &["worktree", "list", "--porcelain"])?;
    let branch_ref = format!("refs/heads/{branch}");
    for entry in listing.replace("\r\n", "\n").split("\n\n") {
        let registered_path = entry
            .lines()
            .find_map(|line| line.strip_prefix("worktree "))
            .map(Path::new)
            .map(absolute_path)
            .transpose()?;
        let registered_branch = entry.lines().find_map(|line| line.strip_prefix("branch "));
        anyhow::ensure!(
            !registered_path
                .as_deref()
                .is_some_and(|path| same_path(path, active)),
            "recycled path is still registered as a worktree"
        );
        anyhow::ensure!(
            registered_branch != Some(branch_ref.as_str()),
            "recorded branch is occupied by another worktree"
        );
    }
    Ok(())
}

fn recreate_worktree(
    base: &Path,
    active: &Path,
    branch: &str,
    head: &str,
    supervision_root_task_id: &str,
) -> Result<()> {
    let branch_ref = format!("refs/heads/{branch}");
    let reason =
        crate::node_agent_supervision_worktree_lease::lease_reason(supervision_root_task_id)?;
    let branch_exists = git_command()
        .args(["show-ref", "--verify", "--quiet", &branch_ref])
        .current_dir(base)
        .status()
        .context("inspect recycled resume branch")?
        .success();
    if branch_exists {
        anyhow::ensure!(
            git_output(base, &["rev-parse", "--verify", &branch_ref])?.trim() == head,
            "recorded branch no longer points at the receipt commit"
        );
        run_git(
            base,
            &[
                "worktree",
                "add",
                "--lock",
                "--reason",
                &reason,
                &path_arg(active),
                branch,
            ],
        )
    } else {
        run_git(
            base,
            &[
                "worktree",
                "add",
                "--lock",
                "--reason",
                &reason,
                "-b",
                branch,
                &path_arg(active),
                head,
            ],
        )
    }
}

fn validate_platform_path(active: &Path, project: &str, conversation: &str) -> Result<()> {
    let component = |path: Option<&Path>, label: &str| -> Result<String> {
        path.and_then(Path::file_name)
            .and_then(|value| value.to_str())
            .map(ToOwned::to_owned)
            .with_context(|| format!("recycled path lacks {label}"))
    };
    anyhow::ensure!(
        same_component(&component(Some(active), "conversation")?, conversation)
            && same_component(&component(active.parent(), "project")?, project)
            && same_component(
                &component(active.parent().and_then(Path::parent), "platform marker")?,
                "conversation-worktrees",
            ),
        "receipt path is not the platform-managed project conversation worktree"
    );
    Ok(())
}

fn valid_commit_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn canonical_directory(path: &Path, label: &str) -> Result<PathBuf> {
    let path = std::fs::canonicalize(path)
        .with_context(|| format!("{label} does not exist: {}", path.display()))?;
    anyhow::ensure!(path.is_dir(), "{label} is not a directory");
    Ok(path)
}

fn absolute_path(path: &Path) -> Result<PathBuf> {
    anyhow::ensure!(path.is_absolute(), "resume workspace path must be absolute");
    Ok(path.to_path_buf())
}

fn run_git(cwd: &Path, args: &[&str]) -> Result<()> {
    let output = git_command()
        .args(args)
        .current_dir(cwd)
        .output()
        .with_context(|| git_spawn_context(args))?;
    if !output.status.success() {
        bail!(git_failure_message(cwd, args, &output));
    }
    Ok(())
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

fn path_arg(path: &Path) -> String {
    let raw = path.to_string_lossy();
    raw.strip_prefix(r"\\?\UNC\")
        .map(|value| format!(r"\\{value}"))
        .or_else(|| raw.strip_prefix(r"\\?\").map(ToOwned::to_owned))
        .unwrap_or_else(|| raw.to_string())
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
#[path = "node_agent_local_task_resume_rebuild_tests.rs"]
mod tests;

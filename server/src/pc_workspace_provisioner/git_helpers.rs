use anyhow::{anyhow, bail, Context, Result};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use elon_pc_dev_runtime::{ensure_project_scaffold, ProjectScaffoldRequest};

use crate::git_command_error::{git_command, git_failure_message, git_spawn_context};
use crate::pc_workspace_git_remote::clean_git_branch;
use crate::project_default_docs::ensure_default_docs_in_workspace;

use super::{
    ConversationWorkspaceResult, ProjectWorkspaceRequest, CONVERSATION_MERGE_LOCKS,
    CONVERSATION_MERGE_PUSH_ATTEMPTS,
};

pub(super) const GENERIC_CONVERSATION_WORKTREE_LEASE_REASON: &str =
    crate::node_agent_supervision_worktree_lease::TRANSITIONAL_ACTIVE_TASK_LOCK_REASON;

pub(super) fn completion_origin_refs(workspace: &ConversationWorkspaceResult) -> Vec<String> {
    let mut refs = Vec::new();
    if let Some(base_workspace_path) = workspace.base_workspace_path.as_ref() {
        let base_workspace = git_path_buf(&PathBuf::from(base_workspace_path));
        if let Some(base_branch) = current_branch(&base_workspace) {
            refs.push(format!("origin/{base_branch}"));
        }
    }
    refs.push("origin/main".to_string());
    refs.sort();
    refs.dedup();
    refs
}

pub(super) fn conversation_merge_lock(base_workspace: &Path) -> Result<Arc<Mutex<()>>> {
    let key =
        std::fs::canonicalize(base_workspace).unwrap_or_else(|_| base_workspace.to_path_buf());
    let registry = CONVERSATION_MERGE_LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut locks = registry
        .lock()
        .map_err(|_| anyhow!("conversation merge lock registry poisoned"))?;
    Ok(locks
        .entry(key)
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone())
}

pub(super) fn merge_conversation_branch_into_base(
    base_workspace: &Path,
    branch: &str,
) -> Result<(String, String)> {
    let base_branch = current_branch(base_workspace).unwrap_or_else(|| "main".into());
    let has_origin = git_output(base_workspace, &["remote", "get-url", "origin"]).is_ok();
    let origin_ref = format!("origin/{base_branch}");

    let mut last_push_error = None;
    for attempt in 1..=CONVERSATION_MERGE_PUSH_ATTEMPTS {
        if has_origin {
            if attempt == 1 {
                fast_forward_current_branch_from_origin(base_workspace, &base_branch)?;
            } else {
                reset_base_branch_to_origin(base_workspace, &origin_ref)?;
            }
        }

        let before = git_output(base_workspace, &["rev-parse", "HEAD"])?;
        merge_session_branch(base_workspace, branch)?;
        let after = git_output(base_workspace, &["rev-parse", "HEAD"])?;

        if !has_origin {
            return Ok((before, after));
        }
        if before == after && commit_contained_in_ref(base_workspace, "HEAD", &origin_ref) {
            return Ok((before, after));
        }

        match push_base_branch(base_workspace, &base_branch) {
            Ok(()) => return Ok((before, after)),
            Err(error) if is_retryable_push_rejection(&error) => {
                last_push_error = Some(error);
                if attempt < CONVERSATION_MERGE_PUSH_ATTEMPTS {
                    continue;
                }
            }
            Err(error) => return Err(anyhow!(error)),
        }
    }

    Err(anyhow!(
        "conversation branch merge push was rejected after {} attempts: {}",
        CONVERSATION_MERGE_PUSH_ATTEMPTS,
        last_push_error.unwrap_or_else(|| "unknown push rejection".to_string())
    ))
}

pub(super) fn merge_session_branch(base_workspace: &Path, branch: &str) -> Result<()> {
    let merge_args = ["merge", "--no-ff", "--no-edit", branch];
    let base_git_cwd = git_path_buf(base_workspace);
    let merge_output = git_command()
        .args(merge_args)
        .current_dir(&base_git_cwd)
        .output()
        .with_context(|| format!("failed to run {}", git_spawn_context(&merge_args)))?;
    if !merge_output.status.success() {
        let _ = git_command()
            .args(["merge", "--abort"])
            .current_dir(&base_git_cwd)
            .output();
        return Err(anyhow!(git_failure_message(
            &base_git_cwd,
            &merge_args,
            &merge_output,
        )));
    }
    Ok(())
}

pub(super) fn reset_base_branch_to_origin(base_workspace: &Path, origin_ref: &str) -> Result<()> {
    git_fetch_origin(base_workspace)?;
    run_git_dynamic(base_workspace, &["reset", "--hard", origin_ref])
}

pub(super) fn push_base_branch(base_workspace: &Path, base_branch: &str) -> Result<(), String> {
    let push_args = ["push", "origin", base_branch];
    let base_git_cwd = git_path_buf(base_workspace);
    let push_output = git_command()
        .args(push_args)
        .current_dir(&base_git_cwd)
        .output()
        .map_err(|error| format!("failed to run {}: {error}", git_spawn_context(&push_args)))?;
    if push_output.status.success() {
        return Ok(());
    }
    Err(git_failure_message(&base_git_cwd, &push_args, &push_output))
}

pub(super) fn commit_contained_in_ref(repo: &Path, commit: &str, reference: &str) -> bool {
    let git_cwd = git_path_buf(repo);
    git_command()
        .args(["merge-base", "--is-ancestor", commit, reference])
        .current_dir(&git_cwd)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub(super) fn is_retryable_push_rejection(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("non-fast-forward")
        || lower.contains("fetch first")
        || lower.contains("remote contains work")
        || lower.contains("failed to push some refs")
        || (lower.contains("rejected") && lower.contains("push"))
}

pub(super) fn ensure_seed_files(repo: &Path, req: &ProjectWorkspaceRequest) -> Result<()> {
    ensure_project_scaffold(
        repo,
        &ProjectScaffoldRequest {
            project_id: &req.project_id,
            user_id: &req.user_id,
            name: &req.name,
            template: &req.template,
            repo_url: req.repo_url.as_deref(),
            branch: req.branch.as_deref(),
        },
    )?;
    let _ = ensure_default_docs_in_workspace(repo)?;
    Ok(())
}

pub(super) fn is_git_work_tree(repo: &Path) -> bool {
    repo.exists()
        && git_output(repo, &["rev-parse", "--is-inside-work-tree"])
            .map(|value| value == "true")
            .unwrap_or(false)
}

pub(super) fn conversation_start_ref(repo: &Path) -> String {
    let branch = git_output(repo, &["rev-parse", "--abbrev-ref", "HEAD"])
        .ok()
        .filter(|branch| !branch.is_empty() && branch != "HEAD")
        .unwrap_or_else(|| "main".to_string());
    let origin_ref = format!("origin/{branch}");
    if git_output(repo, &["rev-parse", "--verify", &origin_ref]).is_ok() {
        origin_ref
    } else {
        branch
    }
}

pub(super) fn local_branch_exists(repo: &Path, branch: &str) -> bool {
    git_output(
        repo,
        &["rev-parse", "--verify", &format!("refs/heads/{branch}")],
    )
    .is_ok()
}

pub(super) fn current_branch(repo: &Path) -> Option<String> {
    git_output(repo, &["rev-parse", "--abbrev-ref", "HEAD"])
        .ok()
        .filter(|branch| !branch.is_empty() && branch != "HEAD")
}

pub(super) fn git_fetch_origin(repo: &Path) -> Result<String> {
    if git_output(repo, &["remote", "get-url", "origin"]).is_ok() {
        git_output(repo, &["fetch", "origin"])
    } else {
        Ok(String::new())
    }
}

pub(super) fn fast_forward_current_branch_from_origin(repo: &Path, branch: &str) -> Result<()> {
    git_fetch_origin(repo)?;
    let origin_ref = format!("origin/{branch}");
    if git_output(repo, &["rev-parse", "--verify", &origin_ref]).is_err() {
        return Ok(());
    }
    run_git_dynamic(repo, &["merge", "--ff-only", &origin_ref])
}

pub(super) fn tracked_worktree_clean(repo: &Path) -> Result<bool> {
    Ok(
        git_output(repo, &["status", "--porcelain", "--untracked-files=no"])?
            .trim()
            .is_empty(),
    )
}

pub(super) fn worktree_clean(repo: &Path) -> Result<bool> {
    Ok(git_output(repo, &["status", "--porcelain"])?
        .trim()
        .is_empty())
}

pub(super) fn lock_registered_conversation_worktree(
    base_workspace: &Path,
    worktree_path: &Path,
) -> Result<()> {
    match crate::node_agent_supervision_worktree_lease::worktree_lock_reason(
        base_workspace,
        worktree_path,
    )? {
        Some(reason) if reason == GENERIC_CONVERSATION_WORKTREE_LEASE_REASON => return Ok(()),
        Some(reason) => bail!("conversation worktree is owned by another lease: {reason}"),
        None => {}
    }
    let path_arg = git_path_arg(worktree_path);
    run_git_dynamic(
        base_workspace,
        &[
            "worktree",
            "lock",
            "--reason",
            GENERIC_CONVERSATION_WORKTREE_LEASE_REASON,
            path_arg.as_str(),
        ],
    )
}

pub(super) fn unlock_conversation_worktree(
    base_workspace: &Path,
    worktree_path: &Path,
) -> Result<()> {
    if !conversation_worktree_is_locked(base_workspace, worktree_path)? {
        return Ok(());
    }
    let path_arg = git_path_arg(worktree_path);
    run_git_dynamic(base_workspace, &["worktree", "unlock", path_arg.as_str()])
}

fn conversation_worktree_is_locked(base_workspace: &Path, worktree_path: &Path) -> Result<bool> {
    let target =
        std::fs::canonicalize(worktree_path).unwrap_or_else(|_| worktree_path.to_path_buf());
    let list = git_output(base_workspace, &["worktree", "list", "--porcelain"])?;
    let mut path_matches = false;
    for line in list.lines().chain(std::iter::once("")) {
        if line.is_empty() {
            if path_matches {
                return Ok(false);
            }
            path_matches = false;
        } else if let Some(path) = line.strip_prefix("worktree ") {
            let registered = std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path));
            path_matches = same_worktree_path(&registered, &target);
        } else if path_matches && line.starts_with("locked") {
            return Ok(true);
        }
    }
    Ok(false)
}

fn same_worktree_path(left: &Path, right: &Path) -> bool {
    if cfg!(windows) {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    } else {
        left == right
    }
}

pub(super) fn ensure_conversation_workspace_committed(repo: &Path) -> Result<Option<String>> {
    if worktree_clean(repo)? {
        return Ok(None);
    }
    run_git_dynamic(repo, &["add", "-A"])?;
    let tree = git_output(repo, &["write-tree"])?;
    let head_tree = git_output(repo, &["rev-parse", "HEAD^{tree}"])?;
    if tree == head_tree {
        return Ok(None);
    }
    let head = git_output(repo, &["rev-parse", "HEAD"])?;
    let commit = git_output(
        repo,
        &[
            "commit-tree",
            &tree,
            "-p",
            &head,
            "-m",
            "chore(ai): 保存会话工作区改动",
        ],
    )
    .context("conversation worktree auto-commit failed")?;
    run_git_dynamic(repo, &["reset", "--hard", &commit])?;
    Ok(Some(commit))
}

pub(super) fn short_sha(sha: &str) -> String {
    sha.chars().take(12).collect()
}

pub(super) fn git_head(repo: &Path) -> Result<String> {
    let args = ["rev-parse", "--short", "HEAD"];
    let git_cwd = git_path_buf(repo);
    let output = git_command()
        .args(args)
        .current_dir(&git_cwd)
        .output()
        .with_context(|| format!("failed to run {}", git_spawn_context(&args)))?;
    if !output.status.success() {
        return Err(anyhow!(git_failure_message(&git_cwd, &args, &output)));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub(super) fn git_output(repo: &Path, args: &[&str]) -> Result<String> {
    let git_cwd = git_path_buf(repo);
    let output = git_command()
        .args(args)
        .current_dir(&git_cwd)
        .output()
        .with_context(|| format!("failed to run {}", git_spawn_context(args)))?;
    if !output.status.success() {
        return Err(anyhow!(git_failure_message(&git_cwd, args, &output)));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub(super) fn run_git_dynamic(repo: &Path, args: &[&str]) -> Result<()> {
    let git_cwd = git_path_buf(repo);
    let output = git_command()
        .args(args)
        .current_dir(&git_cwd)
        .output()
        .with_context(|| format!("failed to run {}", git_spawn_context(args)))?;
    if !output.status.success() {
        return Err(anyhow!(git_failure_message(&git_cwd, args, &output)));
    }
    Ok(())
}

pub(super) fn git_path_arg(path: &Path) -> String {
    git_path_buf(path).to_string_lossy().to_string()
}

#[cfg(windows)]
pub(super) fn git_path_buf(path: &Path) -> PathBuf {
    let value = path.to_string_lossy();
    if let Some(rest) = value.strip_prefix("\\\\?\\UNC\\") {
        return PathBuf::from(format!("\\\\{rest}"));
    }
    if let Some(rest) = value.strip_prefix("\\\\?\\") {
        return PathBuf::from(rest);
    }
    path.to_path_buf()
}

#[cfg(not(windows))]
pub(super) fn git_path_buf(path: &Path) -> PathBuf {
    path.to_path_buf()
}

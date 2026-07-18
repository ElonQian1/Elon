use anyhow::{anyhow, Context, Result};
use elon_pc_dev_runtime::{
    ensure_project_git_baseline, ensure_project_scaffold, safe_path_part, workspace_root,
    ProjectGitBaselineRequest, ProjectScaffoldRequest,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::git_command_error::{git_command, git_failure_message, git_spawn_context};
use crate::pc_workspace_git_remote::{
    clean_git_branch, clean_git_remote, ensure_git_remote_workspace, git_remote_origin,
};
use crate::project_default_docs::ensure_default_docs_in_workspace;

pub(super) const CONVERSATION_MERGE_PUSH_ATTEMPTS: usize = 3;

pub(super) static CONVERSATION_MERGE_LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> =
    OnceLock::new();

pub struct ProjectWorkspaceRequest {
    pub project_id: String,
    pub user_id: String,
    pub name: String,
    pub template: String,
    pub repo_url: Option<String>,
    pub branch: Option<String>,
}

pub struct ProjectWorkspaceResult {
    pub workspace_path: String,
    pub git_head: Option<String>,
    pub git_remote_origin: Option<String>,
    pub git_branch: Option<String>,
    pub created: bool,
}

pub struct ProjectWorkspaceCleanupResult {
    pub removed_paths: Vec<String>,
    pub skipped_paths: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ConversationWorkspaceResult {
    pub base_workspace_path: Option<String>,
    pub workspace_path: String,
    pub isolated: bool,
    pub branch: Option<String>,
}

pub fn provision_project_workspace(req: ProjectWorkspaceRequest) -> Result<ProjectWorkspaceResult> {
    let root = workspace_root();
    provision_project_workspace_in(&root, req)
}

/// Provision a platform-managed project under an already validated workspace
/// root. The full node agent must use this entry point so a missing unified
/// data root can never fall back to the user profile.
pub fn provision_project_workspace_in(
    workspace_root: &Path,
    req: ProjectWorkspaceRequest,
) -> Result<ProjectWorkspaceResult> {
    let user_dir = safe_path_part(&req.user_id, "user", 80);
    let project_dir = safe_path_part(&req.project_id, "project", 80);
    let repo = workspace_root.join(user_dir).join(project_dir).join("repo");
    let created = !repo.exists();

    if let Some(remote) = clean_git_remote(req.repo_url.as_deref(), req.branch.as_deref()) {
        ensure_git_remote_workspace(&repo, &remote, |repo| ensure_seed_files(repo, &req))?;
    } else {
        std::fs::create_dir_all(&repo)
            .with_context(|| format!("failed to create workspace directory {}", repo.display()))?;
        ensure_seed_files(&repo, &req)?;
        let branch = clean_git_branch(req.branch.as_deref());
        ensure_project_git_baseline(
            &repo,
            &ProjectGitBaselineRequest {
                branch: branch.as_deref(),
            },
        )?;
    }

    Ok(ProjectWorkspaceResult {
        workspace_path: repo.to_string_lossy().to_string(),
        git_head: git_head(&repo).ok(),
        git_remote_origin: git_remote_origin(&repo).ok(),
        git_branch: current_branch(&repo),
        created,
    })
}

pub fn cleanup_project_workspace(
    project_id: &str,
    workspace_path: &str,
) -> Result<ProjectWorkspaceCleanupResult> {
    let root = workspace_root();
    cleanup_project_workspace_in(&root, project_id, workspace_path)
}

/// Cleanup a platform-managed project only inside an already validated
/// workspace root. Callers that own node runtime state should never use the
/// legacy environment-resolving wrapper above.
pub fn cleanup_project_workspace_in(
    workspace_root: &Path,
    project_id: &str,
    workspace_path: &str,
) -> Result<ProjectWorkspaceCleanupResult> {
    let project_part = safe_path_part(project_id, "project", 80);
    let repo = PathBuf::from(workspace_path);
    let mut result = ProjectWorkspaceCleanupResult {
        removed_paths: Vec::new(),
        skipped_paths: Vec::new(),
    };

    let Some(project_dir) = managed_project_dir(workspace_root, &repo, &project_part)? else {
        result
            .skipped_paths
            .push(format!("跳过非平台托管 PC 工作区：{}", repo.display()));
        return Ok(result);
    };
    let worktree_root = workspace_root
        .join("conversation-worktrees")
        .join(&project_part);
    let protected_worktree =
        remove_conversation_worktrees(&repo, &worktree_root, workspace_root, &mut result)?;
    if protected_worktree {
        result.skipped_paths.push(format!(
            "PC project cleanup deferred while a worktree lease is active: {}",
            project_dir.display()
        ));
        return Ok(result);
    }
    remove_managed_path(&project_dir, workspace_root, "PC 项目工作区", &mut result)?;
    Ok(result)
}

fn managed_project_dir(root: &Path, repo: &Path, project_part: &str) -> Result<Option<PathBuf>> {
    if repo.file_name().and_then(|value| value.to_str()) != Some("repo") {
        return Ok(None);
    }
    let Some(project_dir) = repo.parent() else {
        return Ok(None);
    };
    if project_dir.file_name().and_then(|value| value.to_str()) != Some(project_part) {
        return Ok(None);
    }
    if !project_dir.exists() {
        return Ok(None);
    }
    ensure_within_root(root, project_dir)?;
    Ok(Some(project_dir.to_path_buf()))
}

fn remove_conversation_worktrees(
    repo: &Path,
    worktree_root: &Path,
    root: &Path,
    result: &mut ProjectWorkspaceCleanupResult,
) -> Result<bool> {
    if !worktree_root.exists() {
        result
            .skipped_paths
            .push(format!("会话 worktree 不存在：{}", worktree_root.display()));
        return Ok(false);
    }
    ensure_within_root(root, worktree_root)?;
    let mut preserved = false;
    if repo.exists() && is_git_work_tree(repo) {
        for entry in std::fs::read_dir(worktree_root)?.filter_map(|entry| entry.ok()) {
            let path = entry.path();
            if path.is_dir() {
                if crate::node_agent_supervision_worktree_lease::worktree_lock_reason(repo, &path)?
                    .is_some()
                {
                    preserved = true;
                    result.skipped_paths.push(format!(
                        "conversation worktree cleanup deferred by persistent lease: {}",
                        path.display()
                    ));
                    continue;
                }
                let removed = run_git_dynamic(
                    repo,
                    &[
                        "worktree",
                        "remove",
                        "--force",
                        git_path_arg(&path).as_str(),
                    ],
                )
                .is_ok();
                if !removed && path.exists() {
                    preserved = true;
                    result.skipped_paths.push(format!(
                        "conversation worktree cleanup failed safely: {}",
                        path.display()
                    ));
                }
            }
        }
        let _ = run_git_dynamic(repo, &["worktree", "prune"]);
    }
    if preserved {
        Ok(true)
    } else {
        remove_managed_path(worktree_root, root, "会话 worktree", result)?;
        Ok(false)
    }
}

fn remove_managed_path(
    path: &Path,
    root: &Path,
    label: &str,
    result: &mut ProjectWorkspaceCleanupResult,
) -> Result<()> {
    if !path.exists() {
        result
            .skipped_paths
            .push(format!("{label}不存在：{}", path.display()));
        return Ok(());
    }
    ensure_within_root(root, path)?;
    if path.is_dir() {
        std::fs::remove_dir_all(path)
            .with_context(|| format!("failed to remove {}", path.display()))?;
    } else {
        std::fs::remove_file(path)
            .with_context(|| format!("failed to remove {}", path.display()))?;
    }
    result
        .removed_paths
        .push(format!("{label}：{}", path.display()));
    Ok(())
}

fn ensure_within_root(root: &Path, path: &Path) -> Result<()> {
    let root = std::fs::canonicalize(root)
        .with_context(|| format!("failed to canonicalize root {}", root.display()))?;
    let target = std::fs::canonicalize(path)
        .with_context(|| format!("failed to canonicalize target {}", path.display()))?;
    if target == root || !target.starts_with(&root) {
        return Err(anyhow!(
            "refusing to cleanup path outside PC workspace root: {} (root: {})",
            target.display(),
            root.display()
        ));
    }
    Ok(())
}

pub fn prepare_conversation_workspace(
    base_workspace_path: &str,
    project_id: &str,
    conversation_id: &str,
) -> Result<ConversationWorkspaceResult> {
    let root = workspace_root();
    prepare_conversation_workspace_in(&root, base_workspace_path, project_id, conversation_id)
}

/// Prepare a conversation worktree below an already validated workspace root.
/// Keeping the root explicit is what lets the node runtime hold its transition
/// gate across worktree creation and build-lease admission.
pub fn prepare_conversation_workspace_in(
    workspace_root: &Path,
    base_workspace_path: &str,
    project_id: &str,
    conversation_id: &str,
) -> Result<ConversationWorkspaceResult> {
    let base_workspace = git_path_buf(&PathBuf::from(base_workspace_path));
    if !is_git_work_tree(&base_workspace) {
        return Ok(ConversationWorkspaceResult {
            base_workspace_path: None,
            workspace_path: base_workspace.to_string_lossy().to_string(),
            isolated: false,
            branch: None,
        });
    }

    let project_part = safe_path_part(project_id, "project", 80);
    let conversation_part = safe_path_part(conversation_id, "conversation", 80);
    let worktree_root = workspace_root
        .join("conversation-worktrees")
        .join(&project_part);
    let worktree_path = worktree_root.join(&conversation_part);
    std::fs::create_dir_all(&worktree_root).with_context(|| {
        format!(
            "failed to create conversation worktree root {}",
            worktree_root.display()
        )
    })?;

    let branch = format!("ai/session/{}/{}", project_part, conversation_part);
    let _ = git_fetch_origin(&base_workspace);
    let _ = run_git_dynamic(&base_workspace, &["worktree", "prune"]);
    if is_git_work_tree(&worktree_path) {
        lock_conversation_worktree(&base_workspace, &worktree_path)?;
        return Ok(ConversationWorkspaceResult {
            base_workspace_path: Some(base_workspace.to_string_lossy().to_string()),
            workspace_path: git_path_arg(&worktree_path),
            isolated: true,
            branch: Some(branch),
        });
    }
    if worktree_path.exists() {
        recover_stale_conversation_worktree_path(&base_workspace, &worktree_root, &worktree_path)?;
    }

    let start_ref = conversation_start_ref(&base_workspace);
    let worktree_arg = git_path_arg(&worktree_path);
    add_conversation_worktree(&base_workspace, &worktree_arg, &branch, &start_ref)?;
    if !is_git_work_tree(&worktree_path) {
        recover_stale_conversation_worktree_path(&base_workspace, &worktree_root, &worktree_path)?;
        add_conversation_worktree(&base_workspace, &worktree_arg, &branch, &start_ref)?;
        if !is_git_work_tree(&worktree_path) {
            return Err(anyhow!(
                "conversation worktree was created but is not a git repository: {}",
                worktree_path.display()
            ));
        }
    }
    lock_conversation_worktree(&base_workspace, &worktree_path)?;

    Ok(ConversationWorkspaceResult {
        base_workspace_path: Some(base_workspace.to_string_lossy().to_string()),
        workspace_path: git_path_arg(&worktree_path),
        isolated: true,
        branch: Some(branch),
    })
}

fn add_conversation_worktree(
    base_workspace: &Path,
    worktree_arg: &str,
    branch: &str,
    start_ref: &str,
) -> Result<()> {
    if local_branch_exists(base_workspace, branch) {
        run_git_dynamic(base_workspace, &["worktree", "add", worktree_arg, branch])
    } else {
        run_git_dynamic(
            base_workspace,
            &["worktree", "add", "-b", branch, worktree_arg, start_ref],
        )
    }
}

fn recover_stale_conversation_worktree_path(
    base_workspace: &Path,
    worktree_root: &Path,
    worktree_path: &Path,
) -> Result<()> {
    ensure_within_root(worktree_root, worktree_path)?;
    let _ = run_git_dynamic(
        base_workspace,
        &[
            "worktree",
            "remove",
            "--force",
            git_path_arg(worktree_path).as_str(),
        ],
    );
    let _ = run_git_dynamic(base_workspace, &["worktree", "prune"]);
    if !worktree_path.exists() {
        return Ok(());
    }

    if worktree_path.is_dir() && std::fs::read_dir(worktree_path)?.next().is_none() {
        std::fs::remove_dir(worktree_path).with_context(|| {
            format!(
                "failed to remove empty conversation worktree path {}",
                worktree_path.display()
            )
        })?;
    } else {
        let archive_path = stale_conversation_archive_path(worktree_path);
        std::fs::rename(worktree_path, &archive_path).with_context(|| {
            format!(
                "failed to archive stale conversation worktree path {} to {}",
                worktree_path.display(),
                archive_path.display()
            )
        })?;
        tracing::warn!(
            path = %worktree_path.display(),
            archived_to = %archive_path.display(),
            "archived stale conversation worktree path before recreating it"
        );
    }
    let _ = run_git_dynamic(base_workspace, &["worktree", "prune"]);
    Ok(())
}

fn stale_conversation_archive_path(worktree_path: &Path) -> PathBuf {
    let parent = worktree_path.parent().unwrap_or_else(|| Path::new("."));
    let name = worktree_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("conversation");
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or(0);
    for index in 0..100 {
        let suffix = if index == 0 {
            format!(".stale-{millis}")
        } else {
            format!(".stale-{millis}-{index}")
        };
        let candidate = parent.join(format!("{name}{suffix}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!("{name}.stale-{millis}-overflow"))
}

pub fn merge_conversation_workspace(workspace: &ConversationWorkspaceResult) -> Result<String> {
    if !workspace.isolated {
        return Ok("shared workspace does not need merge".into());
    }
    let base_workspace = workspace
        .base_workspace_path
        .as_ref()
        .map(|path| git_path_buf(&PathBuf::from(path)))
        .ok_or_else(|| anyhow!("isolated conversation workspace is missing base workspace"))?;
    let active_workspace = git_path_buf(&PathBuf::from(&workspace.workspace_path));
    let branch = workspace
        .branch
        .as_deref()
        .ok_or_else(|| anyhow!("isolated conversation workspace is missing branch"))?;

    let merge_lock = conversation_merge_lock(&base_workspace)?;
    let _merge_guard = merge_lock
        .lock()
        .map_err(|_| anyhow!("conversation workspace merge lock poisoned"))?;

    if !is_git_work_tree(&active_workspace) {
        return Ok(format!(
            "conversation worktree missing git metadata; skipped auto-merge: {}",
            active_workspace.display()
        ));
    }
    let saved_dirty_commit = ensure_conversation_workspace_committed(&active_workspace)?;
    if !worktree_clean(&active_workspace)? {
        return Ok(format!(
            "conversation worktree still has uncommitted changes: {}",
            active_workspace.display()
        ));
    }
    if !tracked_worktree_clean(&base_workspace)? {
        return Ok(format!(
            "base workspace has tracked changes; skipped auto-merge: {}",
            base_workspace.display()
        ));
    }

    let (before, after) = merge_conversation_branch_into_base(&base_workspace, branch)?;
    unlock_conversation_worktree(&base_workspace, &active_workspace)?;
    if let Err(error) = run_git_dynamic(
        &base_workspace,
        &[
            "worktree",
            "remove",
            "--force",
            git_path_arg(&active_workspace).as_str(),
        ],
    ) {
        let _ = lock_conversation_worktree(&base_workspace, &active_workspace);
        return Err(error).context(
            "conversation branch landed, but active worktree cleanup was deferred and re-locked",
        );
    }
    let _ = run_git_dynamic(&base_workspace, &["branch", "-d", branch]);

    if before == after {
        Ok("conversation branch had no new commits".into())
    } else if let Some(commit) = saved_dirty_commit {
        Ok(format!(
            "conversation branch merged: {} (saved dirty worktree as {})",
            short_sha(&after),
            short_sha(&commit)
        ))
    } else {
        Ok(format!("conversation branch merged: {}", short_sha(&after)))
    }
}

pub(crate) fn conversation_workspace_git_head(
    workspace: &ConversationWorkspaceResult,
) -> Option<String> {
    let active_workspace = git_path_buf(&PathBuf::from(&workspace.workspace_path));
    git_output(
        &active_workspace,
        &["rev-parse", "--verify", "HEAD^{commit}"],
    )
    .ok()
}

pub(crate) fn lock_conversation_worktree(
    base_workspace: &Path,
    worktree_path: &Path,
) -> Result<()> {
    lock_registered_conversation_worktree(base_workspace, worktree_path)
}

pub fn conversation_workspace_head_landed(workspace: &ConversationWorkspaceResult) -> Result<bool> {
    if !workspace.isolated {
        return Ok(false);
    }
    let active_workspace = git_path_buf(&PathBuf::from(&workspace.workspace_path));
    if !is_git_work_tree(&active_workspace) {
        return Ok(false);
    }
    if git_output(&active_workspace, &["remote", "get-url", "origin"]).is_err() {
        return Ok(false);
    }

    git_fetch_origin(&active_workspace)?;
    let head = git_output(&active_workspace, &["rev-parse", "HEAD"])?;
    for reference in completion_origin_refs(workspace) {
        if git_output(&active_workspace, &["rev-parse", "--verify", &reference]).is_ok()
            && commit_contained_in_ref(&active_workspace, &head, &reference)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

mod git_helpers;
use self::git_helpers::*;

#[cfg(test)]
#[path = "pc_workspace_provisioner_tests.rs"]
mod tests;

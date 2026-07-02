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

const CONVERSATION_MERGE_PUSH_ATTEMPTS: usize = 3;

static CONVERSATION_MERGE_LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> =
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

pub struct ConversationWorkspaceResult {
    pub base_workspace_path: Option<String>,
    pub workspace_path: String,
    pub isolated: bool,
    pub branch: Option<String>,
}

pub fn provision_project_workspace(req: ProjectWorkspaceRequest) -> Result<ProjectWorkspaceResult> {
    let root = workspace_root();
    let user_dir = safe_path_part(&req.user_id, "user", 80);
    let project_dir = safe_path_part(&req.project_id, "project", 80);
    let repo = root.join(user_dir).join(project_dir).join("repo");
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
    let project_part = safe_path_part(project_id, "project", 80);
    let repo = PathBuf::from(workspace_path);
    let mut result = ProjectWorkspaceCleanupResult {
        removed_paths: Vec::new(),
        skipped_paths: Vec::new(),
    };

    let Some(project_dir) = managed_project_dir(&root, &repo, &project_part)? else {
        result
            .skipped_paths
            .push(format!("跳过非平台托管 PC 工作区：{}", repo.display()));
        return Ok(result);
    };
    let worktree_root = root.join("conversation-worktrees").join(&project_part);
    remove_conversation_worktrees(&repo, &worktree_root, &root, &mut result)?;
    remove_managed_path(&project_dir, &root, "PC 项目工作区", &mut result)?;
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
) -> Result<()> {
    if !worktree_root.exists() {
        result
            .skipped_paths
            .push(format!("会话 worktree 不存在：{}", worktree_root.display()));
        return Ok(());
    }
    ensure_within_root(root, worktree_root)?;
    if repo.exists() && is_git_work_tree(repo) {
        for entry in std::fs::read_dir(worktree_root)?.filter_map(|entry| entry.ok()) {
            let path = entry.path();
            if path.is_dir() {
                let _ = run_git_dynamic(
                    repo,
                    &[
                        "worktree",
                        "remove",
                        "--force",
                        git_path_arg(&path).as_str(),
                    ],
                );
            }
        }
        let _ = run_git_dynamic(repo, &["worktree", "prune"]);
    }
    remove_managed_path(worktree_root, root, "会话 worktree", result)
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
    let worktree_root = workspace_root()
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
    if local_branch_exists(&base_workspace, &branch) {
        run_git_dynamic(
            &base_workspace,
            &["worktree", "add", worktree_arg.as_str(), &branch],
        )?;
    } else {
        run_git_dynamic(
            &base_workspace,
            &[
                "worktree",
                "add",
                "-b",
                &branch,
                worktree_arg.as_str(),
                &start_ref,
            ],
        )?;
    }

    Ok(ConversationWorkspaceResult {
        base_workspace_path: Some(base_workspace.to_string_lossy().to_string()),
        workspace_path: git_path_arg(&worktree_path),
        isolated: true,
        branch: Some(branch),
    })
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
    let _ = run_git_dynamic(
        &base_workspace,
        &[
            "worktree",
            "remove",
            "--force",
            git_path_arg(&active_workspace).as_str(),
        ],
    );
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

fn conversation_merge_lock(base_workspace: &Path) -> Result<Arc<Mutex<()>>> {
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

fn merge_conversation_branch_into_base(
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

fn merge_session_branch(base_workspace: &Path, branch: &str) -> Result<()> {
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

fn reset_base_branch_to_origin(base_workspace: &Path, origin_ref: &str) -> Result<()> {
    git_fetch_origin(base_workspace)?;
    run_git_dynamic(base_workspace, &["reset", "--hard", origin_ref])
}

fn push_base_branch(base_workspace: &Path, base_branch: &str) -> Result<(), String> {
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

fn commit_contained_in_ref(repo: &Path, commit: &str, reference: &str) -> bool {
    let git_cwd = git_path_buf(repo);
    git_command()
        .args(["merge-base", "--is-ancestor", commit, reference])
        .current_dir(&git_cwd)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn is_retryable_push_rejection(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("non-fast-forward")
        || lower.contains("fetch first")
        || lower.contains("remote contains work")
        || lower.contains("failed to push some refs")
        || (lower.contains("rejected") && lower.contains("push"))
}

fn ensure_seed_files(repo: &Path, req: &ProjectWorkspaceRequest) -> Result<()> {
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

fn is_git_work_tree(repo: &Path) -> bool {
    repo.exists()
        && git_output(repo, &["rev-parse", "--is-inside-work-tree"])
            .map(|value| value == "true")
            .unwrap_or(false)
}

fn conversation_start_ref(repo: &Path) -> String {
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

fn local_branch_exists(repo: &Path, branch: &str) -> bool {
    git_output(
        repo,
        &["rev-parse", "--verify", &format!("refs/heads/{branch}")],
    )
    .is_ok()
}

fn current_branch(repo: &Path) -> Option<String> {
    git_output(repo, &["rev-parse", "--abbrev-ref", "HEAD"])
        .ok()
        .filter(|branch| !branch.is_empty() && branch != "HEAD")
}

fn git_fetch_origin(repo: &Path) -> Result<String> {
    if git_output(repo, &["remote", "get-url", "origin"]).is_ok() {
        git_output(repo, &["fetch", "origin"])
    } else {
        Ok(String::new())
    }
}

fn fast_forward_current_branch_from_origin(repo: &Path, branch: &str) -> Result<()> {
    git_fetch_origin(repo)?;
    let origin_ref = format!("origin/{branch}");
    if git_output(repo, &["rev-parse", "--verify", &origin_ref]).is_err() {
        return Ok(());
    }
    run_git_dynamic(repo, &["merge", "--ff-only", &origin_ref])
}

fn tracked_worktree_clean(repo: &Path) -> Result<bool> {
    Ok(
        git_output(repo, &["status", "--porcelain", "--untracked-files=no"])?
            .trim()
            .is_empty(),
    )
}

fn worktree_clean(repo: &Path) -> Result<bool> {
    Ok(git_output(repo, &["status", "--porcelain"])?
        .trim()
        .is_empty())
}

fn ensure_conversation_workspace_committed(repo: &Path) -> Result<Option<String>> {
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

fn short_sha(sha: &str) -> String {
    sha.chars().take(12).collect()
}

fn git_head(repo: &Path) -> Result<String> {
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

fn git_output(repo: &Path, args: &[&str]) -> Result<String> {
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

fn run_git_dynamic(repo: &Path, args: &[&str]) -> Result<()> {
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

fn git_path_arg(path: &Path) -> String {
    git_path_buf(path).to_string_lossy().to_string()
}

#[cfg(windows)]
fn git_path_buf(path: &Path) -> PathBuf {
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
fn git_path_buf(path: &Path) -> PathBuf {
    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::{
        ensure_conversation_workspace_committed, git_output, git_path_arg, is_git_work_tree,
        is_retryable_push_rejection, prepare_conversation_workspace,
        recover_stale_conversation_worktree_path, worktree_clean,
    };
    use elon_pc_dev_runtime::safe_path_part;
    use std::path::Path;
    use std::process::Command;
    use std::{env, ffi::OsString, fs, sync::Mutex};
    use uuid::Uuid;

    static WORKSPACE_ROOT_ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn safe_path_part_removes_path_separators() {
        assert_eq!(
            safe_path_part("../usr:abc\\project", "fallback", 80),
            "usrabcproject"
        );
    }

    #[test]
    fn safe_path_part_uses_fallback_when_empty() {
        assert_eq!(safe_path_part("///", "fallback", 80), "fallback");
    }

    #[test]
    fn retryable_push_rejection_only_matches_remote_race_errors() {
        assert!(is_retryable_push_rejection(
            "! [rejected] HEAD -> main (fetch first)"
        ));
        assert!(is_retryable_push_rejection(
            "error: failed to push some refs to 'origin'"
        ));
        assert!(is_retryable_push_rejection(
            "Updates were rejected because the remote contains work that you do not have locally"
        ));
        assert!(!is_retryable_push_rejection(
            "Permission denied (publickey). fatal: Could not read from remote repository."
        ));
        assert!(!is_retryable_push_rejection(
            "remote: error: GH006: Protected branch update failed for refs/heads/main."
        ));
    }

    #[cfg(windows)]
    #[test]
    fn git_path_arg_strips_windows_verbatim_prefixes() {
        assert_eq!(
            git_path_arg(Path::new(r"\\?\C:\Users\Administrator\repo")),
            r"C:\Users\Administrator\repo"
        );
        assert_eq!(
            git_path_arg(Path::new(r"\\?\UNC\server\share\repo")),
            r"\\server\share\repo"
        );
    }

    #[test]
    fn non_git_conversation_workspace_uses_base_path() {
        let base = std::env::temp_dir().join(format!(
            "elon_non_git_conversation_{}",
            Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&base).expect("base should create");

        let workspace =
            prepare_conversation_workspace(&base.to_string_lossy(), "project-a", "conversation-a")
                .expect("non-git workspace should resolve");

        assert!(!workspace.isolated);
        assert_eq!(workspace.branch, None);
        assert_eq!(workspace.workspace_path, base.to_string_lossy().to_string());
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn stale_conversation_worktree_path_is_archived() {
        let root = std::env::temp_dir().join(format!(
            "elon_stale_conversation_{}",
            Uuid::new_v4().simple()
        ));
        let worktree_root = root.join("conversation-worktrees").join("project-a");
        let worktree_path = worktree_root.join("conversation-a");
        fs::create_dir_all(&worktree_path).expect("stale path should create");
        fs::write(worktree_path.join("leftover.txt"), "partial output\n")
            .expect("leftover file should write");

        recover_stale_conversation_worktree_path(&root, &worktree_root, &worktree_path)
            .expect("stale path should be recovered");

        assert!(!worktree_path.exists());
        let archived = fs::read_dir(&worktree_root)
            .expect("worktree root should be readable")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .find(|path| {
                path.file_name()
                    .and_then(|value| value.to_str())
                    .map(|value| value.starts_with("conversation-a.stale-"))
                    .unwrap_or(false)
            })
            .expect("stale directory should be archived");
        assert!(archived.join("leftover.txt").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn prepare_conversation_workspace_recovers_stale_path() {
        let _guard = WORKSPACE_ROOT_ENV_LOCK
            .lock()
            .expect("env lock should work");
        let root = std::env::temp_dir().join(format!(
            "elon_prepare_stale_conversation_root_{}",
            Uuid::new_v4().simple()
        ));
        let base = std::env::temp_dir().join(format!(
            "elon_prepare_stale_conversation_repo_{}",
            Uuid::new_v4().simple()
        ));
        let _env_guard = EnvVarGuard::set("ELON_NODE_WORKSPACE_ROOT", &root);

        fs::create_dir_all(&base).expect("base repo should create");
        run_git(&base, &["init"]);
        run_git(&base, &["config", "user.email", "ai@example.test"]);
        run_git(&base, &["config", "user.name", "AI Test"]);
        fs::write(base.join("README.md"), "seed\n").expect("seed file should write");
        run_git(&base, &["add", "README.md"]);
        run_git(&base, &["commit", "-m", "seed"]);

        let worktree_root = root.join("conversation-worktrees").join("project-a");
        let stale_path = worktree_root.join("conversation-a");
        fs::create_dir_all(&stale_path).expect("stale path should create");
        fs::write(stale_path.join("leftover.txt"), "partial output\n")
            .expect("leftover file should write");

        let workspace =
            prepare_conversation_workspace(&base.to_string_lossy(), "project-a", "conversation-a")
                .expect("stale path should be recovered");

        assert!(workspace.isolated);
        let active = std::path::PathBuf::from(&workspace.workspace_path);
        assert!(is_git_work_tree(&active));
        assert!(fs::read_dir(&worktree_root)
            .expect("worktree root should be readable")
            .filter_map(|entry| entry.ok())
            .any(|entry| entry
                .file_name()
                .to_str()
                .map(|value| value.starts_with("conversation-a.stale-"))
                .unwrap_or(false)));

        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn dirty_conversation_workspace_is_auto_committed() {
        let repo = std::env::temp_dir().join(format!(
            "elon_dirty_conversation_{}",
            Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&repo).expect("repo should create");
        run_git(&repo, &["init"]);
        run_git(&repo, &["config", "user.email", "ai@example.test"]);
        run_git(&repo, &["config", "user.name", "AI Test"]);
        fs::write(repo.join("README.md"), "seed\n").expect("seed file should write");
        run_git(&repo, &["add", "README.md"]);
        run_git(&repo, &["commit", "-m", "seed"]);

        fs::write(repo.join("README.md"), "changed\n").expect("dirty file should write");
        let commit = ensure_conversation_workspace_committed(&repo)
            .expect("auto commit should succeed")
            .expect("dirty workspace should create a commit");

        assert!(!commit.is_empty());
        assert!(worktree_clean(&repo).expect("status should be readable"));
        let subject = git_output(&repo, &["log", "-1", "--pretty=%s"])
            .expect("commit subject should be readable");
        assert_eq!(subject, "chore(ai): 保存会话工作区改动");
        let _ = fs::remove_dir_all(repo);
    }

    fn run_git(repo: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .expect("git should start");
        assert!(
            output.status.success(),
            "git {:?} failed (status={:?}, stdout={}, stderr={})",
            args,
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    struct EnvVarGuard {
        key: &'static str,
        old: Option<OsString>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &Path) -> Self {
            let old = env::var_os(key);
            env::set_var(key, value);
            Self { key, old }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match self.old.as_ref() {
                Some(value) => env::set_var(self.key, value),
                None => env::remove_var(self.key),
            }
        }
    }
}

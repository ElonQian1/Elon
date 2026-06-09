use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct ProjectWorkspaceRequest {
    pub project_id: String,
    pub user_id: String,
    pub name: String,
    pub template: String,
}

pub struct ProjectWorkspaceResult {
    pub workspace_path: String,
    pub git_head: Option<String>,
    pub created: bool,
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

    std::fs::create_dir_all(&repo)
        .with_context(|| format!("failed to create workspace directory {}", repo.display()))?;

    ensure_seed_files(&repo, &req)?;
    ensure_git_repo(&repo)?;

    Ok(ProjectWorkspaceResult {
        workspace_path: repo.to_string_lossy().to_string(),
        git_head: git_head(&repo).ok(),
        created,
    })
}

pub fn prepare_conversation_workspace(
    base_workspace_path: &str,
    project_id: &str,
    conversation_id: &str,
) -> Result<ConversationWorkspaceResult> {
    let base_workspace = PathBuf::from(base_workspace_path);
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
    if is_git_work_tree(&worktree_path) {
        return Ok(ConversationWorkspaceResult {
            base_workspace_path: Some(base_workspace.to_string_lossy().to_string()),
            workspace_path: worktree_path.to_string_lossy().to_string(),
            isolated: true,
            branch: Some(branch),
        });
    }
    if worktree_path.exists() {
        if std::fs::read_dir(&worktree_path)?.next().is_some() {
            return Err(anyhow!(
                "conversation worktree path exists but is not a git worktree: {}",
                worktree_path.display()
            ));
        }
        std::fs::remove_dir(&worktree_path).with_context(|| {
            format!(
                "failed to remove empty conversation worktree path {}",
                worktree_path.display()
            )
        })?;
    }

    let _ = git_fetch_origin(&base_workspace);
    let start_ref = conversation_start_ref(&base_workspace);
    if local_branch_exists(&base_workspace, &branch) {
        run_git_dynamic(
            &base_workspace,
            &[
                "worktree",
                "add",
                worktree_path.to_string_lossy().as_ref(),
                &branch,
            ],
        )?;
    } else {
        run_git_dynamic(
            &base_workspace,
            &[
                "worktree",
                "add",
                "-b",
                &branch,
                worktree_path.to_string_lossy().as_ref(),
                &start_ref,
            ],
        )?;
    }

    Ok(ConversationWorkspaceResult {
        base_workspace_path: Some(base_workspace.to_string_lossy().to_string()),
        workspace_path: worktree_path.to_string_lossy().to_string(),
        isolated: true,
        branch: Some(branch),
    })
}

pub fn merge_conversation_workspace(workspace: &ConversationWorkspaceResult) -> Result<String> {
    if !workspace.isolated {
        return Ok("shared workspace does not need merge".into());
    }
    let base_workspace = workspace
        .base_workspace_path
        .as_ref()
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("isolated conversation workspace is missing base workspace"))?;
    let active_workspace = PathBuf::from(&workspace.workspace_path);
    let branch = workspace
        .branch
        .as_deref()
        .ok_or_else(|| anyhow!("isolated conversation workspace is missing branch"))?;

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

    let base_branch = current_branch(&base_workspace).unwrap_or_else(|| "main".into());
    if git_output(&base_workspace, &["remote", "get-url", "origin"]).is_ok() {
        git_fetch_origin(&base_workspace)?;
        run_git_dynamic(
            &base_workspace,
            &["pull", "--rebase", "origin", &base_branch],
        )?;
    }

    let before = git_output(&base_workspace, &["rev-parse", "HEAD"])?;
    let merge_output = Command::new("git")
        .args(["merge", "--no-ff", "--no-edit", branch])
        .current_dir(&base_workspace)
        .output()
        .context("failed to run git merge")?;
    if !merge_output.status.success() {
        let _ = Command::new("git")
            .args(["merge", "--abort"])
            .current_dir(&base_workspace)
            .output();
        return Err(anyhow!(
            "git merge failed: {}",
            String::from_utf8_lossy(&merge_output.stderr).trim()
        ));
    }

    if git_output(&base_workspace, &["remote", "get-url", "origin"]).is_ok() {
        run_git_dynamic(&base_workspace, &["push", "origin", &base_branch])?;
    }
    let after = git_output(&base_workspace, &["rev-parse", "HEAD"])?;
    let _ = run_git_dynamic(
        &base_workspace,
        &[
            "worktree",
            "remove",
            "--force",
            active_workspace.to_string_lossy().as_ref(),
        ],
    );
    let _ = run_git_dynamic(&base_workspace, &["branch", "-d", branch]);

    if before == after {
        Ok("conversation branch had no new commits".into())
    } else {
        Ok(format!("conversation branch merged: {}", short_sha(&after)))
    }
}

fn workspace_root() -> PathBuf {
    for key in [
        "ELON_NODE_WORKSPACE_ROOT",
        "ELON_PC_WORKSPACE_ROOT",
        "NODE_WORKSPACE_ROOT",
    ] {
        if let Ok(value) = std::env::var(key) {
            let value = value.trim();
            if !value.is_empty() {
                return PathBuf::from(value);
            }
        }
    }

    #[cfg(windows)]
    {
        if let Ok(profile) = std::env::var("USERPROFILE") {
            return PathBuf::from(profile).join("Elon").join("workspaces");
        }
    }

    #[cfg(not(windows))]
    {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(".elon").join("workspaces");
        }
    }

    std::env::temp_dir().join("elon").join("workspaces")
}

fn safe_path_part(value: &str, fallback: &str, max_len: usize) -> String {
    let cleaned: String = value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(max_len)
        .collect();
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned
    }
}

fn ensure_seed_files(repo: &Path, req: &ProjectWorkspaceRequest) -> Result<()> {
    let readme = repo.join("README.md");
    if !readme.exists() {
        std::fs::write(
            &readme,
            format!(
                "# {}\n\nThis is an Elon PC-managed project workspace.\n\n- project_id: {}\n- template: {}\n- owner_user_id: {}\n\nThe cloud server stores project metadata and chat history. Source code, build caches, and task worktrees live on this PC node.\n",
                req.name.trim(),
                req.project_id,
                req.template,
                req.user_id
            ),
        )?;
    }

    let agents = repo.join("AGENTS.md");
    if !agents.exists() {
        std::fs::write(
            &agents,
            format!(
                "# Project Workspace\n\nThis project is managed by an Elon PC node.\n\nRules:\n- Keep source code and build outputs inside this repository.\n- Use git for every meaningful code change.\n- Prefer task-specific worktrees for parallel conversations.\n- Do not write build artifacts to the cloud server workspace.\n\nProject metadata:\n- project_id: {}\n- template: {}\n",
                req.project_id, req.template
            ),
        )?;
    }

    let gitignore = repo.join(".gitignore");
    if !gitignore.exists() {
        std::fs::write(
            &gitignore,
            ".gradle/\nbuild/\napp/build/\n*.apk\n*.aab\nlocal.properties\n.env\n.env.local\n",
        )?;
    }

    Ok(())
}

fn ensure_git_repo(repo: &Path) -> Result<()> {
    if !repo.join(".git").exists() {
        run_git(repo, ["init"])?;
    }

    let _ = run_git(repo, ["config", "user.name", "Elon PC Node"]);
    let _ = run_git(repo, ["config", "user.email", "node@elon.local"]);
    run_git(repo, ["add", "."])?;
    let _ = run_git(
        repo,
        ["commit", "-m", "chore: initialize pc managed project"],
    );
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

fn short_sha(sha: &str) -> String {
    sha.chars().take(12).collect()
}

fn git_head(repo: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(repo)
        .output()
        .context("failed to run git rev-parse")?;
    if !output.status.success() {
        return Err(anyhow!(
            "git rev-parse failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git_output(repo: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .context("failed to run git")?;
    if !output.status.success() {
        return Err(anyhow!(
            "git command failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_git<const N: usize>(repo: &Path, args: [&str; N]) -> Result<()> {
    run_git_dynamic(repo, &args)
}

fn run_git_dynamic(repo: &Path, args: &[&str]) -> Result<()> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .context("failed to run git")?;
    if !output.status.success() {
        return Err(anyhow!(
            "git command failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{prepare_conversation_workspace, safe_path_part};
    use std::fs;
    use uuid::Uuid;

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
}

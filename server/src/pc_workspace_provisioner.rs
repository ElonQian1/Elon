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

fn run_git<const N: usize>(repo: &Path, args: [&str; N]) -> Result<()> {
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
    use super::safe_path_part;

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
}

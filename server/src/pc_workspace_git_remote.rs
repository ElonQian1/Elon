use anyhow::{anyhow, Context, Result};
use elon_pc_dev_runtime::{ensure_project_git_baseline, ProjectGitBaselineRequest};
use std::path::Path;

use crate::git_command_error::{git_command, git_failure_message, git_spawn_context};

pub(crate) struct GitRemoteConfig {
    pub(crate) repo_url: String,
    pub(crate) branch: Option<String>,
}

pub(crate) fn clean_git_remote(
    repo_url: Option<&str>,
    branch: Option<&str>,
) -> Option<GitRemoteConfig> {
    let repo_url = repo_url
        .map(str::trim)
        .filter(|value| !value.is_empty())?
        .to_string();
    Some(GitRemoteConfig {
        repo_url,
        branch: clean_git_branch(branch),
    })
}

pub(crate) fn clean_git_branch(branch: Option<&str>) -> Option<String> {
    branch
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(crate) fn ensure_git_remote_workspace<F>(
    repo: &Path,
    remote: &GitRemoteConfig,
    seed_files: F,
) -> Result<()>
where
    F: FnOnce(&Path) -> Result<()>,
{
    let parent = repo
        .parent()
        .ok_or_else(|| anyhow!("invalid workspace path: {}", repo.display()))?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("failed to create workspace parent {}", parent.display()))?;

    if !repo.exists() || dir_is_empty(repo)? {
        clone_git_remote(&remote.repo_url, repo)?;
    } else if !is_git_work_tree(repo) {
        anyhow::bail!(
            "目标项目目录已存在但不是 Git 仓库，不能从远端重建: {}",
            repo.display()
        );
    }

    ensure_remote_origin(repo, &remote.repo_url)?;
    let _ = git_output(repo, &["fetch", "origin"])
        .context("Git 远端 fetch 失败，请检查目标 PC 的 Git 凭证和远端权限")?;
    ensure_requested_branch(repo, remote.branch.as_deref())?;

    let needs_seed_commit = !git_has_head(repo);
    if needs_seed_commit {
        seed_files(repo)?;
        ensure_project_git_baseline(
            repo,
            &ProjectGitBaselineRequest {
                branch: remote.branch.as_deref(),
            },
        )?;
        if let Some(branch) = current_branch(repo).or_else(|| remote.branch.clone()) {
            run_git_dynamic(repo, &["push", "-u", "origin", &branch]).with_context(|| {
                format!(
                    "初始化项目已提交，但推送到 Git 远端失败，请检查目标 PC 的 Git 凭证和远端权限: {}",
                    remote.repo_url
                )
            })?;
        }
    }

    Ok(())
}

pub(crate) fn git_remote_origin(repo: &Path) -> Result<String> {
    git_output(repo, &["remote", "get-url", "origin"])
}

fn dir_is_empty(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(true);
    }
    Ok(std::fs::read_dir(path)?.next().is_none())
}

fn clone_git_remote(repo_url: &str, repo: &Path) -> Result<()> {
    let output = git_command()
        .arg("clone")
        .arg(repo_url)
        .arg(repo)
        .output()
        .context("failed to run git clone")?;
    if !output.status.success() {
        return Err(anyhow!(
            "git clone failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

fn ensure_remote_origin(repo: &Path, repo_url: &str) -> Result<()> {
    match git_output(repo, &["remote", "get-url", "origin"]) {
        Ok(existing) if existing.trim() == repo_url => Ok(()),
        Ok(_) => run_git_dynamic(repo, &["remote", "set-url", "origin", repo_url]),
        Err(_) => run_git_dynamic(repo, &["remote", "add", "origin", repo_url]),
    }
}

fn ensure_requested_branch(repo: &Path, branch: Option<&str>) -> Result<()> {
    if let Some(branch) = branch {
        if current_branch(repo).as_deref() != Some(branch) {
            if local_branch_exists(repo, branch) {
                run_git_dynamic(repo, &["checkout", branch])?;
            } else if remote_branch_exists(repo, branch) {
                let remote_ref = format!("origin/{branch}");
                run_git_dynamic(repo, &["checkout", "-b", branch, "--track", &remote_ref])?;
            } else if git_has_head(repo) {
                anyhow::bail!(
                    "Git 远端不存在分支 {branch}，请确认分支名或先把本地代码 push 到远端"
                );
            } else {
                run_git_dynamic(repo, &["checkout", "-B", branch])?;
            }
        }
        if git_has_head(repo) && !remote_branch_exists(repo, branch) {
            anyhow::bail!("Git 远端不存在分支 {branch}，请确认分支名或先把本地代码 push 到远端");
        }
        if remote_branch_exists(repo, branch) {
            let remote_ref = format!("origin/{branch}");
            let _ = run_git_dynamic(repo, &["branch", "--set-upstream-to", &remote_ref, branch]);
            run_git_dynamic(repo, &["pull", "--ff-only", "origin", branch])?;
        }
        return Ok(());
    }

    if current_branch(repo).is_none() {
        if let Some(default_branch) = remote_default_branch(repo) {
            if local_branch_exists(repo, &default_branch) {
                run_git_dynamic(repo, &["checkout", &default_branch])?;
            } else {
                let remote_ref = format!("origin/{default_branch}");
                run_git_dynamic(
                    repo,
                    &["checkout", "-b", &default_branch, "--track", &remote_ref],
                )?;
            }
        }
    }

    if let Some(branch) = current_branch(repo) {
        if remote_branch_exists(repo, &branch) {
            run_git_dynamic(repo, &["pull", "--ff-only", "origin", &branch])?;
        }
    }
    Ok(())
}

fn is_git_work_tree(repo: &Path) -> bool {
    repo.exists()
        && git_output(repo, &["rev-parse", "--is-inside-work-tree"])
            .map(|value| value == "true")
            .unwrap_or(false)
}

fn git_has_head(repo: &Path) -> bool {
    git_output(repo, &["rev-parse", "--verify", "HEAD"]).is_ok()
}

fn local_branch_exists(repo: &Path, branch: &str) -> bool {
    git_output(
        repo,
        &["rev-parse", "--verify", &format!("refs/heads/{branch}")],
    )
    .is_ok()
}

fn remote_branch_exists(repo: &Path, branch: &str) -> bool {
    let remote_ref = format!("refs/remotes/origin/{branch}");
    git_output(repo, &["rev-parse", "--verify", &remote_ref]).is_ok()
}

fn current_branch(repo: &Path) -> Option<String> {
    git_output(repo, &["rev-parse", "--abbrev-ref", "HEAD"])
        .ok()
        .filter(|branch| !branch.is_empty() && branch != "HEAD")
}

fn remote_default_branch(repo: &Path) -> Option<String> {
    git_output(
        repo,
        &[
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ],
    )
    .ok()
    .and_then(|value| value.strip_prefix("origin/").map(ToOwned::to_owned))
}

fn git_output(repo: &Path, args: &[&str]) -> Result<String> {
    let output = git_command()
        .args(args)
        .current_dir(repo)
        .output()
        .with_context(|| format!("failed to run {}", git_spawn_context(args)))?;
    if !output.status.success() {
        return Err(anyhow!(git_failure_message(repo, args, &output)));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_git_dynamic(repo: &Path, args: &[&str]) -> Result<()> {
    let output = git_command()
        .args(args)
        .current_dir(repo)
        .output()
        .with_context(|| format!("failed to run {}", git_spawn_context(args)))?;
    if !output.status.success() {
        return Err(anyhow!(git_failure_message(repo, args, &output)));
    }
    Ok(())
}


#[cfg(test)]
#[path = "pc_workspace_git_remote_tests.rs"]
mod tests;

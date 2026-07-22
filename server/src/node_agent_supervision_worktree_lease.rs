//! Git-backed lease for Desktop-supervised isolated worktrees.
//!
//! Git's native worktree lock survives node restarts and is already honored by
//! the repository cleanup scripts. The reason embeds only the supervision root
//! identity, so terminal reconciliation or an accepted review can release
//! exactly the matching lease without touching foreign locks.

use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::git_command_error::{git_command, git_failure_message, git_spawn_context};

#[path = "node_agent_supervision_worktree_lease_transition.rs"]
mod transition;
pub(crate) use transition::{migrate_legacy_child_lease, ResumeAdmissionGuard};

const LEASE_PREFIX: &str = "elon-supervision:";

#[derive(Clone, Debug)]
pub(crate) struct WorktreeLock {
    pub(crate) path: std::path::PathBuf,
    pub(crate) reason: String,
}

pub(crate) fn acquire(base: &Path, active: &Path, root_task_id: &str) -> Result<()> {
    let expected = lease_reason(root_task_id)?;
    match worktree_lock_reason(base, active)? {
        Some(reason) if reason == expected => return Ok(()),
        Some(reason) => bail!("worktree is already locked by another lease: {reason}"),
        None => {}
    }
    run_git(
        base,
        &["worktree", "lock", "--reason", &expected, &path_arg(active)],
    )?;
    anyhow::ensure!(
        worktree_lock_reason(base, active)?.as_deref() == Some(expected.as_str()),
        "supervision worktree lease was not persisted"
    );
    Ok(())
}

pub(crate) fn release(base: &Path, active: &Path, root_task_id: &str) -> Result<()> {
    let expected = lease_reason(root_task_id)?;
    match worktree_lock_reason(base, active)? {
        None => Ok(()),
        Some(reason) if reason != expected => {
            bail!("refusing to release non-matching worktree lease: {reason}")
        }
        Some(_) => run_git(base, &["worktree", "unlock", &path_arg(active)]),
    }
}

pub(crate) fn worktree_lock_reason(base: &Path, active: &Path) -> Result<Option<String>> {
    let output = git_output(base, &["worktree", "list", "--porcelain"])?;
    let expected = canonical_or_original(active);
    for entry in output.split("\n\n") {
        let mut matches = false;
        let mut locked = None;
        for line in entry.lines() {
            if let Some(path) = line.strip_prefix("worktree ") {
                matches = same_path(&canonical_or_original(Path::new(path)), &expected);
            } else if line == "locked" {
                locked = Some(String::new());
            } else if let Some(reason) = line.strip_prefix("locked ") {
                locked = Some(reason.trim().to_string());
            }
        }
        if matches {
            return Ok(locked);
        }
    }
    Ok(None)
}

pub(crate) fn list_worktree_locks(base: &Path) -> Result<Vec<WorktreeLock>> {
    let output = git_output(base, &["worktree", "list", "--porcelain"])?;
    let mut locks = Vec::new();
    for entry in output.split("\n\n") {
        let mut path = None;
        let mut reason = None;
        for line in entry.lines() {
            if let Some(value) = line.strip_prefix("worktree ") {
                path = Some(canonical_or_original(Path::new(value)));
            } else if line == "locked" {
                reason = Some(String::new());
            } else if let Some(value) = line.strip_prefix("locked ") {
                reason = Some(value.trim().to_string());
            }
        }
        if let (Some(path), Some(reason)) = (path, reason) {
            locks.push(WorktreeLock { path, reason });
        }
    }
    Ok(locks)
}

pub(crate) fn supervision_lease_task_id(reason: &str) -> Option<&str> {
    reason
        .strip_prefix(LEASE_PREFIX)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(crate) fn is_supervision_lease(reason: &str) -> bool {
    reason.starts_with(LEASE_PREFIX)
}

pub(crate) fn lease_reason(root_task_id: &str) -> Result<String> {
    let root = root_task_id.trim();
    anyhow::ensure!(!root.is_empty(), "supervision root task id is empty");
    anyhow::ensure!(
        root.len() <= 200 && !root.chars().any(char::is_control),
        "supervision root task id is invalid"
    );
    Ok(format!("{LEASE_PREFIX}{root}"))
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
    Ok(String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n"))
}

fn canonical_or_original(path: &Path) -> std::path::PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn path_arg(path: &Path) -> String {
    let raw = path.to_string_lossy();
    raw.strip_prefix(r"\\?\UNC\")
        .map(|value| format!(r"\\{value}"))
        .or_else(|| raw.strip_prefix(r"\\?\").map(ToOwned::to_owned))
        .unwrap_or_else(|| raw.to_string())
}

fn same_path(left: &Path, right: &Path) -> bool {
    if cfg!(windows) {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    } else {
        left == right
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use uuid::Uuid;

    use super::*;

    #[test]
    fn lease_is_persistent_idempotent_and_identity_bound() {
        let temp = std::env::temp_dir().join(format!(
            "elon-supervision-lease-{}",
            Uuid::new_v4().simple()
        ));
        let base = temp.join("base");
        let active = temp.join("active");
        fs::create_dir_all(&base).unwrap();
        git(&base, &["init"]);
        git(&base, &["config", "user.email", "ai@example.test"]);
        git(&base, &["config", "user.name", "AI Test"]);
        fs::write(base.join("README.md"), "seed\n").unwrap();
        git(&base, &["add", "README.md"]);
        git(&base, &["commit", "-m", "seed"]);
        git(
            &base,
            &[
                "worktree",
                "add",
                "-b",
                "ai/session/project/conversation",
                &path_arg(&active),
                "HEAD",
            ],
        );

        acquire(&base, &active, "root-1").unwrap();
        acquire(&base, &active, "root-1").unwrap();
        assert_eq!(
            worktree_lock_reason(&base, &active).unwrap().as_deref(),
            Some("elon-supervision:root-1")
        );
        assert!(release(&base, &active, "root-2").is_err());
        release(&base, &active, "root-1").unwrap();
        assert_eq!(worktree_lock_reason(&base, &active).unwrap(), None);

        git(
            &base,
            &[
                "worktree",
                "lock",
                "--reason",
                "active PC CLI task; Resume or successful finalization unlocks",
                &path_arg(&active),
            ],
        );
        assert!(acquire(&base, &active, "root-2").is_err());
        assert_eq!(
            worktree_lock_reason(&base, &active).unwrap().as_deref(),
            Some("active PC CLI task; Resume or successful finalization unlocks")
        );
        git(&base, &["worktree", "unlock", &path_arg(&active)]);

        git(
            &base,
            &[
                "worktree",
                "lock",
                "--reason",
                "foreign-owner",
                &path_arg(&active),
            ],
        );
        assert!(acquire(&base, &active, "root-3").is_err());
        assert_eq!(
            worktree_lock_reason(&base, &active).unwrap().as_deref(),
            Some("foreign-owner")
        );
        let _ = fs::remove_dir_all(temp);
    }

    fn git(cwd: &Path, args: &[&str]) {
        let output = git_command().args(args).current_dir(cwd).output().unwrap();
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

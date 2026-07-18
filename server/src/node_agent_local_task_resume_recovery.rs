//! Safe reconstruction for a platform worktree whose Git registration was lost.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use uuid::Uuid;

use crate::{
    git_command_error::{git_command, git_failure_message, git_spawn_context},
    pc_workspace_provisioner::lock_conversation_worktree,
};

pub(super) struct MissingWorktreeRecovery {
    pub(super) git_head: String,
    pub(super) derivation: String,
}

pub(super) fn is_git_worktree(path: &Path) -> bool {
    git_output(path, &["rev-parse", "--is-inside-work-tree"]).is_ok_and(|value| value == "true")
}

pub(super) fn inspect_or_repair(
    base: &Path,
    active: &Path,
    branch: &str,
    recorded_head: Option<&str>,
    repair: bool,
) -> Result<MissingWorktreeRecovery> {
    if is_git_worktree(active) {
        bail!("活动工作区仍有 Git 元数据，拒绝按 orphaned worktree 重建。")
    }
    let (git_head, head_source) = recovery_head(base, branch, recorded_head)?;
    validate_branch_recovery_identity(base, branch, &git_head)?;
    if repair {
        rebuild_without_overwriting_files(base, active, branch, &git_head)?;
    }
    Ok(MissingWorktreeRecovery {
        git_head,
        derivation: if repair {
            format!("workspace_status_git_rebuilt_{head_source}")
        } else {
            format!("workspace_status_git_recovery_ready_{head_source}")
        },
    })
}

fn recovery_head(
    base: &Path,
    branch: &str,
    recorded_head: Option<&str>,
) -> Result<(String, &'static str)> {
    if let Some(recorded) = recorded_head
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let head = resolve_commit(base, recorded)
            .context("父任务记录的 git_head 在基础仓库中不存在，拒绝重建隔离 worktree")?;
        return Ok((head, "recorded_head"));
    }

    let branch_ref = format!("refs/heads/{branch}");
    let head = resolve_commit(base, &branch_ref)
        .context("旧父任务未记录 git_head，且隔离分支已不存在；无法猜测提交，已保护现场")?;
    Ok((head, "legacy_branch_ref"))
}

fn validate_branch_recovery_identity(base: &Path, branch: &str, head: &str) -> Result<()> {
    let branch_ref = format!("refs/heads/{branch}");
    if let Ok(branch_head) = resolve_commit(base, &branch_ref) {
        if !branch_head.eq_ignore_ascii_case(head) {
            bail!("隔离分支当前 HEAD ({branch_head}) 与父任务记录 ({head}) 不一致，已拒绝覆盖。");
        }
    }

    let registrations = git_output(base, &["worktree", "list", "--porcelain"])?;
    let expected_ref = format!("branch refs/heads/{branch}");
    if registrations.lines().any(|line| line == expected_ref) {
        bail!("隔离分支仍被其它 Git worktree 注册，已拒绝重复重建。")
    }
    Ok(())
}

fn rebuild_without_overwriting_files(
    base: &Path,
    active: &Path,
    branch: &str,
    head: &str,
) -> Result<()> {
    let parent = active
        .parent()
        .ok_or_else(|| anyhow!("活动 worktree 缺少父目录。"))?;
    let backup = parent.join(format!(
        ".{}-resume-backup-{}",
        active
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("conversation"),
        Uuid::new_v4().simple()
    ));

    std::fs::rename(active, &backup).with_context(|| {
        format!(
            "活动目录仍被存活后代进程占用，或无法建立恢复快照；现场保持不变: {}",
            active.display()
        )
    })?;

    let attempt = (|| -> Result<()> {
        add_metadata_only_worktree(base, active, branch, head)?;
        move_user_entries(&backup, active)?;
        validate_rebuilt_identity(base, active, branch, head)?;
        lock_conversation_worktree(base, active)?;
        remove_stale_git_marker_and_empty_backup(&backup)?;
        Ok(())
    })();
    if let Err(error) = attempt {
        let rollback_error = rollback_rebuild(base, active, &backup).err();
        return match rollback_error {
            Some(rollback) => Err(error).context(format!(
                "重建失败且自动回滚不完整；用户文件快照保留在 {}: {rollback:#}",
                backup.display()
            )),
            None => Err(error).context("重建失败，已恢复原目录且未覆盖用户文件"),
        };
    }
    Ok(())
}

fn add_metadata_only_worktree(base: &Path, active: &Path, branch: &str, head: &str) -> Result<()> {
    let active_arg = git_path_arg(active);
    let branch_ref = format!("refs/heads/{branch}");
    if resolve_commit(base, &branch_ref).is_ok() {
        run_git(
            base,
            &["worktree", "add", "--no-checkout", &active_arg, branch],
        )
    } else {
        run_git(
            base,
            &[
                "worktree",
                "add",
                "--no-checkout",
                "-b",
                branch,
                &active_arg,
                head,
            ],
        )
    }
}

fn move_user_entries(from: &Path, to: &Path) -> Result<()> {
    for entry in
        std::fs::read_dir(from).with_context(|| format!("无法读取恢复快照 {}", from.display()))?
    {
        let entry = entry?;
        if entry
            .file_name()
            .to_string_lossy()
            .eq_ignore_ascii_case(".git")
        {
            continue;
        }
        let destination = to.join(entry.file_name());
        if destination.exists() {
            bail!(
                "重建目标已存在同名路径，拒绝覆盖用户文件: {}",
                destination.display()
            );
        }
        std::fs::rename(entry.path(), &destination).with_context(|| {
            format!("无法把用户文件移回重建 worktree: {}", destination.display())
        })?;
    }
    Ok(())
}

fn validate_rebuilt_identity(base: &Path, active: &Path, branch: &str, head: &str) -> Result<()> {
    let actual_head = resolve_commit(active, "HEAD")?;
    if !actual_head.eq_ignore_ascii_case(head) {
        bail!("重建 worktree 的 HEAD 与父任务记录不一致。")
    }
    let actual_branch = git_output(active, &["branch", "--show-current"])?;
    if actual_branch != branch {
        bail!("重建 worktree 的分支与父任务记录不一致。")
    }
    let base_common = resolved_git_path(base, &["rev-parse", "--git-common-dir"])?;
    let active_common = resolved_git_path(active, &["rev-parse", "--git-common-dir"])?;
    if !same_path(&base_common, &active_common) {
        bail!("重建 worktree 未连接到父任务基础仓库。")
    }
    Ok(())
}

fn remove_stale_git_marker_and_empty_backup(backup: &Path) -> Result<()> {
    let stale_git = backup.join(".git");
    if stale_git.is_dir() {
        std::fs::remove_dir_all(&stale_git)?;
    } else if stale_git.exists() {
        std::fs::remove_file(&stale_git)?;
    }
    std::fs::remove_dir(backup).with_context(|| {
        format!(
            "恢复快照仍含未迁移文件，已停止清理并保留: {}",
            backup.display()
        )
    })
}

fn rollback_rebuild(base: &Path, active: &Path, backup: &Path) -> Result<()> {
    if active.exists() && backup.exists() {
        move_user_entries(active, backup)?;
    }
    let active_arg = git_path_arg(active);
    let _ = run_git(
        base,
        &["worktree", "remove", "--force", "--force", &active_arg],
    );
    if active.exists() {
        let git_marker = active.join(".git");
        if git_marker.is_file() {
            std::fs::remove_file(&git_marker)?;
        }
        if std::fs::read_dir(active)?.next().is_none() {
            std::fs::remove_dir(active)?;
        }
    }
    if !active.exists() && backup.exists() {
        std::fs::rename(backup, active)?;
    }
    Ok(())
}

fn resolve_commit(repo: &Path, reference: &str) -> Result<String> {
    git_output(
        repo,
        &["rev-parse", "--verify", &format!("{reference}^{{commit}}")],
    )
}

fn resolved_git_path(cwd: &Path, args: &[&str]) -> Result<PathBuf> {
    let raw = git_output(cwd, args)?;
    let path = PathBuf::from(raw.trim());
    let path = if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    };
    std::fs::canonicalize(&path)
        .with_context(|| format!("无法解析 Git 身份路径: {}", path.display()))
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

fn same_path(left: &Path, right: &Path) -> bool {
    if cfg!(windows) {
        display_path(left).eq_ignore_ascii_case(&display_path(right))
    } else {
        left == right
    }
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(windows)]
fn git_path_arg(path: &Path) -> String {
    let value = path.to_string_lossy();
    if let Some(rest) = value.strip_prefix("\\\\?\\UNC\\") {
        return format!("\\\\{rest}");
    }
    if let Some(rest) = value.strip_prefix("\\\\?\\") {
        return rest.to_string();
    }
    value.to_string()
}

#[cfg(not(windows))]
fn git_path_arg(path: &Path) -> String {
    display_path(path)
}

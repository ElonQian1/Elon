//! Read-only proof for migrating an orphaned platform worktree.
//!
//! The source directory is never renamed, removed, or repaired in place.  A
//! Resume route may use the resulting plan to create a new platform worktree
//! only after durable task/lease occupancy has also been proved safe.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use serde_json::Value;
use uuid::Uuid;

use crate::git_command_error::{git_command, git_failure_message, git_spawn_context};

#[derive(Clone, Debug)]
pub(crate) struct OrphanedWorkspaceMigration {
    pub(crate) source_path: String,
    pub(crate) source_branch: String,
    pub(crate) recorded_head: String,
    pub(crate) target_head: String,
}

pub(super) struct MissingWorktreeRecovery {
    pub(super) git_head: String,
    pub(super) derivation: String,
    pub(super) orphaned_migration: Option<OrphanedWorkspaceMigration>,
}

pub(super) fn is_git_worktree(path: &Path) -> bool {
    git_output(path, &["rev-parse", "--is-inside-work-tree"]).is_ok_and(|value| value == "true")
}

#[allow(clippy::too_many_arguments)]
pub(super) fn inspect_or_repair(
    base: &Path,
    active: &Path,
    branch: &str,
    recorded_head: Option<&str>,
    supervision_root_task_id: &str,
    workspace_status: Option<&Value>,
    _repair: bool,
) -> Result<MissingWorktreeRecovery> {
    if is_git_worktree(active) {
        bail!("活动工作区仍有 Git 元数据，拒绝按 orphaned worktree 迁移。")
    }
    let status = workspace_status.context("孤儿工作区缺少持久 workspace_status")?;
    validate_status_identity(base, active, branch, supervision_root_task_id, status)?;
    validate_stale_git_marker(base, active)?;

    let recorded_head = recorded_head
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .context("孤儿工作区缺少记录 HEAD，拒绝猜测迁移来源")?;
    let recorded_head =
        resolve_commit(base, recorded_head).context("孤儿工作区记录 HEAD 在授权仓库中不存在")?;
    let branch_ref = format!("refs/heads/{branch}");
    let target_head = resolve_commit(base, &branch_ref).context("孤儿工作区记录分支已不存在")?;
    ensure_ancestor(
        base,
        &recorded_head,
        &target_head,
        "记录 HEAD 不是分支 HEAD 的祖先",
    )?;
    let origin_main = resolve_commit(base, "refs/remotes/origin/main")
        .context("授权仓库缺少 origin/main 身份")?;
    ensure_ancestor(
        base,
        &target_head,
        &origin_main,
        "孤儿分支 HEAD 尚未进入 origin/main",
    )?;
    validate_clean_against_commit(base, active, &target_head)?;

    Ok(MissingWorktreeRecovery {
        git_head: target_head.clone(),
        derivation: "orphaned_workspace_controlled_migration_ready_branch_head".to_string(),
        orphaned_migration: Some(OrphanedWorkspaceMigration {
            source_path: active.to_string_lossy().to_string(),
            source_branch: branch.to_string(),
            recorded_head,
            target_head,
        }),
    })
}

fn validate_status_identity(
    base: &Path,
    active: &Path,
    branch: &str,
    root_task_id: &str,
    status: &Value,
) -> Result<()> {
    optional_eq(
        status,
        "platform_provenance",
        "elon.conversation_worktree.v1",
    )?;
    optional_eq(status, "root_task_id", root_task_id)?;
    let recorded_base = canonical(Path::new(required(status, "base_workspace_path")?))?;
    let recorded_active = canonical(Path::new(required(status, "active_workspace_path")?))?;
    anyhow::ensure!(
        same_path(base, &recorded_base),
        "授权 base repo 与记录路径漂移"
    );
    anyhow::ensure!(
        same_path(active, &recorded_active),
        "孤儿目录与记录路径漂移"
    );
    required_eq(status, "branch", branch)?;

    let current_common = git_path(
        base,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    if let Some(recorded) = optional(status, "git_common_dir") {
        let recorded_common = canonical(Path::new(recorded))?;
        anyhow::ensure!(
            same_path(&current_common, &recorded_common),
            "Git common-dir 身份漂移"
        );
    }
    let remote = git_output(base, &["config", "--get", "remote.origin.url"])?;
    anyhow::ensure!(
        !remote.trim().is_empty(),
        "授权 base repo 缺少 origin remote"
    );
    optional_eq(status, "git_remote", remote.trim())
}

fn validate_stale_git_marker(base: &Path, active: &Path) -> Result<()> {
    let marker = active.join(".git");
    let metadata = std::fs::symlink_metadata(&marker)
        .with_context(|| format!("孤儿目录缺少原始 .git marker: {}", marker.display()))?;
    anyhow::ensure!(
        metadata.file_type().is_file(),
        "孤儿 .git marker 不是普通文件"
    );
    let raw = std::fs::read_to_string(&marker)?;
    let gitdir = raw
        .trim()
        .strip_prefix("gitdir:")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .context("孤儿 .git marker 格式无效")?;
    anyhow::ensure!(gitdir.is_absolute(), "孤儿 .git marker 不是绝对路径");
    anyhow::ensure!(
        !gitdir.exists(),
        "孤儿 .git marker 目标仍存在，拒绝绕过 Git 注册"
    );
    let common = git_path(
        base,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    let worktrees = common.join("worktrees");
    let parent = gitdir
        .parent()
        .context("孤儿 .git marker 缺少 admin 父目录")?;
    anyhow::ensure!(
        same_path(parent, &worktrees),
        "孤儿 .git marker 指向其它 Git common-dir"
    );
    Ok(())
}

fn validate_clean_against_commit(base: &Path, active: &Path, head: &str) -> Result<()> {
    let index = std::env::temp_dir().join(format!(
        "elon-orphan-compare-{}-{}.index",
        std::process::id(),
        Uuid::new_v4().simple()
    ));
    let result = (|| -> Result<()> {
        git_with_index(base, &index, None, &["read-tree", head])?;
        let tracked = git_with_index(
            base,
            &index,
            Some(active),
            &["diff", "--name-status", "--no-ext-diff"],
        )?;
        let untracked = git_with_index(
            base,
            &index,
            Some(active),
            &["ls-files", "--others", "--exclude-standard"],
        )?;
        let tracked = business_changes(&tracked);
        let untracked = business_changes(&untracked);
        let status = tracked
            .into_iter()
            .chain(untracked)
            .collect::<Vec<_>>()
            .join("\n");
        anyhow::ensure!(
            status.trim().is_empty(),
            "孤儿目录业务内容与分支目标提交不一致或存在未保存差异: {}",
            status.lines().take(8).collect::<Vec<_>>().join(" | ")
        );
        Ok(())
    })();
    let _ = std::fs::remove_file(&index);
    result
}

fn business_changes(output: &str) -> Vec<&str> {
    output
        .lines()
        .filter(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            let paths = if fields.len() > 1 {
                &fields[1..]
            } else {
                &fields[..]
            };
            paths.iter().any(|path| !platform_control_path(path.trim()))
        })
        .collect()
}

fn platform_control_path(path: &str) -> bool {
    let path = path.trim_start_matches("./").replace('\\', "/");
    path == ".aiignore"
        || [".agents/", ".ai/", ".codex/", ".copilot/", ".elon/"]
            .iter()
            .any(|prefix| path.starts_with(prefix))
}

fn git_with_index(
    cwd: &Path,
    index: &Path,
    worktree: Option<&Path>,
    args: &[&str],
) -> Result<String> {
    let mut command = git_command();
    command
        .args(args)
        .current_dir(cwd)
        .env("GIT_INDEX_FILE", index);
    if let Some(worktree) = worktree {
        command.env("GIT_WORK_TREE", worktree);
    }
    let output = command.output().with_context(|| git_spawn_context(args))?;
    if !output.status.success() {
        bail!(git_failure_message(cwd, args, &output));
    }
    Ok(String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n"))
}

fn ensure_ancestor(base: &Path, ancestor: &str, descendant: &str, message: &str) -> Result<()> {
    let status = git_command()
        .args(["merge-base", "--is-ancestor", ancestor, descendant])
        .current_dir(base)
        .status()
        .context("验证孤儿工作区提交谱系")?;
    anyhow::ensure!(status.success(), "{message}");
    Ok(())
}

fn resolve_commit(base: &Path, reference: &str) -> Result<String> {
    git_output(
        base,
        &["rev-parse", "--verify", &format!("{reference}^{{commit}}")],
    )
}

fn git_path(cwd: &Path, args: &[&str]) -> Result<PathBuf> {
    canonical(Path::new(git_output(cwd, args)?.trim()))
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

fn required<'a>(status: &'a Value, field: &str) -> Result<&'a str> {
    status
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("孤儿 workspace_status 缺少 {field}"))
}

fn optional<'a>(status: &'a Value, field: &str) -> Option<&'a str> {
    status
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn required_eq(status: &Value, field: &str, expected: &str) -> Result<()> {
    anyhow::ensure!(
        required(status, field)? == expected,
        "孤儿 workspace_status 的 {field} 身份漂移"
    );
    Ok(())
}

fn optional_eq(status: &Value, field: &str, expected: &str) -> Result<()> {
    if let Some(recorded) = optional(status, field) {
        anyhow::ensure!(
            recorded == expected,
            "孤儿 workspace_status 的 {field} 身份漂移"
        );
    }
    Ok(())
}

fn canonical(path: &Path) -> Result<PathBuf> {
    std::fs::canonicalize(path).with_context(|| format!("无法解析身份路径 {}", path.display()))
}

fn same_path(left: &Path, right: &Path) -> bool {
    if cfg!(windows) {
        normalize_windows_path(left).eq_ignore_ascii_case(&normalize_windows_path(right))
    } else {
        left == right
    }
}

fn normalize_windows_path(path: &Path) -> String {
    let raw = path.to_string_lossy().replace('/', "\\");
    raw.strip_prefix(r"\\?\UNC\")
        .map(|value| format!(r"\\{value}"))
        .or_else(|| raw.strip_prefix(r"\\?\").map(ToOwned::to_owned))
        .unwrap_or(raw)
        .trim_end_matches('\\')
        .to_string()
}

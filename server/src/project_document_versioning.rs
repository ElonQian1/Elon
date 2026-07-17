//! Readable document history, bounded diffs, and reversible document-only commits.

use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;
use std::{
    path::Path,
    process::{Command, Output},
};

use crate::project_document_vault::{is_managed_vault, restore_version as restore_vault_version};

const MAX_DIFF_CHARS: usize = 60_000;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DocumentVersion {
    pub commit: String,
    pub created_at: String,
    pub summary: String,
    pub changed_paths: Vec<String>,
    pub document_only: bool,
    pub reversible: bool,
    pub mode: &'static str,
}

pub(crate) fn list_document_versions(
    workspace: &Path,
    limit: usize,
) -> Result<Vec<DocumentVersion>> {
    ensure_git(workspace)?;
    let output = git(
        workspace,
        &[
            "log",
            &format!("-n{}", limit.clamp(1, 100)),
            "--format=%H%x1f%cI%x1f%s%x1e",
            "--",
            ":(glob)**/*.md",
            ":(glob).elon/*.json",
        ],
    )?;
    let mut versions = Vec::new();
    for record in String::from_utf8(output.stdout)?.split('\u{1e}') {
        let fields = record.trim().split('\u{1f}').collect::<Vec<_>>();
        if fields.len() != 3 {
            continue;
        }
        let paths = commit_paths(workspace, fields[0])?;
        let document_only = !paths.is_empty() && paths.iter().all(|path| is_document_path(path));
        versions.push(DocumentVersion {
            commit: fields[0].to_string(),
            created_at: fields[1].to_string(),
            summary: fields[2].to_string(),
            changed_paths: paths,
            document_only,
            reversible: is_managed_vault(workspace)
                || document_only && !is_merge_commit(workspace, fields[0])?,
            mode: if is_managed_vault(workspace) {
                "managed_snapshot"
            } else {
                "document_commit"
            },
        });
    }
    Ok(versions)
}

pub(crate) fn document_version_diff(
    workspace: &Path,
    commit: &str,
    path: Option<&str>,
) -> Result<serde_json::Value> {
    let commit = verified_commit(workspace, commit)?;
    let mut args = vec![
        "show",
        "--format=fuller",
        "--stat",
        "--patch",
        "--unified=3",
        &commit,
        "--",
    ];
    let normalized_path;
    if let Some(path) = path.filter(|value| !value.trim().is_empty()) {
        normalized_path = path.trim().replace('\\', "/");
        if !is_document_path(&normalized_path)
            || normalized_path.contains("..")
            || Path::new(&normalized_path).is_absolute()
        {
            bail!("版本差异只能查看项目内文档");
        }
        args.push(&normalized_path);
    } else {
        args.extend([":(glob)**/*.md", ":(glob).elon/*.json"]);
    }
    let output = git(workspace, &args)?;
    let full = String::from_utf8(output.stdout)?;
    let patch = full.chars().take(MAX_DIFF_CHARS).collect::<String>();
    Ok(serde_json::json!({
        "commit": commit,
        "path": path,
        "diff": patch,
        "truncated": patch.chars().count() < full.chars().count(),
        "changed_paths": commit_paths(workspace, &commit)?,
    }))
}

pub(crate) fn restore_document_version(
    workspace: &Path,
    commit: &str,
) -> Result<serde_json::Value> {
    let commit = verified_commit(workspace, commit)?;
    if is_managed_vault(workspace) {
        let restored = restore_vault_version(workspace, &commit)?;
        return Ok(
            serde_json::json!({"commit": restored, "restored_from": commit, "mode": "managed_snapshot"}),
        );
    }
    if is_merge_commit(workspace, &commit)? {
        bail!("合并提交不能一键回滚，请选择独立的仅文档提交")
    }
    let paths = commit_paths(workspace, &commit)?;
    if paths.is_empty() || !paths.iter().all(|path| is_document_path(path)) {
        bail!("为了保护代码，只能一键回滚完全由文档组成的提交");
    }
    let status = git(workspace, &["status", "--porcelain"])?;
    if !status.stdout.is_empty() {
        bail!("工作区存在未提交修改；请先保存或提交，再回滚文档版本")
    }
    let output = Command::new("git")
        .current_dir(workspace)
        .args(["revert", "--no-edit", &commit])
        .output()
        .context("无法启动 Git 文档回滚")?;
    if !output.status.success() {
        let _ = Command::new("git")
            .current_dir(workspace)
            .args(["revert", "--abort"])
            .output();
        return Err(anyhow!(
            "文档回滚产生冲突，已安全中止：{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let restored = String::from_utf8(git(workspace, &["rev-parse", "HEAD"])?.stdout)?
        .trim()
        .to_string();
    Ok(
        serde_json::json!({"commit": restored, "restored_from": commit, "mode": "document_revert", "changed_paths": paths}),
    )
}

fn commit_paths(workspace: &Path, commit: &str) -> Result<Vec<String>> {
    let output = git(
        workspace,
        &[
            "diff-tree",
            "--root",
            "--no-commit-id",
            "--name-only",
            "-r",
            commit,
        ],
    )?;
    Ok(String::from_utf8(output.stdout)?
        .lines()
        .map(|line| line.trim().replace('\\', "/"))
        .filter(|line| !line.is_empty())
        .collect())
}

fn verified_commit(workspace: &Path, value: &str) -> Result<String> {
    ensure_git(workspace)?;
    let value = value.trim();
    if value.len() < 7 || value.len() > 64 || !value.chars().all(|ch| ch.is_ascii_hexdigit()) {
        bail!("版本提交格式无效");
    }
    let output = git(
        workspace,
        &["rev-parse", "--verify", &format!("{value}^{{commit}}")],
    )?;
    let commit = String::from_utf8(output.stdout)?.trim().to_string();
    if !git_status(workspace, &["merge-base", "--is-ancestor", &commit, "HEAD"])? {
        bail!("只能操作当前项目历史中的版本");
    }
    Ok(commit)
}

fn is_merge_commit(workspace: &Path, commit: &str) -> Result<bool> {
    let line =
        String::from_utf8(git(workspace, &["rev-list", "--parents", "-n", "1", commit])?.stdout)?;
    Ok(line.split_whitespace().count() > 2)
}

fn is_document_path(path: &str) -> bool {
    let lower = path.replace('\\', "/").to_ascii_lowercase();
    lower.ends_with(".md") || lower.starts_with(".elon/") && lower.ends_with(".json")
}

fn ensure_git(workspace: &Path) -> Result<()> {
    if !workspace.join(".git").exists()
        && !git_status(workspace, &["rev-parse", "--is-inside-work-tree"])?
    {
        bail!("项目不是 Git 工作区，无法提供版本差异与回滚");
    }
    Ok(())
}

fn git(workspace: &Path, args: &[&str]) -> Result<Output> {
    let output = Command::new("git")
        .current_dir(workspace)
        .args(args)
        .output()
        .context("无法启动 Git 文档版本操作")?;
    if !output.status.success() {
        return Err(anyhow!(
            "git {} 失败：{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output)
}

fn git_status(workspace: &Path, args: &[&str]) -> Result<bool> {
    Ok(Command::new("git")
        .current_dir(workspace)
        .args(args)
        .output()
        .context("无法检查 Git 文档版本")?
        .status
        .success())
}

#[cfg(test)]
#[path = "project_document_versioning_tests.rs"]
mod tests;

//! Platform-managed Git vaults for users who never interact with Git directly.

use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Output,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::project_document_index::state_root;

pub(crate) const MANAGED_VAULT_MARKER: &str = ".elon/managed-vault.json";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ManagedVault {
    pub vault_id: String,
    pub workspace: PathBuf,
    pub created: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ManagedVaultVersion {
    pub commit: String,
    pub created_at: String,
    pub summary: String,
}

pub(crate) fn resolve_or_create(vault_id: &str) -> Result<ManagedVault> {
    let vault_id = validate_vault_id(vault_id)?;
    let root = state_root().join("vaults");
    fs::create_dir_all(&root)?;
    let workspace = root.join(&vault_id);
    let created = !workspace.join(MANAGED_VAULT_MARKER).is_file();
    if created {
        create_vault(&workspace, &vault_id)?;
    } else {
        validate_vault(&workspace, &vault_id)?;
    }
    Ok(ManagedVault {
        vault_id,
        workspace: workspace.canonicalize()?,
        created,
    })
}

pub(crate) fn is_managed_vault(workspace: &Path) -> bool {
    workspace.join(MANAGED_VAULT_MARKER).is_file()
}

pub(crate) fn checkpoint_after_write(workspace: &Path, path: &str) -> Result<Option<String>> {
    if !is_managed_vault(workspace) {
        return Ok(None);
    }
    git(workspace, &["add", "-A", "--", "."])?;
    if git_status(workspace, &["diff", "--cached", "--quiet"])? {
        return Ok(Some(current_head(workspace)?));
    }
    git(
        workspace,
        &[
            "commit",
            "-m",
            &format!("chore(notes): 自动保存 {}", sanitize_summary(path)),
        ],
    )?;
    Ok(Some(current_head(workspace)?))
}

pub(crate) fn list_versions(workspace: &Path, limit: usize) -> Result<Vec<ManagedVaultVersion>> {
    ensure_managed(workspace)?;
    let output = git(
        workspace,
        &[
            "log",
            &format!("-n{}", limit.clamp(1, 100)),
            "--format=%H%x1f%cI%x1f%s%x1e",
        ],
    )?;
    Ok(String::from_utf8(output.stdout)?
        .split('\u{1e}')
        .filter_map(|record| {
            let fields = record.trim().split('\u{1f}').collect::<Vec<_>>();
            (fields.len() == 3).then(|| ManagedVaultVersion {
                commit: fields[0].to_string(),
                created_at: fields[1].to_string(),
                summary: fields[2].to_string(),
            })
        })
        .collect())
}

pub(crate) fn current_version(workspace: &Path) -> Result<String> {
    ensure_managed(workspace)?;
    current_head(workspace)
}

pub(crate) fn contains_version(workspace: &Path, commit: &str) -> Result<bool> {
    ensure_managed(workspace)?;
    git_status(
        workspace,
        &["merge-base", "--is-ancestor", commit.trim(), "HEAD"],
    )
}

pub(crate) fn restore_version(workspace: &Path, commit: &str) -> Result<String> {
    ensure_managed(workspace)?;
    let commit = commit.trim();
    if commit.is_empty()
        || !git_status(workspace, &["merge-base", "--is-ancestor", commit, "HEAD"])?
    {
        bail!("只能恢复当前知识库历史中的版本");
    }
    if commit == current_head(workspace)? {
        return Ok(commit.to_string());
    }
    checkpoint_after_write(workspace, "恢复前检查点")?;
    git(workspace, &["read-tree", "--reset", "-u", commit])?;
    git(workspace, &["commit", "-m", "chore(notes): 恢复历史版本"])?;
    current_head(workspace)
}

fn create_vault(workspace: &Path, vault_id: &str) -> Result<()> {
    fs::create_dir_all(workspace.join(".elon"))?;
    fs::create_dir_all(workspace.join("notes/inbox"))?;
    git(workspace, &["init", "--initial-branch=main"])?;
    git(
        workspace,
        &["config", "user.name", "Yilong Knowledge Vault"],
    )?;
    git(
        workspace,
        &["config", "user.email", "knowledge-vault@local"],
    )?;
    fs::write(
        workspace.join(MANAGED_VAULT_MARKER),
        format!(
            "{{\n  \"version\": 1,\n  \"vault_id\": \"{}\",\n  \"created_at_ms\": {}\n}}\n",
            vault_id,
            now_millis()
        ),
    )?;
    fs::write(
        workspace.join("README.md"),
        "# 我的知识库\n\n这里的笔记由一龙自动保存版本。用户无需了解 Git。\n",
    )?;
    fs::write(
        workspace.join("notes/inbox/README.md"),
        "# 收件箱\n\n尚未整理的想法和笔记先放在这里。\n",
    )?;
    fs::write(
        workspace.join(".elon/document-sections.json"),
        "{\n  \"version\": 1,\n  \"profile\": \"personal-knowledge\",\n  \"home\": {\n    \"title\": \"我的知识库\",\n    \"summary\": \"由一龙持续维护并自动保存历史版本的个人知识库。\",\n    \"entrypoint\": \"README.md\",\n    \"start_here\": [\"README.md\"]\n  },\n  \"sections\": [],\n  \"assignments\": {},\n  \"secondary_assignments\": {},\n  \"governance_facets\": {},\n  \"governance_overrides\": {},\n  \"document_metadata\": {},\n  \"audit_log\": []\n}\n",
    )?;
    git(workspace, &["add", "-A", "--", "."])?;
    git(
        workspace,
        &["commit", "-m", "chore(notes): 创建一龙托管知识库"],
    )?;
    Ok(())
}

fn validate_vault(workspace: &Path, vault_id: &str) -> Result<()> {
    let marker = fs::read_to_string(workspace.join(MANAGED_VAULT_MARKER))?;
    let value: serde_json::Value = serde_json::from_str(&marker)?;
    if value.get("vault_id").and_then(|value| value.as_str()) != Some(vault_id) {
        bail!("托管知识库身份校验失败");
    }
    ensure_managed(workspace)
}

fn ensure_managed(workspace: &Path) -> Result<()> {
    if !is_managed_vault(workspace) || !workspace.join(".git").exists() {
        bail!("该工作区不是一龙托管知识库");
    }
    Ok(())
}

fn validate_vault_id(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 64
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
    {
        bail!("vaultId 只能包含 1-64 个字母、数字、点、下划线或连字符");
    }
    Ok(value.to_string())
}

fn current_head(workspace: &Path) -> Result<String> {
    let output = git(workspace, &["rev-parse", "HEAD"])?;
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn git(workspace: &Path, args: &[&str]) -> Result<Output> {
    fs::create_dir_all(workspace)?;
    let output = crate::git_command_error::git_command()
        .current_dir(workspace)
        .args(args)
        .env("GIT_AUTHOR_NAME", "Yilong Knowledge Vault")
        .env("GIT_AUTHOR_EMAIL", "knowledge-vault@local")
        .env("GIT_COMMITTER_NAME", "Yilong Knowledge Vault")
        .env("GIT_COMMITTER_EMAIL", "knowledge-vault@local")
        .output()
        .context("无法启动 Git 知识库事务")?;
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
    let output = crate::git_command_error::git_command()
        .current_dir(workspace)
        .args(args)
        .output()
        .context("无法检查 Git 知识库状态")?;
    Ok(output.status.success())
}

fn sanitize_summary(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_control())
        .take(120)
        .collect::<String>()
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "project_document_vault_tests.rs"]
mod tests;

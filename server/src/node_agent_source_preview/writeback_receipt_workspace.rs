use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::parser::canonical_project_root;
use crate::node_agent_android_live::fit_run::workspace_fingerprint;

const MAX_CHANGED_FILES: usize = 256;
const SOURCE_HASH_DELETED: &str = "deleted";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct WorkspaceSnapshot {
    pub(super) root: PathBuf,
    pub(super) head: String,
    pub(super) dirty_hashes: BTreeMap<String, String>,
    pub(super) source_revision: String,
}

pub(super) fn snapshot_workspace(project_root: &str) -> Result<WorkspaceSnapshot> {
    let root = canonical_project_root(project_root)?;
    let head = String::from_utf8(git_output(&root, &["rev-parse", "--verify", "HEAD"])?)
        .context("Git HEAD 不是 UTF-8")?
        .trim()
        .to_string();
    if head.is_empty() {
        bail!("WRITEBACK_RECEIPT_GIT_REQUIRED：项目缺少 Git HEAD");
    }
    let dirty = current_dirty_files(&root)?;
    let dirty_hashes = hashes_for_paths(&root, &dirty)?;
    let source_revision = workspace_fingerprint(root.to_string_lossy().as_ref())?
        .ok_or_else(|| anyhow!("无法生成内容敏感 sourceRevision"))?;
    Ok(WorkspaceSnapshot {
        root,
        head,
        dirty_hashes,
        source_revision,
    })
}

pub(super) fn operation_changed_files(
    baseline: &WorkspaceSnapshot,
    current: &WorkspaceSnapshot,
) -> Result<BTreeSet<String>> {
    let current_dirty = current_dirty_files(&current.root)?;
    let mut candidates = current_dirty.clone();
    candidates.extend(baseline.dirty_hashes.keys().cloned());
    if baseline.head != current.head {
        candidates.extend(git_name_list(
            &current.root,
            &[
                "diff",
                "--name-only",
                "-z",
                &baseline.head,
                &current.head,
                "--",
            ],
        )?);
    }
    if candidates.len() > MAX_CHANGED_FILES {
        bail!("本次写回涉及超过 {MAX_CHANGED_FILES} 个文件，拒绝生成截断回执");
    }
    let current_hashes = hashes_for_paths(&current.root, &candidates)?;
    Ok(candidates
        .into_iter()
        .filter(|path| baseline.dirty_hashes.get(path) != current_hashes.get(path))
        .collect())
}

fn current_dirty_files(root: &Path) -> Result<BTreeSet<String>> {
    let mut paths = git_name_list(root, &["diff", "--name-only", "-z", "HEAD", "--"])?;
    paths.extend(git_name_list(
        root,
        &["ls-files", "--others", "--exclude-standard", "-z"],
    )?);
    Ok(paths)
}

fn git_name_list(root: &Path, args: &[&str]) -> Result<BTreeSet<String>> {
    let output = git_output(root, args)?;
    let mut paths = BTreeSet::new();
    for raw in output
        .split(|byte| *byte == 0)
        .filter(|value| !value.is_empty())
    {
        let value = std::str::from_utf8(raw).context("Git 返回了非 UTF-8 路径")?;
        paths.insert(normalize_git_path(value)?);
    }
    Ok(paths)
}

pub(super) fn hashes_for_paths(
    root: &Path,
    paths: &BTreeSet<String>,
) -> Result<BTreeMap<String, String>> {
    let mut result = BTreeMap::new();
    for relative in paths {
        let path = root.join(relative);
        if !path.exists() {
            result.insert(relative.clone(), SOURCE_HASH_DELETED.into());
            continue;
        }
        let canonical = path
            .canonicalize()
            .with_context(|| format!("无法解析回执文件: {relative}"))?;
        if !canonical.starts_with(root) || !canonical.is_file() {
            bail!("回执文件越出项目目录或不是普通文件: {relative}");
        }
        result.insert(
            relative.clone(),
            hex::encode(Sha256::digest(fs::read(&canonical)?)),
        );
    }
    Ok(result)
}

fn git_output(root: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = crate::git_command_error::git_command()
        .args(args)
        .current_dir(root)
        .output()
        .with_context(|| {
            format!(
                "无法执行 {}",
                crate::git_command_error::git_spawn_context(args)
            )
        })?;
    if !output.status.success() {
        bail!(crate::git_command_error::git_failure_message(
            root, args, &output
        ));
    }
    Ok(output.stdout)
}

fn normalize_git_path(value: &str) -> Result<String> {
    let normalized = value.trim().replace('\\', "/");
    let path = Path::new(&normalized);
    if normalized.is_empty()
        || normalized.len() > 1_000
        || path.is_absolute()
        || normalized
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == ".." || part.contains('\0'))
    {
        bail!("Git 返回了不安全的相对路径");
    }
    Ok(normalized)
}

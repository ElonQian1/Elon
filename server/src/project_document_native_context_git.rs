//! Git-object identity for portable project-navigation evidence.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

use crate::project_document_native_context::ProjectContextEvidence;

const MAX_TREE_BYTES: usize = 8 * 1024 * 1024;
const MAX_RELOCATIONS: usize = 3;

#[derive(Debug, Default)]
pub(crate) struct GitRelocationIndex {
    by_oid: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ProjectContextGitIdentity {
    #[serde(default)]
    pub schema: String,
    #[serde(default)]
    pub head_commit: String,
    #[serde(default)]
    pub head_blob_oid: String,
    #[serde(default)]
    pub worktree_blob_oid: String,
    #[serde(default)]
    pub state: String,
}

pub(crate) fn capture(workspace: &Path, evidence_path: &str) -> Option<ProjectContextGitIdentity> {
    let head_commit = git_text(workspace, &["rev-parse", "--verify", "HEAD"])
        .filter(|value| valid_oid(value))
        .unwrap_or_default();
    let revision = format!("HEAD:{evidence_path}");
    let head_blob_oid = git_text(workspace, &["rev-parse", "--verify", &revision])
        .filter(|value| valid_oid(value))
        .unwrap_or_default();
    let worktree_blob_oid = git_text(
        workspace,
        &["hash-object", "--path", evidence_path, "--", evidence_path],
    )
    .filter(|value| valid_oid(value))
    .unwrap_or_default();
    if head_commit.is_empty() && head_blob_oid.is_empty() && worktree_blob_oid.is_empty() {
        return None;
    }
    let status = git_text(
        workspace,
        &["status", "--porcelain=v1", "--", evidence_path],
    );
    let state = match status.as_deref().map(str::trim) {
        Some("") if !head_blob_oid.is_empty() => "tracked_clean",
        Some(value) if value.starts_with("??") => "untracked",
        Some(_) if !head_blob_oid.is_empty() => "tracked_modified",
        Some(_) => "index_only",
        None if !head_blob_oid.is_empty() && head_blob_oid == worktree_blob_oid => "tracked_clean",
        None if !head_blob_oid.is_empty() => "tracked_modified",
        None => "untracked",
    };
    Some(ProjectContextGitIdentity {
        schema: "elon.project_context_git_identity.v1".to_string(),
        head_commit,
        head_blob_oid,
        worktree_blob_oid,
        state: state.to_string(),
    })
}

pub(crate) fn normalize(
    identity: Option<ProjectContextGitIdentity>,
) -> Result<Option<ProjectContextGitIdentity>> {
    let Some(mut identity) = identity else {
        return Ok(None);
    };
    identity.schema = identity.schema.trim().to_string();
    identity.head_commit = identity.head_commit.trim().to_ascii_lowercase();
    identity.head_blob_oid = identity.head_blob_oid.trim().to_ascii_lowercase();
    identity.worktree_blob_oid = identity.worktree_blob_oid.trim().to_ascii_lowercase();
    identity.state = identity.state.trim().to_ascii_lowercase();
    if identity.schema != "elon.project_context_git_identity.v1"
        || (!identity.head_commit.is_empty() && !valid_oid(&identity.head_commit))
        || (!identity.head_blob_oid.is_empty() && !valid_oid(&identity.head_blob_oid))
        || (!identity.worktree_blob_oid.is_empty() && !valid_oid(&identity.worktree_blob_oid))
        || !matches!(
            identity.state.as_str(),
            "tracked_clean" | "tracked_modified" | "index_only" | "untracked"
        )
    {
        bail!("项目导航记忆 git_identity 无效");
    }
    Ok(Some(identity))
}

/// Returns `None` when Git identity is absent or Git is unavailable, allowing
/// callers to fall back to the raw workspace SHA-256 identity.
pub(crate) fn is_current(workspace: &Path, evidence: &ProjectContextEvidence) -> Option<bool> {
    let stored = evidence.git_identity.as_ref()?;
    let current = capture(workspace, &evidence.path)?;
    if stored.state == "tracked_clean"
        && !stored.head_blob_oid.is_empty()
        && current.head_blob_oid == stored.head_blob_oid
        && current.worktree_blob_oid == stored.head_blob_oid
    {
        return Some(true);
    }
    if !stored.worktree_blob_oid.is_empty() && current.worktree_blob_oid == stored.worktree_blob_oid
    {
        return Some(true);
    }
    Some(false)
}

pub(crate) fn relocation_index(workspace: &Path) -> GitRelocationIndex {
    let Some(output) = git_output(workspace, &["ls-tree", "-r", "-z", "--full-tree", "HEAD"])
    else {
        return GitRelocationIndex::default();
    };
    if !output.status.success() || output.stdout.len() > MAX_TREE_BYTES {
        return GitRelocationIndex::default();
    }
    let mut index = GitRelocationIndex::default();
    for entry in output.stdout.split(|byte| *byte == 0) {
        if entry.is_empty() {
            continue;
        }
        let entry = String::from_utf8_lossy(entry);
        let Some((metadata, path)) = entry.split_once('\t') else {
            continue;
        };
        let mut fields = metadata.split_whitespace();
        let _mode = fields.next();
        let kind = fields.next();
        let oid = fields.next();
        if kind == Some("blob") {
            if let Some(oid) = oid {
                index
                    .by_oid
                    .entry(oid.to_string())
                    .or_default()
                    .push(path.to_string());
            }
        }
    }
    index
}

pub(crate) fn relocation_candidates_from_index(
    evidence: &ProjectContextEvidence,
    index: &GitRelocationIndex,
) -> Vec<String> {
    let Some(identity) = evidence.git_identity.as_ref() else {
        return Vec::new();
    };
    if identity.head_blob_oid.is_empty() {
        return Vec::new();
    }
    index
        .by_oid
        .get(&identity.head_blob_oid)
        .into_iter()
        .flatten()
        .filter(|path| path.as_str() != evidence.path)
        .cloned()
        .take(MAX_RELOCATIONS)
        .collect()
}

fn git_text(workspace: &Path, args: &[&str]) -> Option<String> {
    let output = git_output(workspace, args)?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Some(value)
}

fn git_output(workspace: &Path, args: &[&str]) -> Option<std::process::Output> {
    crate::git_command_error::git_command()
        .args(args)
        .current_dir(workspace)
        .output()
        .ok()
}

fn valid_oid(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.chars().all(|ch| ch.is_ascii_hexdigit())
}

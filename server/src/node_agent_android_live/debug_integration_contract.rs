use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DebugMergeCandidateRequest {
    #[serde(default)]
    pub(crate) ready: bool,
    pub(crate) commit_sha: Option<String>,
    #[serde(default)]
    pub(crate) commits: Vec<String>,
    pub(crate) base_sha: Option<String>,
    pub(crate) source_task_id: Option<String>,
    pub(crate) source_session_id: Option<String>,
    pub(crate) preview_owner: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DebugContribution {
    pub(crate) commit_sha: String,
    pub(crate) source_task_id: Option<String>,
    pub(crate) source_session_id: Option<String>,
    pub(crate) accepted_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DebugArtifactStatus {
    pub(crate) apk_path: String,
    pub(crate) sha256: String,
    pub(crate) package_name: String,
    pub(crate) version_code: String,
    pub(crate) version_name: String,
    pub(crate) app_label: String,
    pub(crate) signer_sha256: String,
    pub(crate) generation: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DebugIntegrationStatus {
    pub(crate) schema: String,
    pub(crate) slot_id: String,
    pub(crate) node_fingerprint: String,
    pub(crate) project_id: String,
    pub(crate) device_identity: String,
    pub(crate) package_name: String,
    pub(crate) repository_identity: String,
    pub(crate) base_sha: String,
    pub(crate) desired_generation: u64,
    pub(crate) installed_generation: Option<u64>,
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) lkg_enabled: bool,
    pub(crate) integration_worktree: Option<String>,
    pub(crate) contributions: Vec<DebugContribution>,
    pub(crate) conflicts: Vec<String>,
    #[serde(default)]
    pub(crate) legacy_packages: Vec<String>,
    pub(crate) preview_owner: Option<String>,
    pub(crate) last_error: Option<String>,
    pub(crate) last_usable: Option<DebugArtifactStatus>,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone)]
pub(crate) struct DebugIntegrationPlan {
    pub(crate) slot_id: String,
    pub(crate) generation: u64,
    pub(crate) source_root: PathBuf,
    pub(crate) worktree: PathBuf,
    pub(crate) package_name: String,
    pub(crate) lkg_enabled: bool,
    pub(crate) contributions: Vec<String>,
    pub(crate) base_sha: String,
}

pub(super) struct RepositoryIdentity {
    pub(super) root: PathBuf,
    pub(super) identity: String,
    pub(super) head: String,
}

pub(super) struct NormalizedCandidate {
    pub(super) commits: Vec<String>,
    pub(super) base_sha: String,
    pub(super) source_task_id: Option<String>,
    pub(super) source_session_id: Option<String>,
    pub(super) preview_owner: Option<String>,
}

pub(super) fn inspect_repository(project_root: &str) -> Result<RepositoryIdentity> {
    let root = PathBuf::from(project_root.trim())
        .canonicalize()
        .with_context(|| format!("候选项目目录不存在: {project_root}"))?;
    if !git(&root, &["status", "--porcelain"])?.trim().is_empty() {
        bail!("DEBUG_CANDIDATE_DIRTY: 合并调试只接受已提交且干净的候选，禁止复制脏文件");
    }
    let top =
        PathBuf::from(git(&root, &["rev-parse", "--show-toplevel"])?.trim()).canonicalize()?;
    let common = git(
        &root,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    let identity = PathBuf::from(common.trim())
        .canonicalize()?
        .display()
        .to_string();
    let head = git(&root, &["rev-parse", "HEAD"])?.trim().to_string();
    Ok(RepositoryIdentity {
        root: top,
        identity,
        head,
    })
}

pub(super) fn repository_identity(project_root: &str) -> Result<String> {
    let root = PathBuf::from(project_root.trim())
        .canonicalize()
        .with_context(|| format!("候选项目目录不存在: {project_root}"))?;
    let common = git(
        &root,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    Ok(PathBuf::from(common.trim())
        .canonicalize()?
        .display()
        .to_string())
}

pub(super) fn normalized_candidate(
    candidate: Option<&DebugMergeCandidateRequest>,
    compatibility_source: &str,
    repo: &RepositoryIdentity,
) -> Result<NormalizedCandidate> {
    if candidate.is_some_and(|candidate| !candidate.ready) {
        bail!("DEBUG_CANDIDATE_NOT_READY: 候选尚未明确 ready，拒绝进入节点集成槽");
    }
    let mut commits = candidate
        .map(|value| value.commits.clone())
        .unwrap_or_default();
    if let Some(commit) = candidate.and_then(|value| value.commit_sha.clone()) {
        if commits.is_empty() {
            commits.push(commit);
        } else if commits.last() != Some(&commit) {
            bail!("DEBUG_CANDIDATE_COMMIT_IDENTITY_MISMATCH: commitSha 必须等于 commits 最后一项");
        }
    }
    if commits.is_empty() {
        commits.push(repo.head.clone());
    }
    for commit in &mut commits {
        *commit = git(&repo.root, &["rev-parse", &format!("{commit}^{{commit}}")])?
            .trim()
            .to_string();
    }
    if commits.last() != Some(&repo.head) {
        bail!("DEBUG_CANDIDATE_HEAD_MISMATCH: 候选提交必须是来源 worktree 当前 HEAD，拒绝无来源提交身份");
    }
    let base_sha = match candidate.and_then(|value| value.base_sha.as_deref()) {
        Some(base) => git(&repo.root, &["rev-parse", &format!("{base}^{{commit}}")])?
            .trim()
            .to_string(),
        None => git(&repo.root, &["rev-parse", "HEAD^"])
            .map(|value| value.trim().to_string())
            .unwrap_or_else(|_| repo.head.clone()),
    };
    for commit in &commits {
        if !is_ancestor(&repo.root, &base_sha, commit)?
            || !is_ancestor(&repo.root, commit, &repo.head)?
        {
            bail!("DEBUG_CANDIDATE_COMMIT_LINEAGE_MISMATCH: 候选提交必须位于声明基础 SHA 与来源 worktree HEAD 之间");
        }
    }
    let source_task_id = candidate.and_then(|value| clean(value.source_task_id.as_deref()));
    let source_session_id = candidate
        .and_then(|value| clean(value.source_session_id.as_deref()))
        .or_else(|| clean(Some(compatibility_source)));
    if source_task_id.is_none() && source_session_id.is_none() {
        bail!("DEBUG_CANDIDATE_SOURCE_MISSING: 候选缺少来源 task 或 session 身份");
    }
    Ok(NormalizedCandidate {
        commits,
        base_sha,
        source_task_id,
        source_session_id,
        preview_owner: candidate.and_then(|value| clean(value.preview_owner.as_deref())),
    })
}

pub(super) fn validate_slot_identity(
    status: &DebugIntegrationStatus,
    repository: &str,
    project: &str,
    device: &str,
    package: &str,
) -> Result<()> {
    if status.repository_identity != repository
        || status.project_id != project.trim()
        || status.device_identity != device.trim()
        || status.package_name != package.trim()
    {
        bail!("DEBUG_SLOT_IDENTITY_DRIFT: 固定调试槽的仓库、项目、设备或包身份发生漂移，已 fail-closed");
    }
    Ok(())
}

pub(super) fn plan_from_status(
    root: &Path,
    source_root: &Path,
    status: &DebugIntegrationStatus,
) -> DebugIntegrationPlan {
    DebugIntegrationPlan {
        slot_id: status.slot_id.clone(),
        generation: status.desired_generation,
        source_root: source_root.to_path_buf(),
        worktree: root
            .join(&status.slot_id)
            .join("generations")
            .join(format!("generation-{}", status.desired_generation)),
        package_name: status.package_name.clone(),
        lkg_enabled: status.lkg_enabled,
        contributions: status
            .contributions
            .iter()
            .map(|item| item.commit_sha.clone())
            .collect(),
        base_sha: status.base_sha.clone(),
    }
}

pub(super) fn slot_id(repo: &str, project: &str, device: &str, node: &str) -> String {
    let digest = Sha256::digest(format!("{repo}\n{project}\n{device}\n{node}").as_bytes());
    format!("slot-{}", hex::encode(&digest[..12]))
}

pub(super) fn is_ancestor(root: &Path, ancestor: &str, descendant: &str) -> Result<bool> {
    Ok(crate::git_command_error::git_command()
        .current_dir(root)
        .args(["merge-base", "--is-ancestor", ancestor, descendant])
        .status()?
        .success())
}

pub(super) fn git(root: &Path, args: &[&str]) -> Result<String> {
    let output = crate::git_command_error::git_command()
        .current_dir(root)
        .args(args)
        .output()?;
    if !output.status.success() {
        bail!(
            "git {} 失败: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub(super) fn path_arg(path: &Path) -> Result<&str> {
    path.to_str().context("集成 worktree 路径不是有效 UTF-8")
}

fn clean(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(super) fn now() -> String {
    Utc::now().to_rfc3339()
}

pub(crate) fn debug_candidate_schema() -> Value {
    json!({
        "type":"object",
        "properties":{
            "ready":{"type":"boolean","default":true},
            "commitSha":{"type":"string"},
            "commits":{"type":"array","items":{"type":"string"}},
            "baseSha":{"type":"string"},
            "sourceTaskId":{"type":"string"},
            "sourceSessionId":{"type":"string"},
            "previewOwner":{"type":"string"}
        }
    })
}

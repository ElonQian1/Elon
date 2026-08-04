//! Explicit, review-gated repair candidates for drifted shared navigation memory.

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use std::{fs, path::Path};

use crate::{
    project_document_file_operation_model::normalize_document_path,
    project_document_governance::{parse_manifest, SECTION_CONFIG_PATH},
    project_document_native_context::{
        record_candidate, validate_evidence_current, ProjectContextMemoryReview,
    },
    project_document_native_context_git::{relocation_candidates_from_index, relocation_index},
};

pub(crate) fn create_relocation_repair_candidate(
    workspace: &Path,
    candidate_id: &str,
    source_path: &str,
    replacement_path: &str,
    producer: &str,
) -> Result<Value> {
    let candidate_id = candidate_id.trim();
    if candidate_id.is_empty() {
        bail!("修复共享记忆必须提供 candidate_id");
    }
    let source_path = normalize_document_path(source_path)?;
    let replacement_path = normalize_document_path(replacement_path)?;
    if source_path.eq_ignore_ascii_case(&replacement_path) {
        bail!("修复候选的新旧证据路径不能相同");
    }
    let manifest_path = workspace.join(SECTION_CONFIG_PATH);
    let content = fs::read_to_string(&manifest_path)
        .with_context(|| format!("读取共享项目记忆失败：{}", manifest_path.display()))?;
    let manifest = parse_manifest(Some(&content))?;
    let mut memory = manifest
        .context_memories
        .into_iter()
        .find(|memory| memory.candidate_id == candidate_id)
        .ok_or_else(|| anyhow!("共享项目记忆不存在：{candidate_id}"))?;
    let evidence = memory
        .evidence
        .iter_mut()
        .find(|evidence| evidence.path.eq_ignore_ascii_case(&source_path))
        .ok_or_else(|| anyhow!("共享项目记忆不包含待修复证据路径：{source_path}"))?;
    if validate_evidence_current(workspace, evidence).is_ok() {
        bail!("证据路径仍然有效，不需要创建重定位修复候选：{source_path}");
    }
    let index = relocation_index(workspace);
    let candidates = relocation_candidates_from_index(evidence, &index);
    if !candidates
        .iter()
        .any(|path| path.eq_ignore_ascii_case(&replacement_path))
    {
        bail!("replacement_path 不是当前 Git 对象验证过的重定位候选");
    }

    evidence.path = replacement_path.clone();
    evidence.content_hash.clear();
    evidence.git_identity = None;
    memory.reviewed_at.clear();
    memory.review = ProjectContextMemoryReview::default();
    let candidate = record_candidate(workspace, memory, producer)?;
    if candidate.status != "pending" {
        bail!("修复候选没有进入 pending 审核状态");
    }
    Ok(json!({
        "status": "pending_review",
        "repair_kind": "git_verified_path_relocation",
        "source_candidate_id": candidate_id,
        "source_path": source_path,
        "replacement_path": replacement_path,
        "candidate": candidate,
        "automatic": false,
        "repository_changed": false,
        "source_bodies_stored": 0,
        "next": "Review the replacement candidate, then use the existing suggestions/apply flow."
    }))
}

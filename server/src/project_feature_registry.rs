//! Git-backed feature requirement registry and lifecycle invariants.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

use crate::{
    project_document_file_operation_model::normalize_document_path,
    project_document_native_context::ProjectContextEvidence,
};

pub(crate) const FEATURE_REGISTRY_PATH: &str = ".elon/project-features.json";
pub(crate) const MAX_FEATURES: usize = 512;
const MAX_AUDIT_ENTRIES: usize = 200;
const MAX_ACCEPTANCE_CRITERIA: usize = 32;
const MAX_DEPENDENCIES: usize = 32;
const MAX_EVIDENCE: usize = 32;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProjectFeatureStatus {
    Draft,
    #[default]
    Proposed,
    Accepted,
    Ready,
    Claimed,
    InProgress,
    Blocked,
    Implemented,
    Verified,
    Released,
    Retired,
}

impl ProjectFeatureStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Proposed => "proposed",
            Self::Accepted => "accepted",
            Self::Ready => "ready",
            Self::Claimed => "claimed",
            Self::InProgress => "in_progress",
            Self::Blocked => "blocked",
            Self::Implemented => "implemented",
            Self::Verified => "verified",
            Self::Released => "released",
            Self::Retired => "retired",
        }
    }

    pub(crate) fn is_claim_bound(self) -> bool {
        matches!(self, Self::Claimed | Self::InProgress)
    }

    pub(crate) fn is_implementation_complete(self) -> bool {
        matches!(self, Self::Verified | Self::Released)
    }

    pub(crate) fn is_context_visible(self) -> bool {
        matches!(
            self,
            Self::Accepted
                | Self::Ready
                | Self::Claimed
                | Self::InProgress
                | Self::Blocked
                | Self::Implemented
        )
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProjectFeaturePriority {
    P0,
    P1,
    #[default]
    P2,
    P3,
}

impl ProjectFeaturePriority {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::P0 => "p0",
            Self::P1 => "p1",
            Self::P2 => "p2",
            Self::P3 => "p3",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ProjectFeatureClaim {
    pub claim_id: String,
    pub agent_id: String,
    pub claimed_at_ms: u64,
    pub expires_at_ms: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ProjectFeature {
    pub id: String,
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub status: ProjectFeatureStatus,
    #[serde(default)]
    pub priority: ProjectFeaturePriority,
    pub requirement: ProjectContextEvidence,
    #[serde(default)]
    pub knowledge_node_id: String,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub task_paths: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    #[serde(default)]
    pub implementation_evidence: Vec<ProjectContextEvidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim: Option<ProjectFeatureClaim>,
    #[serde(default)]
    pub created_at_ms: u64,
    #[serde(default)]
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ProjectFeatureAuditEntry {
    pub id: String,
    pub feature_id: String,
    pub action: String,
    #[serde(default)]
    pub actor: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub from_status: String,
    #[serde(default)]
    pub to_status: String,
    pub at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ProjectFeatureRegistry {
    #[serde(default = "schema_version")]
    pub version: u8,
    #[serde(default)]
    pub features: Vec<ProjectFeature>,
    #[serde(default)]
    pub audit_log: Vec<ProjectFeatureAuditEntry>,
}

impl Default for ProjectFeatureRegistry {
    fn default() -> Self {
        Self {
            version: schema_version(),
            features: Vec::new(),
            audit_log: Vec::new(),
        }
    }
}

pub(crate) fn parse_registry(content: Option<&str>) -> Result<ProjectFeatureRegistry> {
    let Some(content) = content.filter(|value| !value.trim().is_empty()) else {
        return Ok(ProjectFeatureRegistry::default());
    };
    normalize_registry(serde_json::from_str(content)?)
}

pub(crate) fn normalize_registry(
    mut registry: ProjectFeatureRegistry,
) -> Result<ProjectFeatureRegistry> {
    if registry.version != schema_version() {
        bail!("project-features.json 仅支持 version=1");
    }
    if registry.features.len() > MAX_FEATURES {
        bail!("项目功能登记最多 {MAX_FEATURES} 条");
    }
    registry.features = registry
        .features
        .into_iter()
        .map(normalize_feature)
        .collect::<Result<Vec<_>>>()?;
    registry
        .features
        .sort_by(|left, right| left.id.cmp(&right.id));
    for pair in registry.features.windows(2) {
        if pair[0].id.eq_ignore_ascii_case(&pair[1].id) {
            bail!("项目功能 id 重复：{}", pair[1].id);
        }
    }
    validate_dependency_graph(&registry.features)?;
    registry.audit_log = registry
        .audit_log
        .into_iter()
        .rev()
        .take(MAX_AUDIT_ENTRIES)
        .map(normalize_audit_entry)
        .filter(|entry| !entry.id.is_empty() && !entry.feature_id.is_empty())
        .collect();
    registry.audit_log.reverse();
    Ok(registry)
}

pub(crate) fn normalize_feature(mut feature: ProjectFeature) -> Result<ProjectFeature> {
    feature.id = normalize_identifier(&feature.id, 96, "feature id")?;
    feature.title = required_text(&feature.title, 160, "feature title")?;
    feature.summary = required_text(&feature.summary, 800, "feature summary")?;
    feature.requirement = normalize_evidence(feature.requirement, true)?;
    feature.knowledge_node_id = optional_identifier(&feature.knowledge_node_id, 96)?;
    feature.owner = bounded_text(&feature.owner, 80);
    feature.tags = unique_text(feature.tags, 12, 48);
    feature.task_paths = normalize_paths(feature.task_paths, 24)?;
    feature.dependencies = unique_identifiers(feature.dependencies, MAX_DEPENDENCIES)?;
    if feature
        .dependencies
        .iter()
        .any(|dependency| dependency.eq_ignore_ascii_case(&feature.id))
    {
        bail!("项目功能不能依赖自身：{}", feature.id);
    }
    feature.acceptance_criteria =
        unique_text(feature.acceptance_criteria, MAX_ACCEPTANCE_CRITERIA, 500);
    if matches!(
        feature.status,
        ProjectFeatureStatus::Accepted
            | ProjectFeatureStatus::Ready
            | ProjectFeatureStatus::Claimed
            | ProjectFeatureStatus::InProgress
            | ProjectFeatureStatus::Blocked
            | ProjectFeatureStatus::Implemented
            | ProjectFeatureStatus::Verified
            | ProjectFeatureStatus::Released
    ) && feature.acceptance_criteria.is_empty()
    {
        bail!(
            "accepted 及后续状态的项目功能必须有验收标准：{}",
            feature.id
        );
    }
    if feature.implementation_evidence.len() > MAX_EVIDENCE {
        bail!("项目功能 implementation_evidence 最多 {MAX_EVIDENCE} 条");
    }
    feature.implementation_evidence = feature
        .implementation_evidence
        .into_iter()
        .map(|evidence| normalize_evidence(evidence, false))
        .collect::<Result<Vec<_>>>()?;
    feature
        .implementation_evidence
        .sort_by(|left, right| (&left.path, &left.locator).cmp(&(&right.path, &right.locator)));
    feature
        .implementation_evidence
        .dedup_by(|left, right| left.path == right.path && left.locator == right.locator);
    if matches!(
        feature.status,
        ProjectFeatureStatus::Implemented
            | ProjectFeatureStatus::Verified
            | ProjectFeatureStatus::Released
    ) && feature.implementation_evidence.is_empty()
    {
        bail!("implemented 及后续状态必须有实现证据：{}", feature.id);
    }
    if feature.status.is_claim_bound() {
        feature.claim = Some(normalize_claim(feature.claim.take().ok_or_else(|| {
            anyhow::anyhow!("claimed/in_progress 状态必须有 claim：{}", feature.id)
        })?)?);
    } else {
        feature.claim = None;
    }
    if feature.created_at_ms == 0 {
        bail!("项目功能 created_at_ms 不能为空：{}", feature.id);
    }
    feature.updated_at_ms = feature.updated_at_ms.max(feature.created_at_ms);
    Ok(feature)
}

pub(crate) fn transition_allowed(from: ProjectFeatureStatus, to: ProjectFeatureStatus) -> bool {
    use ProjectFeatureStatus::*;
    matches!(
        (from, to),
        (Draft, Proposed | Retired)
            | (Proposed, Draft | Accepted | Retired)
            | (Accepted, Proposed | Ready | Retired)
            | (Ready, Blocked | Retired)
            | (Claimed, InProgress | Ready | Blocked | Retired)
            | (InProgress, Ready | Blocked | Implemented | Retired)
            | (Blocked, Ready | Retired)
            | (Implemented, Ready | Blocked | Verified | Retired)
            | (Verified, Ready | Released | Retired)
            | (Released, Retired)
            | (Retired, Proposed)
    )
}

fn normalize_claim(mut claim: ProjectFeatureClaim) -> Result<ProjectFeatureClaim> {
    claim.claim_id = normalize_identifier(&claim.claim_id, 96, "claim id")?;
    claim.agent_id = required_text(&claim.agent_id, 120, "claim agent_id")?;
    if claim.claimed_at_ms == 0 || claim.expires_at_ms <= claim.claimed_at_ms {
        bail!("项目功能 claim 时间范围无效");
    }
    Ok(claim)
}

fn normalize_evidence(
    mut evidence: ProjectContextEvidence,
    requirement: bool,
) -> Result<ProjectContextEvidence> {
    evidence.path = normalize_document_path(&evidence.path)?;
    if requirement
        && !matches!(
            std::path::Path::new(&evidence.path)
                .extension()
                .and_then(|value| value.to_str())
                .map(|value| value.to_ascii_lowercase())
                .as_deref(),
            Some("md" | "markdown" | "mdown")
        )
    {
        bail!("项目功能 requirement 必须引用 Markdown 文档");
    }
    evidence.content_hash = evidence.content_hash.trim().to_ascii_lowercase();
    if evidence.content_hash.len() != 64
        || !evidence
            .content_hash
            .chars()
            .all(|value| value.is_ascii_hexdigit())
    {
        bail!("项目功能 evidence.content_hash 必须是 SHA-256 hex");
    }
    evidence.locator = bounded_text(&evidence.locator, 160);
    evidence.evidence_kind = if requirement {
        "document".to_string()
    } else {
        match evidence.evidence_kind.trim() {
            "source" | "test" | "document" | "configuration" => {
                evidence.evidence_kind.trim().to_string()
            }
            _ => bail!("实现证据类型只支持 source、test、document 或 configuration"),
        }
    };
    evidence.git_identity =
        crate::project_document_native_context_git::normalize(evidence.git_identity)?;
    Ok(evidence)
}

fn validate_dependency_graph(features: &[ProjectFeature]) -> Result<()> {
    let by_id = features
        .iter()
        .map(|feature| (feature.id.as_str(), feature))
        .collect::<BTreeMap<_, _>>();
    for feature in features {
        for dependency in &feature.dependencies {
            if !by_id.contains_key(dependency.as_str()) {
                bail!("项目功能 {} 引用了未知依赖：{}", feature.id, dependency);
            }
        }
    }
    fn visit<'a>(
        id: &'a str,
        by_id: &BTreeMap<&'a str, &'a ProjectFeature>,
        visiting: &mut HashSet<&'a str>,
        visited: &mut HashSet<&'a str>,
    ) -> Result<()> {
        if visited.contains(id) {
            return Ok(());
        }
        if !visiting.insert(id) {
            bail!("项目功能依赖存在循环：{id}");
        }
        if let Some(feature) = by_id.get(id) {
            for dependency in &feature.dependencies {
                visit(dependency, by_id, visiting, visited)?;
            }
        }
        visiting.remove(id);
        visited.insert(id);
        Ok(())
    }
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    for id in by_id.keys().copied() {
        visit(id, &by_id, &mut visiting, &mut visited)?;
    }
    Ok(())
}

fn normalize_audit_entry(mut entry: ProjectFeatureAuditEntry) -> ProjectFeatureAuditEntry {
    entry.id = bounded_text(&entry.id, 96);
    entry.feature_id = bounded_text(&entry.feature_id, 96);
    entry.action = bounded_text(&entry.action, 48);
    entry.actor = bounded_text(&entry.actor, 120);
    entry.reason = bounded_text(&entry.reason, 500);
    entry.from_status = bounded_text(&entry.from_status, 32);
    entry.to_status = bounded_text(&entry.to_status, 32);
    entry
}

fn normalize_identifier(value: &str, limit: usize, label: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > limit
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        bail!("{label} 只能包含字母、数字、点、下划线和连字符，最多 {limit} 字符");
    }
    Ok(value.to_ascii_lowercase())
}

fn optional_identifier(value: &str, limit: usize) -> Result<String> {
    if value.trim().is_empty() {
        Ok(String::new())
    } else {
        normalize_identifier(value, limit, "knowledge_node_id")
    }
}

fn unique_identifiers(values: Vec<String>, limit: usize) -> Result<Vec<String>> {
    if values.len() > limit {
        bail!("项目功能依赖最多 {limit} 条");
    }
    let mut result = values
        .into_iter()
        .map(|value| normalize_identifier(&value, 96, "dependency id"))
        .collect::<Result<Vec<_>>>()?;
    result.sort();
    result.dedup();
    Ok(result)
}

fn normalize_paths(values: Vec<String>, limit: usize) -> Result<Vec<String>> {
    if values.len() > limit {
        bail!("项目功能 task_paths 最多 {limit} 条");
    }
    let mut result = values
        .into_iter()
        .map(|value| normalize_document_path(&value))
        .collect::<Result<Vec<_>>>()?;
    result.sort();
    result.dedup();
    Ok(result)
}

fn unique_text(values: Vec<String>, limit: usize, char_limit: usize) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .map(|value| bounded_text(&value, char_limit))
        .filter(|value| !value.is_empty() && seen.insert(value.to_ascii_lowercase()))
        .take(limit)
        .collect()
}

fn required_text(value: &str, limit: usize, label: &str) -> Result<String> {
    let value = bounded_text(value, limit);
    if value.is_empty() {
        bail!("{label} 不能为空");
    }
    Ok(value)
}

fn bounded_text(value: &str, limit: usize) -> String {
    value.trim().chars().take(limit).collect()
}

fn schema_version() -> u8 {
    1
}

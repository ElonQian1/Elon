//! Feature registration, claims, transitions, evidence, and drift operations.

use anyhow::{anyhow, bail, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    project_feature_projection::{
        dependency_blockers, dependency_snapshot, feature_search_text, feature_snapshot,
    },
    project_feature_registry::{
        transition_allowed, ProjectFeature, ProjectFeatureAuditEntry, ProjectFeatureClaim,
        ProjectFeaturePriority, ProjectFeatureRegistry, ProjectFeatureStatus,
    },
    project_feature_registry_store::{
        bind_evidence, bind_requirement, ensure_implementation_evidence_current,
        ensure_requirement_current, ensure_requirement_is_current, load_registry, save_registry,
        validate_knowledge_node, verify_registry_revision, FeatureEvidenceInput,
    },
};

#[derive(Debug, Deserialize)]
pub(crate) struct RegisterFeatureRequest {
    pub id: String,
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub status: ProjectFeatureStatus,
    #[serde(default)]
    pub priority: ProjectFeaturePriority,
    pub requirement_path: String,
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
    pub actor: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub expected_registry_revision: Option<String>,
}

pub(crate) fn register_feature(workspace: &Path, input: RegisterFeatureRequest) -> Result<Value> {
    require_actor(&input.actor)?;
    if !matches!(
        input.status,
        ProjectFeatureStatus::Draft
            | ProjectFeatureStatus::Proposed
            | ProjectFeatureStatus::Accepted
            | ProjectFeatureStatus::Ready
    ) {
        bail!("新登记功能状态只允许 draft、proposed、accepted 或 ready");
    }
    let feature_id = input.id.trim().to_ascii_lowercase();
    let mut loaded = load_registry(workspace)?;
    verify_registry_revision(
        loaded.revision.as_deref(),
        input.expected_registry_revision.as_deref(),
    )?;
    if loaded
        .registry
        .features
        .iter()
        .any(|feature| feature.id.eq_ignore_ascii_case(input.id.trim()))
    {
        bail!("项目功能 id 已存在：{}", input.id.trim());
    }
    let requirement = bind_requirement(workspace, &input.requirement_path)?;
    if matches!(
        input.status,
        ProjectFeatureStatus::Accepted | ProjectFeatureStatus::Ready
    ) {
        ensure_requirement_is_current(workspace, &requirement.path)?;
    }
    validate_knowledge_node(workspace, &input.knowledge_node_id)?;
    let now = now_millis();
    let feature = ProjectFeature {
        id: input.id,
        title: input.title,
        summary: input.summary,
        status: input.status,
        priority: input.priority,
        requirement,
        knowledge_node_id: input.knowledge_node_id,
        owner: input.owner,
        tags: input.tags,
        task_paths: input.task_paths,
        dependencies: input.dependencies,
        acceptance_criteria: input.acceptance_criteria,
        implementation_evidence: Vec::new(),
        claim: None,
        created_at_ms: now,
        updated_at_ms: now,
    };
    loaded.registry.features.push(feature);
    if input.status == ProjectFeatureStatus::Ready {
        let feature = loaded
            .registry
            .features
            .last()
            .ok_or_else(|| anyhow!("功能登记写入失败"))?;
        let blockers = dependency_blockers(&loaded.registry, feature);
        if !blockers.is_empty() {
            bail!("ready 功能仍有未完成依赖：{}", blockers.join(", "));
        }
    }
    append_audit(
        &mut loaded.registry,
        &feature_id,
        "registered",
        &input.actor,
        &input.reason,
        "",
        input.status.as_str(),
    );
    let saved = save_registry(workspace, loaded.registry, loaded.revision.as_deref())?;
    Ok(json!({
        "status":"registered",
        "feature":find_feature(&saved.registry, &feature_id)?,
        "registry_revision":saved.revision,
        "repository_changed":true,
        "source_bodies_stored":0,
        "next":"Commit the requirement document and registry together; agents can query project_features_plan or receive a bounded context projection."
    }))
}

pub(crate) fn list_features(
    workspace: &Path,
    statuses: &[ProjectFeatureStatus],
    query: &str,
    offset: usize,
    limit: usize,
) -> Result<Value> {
    let loaded = load_registry(workspace)?;
    let query = query.trim().to_ascii_lowercase();
    let mut items = loaded
        .registry
        .features
        .iter()
        .filter(|feature| statuses.is_empty() || statuses.contains(&feature.status))
        .filter(|feature| query.is_empty() || feature_search_text(feature).contains(&query))
        .collect::<Vec<_>>();
    items.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| right.updated_at_ms.cmp(&left.updated_at_ms))
    });
    let total = items.len();
    let limit = limit.clamp(1, 100);
    let page = items
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|feature| feature_snapshot(workspace, &loaded.registry, feature, false))
        .collect::<Vec<_>>();
    Ok(json!({
        "schema":"elon.project_feature_list.v1",
        "registry_revision":loaded.revision,
        "total":total,
        "offset":offset,
        "returned":page.len(),
        "features":page,
        "source_bodies_returned":0,
    }))
}

pub(crate) fn plan_feature(workspace: &Path, feature_id: &str) -> Result<Value> {
    let loaded = load_registry(workspace)?;
    let feature = find_feature(&loaded.registry, feature_id)?;
    let snapshot = feature_snapshot(workspace, &loaded.registry, feature, true);
    Ok(json!({
        "schema":"elon.project_feature_plan.v1",
        "registry_revision":loaded.revision,
        "feature":snapshot,
        "acceptance_criteria":feature.acceptance_criteria,
        "dependencies":feature.dependencies.iter().map(|id| dependency_snapshot(&loaded.registry, id)).collect::<Vec<_>>(),
        "implementation_evidence":feature.implementation_evidence,
        "source_policy":{
            "workflow_metadata_only":true,
            "requirement_body_returned":false,
            "implementation_truth":"current source, tests, and runtime evidence",
            "accepted_direction":"open the hash-bound requirement document with native tools before implementation"
        },
        "native_tool_handoff":{
            "open_first":feature.requirement.path,
            "then":"Verify task_paths and current implementation with native search/read.",
            "complete":"Record hash-bound implementation evidence, transition to implemented, then verify before released."
        }
    }))
}

pub(crate) fn claim_feature(
    workspace: &Path,
    feature_id: &str,
    agent_id: &str,
    lease_minutes: u64,
    expected_revision: Option<&str>,
) -> Result<Value> {
    if agent_id.trim().is_empty() || agent_id.chars().count() > 120 {
        bail!("agent_id 不能为空且最多 120 字符");
    }
    if !(5..=1_440).contains(&lease_minutes) {
        bail!("lease_minutes 必须在 5 至 1440 之间");
    }
    let mut loaded = load_registry(workspace)?;
    verify_registry_revision(loaded.revision.as_deref(), expected_revision)?;
    let now = now_millis();
    let index = feature_index(&loaded.registry, feature_id)?;
    let reclaiming = loaded.registry.features[index]
        .claim
        .as_ref()
        .is_some_and(|claim| claim.expires_at_ms <= now);
    if loaded.registry.features[index].status != ProjectFeatureStatus::Ready && !reclaiming {
        bail!("只有 ready 或认领已过期的功能可以认领");
    }
    ensure_requirement_current(workspace, &loaded.registry.features[index])?;
    let blockers = dependency_blockers(&loaded.registry, &loaded.registry.features[index]);
    if !blockers.is_empty() {
        bail!("功能依赖尚未完成：{}", blockers.join(", "));
    }
    let claim = ProjectFeatureClaim {
        claim_id: format!("claim_{}", uuid::Uuid::new_v4().simple()),
        agent_id: agent_id.trim().to_string(),
        claimed_at_ms: now,
        expires_at_ms: now.saturating_add(lease_minutes.saturating_mul(60_000)),
    };
    let from = loaded.registry.features[index].status;
    loaded.registry.features[index].status = ProjectFeatureStatus::Claimed;
    loaded.registry.features[index].claim = Some(claim.clone());
    loaded.registry.features[index]
        .implementation_evidence
        .clear();
    loaded.registry.features[index].updated_at_ms = now;
    append_audit(
        &mut loaded.registry,
        feature_id,
        if reclaiming { "reclaimed" } else { "claimed" },
        agent_id,
        "",
        from.as_str(),
        ProjectFeatureStatus::Claimed.as_str(),
    );
    let saved = save_registry(workspace, loaded.registry, loaded.revision.as_deref())?;
    Ok(json!({
        "status":"claimed",
        "claim":claim,
        "feature":find_feature(&saved.registry, feature_id)?,
        "registry_revision":saved.revision,
        "repository_changed":true,
    }))
}

pub(crate) fn release_claim(
    workspace: &Path,
    feature_id: &str,
    claim_id: &str,
    reason: &str,
    expected_revision: Option<&str>,
) -> Result<Value> {
    let mut loaded = load_registry(workspace)?;
    verify_registry_revision(loaded.revision.as_deref(), expected_revision)?;
    let index = feature_index(&loaded.registry, feature_id)?;
    verify_claim_identity(&loaded.registry.features[index], claim_id)?;
    let actor = loaded.registry.features[index]
        .claim
        .as_ref()
        .map(|claim| claim.agent_id.clone())
        .unwrap_or_default();
    let from = loaded.registry.features[index].status;
    let requirement_current =
        ensure_requirement_current(workspace, &loaded.registry.features[index]).is_ok();
    let blockers = dependency_blockers(&loaded.registry, &loaded.registry.features[index]);
    let target = if requirement_current && blockers.is_empty() {
        ProjectFeatureStatus::Ready
    } else {
        ProjectFeatureStatus::Blocked
    };
    loaded.registry.features[index].status = target;
    loaded.registry.features[index].claim = None;
    loaded.registry.features[index].updated_at_ms = now_millis();
    append_audit(
        &mut loaded.registry,
        feature_id,
        "claim_released",
        &actor,
        reason,
        from.as_str(),
        target.as_str(),
    );
    let saved = save_registry(workspace, loaded.registry, loaded.revision.as_deref())?;
    Ok(
        json!({"status":target.as_str(),"feature":find_feature(&saved.registry, feature_id)?,"registry_revision":saved.revision,"repository_changed":true}),
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn transition_feature(
    workspace: &Path,
    feature_id: &str,
    to: ProjectFeatureStatus,
    actor: &str,
    reason: &str,
    claim_id: &str,
    expected_revision: Option<&str>,
) -> Result<Value> {
    require_actor(actor)?;
    let mut loaded = load_registry(workspace)?;
    verify_registry_revision(loaded.revision.as_deref(), expected_revision)?;
    let index = feature_index(&loaded.registry, feature_id)?;
    let from = loaded.registry.features[index].status;
    if !transition_allowed(from, to) {
        bail!("不允许的功能状态转换：{} -> {}", from.as_str(), to.as_str());
    }
    if from.is_claim_bound() || to == ProjectFeatureStatus::InProgress {
        verify_claim(&loaded.registry.features[index], claim_id)?;
    }
    if matches!(
        to,
        ProjectFeatureStatus::Accepted | ProjectFeatureStatus::Ready
    ) {
        ensure_requirement_current(workspace, &loaded.registry.features[index])?;
    }
    if to == ProjectFeatureStatus::Ready {
        let blockers = dependency_blockers(&loaded.registry, &loaded.registry.features[index]);
        if !blockers.is_empty() {
            bail!("ready 功能仍有未完成依赖：{}", blockers.join(", "));
        }
    }
    if matches!(
        to,
        ProjectFeatureStatus::Implemented
            | ProjectFeatureStatus::Verified
            | ProjectFeatureStatus::Released
    ) {
        ensure_implementation_evidence_current(workspace, &loaded.registry.features[index], to)?;
    }
    loaded.registry.features[index].status = to;
    if !to.is_claim_bound() {
        loaded.registry.features[index].claim = None;
    }
    loaded.registry.features[index].updated_at_ms = now_millis();
    append_audit(
        &mut loaded.registry,
        feature_id,
        "transitioned",
        actor,
        reason,
        from.as_str(),
        to.as_str(),
    );
    let saved = save_registry(workspace, loaded.registry, loaded.revision.as_deref())?;
    Ok(
        json!({"status":to.as_str(),"feature":find_feature(&saved.registry, feature_id)?,"registry_revision":saved.revision,"repository_changed":true}),
    )
}

pub(crate) fn record_evidence(
    workspace: &Path,
    feature_id: &str,
    claim_id: &str,
    actor: &str,
    evidence: Vec<FeatureEvidenceInput>,
    expected_revision: Option<&str>,
) -> Result<Value> {
    require_actor(actor)?;
    if evidence.is_empty() || evidence.len() > 16 {
        bail!("一次需要提交 1 至 16 条实现证据");
    }
    let mut loaded = load_registry(workspace)?;
    verify_registry_revision(loaded.revision.as_deref(), expected_revision)?;
    let index = feature_index(&loaded.registry, feature_id)?;
    if loaded.registry.features[index].status.is_claim_bound() {
        verify_claim(&loaded.registry.features[index], claim_id)?;
    } else if !matches!(
        loaded.registry.features[index].status,
        ProjectFeatureStatus::Blocked | ProjectFeatureStatus::Implemented
    ) {
        bail!("只有 claimed、in_progress、blocked 或 implemented 功能可以记录实现证据");
    }
    let bound = evidence
        .into_iter()
        .map(|item| bind_evidence(workspace, item))
        .collect::<Result<Vec<_>>>()?;
    for evidence in bound {
        loaded.registry.features[index]
            .implementation_evidence
            .retain(|current| current.path != evidence.path || current.locator != evidence.locator);
        loaded.registry.features[index]
            .implementation_evidence
            .push(evidence);
    }
    loaded.registry.features[index].updated_at_ms = now_millis();
    let status = loaded.registry.features[index].status;
    append_audit(
        &mut loaded.registry,
        feature_id,
        "evidence_recorded",
        actor,
        "",
        status.as_str(),
        status.as_str(),
    );
    let saved = save_registry(workspace, loaded.registry, loaded.revision.as_deref())?;
    Ok(
        json!({"status":"evidence_recorded","feature":find_feature(&saved.registry, feature_id)?,"registry_revision":saved.revision,"repository_changed":true}),
    )
}

pub(crate) fn check_drift(workspace: &Path, feature_id: Option<&str>) -> Result<Value> {
    let loaded = load_registry(workspace)?;
    let features = loaded
        .registry
        .features
        .iter()
        .filter(|feature| {
            feature_id
                .map(|id| feature.id.eq_ignore_ascii_case(id.trim()))
                .unwrap_or(true)
        })
        .map(|feature| feature_snapshot(workspace, &loaded.registry, feature, true))
        .collect::<Vec<_>>();
    if feature_id.is_some() && features.is_empty() {
        bail!("项目功能不存在：{}", feature_id.unwrap_or_default());
    }
    let drifted = features
        .iter()
        .filter(|item| item["drift_status"] != "current")
        .count();
    Ok(json!({
        "schema":"elon.project_feature_drift.v1",
        "registry_revision":loaded.revision,
        "checked":features.len(),
        "drifted":drifted,
        "features":features,
        "repair_plan":{"automatic":false,"steps":["Verify current requirement/source with native tools.","Update the requirement or record fresh evidence explicitly.","Use expected_registry_revision so concurrent changes fail closed."]}
    }))
}

pub(crate) fn feature_history(
    workspace: &Path,
    feature_id: Option<&str>,
    offset: usize,
    limit: usize,
) -> Result<Value> {
    let loaded = load_registry(workspace)?;
    if let Some(id) = feature_id.filter(|value| !value.trim().is_empty()) {
        let _ = find_feature(&loaded.registry, id)?;
    }
    let mut entries = loaded
        .registry
        .audit_log
        .iter()
        .filter(|entry| {
            feature_id
                .filter(|value| !value.trim().is_empty())
                .map(|id| entry.feature_id.eq_ignore_ascii_case(id.trim()))
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        right
            .at_ms
            .cmp(&left.at_ms)
            .then_with(|| right.id.cmp(&left.id))
    });
    let total = entries.len();
    let limit = limit.clamp(1, 100);
    let page = entries
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    Ok(json!({
        "schema":"elon.project_feature_history.v1",
        "registry_revision":loaded.revision,
        "feature_id":feature_id.filter(|value| !value.trim().is_empty()),
        "total":total,
        "offset":offset,
        "returned":page.len(),
        "entries":page,
        "retention_limit":200,
        "source_bodies_returned":0,
    }))
}

fn feature_index(registry: &ProjectFeatureRegistry, id: &str) -> Result<usize> {
    registry
        .features
        .iter()
        .position(|feature| feature.id.eq_ignore_ascii_case(id.trim()))
        .ok_or_else(|| anyhow!("项目功能不存在：{}", id.trim()))
}

fn find_feature<'a>(registry: &'a ProjectFeatureRegistry, id: &str) -> Result<&'a ProjectFeature> {
    Ok(&registry.features[feature_index(registry, id)?])
}

fn verify_claim(feature: &ProjectFeature, claim_id: &str) -> Result<()> {
    let claim = verify_claim_identity(feature, claim_id)?;
    if claim.expires_at_ms <= now_millis() {
        bail!("功能认领已过期，请重新认领");
    }
    Ok(())
}

fn verify_claim_identity<'a>(
    feature: &'a ProjectFeature,
    claim_id: &str,
) -> Result<&'a ProjectFeatureClaim> {
    let claim = feature
        .claim
        .as_ref()
        .ok_or_else(|| anyhow!("功能当前没有有效认领"))?;
    if claim.claim_id != claim_id.trim() {
        bail!("claim_id 不匹配");
    }
    Ok(claim)
}

fn require_actor(actor: &str) -> Result<()> {
    if actor.trim().is_empty() || actor.chars().count() > 120 {
        bail!("actor 不能为空且最多 120 字符");
    }
    Ok(())
}

pub(crate) fn append_audit(
    registry: &mut ProjectFeatureRegistry,
    feature_id: &str,
    action: &str,
    actor: &str,
    reason: &str,
    from: &str,
    to: &str,
) {
    registry.audit_log.push(ProjectFeatureAuditEntry {
        id: format!("feature_event_{}", uuid::Uuid::new_v4().simple()),
        feature_id: feature_id.trim().to_ascii_lowercase(),
        action: action.to_string(),
        actor: actor.trim().chars().take(120).collect(),
        reason: reason.trim().chars().take(500).collect(),
        from_status: from.to_string(),
        to_status: to.to_string(),
        at_ms: now_millis(),
    });
}

pub(crate) fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

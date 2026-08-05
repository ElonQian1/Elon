//! Explicit metadata updates and requirement rebinding for feature records.

use anyhow::{bail, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;

use crate::{
    project_feature_registry::{ProjectFeaturePriority, ProjectFeatureStatus},
    project_feature_registry_service::{append_audit, now_millis},
    project_feature_registry_store::{
        bind_requirement, ensure_requirement_current, ensure_requirement_is_current, load_registry,
        save_registry, validate_knowledge_node, verify_registry_revision,
    },
};

#[derive(Debug, Deserialize)]
pub(crate) struct UpdateFeatureRequest {
    pub feature_id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub priority: Option<ProjectFeaturePriority>,
    #[serde(default)]
    pub knowledge_node_id: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub task_paths: Option<Vec<String>>,
    #[serde(default)]
    pub dependencies: Option<Vec<String>>,
    #[serde(default)]
    pub acceptance_criteria: Option<Vec<String>>,
    pub actor: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub expected_registry_revision: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RebindRequirementRequest {
    pub feature_id: String,
    #[serde(default)]
    pub requirement_path: String,
    pub actor: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub expected_registry_revision: Option<String>,
}

pub(crate) fn update_feature(workspace: &Path, input: UpdateFeatureRequest) -> Result<Value> {
    require_actor(&input.actor)?;
    if !has_update(&input) {
        bail!("至少提供一个需要更新的功能字段");
    }
    let mut loaded = load_registry(workspace)?;
    verify_registry_revision(
        loaded.revision.as_deref(),
        input.expected_registry_revision.as_deref(),
    )?;
    let index = feature_index(&loaded, &input.feature_id)?;
    let status = loaded.registry.features[index].status;
    if !matches!(
        status,
        ProjectFeatureStatus::Draft
            | ProjectFeatureStatus::Proposed
            | ProjectFeatureStatus::Accepted
            | ProjectFeatureStatus::Ready
            | ProjectFeatureStatus::Blocked
    ) {
        bail!("当前状态不能修改功能元数据；先释放认领、重新打开功能或创建后续功能");
    }
    if matches!(
        status,
        ProjectFeatureStatus::Accepted
            | ProjectFeatureStatus::Ready
            | ProjectFeatureStatus::Blocked
    ) {
        ensure_requirement_current(workspace, &loaded.registry.features[index])?;
    }
    if let Some(node_id) = input.knowledge_node_id.as_deref() {
        validate_knowledge_node(workspace, node_id)?;
    }
    let scope_changed = input.task_paths.is_some()
        || input.dependencies.is_some()
        || input.acceptance_criteria.is_some();
    let feature = &mut loaded.registry.features[index];
    if let Some(value) = input.title {
        feature.title = value;
    }
    if let Some(value) = input.summary {
        feature.summary = value;
    }
    if let Some(value) = input.priority {
        feature.priority = value;
    }
    if let Some(value) = input.knowledge_node_id {
        feature.knowledge_node_id = value;
    }
    if let Some(value) = input.owner {
        feature.owner = value;
    }
    if let Some(value) = input.tags {
        feature.tags = value;
    }
    if let Some(value) = input.task_paths {
        feature.task_paths = value;
    }
    if let Some(value) = input.dependencies {
        feature.dependencies = value;
    }
    if let Some(value) = input.acceptance_criteria {
        feature.acceptance_criteria = value;
    }
    let mut to = status;
    if scope_changed
        && matches!(
            status,
            ProjectFeatureStatus::Accepted
                | ProjectFeatureStatus::Ready
                | ProjectFeatureStatus::Blocked
        )
    {
        to = ProjectFeatureStatus::Proposed;
        feature.status = to;
        feature.claim = None;
        feature.implementation_evidence.clear();
    }
    feature.updated_at_ms = now_millis();
    append_audit(
        &mut loaded.registry,
        &input.feature_id,
        "metadata_updated",
        &input.actor,
        &input.reason,
        status.as_str(),
        to.as_str(),
    );
    let saved = save_registry(workspace, loaded.registry, loaded.revision.as_deref())?;
    let feature = saved
        .registry
        .features
        .iter()
        .find(|feature| feature.id.eq_ignore_ascii_case(input.feature_id.trim()))
        .ok_or_else(|| anyhow::anyhow!("更新后的功能不存在"))?;
    Ok(json!({
        "status":"updated",
        "feature":feature,
        "registry_revision":saved.revision,
        "repository_changed":true,
        "review_reset":to != status,
    }))
}

pub(crate) fn rebind_requirement(
    workspace: &Path,
    input: RebindRequirementRequest,
) -> Result<Value> {
    require_actor(&input.actor)?;
    let mut loaded = load_registry(workspace)?;
    verify_registry_revision(
        loaded.revision.as_deref(),
        input.expected_registry_revision.as_deref(),
    )?;
    let index = feature_index(&loaded, &input.feature_id)?;
    let from = loaded.registry.features[index].status;
    if !matches!(
        from,
        ProjectFeatureStatus::Draft
            | ProjectFeatureStatus::Proposed
            | ProjectFeatureStatus::Accepted
            | ProjectFeatureStatus::Ready
            | ProjectFeatureStatus::Blocked
    ) {
        bail!("当前状态不能重绑需求；先释放认领或把已实现功能显式重新打开");
    }
    let path = if input.requirement_path.trim().is_empty() {
        loaded.registry.features[index].requirement.path.clone()
    } else {
        input.requirement_path.clone()
    };
    let requirement = bind_requirement(workspace, &path)?;
    if matches!(
        from,
        ProjectFeatureStatus::Accepted
            | ProjectFeatureStatus::Ready
            | ProjectFeatureStatus::Blocked
    ) {
        ensure_requirement_is_current(workspace, &requirement.path)?;
    }
    if loaded.registry.features[index].requirement == requirement {
        bail!("需求路径、内容哈希和 Git 身份均未变化，无需重绑");
    }
    let to = if matches!(
        from,
        ProjectFeatureStatus::Accepted
            | ProjectFeatureStatus::Ready
            | ProjectFeatureStatus::Blocked
    ) {
        ProjectFeatureStatus::Proposed
    } else {
        from
    };
    let feature = &mut loaded.registry.features[index];
    feature.requirement = requirement;
    feature.status = to;
    feature.claim = None;
    feature.implementation_evidence.clear();
    feature.updated_at_ms = now_millis();
    append_audit(
        &mut loaded.registry,
        &input.feature_id,
        "requirement_rebound",
        &input.actor,
        &input.reason,
        from.as_str(),
        to.as_str(),
    );
    let saved = save_registry(workspace, loaded.registry, loaded.revision.as_deref())?;
    let feature = saved
        .registry
        .features
        .iter()
        .find(|feature| feature.id.eq_ignore_ascii_case(input.feature_id.trim()))
        .ok_or_else(|| anyhow::anyhow!("重绑后的功能不存在"))?;
    Ok(json!({
        "status":"requirement_rebound",
        "feature":feature,
        "registry_revision":saved.revision,
        "repository_changed":true,
        "review_required":to == ProjectFeatureStatus::Proposed,
        "next":"Review the changed requirement, then transition proposed -> accepted -> ready explicitly."
    }))
}

fn feature_index(
    loaded: &crate::project_feature_registry_store::LoadedFeatureRegistry,
    id: &str,
) -> Result<usize> {
    loaded
        .registry
        .features
        .iter()
        .position(|feature| feature.id.eq_ignore_ascii_case(id.trim()))
        .ok_or_else(|| anyhow::anyhow!("项目功能不存在：{}", id.trim()))
}

fn has_update(input: &UpdateFeatureRequest) -> bool {
    input.title.is_some()
        || input.summary.is_some()
        || input.priority.is_some()
        || input.knowledge_node_id.is_some()
        || input.owner.is_some()
        || input.tags.is_some()
        || input.task_paths.is_some()
        || input.dependencies.is_some()
        || input.acceptance_criteria.is_some()
}

fn require_actor(actor: &str) -> Result<()> {
    if actor.trim().is_empty() || actor.chars().count() > 120 {
        bail!("actor 不能为空且最多 120 字符");
    }
    Ok(())
}

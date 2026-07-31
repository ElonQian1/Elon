use std::collections::HashSet;

use anyhow::{bail, Result};

use crate::{node_runtime::user_node_runtimes, types::AppState};

use super::model::{
    AiResourceOverview, AiResourcePolicy, AiResourceSummary, AiRoutePreview, AiRoutePreviewRequest,
    UpdateAiResourcePolicy, RESOURCE_CLASSES,
};

pub(crate) async fn overview(
    state: &AppState,
    project_id: &str,
    user_id: &str,
) -> Result<AiResourceOverview> {
    let policy = state
        .store
        .project_ai_resource_policy(project_id, user_id)?;
    Ok(AiResourceOverview {
        schema: "ai_resource_control.overview.v1",
        project_id: project_id.to_string(),
        policy,
        resources: inventory(state, user_id).await?,
        cautions: vec![
            "资源存在不代表外部额度充足；当前不会读取或推断第三方账户余额。",
            "共享 Codex 只展示已授权关系，不公开凭据，也不允许转售 API Token。",
            "路由预演不会启动任务；实际执行仍使用现有权限、计费和节点健康检查。",
        ],
    })
}

pub(crate) fn update_policy(
    state: &AppState,
    project_id: &str,
    user_id: &str,
    request: UpdateAiResourcePolicy,
) -> Result<AiResourcePolicy> {
    validate_policy(&request)?;
    state
        .store
        .upsert_project_ai_resource_policy(project_id, user_id, &request)
}

pub(crate) async fn preview(
    state: &AppState,
    project_id: &str,
    user_id: &str,
    request: AiRoutePreviewRequest,
) -> Result<AiRoutePreview> {
    validate_preview(&request)?;
    let policy = state
        .store
        .project_ai_resource_policy(project_id, user_id)?;
    let resources = inventory(state, user_id).await?;
    Ok(select_route(project_id, &policy, resources, request))
}

pub(crate) fn validate_policy(request: &UpdateAiResourcePolicy) -> Result<()> {
    if !matches!(
        request.privacy_mode.as_str(),
        "prefer_local" | "balanced" | "prefer_available"
    ) {
        bail!("privacy_mode 必须是 prefer_local、balanced 或 prefer_available");
    }
    if request.enabled_classes.is_empty() || request.priority.is_empty() {
        bail!("至少启用并排序一种 AI 资源");
    }
    let valid = RESOURCE_CLASSES.into_iter().collect::<HashSet<_>>();
    let enabled = request
        .enabled_classes
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let priority = request
        .priority
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    if enabled.len() != request.enabled_classes.len()
        || priority.len() != request.priority.len()
        || !enabled.iter().all(|value| valid.contains(value))
        || !priority.iter().all(|value| valid.contains(value))
        || enabled != priority
    {
        bail!("enabled_classes 与 priority 必须包含相同且不重复的有效资源类型");
    }
    if request
        .max_estimated_unit_cost_micros
        .is_some_and(|value| value < 0)
    {
        bail!("任务成本上限不能为负数");
    }
    Ok(())
}

fn validate_preview(request: &AiRoutePreviewRequest) -> Result<()> {
    if !matches!(
        request.task_kind.trim(),
        "chat" | "code" | "analysis" | "image"
    ) {
        bail!("task_kind 必须是 chat、code、analysis 或 image");
    }
    Ok(())
}

async fn inventory(state: &AppState, user_id: &str) -> Result<Vec<AiResourceSummary>> {
    let mut resources = own_codex_resources(state, user_id)?;
    resources.extend(shared_codex_resources(state, user_id)?);
    resources.extend(platform_resources(state).await);
    resources.extend(remote_node_resources(state, user_id).await?);
    Ok(resources)
}

fn own_codex_resources(state: &AppState, user_id: &str) -> Result<Vec<AiResourceSummary>> {
    let slots = state.store.list_user_codex_credential_slots(user_id)?;
    let legacy_available =
        slots.is_empty() && state.store.get_user_codex_credential(user_id)?.is_some();
    let mut resources = slots
        .into_iter()
        .filter(|slot| matches!(slot.status.as_str(), "active" | "degraded"))
        .map(|slot| AiResourceSummary {
            resource_id: format!("own_codex:{}", slot.slot_id),
            resource_class: "own_codex".to_string(),
            label: format!(
                "自己的 Codex {}",
                slot.source_device.as_deref().unwrap_or("凭据槽")
            ),
            provider: "codex".to_string(),
            model: None,
            availability: slot.status,
            execution_scope: "authorized_account".to_string(),
            cost_basis: "own_subscription".to_string(),
            quota_state: "external_unverified".to_string(),
            task_kinds: text_task_kinds(),
            estimated_unit_cost_micros: None,
            evidence: vec![
                format!("failure_count={}", slot.failure_count),
                "encrypted_vault_slot_present".to_string(),
            ],
        })
        .collect::<Vec<_>>();
    if legacy_available {
        resources.push(AiResourceSummary {
            resource_id: "own_codex:legacy".to_string(),
            resource_class: "own_codex".to_string(),
            label: "自己的 Codex 凭据".to_string(),
            provider: "codex".to_string(),
            model: None,
            availability: "configured".to_string(),
            execution_scope: "authorized_account".to_string(),
            cost_basis: "own_subscription".to_string(),
            quota_state: "external_unverified".to_string(),
            task_kinds: text_task_kinds(),
            estimated_unit_cost_micros: None,
            evidence: vec!["encrypted_legacy_vault_present".to_string()],
        });
    }
    Ok(resources)
}

fn shared_codex_resources(state: &AppState, user_id: &str) -> Result<Vec<AiResourceSummary>> {
    Ok(state
        .store
        .list_codex_vault_emergency_grants(user_id)?
        .into_iter()
        .filter(|grant| {
            grant.consumer_user_id == user_id
                && grant.status == "active"
                && grant.provider_vault_available
        })
        .map(|grant| AiResourceSummary {
            resource_id: format!("shared_codex:{}", grant.id),
            resource_class: "shared_codex".to_string(),
            label: grant
                .label
                .unwrap_or_else(|| format!("{} 授权的 Codex", grant.provider_account)),
            provider: "codex".to_string(),
            model: None,
            availability: "authorized".to_string(),
            execution_scope: "emergency_grant".to_string(),
            cost_basis: "provider_agreement".to_string(),
            quota_state: "external_unverified".to_string(),
            task_kinds: text_task_kinds(),
            estimated_unit_cost_micros: None,
            evidence: vec![
                format!("grant_id={}", grant.id),
                format!("max_lease_seconds={}", grant.max_lease_seconds),
            ],
        })
        .collect())
}

async fn platform_resources(state: &AppState) -> Vec<AiResourceSummary> {
    let config = state.agents_config.read().await;
    config
        .agents
        .iter()
        .map(|(id, agent)| AiResourceSummary {
            resource_id: format!("platform_model:{id}"),
            resource_class: "platform_model".to_string(),
            label: agent.name.clone(),
            provider: id.clone(),
            model: Some(agent.model.clone()),
            availability: "configured".to_string(),
            execution_scope: "platform_runtime".to_string(),
            cost_basis: "platform_billing".to_string(),
            quota_state: "external_unverified".to_string(),
            task_kinds: text_task_kinds(),
            estimated_unit_cost_micros: None,
            evidence: vec![format!("usage_mode={}", agent.usage_mode())],
        })
        .collect()
}

async fn remote_node_resources(state: &AppState, user_id: &str) -> Result<Vec<AiResourceSummary>> {
    let mut resources = Vec::new();
    for node in user_node_runtimes(state, user_id).await? {
        if !node.online {
            continue;
        }
        if node.models.is_empty() {
            resources.push(AiResourceSummary {
                resource_id: format!("remote_node:{}", node.node_id),
                resource_class: "remote_node".to_string(),
                label: node.display_name,
                provider: "user_node".to_string(),
                model: None,
                availability: if node.last_handshake_ai_cli_ready {
                    "ready"
                } else {
                    "online_unverified"
                }
                .to_string(),
                execution_scope: "user_owned_node".to_string(),
                cost_basis: "node_metered".to_string(),
                quota_state: "not_applicable".to_string(),
                task_kinds: text_task_kinds(),
                estimated_unit_cost_micros: None,
                evidence: vec![format!("node_id={}", node.node_id)],
            });
            continue;
        }
        for model in node.models {
            resources.push(AiResourceSummary {
                resource_id: format!("remote_node:{}:{}", node.node_id, model.model_id),
                resource_class: "remote_node".to_string(),
                label: format!("{} / {}", node.display_name, model.display_name),
                provider: model.provider,
                model: Some(model.model_id),
                availability: "ready".to_string(),
                execution_scope: "user_owned_node".to_string(),
                cost_basis: "node_metered".to_string(),
                quota_state: "not_applicable".to_string(),
                task_kinds: text_task_kinds(),
                estimated_unit_cost_micros: credits_to_micros(model.price_per_1k_credits),
                evidence: vec![format!("node_id={}", node.node_id)],
            });
        }
    }
    Ok(resources)
}

fn credits_to_micros(value: f64) -> Option<i64> {
    value
        .is_finite()
        .then(|| (value.max(0.0) * 1_000_000.0).round() as i64)
}

fn text_task_kinds() -> Vec<String> {
    ["chat", "code", "analysis"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

pub(crate) fn select_route(
    project_id: &str,
    policy: &AiResourcePolicy,
    resources: Vec<AiResourceSummary>,
    request: AiRoutePreviewRequest,
) -> AiRoutePreview {
    let mut candidates = resources
        .into_iter()
        .filter(|resource| policy.enabled_classes.contains(&resource.resource_class))
        .filter(|resource| resource.task_kinds.contains(&request.task_kind))
        .filter(|resource| {
            !request.require_local_execution || resource.execution_scope == "user_owned_node"
        })
        .filter(|resource| {
            request
                .preferred_model
                .as_deref()
                .map(|model| resource.model.as_deref() == Some(model))
                .unwrap_or(true)
        })
        .filter(|resource| route_eligible(resource))
        .filter(|resource| match policy.max_estimated_unit_cost_micros {
            Some(limit) => resource
                .estimated_unit_cost_micros
                .is_some_and(|cost| cost <= limit),
            None => true,
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|resource| route_rank(policy, resource));
    let selected = candidates.first().cloned();
    let fallbacks = if policy.allow_fallback {
        candidates.into_iter().skip(1).take(3).collect()
    } else {
        Vec::new()
    };
    let reasons = selected
        .as_ref()
        .map(|resource| {
            vec![
                format!("命中项目优先级：{}", resource.resource_class),
                format!("可用状态：{}", resource.availability),
                format!("成本依据：{}", resource.cost_basis),
            ]
        })
        .unwrap_or_else(|| vec!["没有满足策略、任务类型和本地执行要求的资源".to_string()]);
    AiRoutePreview {
        schema: "ai_resource_control.route_preview.v1",
        project_id: project_id.to_string(),
        task_kind: request.task_kind,
        selected,
        fallbacks,
        reasons,
        execution_started: false,
        quota_verified: false,
    }
}

fn route_eligible(resource: &AiResourceSummary) -> bool {
    matches!(
        resource.availability.as_str(),
        "ready" | "configured" | "active" | "degraded" | "authorized"
    )
}

fn route_rank(policy: &AiResourcePolicy, resource: &AiResourceSummary) -> (usize, usize) {
    let policy_rank = policy
        .priority
        .iter()
        .position(|class| class == &resource.resource_class)
        .unwrap_or(usize::MAX);
    let mode_rank = match policy.privacy_mode.as_str() {
        "prefer_local" => usize::from(resource.execution_scope != "user_owned_node"),
        "prefer_available" => usize::from(resource.availability != "ready"),
        _ => 0,
    };
    (mode_rank, policy_rank)
}

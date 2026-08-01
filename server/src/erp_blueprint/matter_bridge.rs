use anyhow::{anyhow, bail, Result};

use crate::{
    group_ai::{
        planner::build_matter_plan,
        types::{
            CreateMatterPlanRequest, CreateMatterRecord, ProjectAiBot, ProjectAiMatter,
            MATTER_STATUS_CANCELED, MATTER_STATUS_DONE, MATTER_STATUS_FAILED,
            MATTER_STATUS_PLAN_READY,
        },
    },
    store::Store,
};

use super::{
    instance_service::ONBOARDING_EXISTING_PROJECT,
    materialization,
    model::{ErpBlueprint, ErpBlueprintVersion, ErpFeatureProposal, ErpInstance},
};

pub(crate) fn create_proposal_matter(
    store: &Store,
    blueprint: &ErpBlueprint,
    proposal: &ErpFeatureProposal,
    actor_user_id: &str,
    bots: &[ProjectAiBot],
) -> Result<ProjectAiMatter> {
    if proposal.blueprint_id != blueprint.id
        || !matches!(proposal.status.as_str(), "accepted" | "matter_created")
    {
        bail!("只有当前蓝图已接受的提案可以创建 Matter");
    }
    if let Some(matter_id) = proposal.matter_id.as_deref() {
        return store
            .get_project_ai_matter(&blueprint.definition.source_project_id, matter_id)?
            .ok_or_else(|| anyhow!("提案关联的 Matter 不存在"));
    }
    let channel = store
        .list_project_space_channels(actor_user_id, &blueprint.definition.source_project_id)?
        .into_iter()
        .find(|channel| channel.kind == "ai_development")
        .ok_or_else(|| anyhow!("蓝图项目缺少 AI 开发频道"))?;
    let brief = format!(
        "基于 ERP 通用提案 {} 实现公共能力。\n需求摘要：{}\n支持商户数：{}\n行业：{}\n\n边界：不得上传商户原始经营数据、密钥或私有源码；不得自动合并、发布或升级商户实例。",
        proposal.need_key,
        proposal.summary,
        proposal.support_count,
        proposal.industries.join("、")
    );
    let request = CreateMatterPlanRequest {
        channel_id: channel.id.clone(),
        source_message_id: None,
        title: Some(format!("ERP 通用能力：{}", proposal.title)),
        brief: brief.clone(),
        collaboration_mode: Some("solo".into()),
        acceptance_criteria: vec![
            "先更新能力目录和发布清单，再实现公共模块".into(),
            "必须验证咖啡店与最小零售两个参考实例".into(),
            "升级不得覆盖任何商户私有扩展".into(),
            "合并与发布必须由维护者人工确认".into(),
        ],
    };
    let draft = build_matter_plan(&request, actor_user_id, bots);
    store.create_project_ai_matter_for_erp_proposal(
        &proposal.id,
        CreateMatterRecord {
            project_id: blueprint.definition.source_project_id.clone(),
            channel_id: channel.id,
            requester_user_id: actor_user_id.to_string(),
            source_message_id: None,
            title: draft.title,
            brief,
            collaboration_mode: draft.collaboration_mode,
            participant_user_ids: draft.participant_user_ids,
            node_policy_json: draft.node_policy_json,
            acceptance_criteria: draft.acceptance_criteria,
            plan_json: draft.plan_json,
        },
    )
}

pub(crate) fn create_bootstrap_matter(
    store: &Store,
    blueprint: &ErpBlueprint,
    version: &ErpBlueprintVersion,
    instance: &ErpInstance,
    actor_user_id: &str,
    bots: &[ProjectAiBot],
) -> Result<ProjectAiMatter> {
    if instance.blueprint_id != blueprint.id || instance.pinned_version_id != version.id {
        bail!("实例、蓝图与固定版本不一致");
    }
    let contract = materialization::build_contract(blueprint, version, instance);
    let replacement_of = if let Some(matter_id) = instance.bootstrap_matter_id.as_deref() {
        let matter = store
            .get_project_ai_matter(&instance.project_id, matter_id)?
            .ok_or_else(|| anyhow!("实例关联的初始化 Matter 不存在"))?;
        let missing_roles = matter
            .plan
            .get("roles")
            .and_then(serde_json::Value::as_array)
            .map(Vec::is_empty)
            .unwrap_or(true);
        if matter.plan.get("execution_contract") == Some(&serde_json::to_value(&contract)?)
            && matter.status != MATTER_STATUS_CANCELED
            && !missing_roles
        {
            return Ok(matter);
        }
        let assignments = store.list_project_ai_matter_assignments(matter_id)?;
        let has_active_assignment = assignments.iter().any(|assignment| {
            matches!(
                assignment.status.as_str(),
                "queued" | "dispatching" | "running"
            )
        });
        if has_active_assignment
            || !matches!(
                matter.status.as_str(),
                MATTER_STATUS_PLAN_READY
                    | MATTER_STATUS_FAILED
                    | MATTER_STATUS_CANCELED
                    | MATTER_STATUS_DONE
            )
        {
            bail!("旧初始化 Matter 正在执行或等待验收，不能覆盖；请先完成或取消旧流程");
        }
        Some(matter_id.to_string())
    } else {
        None
    };
    let channel = store
        .list_project_space_channels(actor_user_id, &instance.project_id)?
        .into_iter()
        .find(|channel| channel.kind == "ai_development")
        .ok_or_else(|| anyhow!("商户项目缺少 AI 开发频道"))?;
    let modules = instance.enabled_modules.join("、");
    let plugins = instance
        .plugins
        .iter()
        .map(|extension| extension.extension_key.as_str())
        .collect::<Vec<_>>()
        .join("、");
    let private_extensions = instance
        .private_extensions
        .iter()
        .map(|extension| extension.extension_key.as_str())
        .collect::<Vec<_>>()
        .join("、");
    let project_instruction = if instance.onboarding_mode == ONBOARDING_EXISTING_PROJECT {
        "目标是已存在的真实商户项目。先盘点现有模块、数据库迁移、测试和私有扩展，只补齐缺口；不得用空白模板覆盖现有代码。"
    } else {
        "目标是新建的独立商户项目，按固定蓝图版本建立最小可运行基线。"
    };
    let brief = format!(
        "基于 ERP 蓝图 {} v{} 初始化独立商户项目 {}。\n纳入方式：{}\n主题：{}\n公共模块：{}\n行业插件：{}\n私有扩展边界：{}\n源版本提交：{}\n\n{}\n只在当前商户项目工作区实现；不得复制其他商户数据、密钥或私有源码；不得自动发布。",
        blueprint.definition.blueprint_key,
        version.manifest.version,
        instance.instance_key,
        instance.onboarding_mode,
        instance.theme_key,
        modules,
        if plugins.is_empty() { "无" } else { &plugins },
        if private_extensions.is_empty() { "无" } else { &private_extensions },
        version.manifest.source_git_commit,
        project_instruction,
    );
    let request = CreateMatterPlanRequest {
        channel_id: channel.id.clone(),
        source_message_id: None,
        title: Some(format!("初始化 ERP：{}", instance.instance_key)),
        brief: brief.clone(),
        collaboration_mode: Some("solo".into()),
        acceptance_criteria: vec![
            if instance.onboarding_mode == ONBOARDING_EXISTING_PROJECT {
                "先输出已有能力与蓝图能力的差异清单，只实现缺失项".into()
            } else {
                "在独立商户项目中实现发布清单声明的公共模块和能力".into()
            },
            "应用商户主题并保持插件与私有扩展命名空间隔离".into(),
            "生成机器可读实例清单和升级基线，不写入密钥或经营原始数据".into(),
            "完成项目测试；合并与发布由商户人工确认".into(),
        ],
    };
    let mut draft = build_matter_plan(&request, actor_user_id, bots);
    draft.plan_json["execution_contract"] = serde_json::to_value(contract)?;
    let record = CreateMatterRecord {
        project_id: instance.project_id.clone(),
        channel_id: channel.id,
        requester_user_id: actor_user_id.to_string(),
        source_message_id: None,
        title: draft.title,
        brief,
        collaboration_mode: draft.collaboration_mode,
        participant_user_ids: draft.participant_user_ids,
        node_policy_json: draft.node_policy_json,
        acceptance_criteria: draft.acceptance_criteria,
        plan_json: draft.plan_json,
    };
    match replacement_of {
        Some(previous_matter_id) => store.replace_project_ai_matter_for_erp_instance(
            &instance.id,
            &previous_matter_id,
            instance.configuration_revision,
            actor_user_id,
            record,
        ),
        None => store.create_project_ai_matter_for_erp_instance(&instance.id, record),
    }
}

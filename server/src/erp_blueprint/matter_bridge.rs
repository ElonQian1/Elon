use anyhow::{anyhow, bail, Result};

use crate::{
    group_ai::{
        planner::build_matter_plan,
        types::{CreateMatterPlanRequest, CreateMatterRecord, ProjectAiMatter},
    },
    store::Store,
};

use super::model::{ErpBlueprint, ErpFeatureProposal};

pub(crate) fn create_matter(
    store: &Store,
    blueprint: &ErpBlueprint,
    proposal: &ErpFeatureProposal,
    actor_user_id: &str,
) -> Result<ProjectAiMatter> {
    if proposal.blueprint_id != blueprint.id || proposal.status != "accepted" {
        bail!("只有当前蓝图已接受的提案可以创建 Matter");
    }
    if proposal.matter_id.is_some() {
        bail!("该提案已经创建 Matter");
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
    let draft = build_matter_plan(&request, actor_user_id, &[]);
    let matter = store.create_project_ai_matter(CreateMatterRecord {
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
    })?;
    store.attach_matter_to_erp_proposal(&proposal.id, &matter.id)?;
    Ok(matter)
}

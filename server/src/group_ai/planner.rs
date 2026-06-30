use serde_json::{json, Value};
use std::collections::BTreeSet;

use crate::group_ai::{
    context_policy::{plan_ownership, verification_commands},
    types::{
        CreateMatterPlanRequest, ProjectAiBot, COLLAB_MODE_CRITIC, COLLAB_MODE_SOLO,
        COLLAB_MODE_SPLIT,
    },
};

pub(crate) struct MatterPlanDraft {
    pub title: String,
    pub collaboration_mode: String,
    pub participant_user_ids: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub node_policy_json: Value,
    pub plan_json: Value,
}

pub(crate) fn build_matter_plan(
    request: &CreateMatterPlanRequest,
    requester_user_id: &str,
    bots: &[ProjectAiBot],
) -> MatterPlanDraft {
    let collaboration_mode = normalize_collaboration_mode(
        request.collaboration_mode.as_deref(),
        bots.iter().filter(|bot| bot.online).count(),
    );
    let selected_bots = select_bots(&collaboration_mode, bots);
    let acceptance_criteria = clean_criteria(&request.acceptance_criteria);
    let title = clean_title(request.title.as_deref(), &request.brief);
    let participant_user_ids =
        participant_user_ids(requester_user_id, selected_bots.iter().copied());
    let ownership = plan_ownership(&collaboration_mode, &request.brief, &selected_bots);
    let verification_commands = verification_commands(&request.brief);
    let requires_review_gate = collaboration_mode != COLLAB_MODE_SOLO;
    let warnings = if selected_bots.is_empty() {
        vec!["当前项目还没有可用的授权 AI Bot，Matter 只会保存计划。"]
    } else {
        Vec::new()
    };

    MatterPlanDraft {
        title,
        collaboration_mode: collaboration_mode.clone(),
        participant_user_ids,
        acceptance_criteria: acceptance_criteria.clone(),
        node_policy_json: json!({
            "schema_version": 1,
            "requires_project_node_authorization": true,
            "dispatch_state": "not_dispatched",
            "authorized_bot_count": bots.len(),
            "selected_bot_count": selected_bots.len(),
            "budget": {
                "max_billed_cost_rmb_fen": null,
                "pause_on_budget_exceeded": true
            },
            "merge_policy": {
                "requires_human_merge": true,
                "requires_review_gate": requires_review_gate
            },
        }),
        plan_json: json!({
            "schema_version": 1,
            "collaboration_mode": collaboration_mode,
            "brief": request.brief.trim(),
            "roles": plan_roles(&selected_bots),
            "ownership": ownership,
            "verification_commands": verification_commands,
            "merge_policy": {
                "requires_human_merge": true,
                "requires_review_gate": requires_review_gate,
                "acceptance_requires_empty_merge_queue": true
            },
            "steps": [
                "梳理需求、上下文、风险与验收条件",
                "由实现 Bot 在隔离工作区完成实现并输出 diff 与验证命令",
                "由审核 Bot 独立检查实现结果、风险和遗漏",
                "由项目成员决定是否合并、发布或继续拆分"
            ],
            "warnings": warnings,
        }),
    }
}

fn normalize_collaboration_mode(requested: Option<&str>, online_bot_count: usize) -> String {
    match requested.map(str::trim).filter(|value| !value.is_empty()) {
        Some(COLLAB_MODE_SOLO) => COLLAB_MODE_SOLO.to_string(),
        Some(COLLAB_MODE_CRITIC) => COLLAB_MODE_CRITIC.to_string(),
        Some(COLLAB_MODE_SPLIT) => COLLAB_MODE_SPLIT.to_string(),
        _ if online_bot_count >= 3 => COLLAB_MODE_SPLIT.to_string(),
        _ if online_bot_count >= 2 => COLLAB_MODE_CRITIC.to_string(),
        _ => COLLAB_MODE_SOLO.to_string(),
    }
}

fn select_bots<'a>(collaboration_mode: &str, bots: &'a [ProjectAiBot]) -> Vec<&'a ProjectAiBot> {
    let limit = match collaboration_mode {
        COLLAB_MODE_SPLIT => 4,
        COLLAB_MODE_CRITIC => 2,
        _ => 1,
    };
    bots.iter().filter(|bot| bot.online).take(limit).collect()
}

fn clean_criteria(values: &[String]) -> Vec<String> {
    let cleaned: Vec<String> = values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    if !cleaned.is_empty() {
        return cleaned;
    }
    vec![
        "实现前必须确认项目上下文和修改范围".to_string(),
        "实现结果必须附带 diff、验证命令和风险说明".to_string(),
        "审核 Bot 必须独立检查实现结果".to_string(),
    ]
}

fn clean_title(title: Option<&str>, brief: &str) -> String {
    if let Some(title) = title.map(str::trim).filter(|value| !value.is_empty()) {
        return title.chars().take(80).collect();
    }
    let fallback = brief.lines().next().unwrap_or("群体 AI 开发 Matter").trim();
    if fallback.is_empty() {
        "群体 AI 开发 Matter".to_string()
    } else {
        fallback.chars().take(48).collect()
    }
}

fn participant_user_ids<'a>(
    requester_user_id: &str,
    selected_bots: impl Iterator<Item = &'a ProjectAiBot>,
) -> Vec<String> {
    let mut ids = BTreeSet::new();
    ids.insert(requester_user_id.to_string());
    for bot in selected_bots {
        ids.insert(bot.provider_user_id.clone());
    }
    ids.into_iter().collect()
}

fn plan_roles(selected_bots: &[&ProjectAiBot]) -> Vec<Value> {
    selected_bots
        .iter()
        .enumerate()
        .map(|(index, bot)| {
            let role = match index {
                0 => "lead_implementer",
                1 => "reviewer",
                _ => "parallel_worker",
            };
            json!({
                "role": role,
                "bot_id": bot.bot_id,
                "node_id": bot.node_id,
                "provider_user_id": bot.provider_user_id,
                "runtime_route": bot.runtime_route,
                "cli_name": bot.cli_name,
            })
        })
        .collect()
}

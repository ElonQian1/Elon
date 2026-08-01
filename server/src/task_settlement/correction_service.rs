use anyhow::{anyhow, bail, Result};

use crate::{
    group_ai::{
        planner::build_matter_plan,
        types::{CreateMatterPlanRequest, CreateMatterRecord, ProjectAiBot},
    },
    store::Store,
};

use super::model::{
    CreateSettlementCorrection, CreateSettlementCorrectionRequest, SettlementCorrectionDetail,
    DISPUTE_ACCEPTED,
};

pub(super) fn create(
    store: &Store,
    project_id: &str,
    dispute_id: &str,
    actor_user_id: &str,
    request: &CreateSettlementCorrectionRequest,
    bots: &[ProjectAiBot],
) -> Result<SettlementCorrectionDetail> {
    validate_amounts(
        request.corrected_compute_amount_micros,
        request.corrected_provider_amount_micros,
    )?;
    let summary = required_text(&request.summary, "纠正说明", 8, 1000)?;
    let evidence_ref = optional_text(request.evidence_ref.as_deref(), "证据引用", 512)?;
    let dispute = store
        .task_settlement_dispute_detail(project_id, dispute_id)?
        .ok_or_else(|| anyhow!("影子结算争议不存在"))?;
    if dispute.dispute.status != DISPUTE_ACCEPTED {
        bail!("只有已接受的争议可以创建纠正 Matter");
    }
    let original = store
        .task_settlement_receipt(project_id, &dispute.dispute.settlement_receipt_id)?
        .ok_or_else(|| anyhow!("争议关联的原影子凭证不存在"))?;
    let channel = store
        .list_project_space_channels(actor_user_id, project_id)?
        .into_iter()
        .find(|channel| channel.kind == "ai_development")
        .ok_or_else(|| anyhow!("当前项目缺少 AI 开发频道，不能创建纠正 Matter"))?;
    let brief = format!(
        "核查并纠正影子结算凭证 {}。\n争议：{}\n原计算金额：{} 微元；原节点金额：{} 微元。\n拟纠正为：计算金额 {} 微元；节点金额 {} 微元。\n纠正说明：{}\n证据引用：{}\n\n只核查链外影子事实。不得修改或删除原凭证，不得操作真实余额、退款、提现、钱包或链上资产。验收后由系统原子追加冲销与替换凭证。",
        original.id,
        dispute.dispute.summary,
        original.compute_amount_micros,
        original.provider_amount_micros,
        request.corrected_compute_amount_micros,
        request.corrected_provider_amount_micros,
        summary,
        evidence_ref.as_deref().unwrap_or("未提供"),
    );
    let plan_request = CreateMatterPlanRequest {
        channel_id: channel.id.clone(),
        source_message_id: None,
        title: Some(format!("影子结算纠正：{}", short_id(&original.id))),
        brief: brief.clone(),
        collaboration_mode: Some("critic".into()),
        acceptance_criteria: vec![
            "核对原始用量、计价策略、节点分配与争议证据，并提交可复核 Artifact".into(),
            "明确纠正前后计算、节点和平台金额，确认节点金额不高于计算金额".into(),
            "确认原凭证保持不可变，纠正采用追加式冲销与替换凭证".into(),
            "确认不修改真实余额、不退款、不提现、不提交任何链上交易".into(),
            "Review Gate 通过后仍需项目人员显式人工验收".into(),
        ],
    };
    let mut draft = build_matter_plan(&plan_request, actor_user_id, bots);
    draft.plan_json["settlement_correction_contract"] = serde_json::json!({
        "schema": "task_economy.settlement_correction.v1",
        "dispute_id": dispute_id.trim(),
        "original_receipt_id": original.id,
        "corrected_compute_amount_micros": request.corrected_compute_amount_micros,
        "corrected_provider_amount_micros": request.corrected_provider_amount_micros,
        "shadow_only": true,
        "posting_mode": "atomic_reversal_and_replacement",
    });
    store.create_task_settlement_correction_with_matter(
        CreateSettlementCorrection {
            project_id,
            dispute_id,
            corrected_compute_amount_micros: request.corrected_compute_amount_micros,
            corrected_provider_amount_micros: request.corrected_provider_amount_micros,
            summary: &summary,
            evidence_ref: evidence_ref.as_deref(),
            actor_user_id,
        },
        CreateMatterRecord {
            project_id: project_id.trim().to_string(),
            channel_id: channel.id,
            requester_user_id: actor_user_id.trim().to_string(),
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

pub(super) fn list(
    store: &Store,
    project_id: &str,
    receipt_id: &str,
) -> Result<Vec<SettlementCorrectionDetail>> {
    store
        .task_settlement_receipt(project_id, receipt_id)?
        .ok_or_else(|| anyhow!("影子结算凭证不存在"))?;
    store.list_task_settlement_corrections(project_id, receipt_id, 100)
}

pub(super) fn finalize(
    store: &Store,
    project_id: &str,
    correction_id: &str,
    actor_user_id: &str,
) -> Result<SettlementCorrectionDetail> {
    if !super::service::project_active(store, project_id)? {
        bail!("影子经济未启用，不能生成纠正凭证");
    }
    store.post_task_settlement_correction(project_id, correction_id, actor_user_id)
}

pub(super) fn finalize_for_accepted_matter(
    store: &Store,
    project_id: &str,
    matter_id: &str,
) -> Result<usize> {
    if !super::service::project_active(store, project_id)? {
        return Ok(0);
    }
    let matter = store
        .get_project_ai_matter(project_id, matter_id)?
        .ok_or_else(|| anyhow!("Matter 不存在"))?;
    let actor = matter
        .decision_user_id
        .as_deref()
        .unwrap_or(&matter.requester_user_id);
    store.post_task_settlement_corrections_for_matter(project_id, matter_id, actor)
}

pub(super) fn cancel_for_matter(store: &Store, project_id: &str, matter_id: &str) -> Result<usize> {
    let matter = store
        .get_project_ai_matter(project_id, matter_id)?
        .ok_or_else(|| anyhow!("Matter 不存在"))?;
    let actor = matter
        .decision_user_id
        .as_deref()
        .unwrap_or(&matter.requester_user_id);
    store.cancel_task_settlement_corrections_for_matter(project_id, matter_id, actor)
}

pub(super) fn ensure_standard_projection(
    store: &Store,
    project_id: &str,
    receipt_id: &str,
) -> Result<()> {
    if store.task_settlement_receipt_is_correction(project_id, receipt_id)? {
        bail!("纠正冲销与替换凭证必须作为同一纠正包原子投影，不能单独准备 Sui 投影");
    }
    Ok(())
}

fn validate_amounts(compute: i64, provider: i64) -> Result<()> {
    if compute < 0 || provider < 0 {
        bail!("纠正金额不能为负数");
    }
    if provider > compute {
        bail!("纠正后的节点金额不能高于计算金额");
    }
    Ok(())
}

fn required_text(value: &str, label: &str, min: usize, max: usize) -> Result<String> {
    let value = value.trim();
    let length = value.chars().count();
    if length < min || length > max {
        bail!("{label}长度必须为 {min} 至 {max} 个字符");
    }
    Ok(value.to_string())
}

fn optional_text(value: Option<&str>, label: &str, max: usize) -> Result<Option<String>> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if value.chars().count() > max {
        bail!("{label}不能超过 {max} 个字符");
    }
    Ok(Some(value.to_string()))
}

fn short_id(value: &str) -> &str {
    value.get(..value.len().min(12)).unwrap_or(value)
}

#[cfg(test)]
#[path = "correction_service_tests.rs"]
mod tests;

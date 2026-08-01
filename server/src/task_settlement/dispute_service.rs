use anyhow::{anyhow, bail, Result};

use crate::store::Store;

use super::model::{
    CreateSettlementDispute, OpenSettlementDisputeRequest, ResolveSettlementDisputeRequest,
    SettlementDisputeDetail, WithdrawSettlementDisputeRequest, DISPUTE_ACCEPTED, DISPUTE_REJECTED,
    DISPUTE_WITHDRAWN, RECEIPT_KIND_CORRECTION_REVERSAL,
};

pub(super) fn open(
    store: &Store,
    project_id: &str,
    receipt_id: &str,
    actor_user_id: &str,
    request: &OpenSettlementDisputeRequest,
) -> Result<SettlementDisputeDetail> {
    let receipt = store
        .task_settlement_receipt(project_id, receipt_id)?
        .ok_or_else(|| anyhow!("影子结算凭证不存在"))?;
    if receipt.receipt_kind == RECEIPT_KIND_CORRECTION_REVERSAL {
        bail!("冲销凭证是不可变的反向记录，不能单独发起争议；请针对替换凭证继续纠正");
    }
    let reason_code = normalized_reason_code(&request.reason_code)?;
    let summary = required_text(&request.summary, "争议摘要", 8, 500)?;
    let evidence_ref = optional_text(request.evidence_ref.as_deref(), "证据引用", 512)?;
    store.create_task_settlement_dispute(CreateSettlementDispute {
        project_id,
        settlement_receipt_id: receipt_id,
        reason_code,
        summary: &summary,
        evidence_ref: evidence_ref.as_deref(),
        actor_user_id,
    })
}

pub(super) fn list(
    store: &Store,
    project_id: &str,
    receipt_id: &str,
) -> Result<Vec<SettlementDisputeDetail>> {
    store
        .task_settlement_receipt(project_id, receipt_id)?
        .ok_or_else(|| anyhow!("影子结算凭证不存在"))?;
    store.list_task_settlement_disputes(project_id, receipt_id, 100)
}

pub(super) fn withdraw(
    store: &Store,
    project_id: &str,
    dispute_id: &str,
    actor_user_id: &str,
    request: &WithdrawSettlementDisputeRequest,
) -> Result<SettlementDisputeDetail> {
    let note = required_text(&request.note, "撤回说明", 4, 1000)?;
    store.transition_task_settlement_dispute(
        project_id,
        dispute_id,
        actor_user_id,
        DISPUTE_WITHDRAWN,
        &note,
    )
}

pub(super) fn resolve(
    store: &Store,
    project_id: &str,
    dispute_id: &str,
    actor_user_id: &str,
    request: &ResolveSettlementDisputeRequest,
) -> Result<SettlementDisputeDetail> {
    let target_status = match request.decision.trim().to_ascii_lowercase().as_str() {
        "accept" | "accepted" => DISPUTE_ACCEPTED,
        "reject" | "rejected" => DISPUTE_REJECTED,
        _ => bail!("争议审核决定必须是 accept 或 reject"),
    };
    let note = required_text(&request.note, "审核结论", 4, 1000)?;
    store.transition_task_settlement_dispute(
        project_id,
        dispute_id,
        actor_user_id,
        target_status,
        &note,
    )
}

pub(super) fn ensure_projection_allowed(
    store: &Store,
    project_id: &str,
    receipt_id: &str,
) -> Result<()> {
    if store.task_settlement_has_blocking_dispute(project_id, receipt_id)? {
        bail!("影子凭证存在待审核或已接受争议，不能生成或准备 Sui 投影");
    }
    Ok(())
}

fn normalized_reason_code(value: &str) -> Result<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "amount" => Ok("amount"),
        "provider_allocation" => Ok("provider_allocation"),
        "policy" => Ok("policy"),
        "source_evidence" => Ok("source_evidence"),
        "other" => Ok("other"),
        _ => bail!("争议原因必须是金额、节点分配、策略、来源证据或其他"),
    }
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

#[cfg(test)]
#[path = "dispute_service_tests.rs"]
mod tests;

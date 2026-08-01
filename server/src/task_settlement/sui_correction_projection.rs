use anyhow::{bail, Result};
use serde_json::json;
use sha2::{Digest, Sha256};

use super::{
    model::{
        SettlementCorrectionDetail, SettlementReceipt, CORRECTION_POSTED,
        RECEIPT_KIND_CORRECTION_REPLACEMENT, RECEIPT_KIND_CORRECTION_REVERSAL, RECEIPT_RECONCILED,
        SUI_NETWORK_NOT_SUBMITTED,
    },
    sui_correction_model::{
        SuiCorrectionProjectionEnvelope, SuiCorrectionProjectionLeg, SUI_CORRECTION_PACKAGE_SCHEMA,
        SUI_CORRECTION_PROJECTION_SCHEMA,
    },
};

pub(super) fn envelope_and_source_digest(
    detail: &SettlementCorrectionDetail,
) -> Result<(SuiCorrectionProjectionEnvelope, String)> {
    validate_detail(detail)?;
    let correction = &detail.correction;
    let reversal = detail
        .reversal_receipt
        .as_ref()
        .expect("validated reversal");
    let replacement = detail
        .replacement_receipt
        .as_ref()
        .expect("validated replacement");
    let original_digest = receipt_digest(&detail.original_receipt)?;
    let reversal_leg = leg(reversal)?;
    let replacement_leg = leg(replacement)?;
    let source_bundle_digest = digest_json(&json!({
        "schema": "task_economy.settlement_correction_source_bundle.v1",
        "correction_id": correction.id,
        "correction_matter_id": correction.correction_matter_id,
        "matter_status": correction.matter_status,
        "matter_final_decision": correction.matter_final_decision,
        "original_receipt_digest": original_digest,
        "reversal_receipt_digest": reversal_leg.source_receipt_digest,
        "replacement_receipt_digest": replacement_leg.source_receipt_digest,
        "corrected_compute_amount_micros": correction.corrected_compute_amount_micros,
        "corrected_provider_amount_micros": correction.corrected_provider_amount_micros,
        "corrected_platform_amount_micros": correction.corrected_platform_amount_micros,
    }))?;
    Ok((
        SuiCorrectionProjectionEnvelope {
            schema: SUI_CORRECTION_PROJECTION_SCHEMA.to_string(),
            correction_id: correction.id.clone(),
            correction_matter_id: correction.correction_matter_id.clone(),
            original_receipt_id: correction.original_settlement_receipt_id.clone(),
            project_object_key: format!("project:{}", correction.project_id),
            reversal: reversal_leg,
            replacement: replacement_leg,
            shadow_only: true,
            atomic_bundle: true,
            ptb_steps: vec![
                "verify_correction_matter_acceptance_and_source_bundle_digest".to_string(),
                "require_reversal_and_replacement_receipts_together".to_string(),
                "apply_net_shadow_correction_without_moving_funds".to_string(),
                "emit_atomic_settlement_correction_event".to_string(),
            ],
            network_submission: SUI_NETWORK_NOT_SUBMITTED.to_string(),
        },
        source_bundle_digest,
    ))
}

pub(super) fn projection_digest(
    target_network: &str,
    envelope: &SuiCorrectionProjectionEnvelope,
) -> Result<String> {
    digest_json(&json!({
        "schema": SUI_CORRECTION_PACKAGE_SCHEMA,
        "target_network": target_network,
        "envelope": envelope,
    }))
}

fn validate_detail(detail: &SettlementCorrectionDetail) -> Result<()> {
    let correction = &detail.correction;
    if correction.status != CORRECTION_POSTED
        || correction.matter_status != "done"
        || correction.matter_final_decision.as_deref() != Some("accepted")
    {
        bail!("只有已通过人工验收并完成过账的纠正可以准备 Sui 纠正投影包");
    }
    let reversal = detail
        .reversal_receipt
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("纠正缺少冲销凭证"))?;
    let replacement = detail
        .replacement_receipt
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("纠正缺少替换凭证"))?;
    validate_leg(reversal, correction, RECEIPT_KIND_CORRECTION_REVERSAL)?;
    validate_leg(replacement, correction, RECEIPT_KIND_CORRECTION_REPLACEMENT)?;
    if correction.reversal_receipt_id.as_deref() != Some(reversal.id.as_str())
        || correction.replacement_receipt_id.as_deref() != Some(replacement.id.as_str())
    {
        bail!("纠正记录与冲销或替换凭证关联不一致");
    }
    let original = &detail.original_receipt;
    if reversal.compute_amount_micros != original.compute_amount_micros
        || reversal.provider_amount_micros != original.provider_amount_micros
        || reversal.platform_amount_micros != original.platform_amount_micros
        || reversal.currency != original.currency
    {
        bail!("冲销凭证没有完整反向绑定原凭证金额");
    }
    if replacement.compute_amount_micros != correction.corrected_compute_amount_micros
        || replacement.provider_amount_micros != correction.corrected_provider_amount_micros
        || replacement.platform_amount_micros != correction.corrected_platform_amount_micros
        || replacement.currency != original.currency
    {
        bail!("替换凭证与纠正后金额不一致");
    }
    Ok(())
}

fn validate_leg(
    receipt: &SettlementReceipt,
    correction: &super::model::SettlementCorrection,
    expected_kind: &str,
) -> Result<()> {
    if !receipt.shadow_only
        || receipt.status != RECEIPT_RECONCILED
        || receipt.project_id != correction.project_id
        || receipt.receipt_kind != expected_kind
        || receipt.correction_id.as_deref() != Some(correction.id.as_str())
        || receipt.accepted_matter_id.as_deref() != Some(correction.correction_matter_id.as_str())
    {
        bail!("纠正投影腿与项目、Matter、种类或影子状态不一致");
    }
    Ok(())
}

fn leg(receipt: &SettlementReceipt) -> Result<SuiCorrectionProjectionLeg> {
    Ok(SuiCorrectionProjectionLeg {
        receipt_id: receipt.id.clone(),
        receipt_kind: receipt.receipt_kind.clone(),
        intent_id: receipt.intent_id.clone(),
        posting_key: receipt.posting_key.clone(),
        compute_amount_micros: receipt.compute_amount_micros,
        provider_amount_micros: receipt.provider_amount_micros,
        platform_amount_micros: receipt.platform_amount_micros,
        currency: receipt.currency.clone(),
        source_receipt_digest: receipt_digest(receipt)?,
    })
}

fn receipt_digest(receipt: &SettlementReceipt) -> Result<String> {
    digest_json(&json!({
        "schema": "task_economy.correction_receipt_digest.v1",
        "id": receipt.id,
        "project_id": receipt.project_id,
        "intent_id": receipt.intent_id,
        "posting_key": receipt.posting_key,
        "status": receipt.status,
        "compute_amount_micros": receipt.compute_amount_micros,
        "provider_amount_micros": receipt.provider_amount_micros,
        "platform_amount_micros": receipt.platform_amount_micros,
        "outcome_reward_micros": receipt.outcome_reward_micros,
        "review_reward_micros": receipt.review_reward_micros,
        "currency": receipt.currency,
        "shadow_only": receipt.shadow_only,
        "accepted_matter_id": receipt.accepted_matter_id,
        "reason": receipt.reason,
        "receipt_kind": receipt.receipt_kind,
        "correction_id": receipt.correction_id,
        "created_at": receipt.created_at,
    }))
}

fn digest_json(value: &serde_json::Value) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(value)?)))
}

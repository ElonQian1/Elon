use anyhow::{bail, Result};
use serde_json::json;
use sha2::{Digest, Sha256};

use super::model::{
    SettlementReceipt, SuiSettlementEnvelope, RECEIPT_RECONCILED, SUI_NETWORK_NOT_SUBMITTED,
    SUI_PROJECTION_PACKAGE_SCHEMA, SUI_PROJECTION_SCHEMA,
};

pub(super) fn envelope(receipt: &SettlementReceipt) -> Result<SuiSettlementEnvelope> {
    if !receipt.shadow_only || receipt.status != RECEIPT_RECONCILED {
        bail!("仅已对账的影子凭证可以生成 Sui 投影信封");
    }
    Ok(SuiSettlementEnvelope {
        schema: SUI_PROJECTION_SCHEMA.to_string(),
        source_receipt_id: receipt.id.clone(),
        source_posting_key: receipt.posting_key.clone(),
        project_object_key: format!("project:{}", receipt.project_id),
        intent_object_key: format!("intent:{}", receipt.intent_id),
        receipt_object_key: format!("receipt:{}", receipt.id),
        amount_micros: receipt.compute_amount_micros,
        provider_amount_micros: receipt.provider_amount_micros,
        platform_amount_micros: receipt.platform_amount_micros,
        currency: receipt.currency.clone(),
        shadow_only: true,
        ptb_steps: vec![
            "verify_project_economy_policy_object".to_string(),
            "verify_source_receipt_digest_and_idempotency_key".to_string(),
            "create_or_update_provider_reputation_object".to_string(),
            "emit_settlement_receipt_event_without_moving_funds".to_string(),
        ],
        network_submission: SUI_NETWORK_NOT_SUBMITTED.to_string(),
    })
}

pub(super) fn normalized_target_network(value: &str) -> Result<&str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "devnet" => Ok("devnet"),
        "testnet" => Ok("testnet"),
        "mainnet" => Ok("mainnet"),
        _ => bail!("Sui 目标网络必须是 devnet、testnet 或 mainnet"),
    }
}

pub(super) fn envelope_json(envelope: &SuiSettlementEnvelope) -> Result<String> {
    Ok(serde_json::to_string(envelope)?)
}

pub(super) fn projection_digest(
    target_network: &str,
    envelope: &SuiSettlementEnvelope,
) -> Result<String> {
    digest_json(&json!({
        "schema": SUI_PROJECTION_PACKAGE_SCHEMA,
        "target_network": target_network,
        "envelope": envelope,
    }))
}

pub(super) fn source_receipt_digest(receipt: &SettlementReceipt) -> Result<String> {
    digest_json(&json!({
        "schema": "task_economy.settlement_receipt_digest.v1",
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
        "created_at": receipt.created_at,
    }))
}

fn digest_json(value: &serde_json::Value) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(value)?)))
}

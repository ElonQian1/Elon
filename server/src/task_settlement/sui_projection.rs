use anyhow::{bail, Result};

use super::model::{SettlementReceipt, SuiSettlementEnvelope, RECEIPT_RECONCILED};

pub(super) fn envelope(receipt: &SettlementReceipt) -> Result<SuiSettlementEnvelope> {
    if !receipt.shadow_only || receipt.status != RECEIPT_RECONCILED {
        bail!("仅已对账的影子凭证可以生成 Sui 投影信封");
    }
    Ok(SuiSettlementEnvelope {
        schema: "task_economy.sui_projection.v1",
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
        network_submission: "not_submitted",
    })
}

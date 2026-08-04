use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

use super::{
    super::{
        super::{
            compute_attempt_settlement_challenge_resolutions::compute_settlement_challenge_resolution_on,
            compute_attempt_settlements::{
                calculation::MICROS_PER_CNY_FEN, compute_attempt_settlement_on,
            },
        },
        ComputeSettlementCorrectionReceipt, COMPUTE_SETTLEMENT_CORRECTION_POLICY_ID,
        COMPUTE_SETTLEMENT_CORRECTION_POLICY_VERSION, COMPUTE_SETTLEMENT_CORRECTION_SCHEMA,
    },
    correction_event_digest, correction_request_digest, evidence_refs_digest, money,
    normalize_correction_request, StoredCorrection,
};

pub(super) fn audited_correction_on(
    conn: &Connection,
    stored: &StoredCorrection,
    replayed: bool,
) -> Result<ComputeSettlementCorrectionReceipt> {
    let request = normalize_correction_request(&serde_json::from_str(&stored.request_json)?)?;
    let evidence_refs: Vec<String> = serde_json::from_str(&stored.evidence_refs_json)?;
    let mut receipt: ComputeSettlementCorrectionReceipt =
        serde_json::from_str(&stored.receipt_json)?;
    if request.lease_id != stored.lease_id
        || request.expected_challenge_id != stored.challenge_id
        || request.expected_challenge_event_digest != stored.challenge_event_digest
        || request.expected_resolution_id != stored.resolution_id
        || request.expected_resolution_event_digest != stored.resolution_event_digest
        || request.expected_settlement_receipt_id != stored.settlement_receipt_id
        || request.expected_settlement_event_digest != stored.settlement_event_digest
        || request.idempotency_key != stored.idempotency_key
        || request.corrected_by_user_id != stored.corrected_by_user_id
        || stored.idempotency_scope
            != format!(
                "compute_settlement_correction:{}",
                request.corrected_by_user_id
            )
        || correction_request_digest(&request)? != stored.request_digest
        || receipt.schema != COMPUTE_SETTLEMENT_CORRECTION_SCHEMA
        || receipt.correction_id != stored.correction_id
        || receipt.challenge_id != stored.challenge_id
        || receipt.challenge_event_digest != stored.challenge_event_digest
        || receipt.resolution_id != stored.resolution_id
        || receipt.resolution_event_digest != stored.resolution_event_digest
        || receipt.settlement_receipt_id != stored.settlement_receipt_id
        || receipt.settlement_event_digest != stored.settlement_event_digest
        || receipt.lease_id != stored.lease_id
        || receipt.consumer_account_id != stored.consumer_account_id
        || receipt.provider_account_id != stored.provider_account_id
        || receipt.original_consumer_charge_micros != stored.original_consumer_charge_micros
        || receipt.corrected_consumer_charge_micros != stored.corrected_consumer_charge_micros
        || receipt.consumer_refund_micros != stored.consumer_refund_micros
        || receipt.original_provider_payable_micros != stored.original_provider_payable_micros
        || receipt.corrected_provider_payable_micros != stored.corrected_provider_payable_micros
        || receipt.provider_reversal_micros != stored.provider_reversal_micros
        || receipt.original_platform_margin_micros != stored.original_platform_margin_micros
        || receipt.corrected_platform_margin_micros != stored.corrected_platform_margin_micros
        || receipt.platform_reversal_micros != stored.platform_reversal_micros
        || receipt.statement != stored.statement
        || receipt.evidence_refs != evidence_refs
        || receipt.evidence_refs_digest != stored.evidence_refs_digest
        || receipt.policy_id != stored.policy_id
        || receipt.policy_version != stored.policy_version
        || receipt.posting_id != stored.posting_id
        || receipt.posting_digest != stored.posting_digest
        || receipt.request_digest != stored.request_digest
        || receipt.event_digest != stored.event_digest
        || receipt.corrected_by_user_id != stored.corrected_by_user_id
        || receipt.corrected_at != stored.corrected_at
        || receipt.replayed
    {
        bail!("结算纠正数据库列、请求或回执 JSON 不一致");
    }
    if evidence_refs_digest(&receipt.evidence_refs)? != stored.evidence_refs_digest
        || correction_event_digest(&receipt)? != stored.event_digest
    {
        bail!("结算纠正证据或事件摘要审计失败");
    }

    let settlement = compute_attempt_settlement_on(conn, &stored.lease_id)?;
    let resolution = compute_settlement_challenge_resolution_on(conn, &stored.lease_id)?;
    if settlement.settlement.settlement_receipt_id != stored.settlement_receipt_id
        || settlement.event_digest != stored.settlement_event_digest
        || settlement.settlement.consumer_account_id != stored.consumer_account_id
        || settlement.settlement.provider_account_id != stored.provider_account_id
        || resolution.challenge_id != stored.challenge_id
        || resolution.challenge_event_digest != stored.challenge_event_digest
        || resolution.resolution_id != stored.resolution_id
        || resolution.event_digest != stored.resolution_event_digest
        || resolution.settlement_receipt_id != stored.settlement_receipt_id
        || resolution.settlement_event_digest != stored.settlement_event_digest
        || resolution.action != "accepted"
        || !resolution.correction_required
    {
        bail!("结算纠正上游 accepted 挑战、决议或 Settlement Receipt 审计失败");
    }
    audit_amounts(&settlement, &request, &receipt)?;
    audit_posting(conn, &receipt)?;
    audit_pending_projection(
        conn,
        "provider",
        &receipt.provider_account_id,
        "provider_pending",
    )?;
    audit_pending_projection(
        conn,
        "platform",
        money::PLATFORM_ACCOUNT_ID,
        "platform_pending",
    )?;
    if receipt.platform_account_id != money::PLATFORM_ACCOUNT_ID
        || receipt.currency != "CNY"
        || receipt.policy_id != COMPUTE_SETTLEMENT_CORRECTION_POLICY_ID
        || receipt.policy_version != COMPUTE_SETTLEMENT_CORRECTION_POLICY_VERSION
        || receipt.balance_effect != "consumer_refunded_provider_and_platform_pending_reversed"
        || receipt.settlement_release_effect != "accepted_challenge_corrected_release_net_amounts"
    {
        bail!("结算纠正策略或资金效果字段无效");
    }
    receipt.replayed = replayed;
    Ok(receipt)
}

fn audit_amounts(
    settlement: &super::super::super::compute_attempt_settlements::ComputeAttemptSettlementReceipt,
    request: &super::super::CorrectComputeAttemptSettlementRequest,
    receipt: &ComputeSettlementCorrectionReceipt,
) -> Result<()> {
    let corrected_consumer_micros = request
        .corrected_consumer_charge_fen
        .checked_mul(MICROS_PER_CNY_FEN)
        .ok_or_else(|| anyhow!("结算纠正消费者金额换算溢出"))?;
    let expected_refund_fen = settlement
        .consumer_charged_fen
        .checked_sub(request.corrected_consumer_charge_fen)
        .ok_or_else(|| anyhow!("结算纠正消费者退款金额下溢"))?;
    let expected_refund_micros = settlement
        .settlement
        .amounts
        .consumer_charge_micros
        .checked_sub(corrected_consumer_micros)
        .ok_or_else(|| anyhow!("结算纠正消费者退款微单位下溢"))?;
    let expected_provider_reversal = settlement
        .settlement
        .amounts
        .provider_payable_micros
        .checked_sub(request.corrected_provider_payable_micros)
        .ok_or_else(|| anyhow!("结算纠正 Provider 冲减金额下溢"))?;
    let expected_platform_reversal = settlement
        .settlement
        .amounts
        .platform_margin_micros
        .checked_sub(request.corrected_platform_margin_micros)
        .ok_or_else(|| anyhow!("结算纠正平台冲减金额下溢"))?;
    let corrected_credit_total = request
        .corrected_provider_payable_micros
        .checked_add(request.corrected_platform_margin_micros)
        .ok_or_else(|| anyhow!("结算纠正后的贷方金额溢出"))?;
    let expected_reversal_total = expected_provider_reversal
        .checked_add(expected_platform_reversal)
        .ok_or_else(|| anyhow!("结算纠正冲减金额溢出"))?;
    if request.corrected_consumer_charge_fen >= settlement.consumer_charged_fen
        || corrected_consumer_micros != corrected_credit_total
        || expected_refund_micros != expected_reversal_total
        || receipt.original_consumer_charge_fen != settlement.consumer_charged_fen
        || receipt.original_consumer_charge_micros
            != settlement.settlement.amounts.consumer_charge_micros
        || receipt.corrected_consumer_charge_fen != request.corrected_consumer_charge_fen
        || receipt.corrected_consumer_charge_micros != corrected_consumer_micros
        || receipt.consumer_refund_fen != expected_refund_fen
        || receipt.consumer_refund_micros != expected_refund_micros
        || receipt.original_provider_payable_micros
            != settlement.settlement.amounts.provider_payable_micros
        || receipt.corrected_provider_payable_micros != request.corrected_provider_payable_micros
        || receipt.provider_reversal_micros != expected_provider_reversal
        || receipt.original_platform_margin_micros
            != settlement.settlement.amounts.platform_margin_micros
        || receipt.corrected_platform_margin_micros != request.corrected_platform_margin_micros
        || receipt.platform_reversal_micros != expected_platform_reversal
    {
        bail!("结算纠正金额守恒审计失败");
    }
    Ok(())
}

fn audit_posting(conn: &Connection, receipt: &ComputeSettlementCorrectionReceipt) -> Result<()> {
    let row = money::correction_posting_row_on(conn, &receipt.posting_id)?
        .ok_or_else(|| anyhow!("结算纠正 posting 不存在"))?;
    let input = money::PostCorrectionMoneyInput {
        correction_id: &receipt.correction_id,
        settlement_receipt_id: &receipt.settlement_receipt_id,
        consumer_account_id: &receipt.consumer_account_id,
        provider_account_id: &receipt.provider_account_id,
        consumer_refund_fen: receipt.consumer_refund_fen,
        consumer_refund_micros: receipt.consumer_refund_micros,
        provider_reversal_micros: receipt.provider_reversal_micros,
        platform_reversal_micros: receipt.platform_reversal_micros,
        corrected_at: &receipt.corrected_at,
    };
    let provider = money::PendingReversalOutcome {
        pending_after_micros: receipt.provider_pending_balance_after_micros,
        revision_after: receipt.provider_account_revision_after,
    };
    let platform = money::PendingReversalOutcome {
        pending_after_micros: receipt.platform_pending_balance_after_micros,
        revision_after: receipt.platform_account_revision_after,
    };
    let expected_digest = money::correction_posting_digest(
        &receipt.posting_id,
        &input,
        receipt.consumer_balance_after_fen,
        &provider,
        &platform,
    )?;
    if row.0 != receipt.correction_id
        || row.1 != receipt.settlement_receipt_id
        || row.2 != receipt.consumer_refund_micros
        || row.3 != receipt.provider_reversal_micros
        || row.4 != receipt.platform_reversal_micros
        || row.5 != receipt.posting_digest
        || row.5 != expected_digest
        || row.6 != receipt.corrected_at
    {
        bail!("结算纠正 posting 摘要或金额审计失败");
    }
    audit_ledger_legs(conn, receipt)
}

fn audit_ledger_legs(
    conn: &Connection,
    receipt: &ComputeSettlementCorrectionReceipt,
) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT line_no, account_kind, leg_kind, account_id, direction,
                amount_micros, balance_state, balance_after_micros,
                account_revision_after
           FROM compute_settlement_correction_ledger_legs
          WHERE posting_id=?1 ORDER BY line_no",
    )?;
    let legs = stmt
        .query_map(params![receipt.posting_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, Option<i64>>(8)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let consumer_balance_after_micros = receipt
        .consumer_balance_after_fen
        .checked_mul(MICROS_PER_CNY_FEN)
        .ok_or_else(|| anyhow!("消费者纠正后余额换算溢出"))?;
    let expected = vec![
        (
            1,
            "consumer".to_string(),
            "consumer_correction_refund".to_string(),
            receipt.consumer_account_id.clone(),
            "credit".to_string(),
            receipt.consumer_refund_micros,
            "consumer_balance".to_string(),
            consumer_balance_after_micros,
            None,
        ),
        (
            2,
            "provider".to_string(),
            "provider_pending_reversal".to_string(),
            receipt.provider_account_id.clone(),
            "debit".to_string(),
            receipt.provider_reversal_micros,
            "pending".to_string(),
            receipt.provider_pending_balance_after_micros,
            Some(receipt.provider_account_revision_after),
        ),
        (
            3,
            "platform".to_string(),
            "platform_pending_reversal".to_string(),
            money::PLATFORM_ACCOUNT_ID.to_string(),
            "debit".to_string(),
            receipt.platform_reversal_micros,
            "pending".to_string(),
            receipt.platform_pending_balance_after_micros,
            Some(receipt.platform_account_revision_after),
        ),
    ];
    if legs != expected {
        bail!("结算纠正账本腿或历史余额快照审计失败");
    }
    Ok(())
}

fn audit_pending_projection(
    conn: &Connection,
    account_kind: &str,
    account_id: &str,
    settlement_leg_kind: &str,
) -> Result<()> {
    let projected = conn
        .query_row(
            "SELECT pending_micros FROM compute_settlement_account_balances
              WHERE account_kind=?1 AND account_id=?2 AND currency='CNY'",
            params![account_kind, account_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .unwrap_or(0);
    let credited = conn.query_row(
        "SELECT COALESCE(SUM(amount_micros),0)
           FROM compute_settlement_ledger_legs
          WHERE account_id=?1 AND currency='CNY' AND balance_state='pending'
            AND direction='credit' AND leg_kind=?2",
        params![account_id, settlement_leg_kind],
        |row| row.get::<_, i64>(0),
    )?;
    let corrected = conn.query_row(
        "SELECT COALESCE(SUM(amount_micros),0)
           FROM compute_settlement_correction_ledger_legs
          WHERE account_kind=?1 AND account_id=?2 AND currency='CNY'
            AND balance_state='pending' AND direction='debit'",
        params![account_kind, account_id],
        |row| row.get::<_, i64>(0),
    )?;
    let released = conn.query_row(
        "SELECT COALESCE(SUM(amount_micros),0)
           FROM compute_settlement_release_ledger_legs
          WHERE account_kind=?1 AND account_id=?2 AND currency='CNY'
            AND balance_state='pending' AND direction='debit'",
        params![account_kind, account_id],
        |row| row.get::<_, i64>(0),
    )?;
    if projected != credited - corrected - released || projected < 0 {
        bail!("结算纠正 pending 余额与不可变账本投影不一致");
    }
    Ok(())
}

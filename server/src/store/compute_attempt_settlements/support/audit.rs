use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::execution::{JOB_STATUS_SETTLED, JOB_STATUS_VERIFICATION_PENDING};

use crate::store::{
    compute_attempt_execution_receipts::compute_attempt_execution_receipt_on,
    compute_attempt_finalizations::compute_attempt_finalization_on,
    compute_broker_reservation::broker_reserve_binding_on,
    compute_job_registry::{current_registered_job_on, registered_job_version_on},
    compute_offer_registry::registered_offer_version_on,
    compute_price_snapshot_registry::registered_price_snapshot_on,
    compute_provider_registry::registered_provider_version_on,
    compute_reservation_registry::registered_reservation_version_on,
};

use super::super::{
    calculation::{calculate_settlement, MICROS_PER_CNY_FEN},
    money::{posting_row_on, settlement_posting_digest},
    ComputeAttemptSettlementReceipt, COMPUTE_ATTEMPT_SETTLEMENT_SCHEMA,
};

use super::{
    attempt_settlement_event_digest, compute_settlement_receipt_digest,
    normalize_settlement_request, settlement_request_digest, StoredSettlement,
};

pub(super) fn audited_settlement_on(
    conn: &Connection,
    stored: &StoredSettlement,
    replayed: bool,
) -> Result<ComputeAttemptSettlementReceipt> {
    let request = normalize_settlement_request(&serde_json::from_str(&stored.request_json)?)?;
    let mut receipt: ComputeAttemptSettlementReceipt = serde_json::from_str(&stored.receipt_json)?;
    if request.lease_id != stored.lease_id
        || request.expected_finalization_id != stored.finalization_id
        || request.expected_finalization_event_digest != stored.finalization_event_digest
        || request.expected_execution_receipt_id != stored.execution_receipt_id
        || request.expected_execution_receipt_digest != stored.execution_receipt_digest
        || request.expected_budget_reservation_id != stored.budget_reservation_id
        || request.expected_price_snapshot_id != stored.price_snapshot_id
        || request.expected_price_snapshot_digest != stored.price_snapshot_digest
        || request.expected_job_revision != stored.source_job_revision
        || request.expected_job_digest != stored.source_job_digest
        || request.idempotency_key != stored.idempotency_key
        || request.settled_by_user_id != stored.settled_by_user_id
        || stored.idempotency_scope
            != format!("compute_attempt_settlement:{}", request.settled_by_user_id)
        || settlement_request_digest(&request)? != stored.request_digest
        || receipt.schema != COMPUTE_ATTEMPT_SETTLEMENT_SCHEMA
        || receipt.settlement.settlement_receipt_id != stored.settlement_receipt_id
        || receipt.lease_id != stored.lease_id
        || receipt.finalization_id != stored.finalization_id
        || receipt.finalization_event_digest != stored.finalization_event_digest
        || receipt.settlement.execution_receipt_id != stored.execution_receipt_id
        || receipt.settlement.execution_receipt_digest != stored.execution_receipt_digest
        || receipt.budget_reservation_id != stored.budget_reservation_id
        || receipt.settlement.price_snapshot_id != stored.price_snapshot_id
        || receipt.settlement.price_snapshot_digest != stored.price_snapshot_digest
        || receipt.source_job.job_id != stored.job_id
        || receipt.source_job.job_revision != stored.source_job_revision
        || receipt.source_job.job_digest != stored.source_job_digest
        || receipt.terminal_job.job_id != stored.job_id
        || receipt.terminal_job.job_revision != stored.terminal_job_revision
        || receipt.terminal_job.job_digest != stored.terminal_job_digest
        || receipt.request_digest != stored.request_digest
        || receipt.event_digest != stored.event_digest
        || receipt.settled_by_user_id != stored.settled_by_user_id
        || receipt.settled_at != stored.settled_at
        || receipt.replayed
    {
        bail!("Attempt 结算数据库列、请求或回执 JSON 不一致");
    }
    if compute_settlement_receipt_digest(&receipt.settlement)?
        != receipt.settlement.settlement_receipt_digest
        || attempt_settlement_event_digest(&receipt)? != stored.event_digest
    {
        bail!("Attempt 结算回执摘要审计失败");
    }

    let finalization = compute_attempt_finalization_on(conn, &stored.lease_id)?;
    let execution = compute_attempt_execution_receipt_on(conn, &stored.lease_id)?;
    if finalization.finalization_id != stored.finalization_id
        || finalization.event_digest != stored.finalization_event_digest
        || execution.receipt.receipt_id != stored.execution_receipt_id
        || execution.receipt.receipt_digest != stored.execution_receipt_digest
        || finalization.execution_receipt_id != execution.receipt.receipt_id
    {
        bail!("Attempt 结算上游可信终态或执行回执发生不一致");
    }
    let source_job = registered_job_version_on(conn, &stored.job_id, stored.source_job_revision)?
        .ok_or_else(|| anyhow!("Attempt 结算源 Job 历史版本不存在"))?;
    let terminal_job =
        registered_job_version_on(conn, &stored.job_id, stored.terminal_job_revision)?
            .ok_or_else(|| anyhow!("Attempt 结算目标 Job 历史版本不存在"))?;
    let current_job = current_registered_job_on(conn, &stored.job_id)?
        .ok_or_else(|| anyhow!("Attempt 结算当前 Job 不存在"))?;
    if source_job.job_digest != stored.source_job_digest
        || source_job.job.status != JOB_STATUS_VERIFICATION_PENDING
        || terminal_job.job_digest != stored.terminal_job_digest
        || terminal_job.job.status != JOB_STATUS_SETTLED
        || terminal_job.job.updated_at != stored.settled_at
        || current_job.revision != terminal_job.revision
        || current_job.job_digest != terminal_job.job_digest
    {
        bail!("Attempt 结算 Job 历史或当前投影审计失败");
    }
    let reservation = registered_reservation_version_on(
        conn,
        &execution.receipt.reservation_id,
        finalization.terminal_reservation.revision,
    )?
    .ok_or_else(|| anyhow!("Attempt 结算 Reservation 历史版本不存在"))?;
    let snapshot = registered_price_snapshot_on(conn, &stored.price_snapshot_id)?
        .ok_or_else(|| anyhow!("Attempt 结算价格快照不存在"))?;
    if reservation.reservation_digest != finalization.terminal_reservation.digest
        || reservation.reservation.price_snapshot != snapshot
        || snapshot.snapshot_digest != stored.price_snapshot_digest
    {
        bail!("Attempt 结算 Reservation 或价格快照审计失败");
    }
    let broker = broker_reserve_binding_on(
        conn,
        &reservation.reservation.reservation_id,
        &source_job.job.consumer_account_id,
    )?;
    if broker.budget_reservation_id != stored.budget_reservation_id {
        bail!("Attempt 结算预授权绑定审计失败");
    }
    let offer = registered_offer_version_on(conn, &snapshot.offer_id, snapshot.offer_version)?
        .ok_or_else(|| anyhow!("Attempt 结算 Offer 历史版本不存在"))?;
    let provider = registered_provider_version_on(
        conn,
        &offer.offer.provider_id,
        offer.provider_policy_revision,
    )?
    .ok_or_else(|| anyhow!("Attempt 结算 Provider 历史版本不存在"))?;
    let provider_account_id = provider
        .provider
        .settlement_account_id
        .as_deref()
        .unwrap_or(provider.provider.owner_account_id.as_str())
        .trim();
    let computed = calculate_settlement(&snapshot, &execution.receipt, broker.budget_reserved_fen)?;
    if provider.provider_digest != receipt.provider_digest
        || offer.provider_policy_revision != receipt.provider_policy_revision
        || receipt.settlement.consumer_account_id != source_job.job.consumer_account_id
        || receipt.settlement.provider_account_id != provider_account_id
        || receipt.settlement.currency != "CNY"
        || receipt.settlement.amounts != computed.amounts
        || receipt.settlement.verified_usage_digest != computed.verified_usage_digest
        || receipt.settlement.compensable_usage_digest != computed.compensable_usage_digest
        || receipt.settlement.reason_codes != computed.reason_codes
        || receipt.settlement.balance_state != "pending"
        || receipt.settlement.correction_of_receipt_id.is_some()
        || receipt.settlement.available_at.is_some()
        || receipt.settlement.created_at != stored.settled_at
        || receipt.settlement.ledger_posting_ref.as_deref() != Some(receipt.posting_id.as_str())
    {
        bail!("Attempt 结算价格、账户、用量或 Settlement Receipt 审计失败");
    }
    audit_billing(conn, &receipt, computed.consumer_charge_fen)?;
    audit_posting(conn, &receipt)?;
    if receipt.money_effect != "consumer_preauthorization_captured_and_unused_refunded"
        || receipt.provider_balance_effect != "provider_and_platform_credited_pending"
    {
        bail!("Attempt 结算资金效果字段无效");
    }
    receipt.replayed = replayed;
    Ok(receipt)
}

fn audit_billing(
    conn: &Connection,
    receipt: &ComputeAttemptSettlementReceipt,
    expected_charge_fen: i64,
) -> Result<()> {
    let row = conn
        .query_row(
            "SELECT user_id, reserved_fen, settled_cost_fen, refunded_fen,
                    status, balance_after_fen
               FROM billing_reservations WHERE id=?1",
            params![receipt.budget_reservation_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| anyhow!("Attempt 结算预授权记录不存在"))?;
    let charged_micros = row
        .2
        .checked_mul(MICROS_PER_CNY_FEN)
        .ok_or_else(|| anyhow!("Attempt 结算消费者扣结金额换算溢出"))?;
    let refunded_micros = row
        .3
        .checked_mul(MICROS_PER_CNY_FEN)
        .ok_or_else(|| anyhow!("Attempt 结算消费者退款金额换算溢出"))?;
    if row.0 != receipt.settlement.consumer_account_id
        || row.1 != receipt.budget_reserved_fen
        || row.2 != expected_charge_fen
        || row.2 != receipt.consumer_charged_fen
        || row.3 != receipt.consumer_refunded_fen
        || row.4 != "settled"
        || row.5 != Some(receipt.consumer_balance_after_fen)
        || receipt.settlement.amounts.consumer_charge_micros != charged_micros
        || receipt.settlement.amounts.refund_micros != refunded_micros
    {
        bail!("Attempt 结算消费者预授权结果审计失败");
    }
    Ok(())
}

fn audit_posting(conn: &Connection, receipt: &ComputeAttemptSettlementReceipt) -> Result<()> {
    let row = posting_row_on(conn, &receipt.posting_id)?
        .ok_or_else(|| anyhow!("Attempt 结算 posting 不存在"))?;
    let expected_digest = settlement_posting_digest(
        &receipt.posting_id,
        &receipt.settlement.settlement_receipt_id,
        receipt.settlement.amounts.consumer_charge_micros,
        receipt.settlement.amounts.refund_micros,
        receipt.settlement.amounts.provider_payable_micros,
        receipt.settlement.amounts.platform_margin_micros,
        &receipt.settled_at,
    )?;
    if row.0 != receipt.settlement.settlement_receipt_id
        || row.1 != receipt.settlement.amounts.consumer_charge_micros
        || row.2 != receipt.settlement.amounts.refund_micros
        || row.3 != receipt.settlement.amounts.provider_payable_micros
        || row.4 != receipt.settlement.amounts.platform_margin_micros
        || row.5 != receipt.posting_digest
        || row.5 != expected_digest
        || row.6 != receipt.settled_at
    {
        bail!("Attempt 结算 posting 摘要或金额审计失败");
    }
    let mut stmt = conn.prepare(
        "SELECT line_no, leg_kind, account_id, direction, amount_micros,
                balance_state, balance_after_micros
           FROM compute_settlement_ledger_legs
          WHERE posting_id=?1 ORDER BY line_no",
    )?;
    let legs = stmt
        .query_map(params![receipt.posting_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<i64>>(6)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if legs.len() != 4
        || legs[0]
            != (
                1,
                "consumer_capture".to_string(),
                receipt.settlement.consumer_account_id.clone(),
                "debit".to_string(),
                receipt.settlement.amounts.consumer_charge_micros,
                "preauthorization".to_string(),
                None,
            )
        || legs[1]
            != (
                2,
                "consumer_refund".to_string(),
                receipt.settlement.consumer_account_id.clone(),
                "release".to_string(),
                receipt.settlement.amounts.refund_micros,
                "preauthorization".to_string(),
                None,
            )
        || legs[2]
            != (
                3,
                "provider_pending".to_string(),
                receipt.settlement.provider_account_id.clone(),
                "credit".to_string(),
                receipt.settlement.amounts.provider_payable_micros,
                "pending".to_string(),
                Some(receipt.provider_pending_balance_micros),
            )
        || legs[3]
            != (
                4,
                "platform_pending".to_string(),
                "platform:compute_market".to_string(),
                "credit".to_string(),
                receipt.settlement.amounts.platform_margin_micros,
                "pending".to_string(),
                Some(receipt.platform_pending_balance_micros),
            )
    {
        bail!("Attempt 结算双价格腿或余额快照审计失败");
    }
    audit_pending_projection(
        conn,
        "provider",
        &receipt.settlement.provider_account_id,
        "provider_pending",
    )?;
    audit_pending_projection(
        conn,
        "platform",
        "platform:compute_market",
        "platform_pending",
    )?;
    Ok(())
}

fn audit_pending_projection(
    conn: &Connection,
    account_kind: &str,
    account_id: &str,
    leg_kind: &str,
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
        "SELECT COALESCE(SUM(CASE direction WHEN 'credit' THEN amount_micros
                                             WHEN 'debit' THEN -amount_micros ELSE 0 END),0)
           FROM compute_settlement_ledger_legs
          WHERE account_id=?1 AND currency='CNY' AND balance_state='pending' AND leg_kind=?2",
        params![account_id, leg_kind],
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
    let corrected = conn.query_row(
        "SELECT COALESCE(SUM(amount_micros),0)
           FROM compute_settlement_correction_ledger_legs
          WHERE account_kind=?1 AND account_id=?2 AND currency='CNY'
            AND balance_state='pending' AND direction='debit'",
        params![account_kind, account_id],
        |row| row.get::<_, i64>(0),
    )?;
    let rebuilt = credited - corrected - released;
    if projected != rebuilt || projected < 0 {
        bail!("Attempt 结算待结算余额投影与不可变账本不一致");
    }
    Ok(())
}

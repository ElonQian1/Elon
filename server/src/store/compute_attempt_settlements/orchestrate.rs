use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::Transaction;

use crate::compute_federation::{
    execution::{
        ComputeJobVersionBinding, JOB_STATUS_SETTLED, JOB_STATUS_VERIFICATION_PENDING,
        RESERVATION_STATUS_CONSUMED,
    },
    receipts::{
        ComputeSettlementReceipt, BALANCE_STATE_PENDING, COMPUTE_SETTLEMENT_RECEIPT_SCHEMA,
    },
};

use super::{
    super::{
        compute_attempt_execution_receipts::compute_attempt_execution_receipt_on,
        compute_attempt_finalizations::compute_attempt_finalization_on,
        compute_broker_reservation::broker_reserve_binding_on,
        compute_job_registry::{current_registered_job_on, register_compute_job_on},
        compute_offer_registry::registered_offer_version_on,
        compute_price_snapshot_registry::registered_price_snapshot_on,
        compute_provider_registry::registered_provider_version_on,
        compute_reservation_registry::registered_reservation_version_on,
        new_id,
    },
    calculation::calculate_settlement,
    money::{post_settlement_money_on, PostSettlementMoneyInput},
    support::{attempt_settlement_event_digest, compute_settlement_receipt_digest},
    ComputeAttemptSettlementReceipt, SettleComputeAttemptRequest,
    COMPUTE_ATTEMPT_SETTLEMENT_SCHEMA,
};

pub(super) fn settle_attempt_on(
    tx: &Transaction<'_>,
    request: &SettleComputeAttemptRequest,
    request_digest: &str,
    _idempotency_scope: &str,
) -> Result<ComputeAttemptSettlementReceipt> {
    let finalization = compute_attempt_finalization_on(tx, &request.lease_id)?;
    if finalization.finalization_id != request.expected_finalization_id
        || finalization.event_digest != request.expected_finalization_event_digest
        || finalization.execution_receipt_id != request.expected_execution_receipt_id
        || finalization.execution_receipt_digest != request.expected_execution_receipt_digest
        || finalization.money_effect != "preauthorization_unchanged"
        || finalization.settlement_effect != "pending"
    {
        bail!("Attempt 结算引用的可信终态或资金边界不匹配");
    }
    let execution = compute_attempt_execution_receipt_on(tx, &request.lease_id)?;
    if execution.receipt.receipt_id != request.expected_execution_receipt_id
        || execution.receipt.receipt_digest != request.expected_execution_receipt_digest
        || execution.receipt.job_id != finalization.terminal_job.job_id
    {
        bail!("Attempt 结算引用的 Execution Receipt 与可信终态不一致");
    }

    let source_job = current_registered_job_on(tx, &finalization.terminal_job.job_id)?
        .ok_or_else(|| anyhow!("Attempt 结算引用的当前 Job 不存在"))?;
    if source_job.revision != request.expected_job_revision
        || source_job.job_digest != request.expected_job_digest
        || source_job.revision != finalization.terminal_job.job_revision
        || source_job.job_digest != finalization.terminal_job.job_digest
        || source_job.job.status != JOB_STATUS_VERIFICATION_PENDING
    {
        bail!("Attempt 结算前 Job 已发生变化或不在 verification_pending");
    }
    let reservation = registered_reservation_version_on(
        tx,
        &execution.receipt.reservation_id,
        finalization.terminal_reservation.revision,
    )?
    .ok_or_else(|| anyhow!("Attempt 结算引用的 consumed Reservation 历史版本不存在"))?;
    if reservation.reservation_digest != finalization.terminal_reservation.digest
        || reservation.reservation.status != RESERVATION_STATUS_CONSUMED
        || reservation.reservation.job != finalization.terminal_job
        || reservation.reservation.price_snapshot.snapshot_id != request.expected_price_snapshot_id
        || reservation.reservation.price_snapshot.snapshot_digest
            != request.expected_price_snapshot_digest
    {
        bail!("Attempt 结算引用的 Reservation、Job 或价格快照不一致");
    }
    let snapshot = registered_price_snapshot_on(tx, &request.expected_price_snapshot_id)?
        .ok_or_else(|| anyhow!("Attempt 结算引用的价格快照不存在"))?;
    if snapshot.snapshot_digest != request.expected_price_snapshot_digest
        || snapshot != reservation.reservation.price_snapshot
    {
        bail!("Attempt 结算引用的价格快照历史审计失败");
    }
    let broker = broker_reserve_binding_on(
        tx,
        &reservation.reservation.reservation_id,
        &source_job.job.consumer_account_id,
    )?;
    if broker.budget_reservation_id != request.expected_budget_reservation_id
        || reservation.reservation.consumer_authorization_ref != broker.budget_reservation_id
    {
        bail!("Attempt 结算引用的消费者预授权不一致");
    }
    let offer = registered_offer_version_on(tx, &snapshot.offer_id, snapshot.offer_version)?
        .ok_or_else(|| anyhow!("Attempt 结算引用的 Offer 历史版本不存在"))?;
    if offer.offer.offer_digest != snapshot.offer_digest
        || offer.offer.provider_id != finalization.provider_id
    {
        bail!("Attempt 结算引用的 Offer 或 Provider 不一致");
    }
    let provider = registered_provider_version_on(
        tx,
        &offer.offer.provider_id,
        offer.provider_policy_revision,
    )?
    .ok_or_else(|| anyhow!("Attempt 结算引用的 Provider 历史版本不存在"))?;
    if provider.provider_digest != offer.provider_digest {
        bail!("Attempt 结算引用的 Provider 历史摘要不一致");
    }
    let provider_account_id = provider
        .provider
        .settlement_account_id
        .as_deref()
        .unwrap_or(provider.provider.owner_account_id.as_str())
        .trim()
        .to_string();
    if provider_account_id.is_empty() {
        bail!("Provider 没有可用的结算账户");
    }

    let computed = calculate_settlement(&snapshot, &execution.receipt, broker.budget_reserved_fen)?;
    let settled_at = settlement_time(&[
        finalization.finalized_at.as_str(),
        source_job.job.updated_at.as_str(),
        execution.receipt.created_at.as_str(),
    ])?;
    let settlement_receipt_id = new_id("compute_settlement_receipt");
    let money = post_settlement_money_on(
        tx,
        PostSettlementMoneyInput {
            settlement_receipt_id: &settlement_receipt_id,
            consumer_account_id: &source_job.job.consumer_account_id,
            provider_account_id: &provider_account_id,
            budget_reservation_id: &broker.budget_reservation_id,
            budget_reserved_fen: broker.budget_reserved_fen,
            consumer_charge_fen: computed.consumer_charge_fen,
            consumer_charge_micros: computed.amounts.consumer_charge_micros,
            provider_payable_micros: computed.amounts.provider_payable_micros,
            platform_margin_micros: computed.amounts.platform_margin_micros,
            consumer_refund_micros: computed.amounts.refund_micros,
            settled_at: &settled_at,
        },
    )?;

    let mut settled_job = source_job.job.clone();
    settled_job.status = JOB_STATUS_SETTLED.to_string();
    settled_job.updated_at = settled_at.clone();
    let terminal_job = register_compute_job_on(tx, &settled_job, source_job.revision)?;

    let mut settlement = ComputeSettlementReceipt {
        schema: COMPUTE_SETTLEMENT_RECEIPT_SCHEMA.to_string(),
        settlement_receipt_id,
        settlement_receipt_digest: String::new(),
        execution_receipt_id: execution.receipt.receipt_id,
        execution_receipt_digest: execution.receipt.receipt_digest,
        reservation_id: reservation.reservation.reservation_id,
        price_snapshot_id: snapshot.snapshot_id,
        price_snapshot_digest: snapshot.snapshot_digest,
        consumer_account_id: source_job.job.consumer_account_id,
        provider_account_id,
        currency: snapshot.currency,
        amounts: computed.amounts,
        verified_usage_digest: computed.verified_usage_digest,
        compensable_usage_digest: computed.compensable_usage_digest,
        balance_state: BALANCE_STATE_PENDING.to_string(),
        correction_of_receipt_id: None,
        ledger_posting_ref: Some(money.posting_id.clone()),
        reason_codes: computed.reason_codes,
        created_at: settled_at.clone(),
        available_at: None,
    };
    settlement.settlement_receipt_digest = compute_settlement_receipt_digest(&settlement)?;

    let mut receipt = ComputeAttemptSettlementReceipt {
        schema: COMPUTE_ATTEMPT_SETTLEMENT_SCHEMA.to_string(),
        settlement,
        lease_id: request.lease_id.clone(),
        finalization_id: finalization.finalization_id,
        finalization_event_digest: finalization.event_digest,
        budget_reservation_id: money.billing.reservation_id,
        budget_reserved_fen: money.billing.reserved_fen,
        consumer_charged_fen: money.billing.charged_fen,
        consumer_refunded_fen: money.billing.refunded_fen,
        consumer_balance_after_fen: money.billing.consumer_balance_after_fen,
        provider_policy_revision: offer.provider_policy_revision,
        provider_digest: offer.provider_digest,
        source_job: ComputeJobVersionBinding {
            job_id: source_job.job.job_id,
            job_revision: source_job.revision,
            job_digest: source_job.job_digest,
        },
        terminal_job: ComputeJobVersionBinding {
            job_id: terminal_job.job.job_id,
            job_revision: terminal_job.revision,
            job_digest: terminal_job.job_digest,
        },
        posting_id: money.posting_id,
        posting_digest: money.posting_digest,
        provider_pending_balance_micros: money.provider_pending_balance_micros,
        platform_pending_balance_micros: money.platform_pending_balance_micros,
        request_digest: request_digest.to_string(),
        event_digest: String::new(),
        settled_by_user_id: request.settled_by_user_id.clone(),
        settled_at,
        money_effect: "consumer_preauthorization_captured_and_unused_refunded".to_string(),
        provider_balance_effect: "provider_and_platform_credited_pending".to_string(),
        replayed: false,
    };
    receipt.event_digest = attempt_settlement_event_digest(&receipt)?;
    Ok(receipt)
}

fn settlement_time(values: &[&str]) -> Result<String> {
    let mut floor = Utc::now();
    for value in values {
        let parsed = DateTime::parse_from_rfc3339(value)
            .with_context(|| format!("结算前序时间无效: {value}"))?
            .with_timezone(&Utc);
        floor = std::cmp::max(floor, parsed);
    }
    floor
        .checked_add_signed(Duration::microseconds(1))
        .context("Attempt 结算时间超出范围")
        .map(|value| value.to_rfc3339())
}

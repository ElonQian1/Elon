use anyhow::{anyhow, bail, Context, Result};
use rusqlite::Connection;

use crate::compute_federation::execution::{
    ComputeJobVersionBinding, JOB_STATUS_VERIFICATION_PENDING, RESERVATION_STATUS_CONSUMED,
};

use super::{
    super::{
        compute_attempt_execution_receipts::compute_attempt_execution_receipt_on,
        compute_attempt_finalizations::compute_attempt_finalization_on,
        compute_broker_reservation::broker_reserve_binding_on,
        compute_job_registry::current_registered_job_on,
        compute_offer_registry::registered_offer_version_on,
        compute_price_snapshot_registry::registered_price_snapshot_on,
        compute_provider_registry::registered_provider_version_on,
        compute_reservation_registry::registered_reservation_version_on,
    },
    calculation::calculate_settlement,
    ComputePendingAttemptSettlementCandidate, ComputePendingAttemptSettlementPreview,
};

pub(super) fn build_pending_settlement_candidate_on(
    conn: &Connection,
    lease_id: &str,
) -> Result<ComputePendingAttemptSettlementCandidate> {
    let finalization = compute_attempt_finalization_on(conn, lease_id)?;
    if finalization.money_effect != "preauthorization_unchanged"
        || finalization.settlement_effect != "pending"
    {
        bail!("待结算可信终态的资金边界不匹配");
    }
    let execution = compute_attempt_execution_receipt_on(conn, lease_id)?;
    if execution.receipt.receipt_id != finalization.execution_receipt_id
        || execution.receipt.receipt_digest != finalization.execution_receipt_digest
        || execution.receipt.job_id != finalization.terminal_job.job_id
    {
        bail!("待结算 Execution Receipt 与可信终态不一致");
    }

    let job = current_registered_job_on(conn, &finalization.terminal_job.job_id)?
        .ok_or_else(|| anyhow!("待结算队列引用的当前 Job 不存在"))?;
    if job.revision != finalization.terminal_job.job_revision
        || job.job_digest != finalization.terminal_job.job_digest
        || job.job.status != JOB_STATUS_VERIFICATION_PENDING
    {
        bail!("待结算 Job 已漂移或不再处于 verification_pending");
    }

    let reservation = registered_reservation_version_on(
        conn,
        &execution.receipt.reservation_id,
        finalization.terminal_reservation.revision,
    )?
    .ok_or_else(|| anyhow!("待结算队列引用的 consumed Reservation 不存在"))?;
    if reservation.reservation_digest != finalization.terminal_reservation.digest
        || reservation.reservation.status != RESERVATION_STATUS_CONSUMED
        || reservation.reservation.job != finalization.terminal_job
    {
        bail!("待结算 Reservation 与可信终态不一致");
    }

    let snapshot =
        registered_price_snapshot_on(conn, &reservation.reservation.price_snapshot.snapshot_id)?
            .ok_or_else(|| anyhow!("待结算队列引用的价格快照不存在"))?;
    if snapshot != reservation.reservation.price_snapshot {
        bail!("待结算价格快照历史审计失败");
    }
    let broker = broker_reserve_binding_on(
        conn,
        &reservation.reservation.reservation_id,
        &job.job.consumer_account_id,
    )?;
    if reservation.reservation.consumer_authorization_ref != broker.budget_reservation_id {
        bail!("待结算消费者预授权与 Reservation 不一致");
    }

    let offer = registered_offer_version_on(conn, &snapshot.offer_id, snapshot.offer_version)?
        .ok_or_else(|| anyhow!("待结算队列引用的 Offer 历史版本不存在"))?;
    if offer.offer.offer_digest != snapshot.offer_digest
        || offer.offer.provider_id != finalization.provider_id
    {
        bail!("待结算 Offer 或 Provider 与可信终态不一致");
    }
    let provider = registered_provider_version_on(
        conn,
        &offer.offer.provider_id,
        offer.provider_policy_revision,
    )?
    .ok_or_else(|| anyhow!("待结算队列引用的 Provider 历史版本不存在"))?;
    if provider.provider_digest != offer.provider_digest {
        bail!("待结算 Provider 历史摘要不一致");
    }
    let provider_account_id = provider
        .provider
        .settlement_account_id
        .as_deref()
        .unwrap_or(provider.provider.owner_account_id.as_str())
        .trim()
        .to_string();
    if provider_account_id.is_empty() {
        bail!("待结算 Provider 没有可用的结算账户");
    }

    let computed = calculate_settlement(&snapshot, &execution.receipt, broker.budget_reserved_fen)?;
    let consumer_refund_fen = broker
        .budget_reserved_fen
        .checked_sub(computed.consumer_charge_fen)
        .context("待结算消费者退款金额下溢")?;

    Ok(ComputePendingAttemptSettlementCandidate {
        finalization,
        execution_receipt: execution,
        expected_job: ComputeJobVersionBinding {
            job_id: job.job.job_id,
            job_revision: job.revision,
            job_digest: job.job_digest,
        },
        expected_budget_reservation_id: broker.budget_reservation_id,
        price_snapshot: snapshot,
        provider_account_id,
        preview: ComputePendingAttemptSettlementPreview {
            currency: "CNY",
            budget_reserved_fen: broker.budget_reserved_fen,
            consumer_charge_fen: computed.consumer_charge_fen,
            consumer_refund_fen,
            amounts: computed.amounts,
            reason_codes: computed.reason_codes,
        },
        money_effect: "consumer_preauthorization_captured_and_unused_refunded",
        provider_balance_effect: "provider_and_platform_credited_pending",
        external_payment_effect: "none",
    })
}

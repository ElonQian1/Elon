use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Result};
use rusqlite::Transaction;

use crate::compute_federation::{
    capacity::{
        ComputeCapacityClaimBinding, ComputeCapacityClaimKind, ComputeCapacityOfferBinding,
    },
    execution::{
        ComputeJobVersionBinding, ComputeReservation, COMPUTE_RESERVATION_SCHEMA,
        JOB_STATUS_QUOTED, JOB_STATUS_RESERVED, RESERVATION_STATUS_ACTIVE,
        RESERVATION_STATUS_PENDING,
    },
    offer::OFFER_STATUS_ACTIVE,
};

use super::super::{
    billing_reservations::{reserve_billing_call_until_on, BillingReservationRequest},
    compute_capacity_claim_rows::stored_claim_on,
    compute_capacity_claims::{
        hold_compute_capacity_claim_on, HoldComputeCapacityClaim, HoldComputeCapacityClaimLine,
    },
    compute_capacity_posting::reservation_capacity_causal_binding,
    compute_job_registry::{current_registered_job_on, register_compute_job_on},
    compute_offer_registry::current_registered_offer_on,
    compute_price_snapshot_registry::registered_price_snapshot_on,
    compute_reservation_registry::register_compute_reservation_on,
    now,
};
use super::{
    receipt::persist_broker_reserve_receipt_on,
    validation::{
        broker_compute_call_id, cny_micros_to_fen, timestamp_after, NormalizedBrokerReserveRequest,
    },
    ComputeBrokerReservationReceipt, BROKER_BILLING_FEATURE, BROKER_BILLING_USAGE_MODE,
};

pub(super) fn reserve_new_broker_contract_on(
    conn: &Transaction<'_>,
    request: &NormalizedBrokerReserveRequest,
) -> Result<ComputeBrokerReservationReceipt> {
    let source_job = current_registered_job_on(conn, &request.job_id)?
        .ok_or_else(|| anyhow!("Broker 绑定的算力 Job 不存在"))?;
    ensure_source_job_matches(request, &source_job)?;
    let selected_offer = source_job
        .job
        .selected_offer
        .as_ref()
        .ok_or_else(|| anyhow!("Broker 只能预留已选择 Offer 的 quoted Job"))?;
    let offer = current_registered_offer_on(conn, &selected_offer.offer_id)?
        .ok_or_else(|| anyhow!("Broker 绑定的当前 Offer 不存在"))?;
    if offer.offer.offer_version != selected_offer.offer_version
        || offer.offer.offer_digest != selected_offer.offer_digest
        || offer.offer.status != OFFER_STATUS_ACTIVE
    {
        bail!("Broker 只能预留当前 active Offer 的精确版本");
    }
    let snapshot_id = source_job
        .job
        .price_snapshot_id
        .as_deref()
        .ok_or_else(|| anyhow!("Broker 绑定的 quoted Job 缺少 Price Snapshot"))?;
    let snapshot = registered_price_snapshot_on(conn, snapshot_id)?
        .ok_or_else(|| anyhow!("Broker 绑定的 Price Snapshot 不存在"))?;
    ensure_platform_cny_contract(&source_job.job.currency, &snapshot.currency)?;

    let reserve_fen = cny_micros_to_fen(snapshot.consumer_max_amount_micros)?;
    let compute_call_id = broker_compute_call_id(&request.reservation_id);
    let model = source_job
        .job
        .workload
        .model
        .as_ref()
        .map(|value| value.model_id.as_str());
    let budget = reserve_billing_call_until_on(
        conn,
        &BillingReservationRequest {
            user_id: &request.consumer_account_id,
            compute_call_id: &compute_call_id,
            feature: BROKER_BILLING_FEATURE,
            usage_mode: BROKER_BILLING_USAGE_MODE,
            model,
            reserve_fen,
            bill_missing_balance: true,
        },
        &request.expires_at,
    )?;
    if budget.status != "reserved" {
        bail!("Broker 平台人民币余额预授权未进入 reserved 状态");
    }

    let claim_lines = claim_lines_for_offer(&offer.offer, request)?;
    let held = hold_compute_capacity_claim_on(
        conn,
        HoldComputeCapacityClaim {
            pool: offer.offer.capacity_pool.clone(),
            delivery_window: snapshot.delivery_window.binding.clone(),
            claim_kind: ComputeCapacityClaimKind::Reservation,
            subject_kind: "compute_reservation".to_string(),
            subject_id: request.reservation_id.clone(),
            idempotency_scope: format!("compute_broker_reserve:{}", request.consumer_account_id),
            idempotency_key: request.idempotency_key.clone(),
            lines: claim_lines,
            expires_at: Some(request.expires_at.clone()),
            occurred_at: now(),
            causal_binding: reservation_capacity_causal_binding(
                ComputeCapacityOfferBinding {
                    offer_id: offer.offer.offer_id.clone(),
                    offer_version: offer.offer.offer_version,
                    offer_digest: offer.offer.offer_digest.clone(),
                },
                &source_job.job.job_id,
                &request.reservation_id,
            )?,
        },
    )?;
    let claim = stored_claim_on(conn, &held.claim_id)?
        .ok_or_else(|| anyhow!("Broker 已持有的 Capacity Claim 无法读取"))?;
    let reservation_expires_at = claim
        .expires_at
        .clone()
        .ok_or_else(|| anyhow!("Broker Capacity Claim 缺少到期时间"))?;
    let pending = ComputeReservation {
        schema: COMPUTE_RESERVATION_SCHEMA.to_string(),
        reservation_id: request.reservation_id.clone(),
        job: ComputeJobVersionBinding {
            job_id: source_job.job.job_id.clone(),
            job_revision: source_job.revision,
            job_digest: source_job.job_digest.clone(),
        },
        idempotency_key: request.idempotency_key.clone(),
        offer: selected_offer.clone(),
        price_snapshot: snapshot,
        capacity_claim: ComputeCapacityClaimBinding {
            claim_id: claim.claim_id.clone(),
            claim_revision: claim.revision,
            claim_digest: claim.claim_digest.clone(),
        },
        reserved_capacity: request.reserved_capacity.clone(),
        consumer_authorization_ref: budget.reservation_id.clone(),
        status: RESERVATION_STATUS_PENDING.to_string(),
        created_at: claim.created_at.clone(),
        updated_at: claim.created_at.clone(),
        expires_at: reservation_expires_at,
        consumed_at: None,
        released_at: None,
    };
    let pending_receipt = register_compute_reservation_on(conn, &pending, 0)?;

    let active_at = timestamp_after(&pending.created_at)?;
    let mut reserved_job = source_job.job.clone();
    reserved_job.status = JOB_STATUS_RESERVED.to_string();
    reserved_job.updated_at = active_at.clone();
    let reserved_job_receipt = register_compute_job_on(conn, &reserved_job, source_job.revision)?;

    let mut active = pending;
    active.status = RESERVATION_STATUS_ACTIVE.to_string();
    active.updated_at = active_at;
    active.job = ComputeJobVersionBinding {
        job_id: reserved_job_receipt.job.job_id.clone(),
        job_revision: reserved_job_receipt.revision,
        job_digest: reserved_job_receipt.job_digest.clone(),
    };
    let active_receipt = register_compute_reservation_on(conn, &active, pending_receipt.revision)?;
    persist_broker_reserve_receipt_on(
        conn,
        request,
        &budget,
        &claim,
        &source_job,
        &reserved_job_receipt,
        &active_receipt,
    )
}

fn ensure_source_job_matches(
    request: &NormalizedBrokerReserveRequest,
    source_job: &super::super::compute_job_registry::ComputeJobRegistrationReceipt,
) -> Result<()> {
    if source_job.job.consumer_account_id != request.consumer_account_id
        || source_job.job.status != JOB_STATUS_QUOTED
        || source_job.revision != request.expected_job_revision
        || source_job.job_digest != request.expected_job_digest
    {
        bail!("Broker 只能基于消费者自己的当前 quoted Job 精确版本预留");
    }
    Ok(())
}

fn ensure_platform_cny_contract(job_currency: &str, snapshot_currency: &str) -> Result<()> {
    if job_currency != "CNY" || snapshot_currency != "CNY" {
        bail!("Broker 首版只支持 platform_balance_cny 人民币余额适配器");
    }
    Ok(())
}

fn claim_lines_for_offer(
    offer: &crate::compute_federation::offer::ComputeOffer,
    request: &NormalizedBrokerReserveRequest,
) -> Result<Vec<HoldComputeCapacityClaimLine>> {
    let by_meter = offer
        .capacity
        .iter()
        .map(|capacity| (capacity.bucket.meter.as_str(), capacity))
        .collect::<BTreeMap<_, _>>();
    if by_meter.len() != request.reserved_capacity.len() {
        bail!("Broker 预留容量必须覆盖 Offer 的全部 meter");
    }
    request
        .reserved_capacity
        .iter()
        .map(|reserved| {
            let capacity = by_meter
                .get(reserved.meter.as_str())
                .ok_or_else(|| anyhow!("Broker 容量 meter 不在当前 Offer 中"))?;
            Ok(HoldComputeCapacityClaimLine {
                bucket_id: capacity.bucket.bucket_id.clone(),
                quantity_units: reserved.quantity,
            })
        })
        .collect()
}

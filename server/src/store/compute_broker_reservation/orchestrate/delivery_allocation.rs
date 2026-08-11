use anyhow::{anyhow, bail, Context, Result};
use chrono::DateTime;
use rusqlite::Transaction;

use crate::compute_federation::{
    capacity::{ComputeCapacityClaimKind, ComputeCapacityClaimState},
    market::{ComputePriceSnapshot, PRICING_MODE_CAPACITY_FUTURE},
};
use crate::store::{
    billing_reservations::{
        reserve_billing_call_until_on, BillingReservationOutcome, BillingReservationRequest,
    },
    compute_capacity_claim_rows::stored_claim_on,
    compute_delivery_allocations::{
        DeliveryAllocationClaimTransferAuthority, DeliveryAllocationReservationAuthority,
    },
    compute_job_registry::{current_registered_job_on, ComputeJobRegistrationReceipt},
    compute_offer_registry::registered_offer_version_on,
    compute_price_snapshot_registry::registered_price_snapshot_on,
};

use super::{
    ensure_platform_cny_contract, ensure_source_job_matches, register_broker_contract_with_claim_on,
};
use crate::store::compute_broker_reservation::{
    receipt::replay_broker_reserve_on,
    validation::{
        broker_compute_call_id, cny_micros_to_fen, ensure_future_expiry,
        normalize_broker_reserve_request, NormalizedBrokerReserveRequest,
    },
    ComputeBrokerReservationReceipt, ReserveComputeBrokerRequest, BROKER_BILLING_FEATURE,
    BROKER_BILLING_USAGE_MODE,
};

pub(in crate::store) struct PreparedDeliveryAllocationBrokerReserve {
    request: NormalizedBrokerReserveRequest,
    source_job: ComputeJobRegistrationReceipt,
    snapshot: ComputePriceSnapshot,
    budget: BillingReservationOutcome,
}

pub(in crate::store) fn prepare_delivery_allocation_broker_budget_on(
    tx: &Transaction<'_>,
    request: &ReserveComputeBrokerRequest,
    authority: &DeliveryAllocationClaimTransferAuthority,
) -> Result<PreparedDeliveryAllocationBrokerReserve> {
    let request = normalize_broker_reserve_request(request)?;
    ensure_future_expiry(&request.expires_at)?;
    ensure_request_matches_transfer_authority(&request, authority)?;
    if replay_broker_reserve_on(tx, &request)?.is_some() {
        bail!("Delivery Allocation fresh exercise found an existing Broker receipt");
    }

    let source_job = current_registered_job_on(tx, &request.job_id)?
        .ok_or_else(|| anyhow!("Delivery Allocation Broker source Job is missing"))?;
    ensure_source_job_matches(&request, &source_job)?;
    let authorized_job = authority.source_job();
    if source_job.revision != authorized_job.revision
        || source_job.job_digest != authorized_job.job_digest
        || source_job.job != authorized_job.job
    {
        bail!("Delivery Allocation Broker source Job no longer matches its sealed authority");
    }
    let selected_offer = source_job
        .job
        .selected_offer
        .as_ref()
        .ok_or_else(|| anyhow!("Delivery Allocation Broker Job lacks an exact Offer"))?;
    if selected_offer != authority.offer_binding() {
        bail!("Delivery Allocation Broker Job and Commitment Offer differ");
    }
    let offer =
        registered_offer_version_on(tx, &selected_offer.offer_id, selected_offer.offer_version)?
            .ok_or_else(|| anyhow!("Delivery Allocation historical Offer is missing"))?;
    if offer.offer.offer_digest != selected_offer.offer_digest
        || offer.offer.capacity_pool != *authority.pool_binding()
    {
        bail!("Delivery Allocation historical Offer binding failed exact audit");
    }
    let snapshot_id = source_job
        .job
        .price_snapshot_id
        .as_deref()
        .ok_or_else(|| anyhow!("Delivery Allocation Broker Job lacks Price Snapshot"))?;
    let snapshot = registered_price_snapshot_on(tx, snapshot_id)?
        .ok_or_else(|| anyhow!("Delivery Allocation historical Price Snapshot is missing"))?;
    let commitment = authority.commitment();
    if snapshot.snapshot_id != authority.snapshot_id()
        || snapshot.snapshot_digest != authority.snapshot_digest()
        || snapshot.offer_id != selected_offer.offer_id
        || snapshot.offer_version != selected_offer.offer_version
        || snapshot.offer_digest != selected_offer.offer_digest
        || snapshot.delivery_window.binding != *authority.delivery_window()
        || snapshot.pricing_mode != PRICING_MODE_CAPACITY_FUTURE
        || snapshot.instrument_id.as_deref() != Some(commitment.instrument_id.as_str())
    {
        bail!("Delivery Allocation Broker Snapshot does not match the frozen future contract");
    }
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
        tx,
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
    )
    .context("Delivery Allocation Broker budget preauthorization failed")?;
    if budget.status != "reserved" || budget.balance_after_fen.is_none() || budget.deduplicated {
        bail!("Delivery Allocation fresh exercise did not create a new reserved budget");
    }
    Ok(PreparedDeliveryAllocationBrokerReserve {
        request,
        source_job,
        snapshot,
        budget,
    })
}

pub(in crate::store) fn reserve_compute_job_with_preheld_claim_on(
    tx: &Transaction<'_>,
    prepared: PreparedDeliveryAllocationBrokerReserve,
    authority: &DeliveryAllocationReservationAuthority,
) -> Result<ComputeBrokerReservationReceipt> {
    ensure_request_matches_transfer_authority(&prepared.request, authority.transfer())?;
    let child = authority.child_claim();
    let child_expiry_matches = match child.expires_at.as_deref() {
        Some(value) => same_utc_instant(
            "Delivery Allocation child Claim expiry",
            value,
            &prepared.request.expires_at,
        )?,
        None => false,
    };
    let current_child = stored_claim_on(tx, &child.claim_id)?
        .ok_or_else(|| anyhow!("Delivery Allocation pre-held Reservation Claim is missing"))?;
    if current_child != *child
        || child.claim_kind != ComputeCapacityClaimKind::Reservation
        || child.state != ComputeCapacityClaimState::Held
        || child.revision != 1
        || child.subject_kind != "compute_reservation"
        || child.subject_id != prepared.request.reservation_id
        || child.idempotency_key != prepared.request.idempotency_key
        || child.parent_claim_id.as_deref() != Some(authority.parent_claim().claim_id.as_str())
        || !child_expiry_matches
        || authority.source_job_binding().job_revision != prepared.source_job.revision
        || authority.source_job_binding().job_digest != prepared.source_job.job_digest
    {
        bail!("Delivery Allocation Broker cannot adopt a non-exact pre-held child Claim");
    }
    let reservation_expires_at = child
        .expires_at
        .as_deref()
        .ok_or_else(|| anyhow!("Delivery Allocation child Claim lacks expires_at"))?;
    register_broker_contract_with_claim_on(
        tx,
        &prepared.request,
        &prepared.budget,
        child,
        &prepared.source_job,
        prepared.snapshot,
        reservation_expires_at,
        Some(authority),
    )
}

fn ensure_request_matches_transfer_authority(
    request: &NormalizedBrokerReserveRequest,
    authority: &DeliveryAllocationClaimTransferAuthority,
) -> Result<()> {
    let source = authority.source_job();
    let expiry_matches = same_utc_instant(
        "Delivery Allocation Broker expiry",
        &request.expires_at,
        authority.reservation_expires_at(),
    )?;
    if request.reservation_id != authority.reservation_id()
        || request.consumer_account_id != authority.consumer_account_id()
        || request.idempotency_key != authority.reservation_idempotency_key()
        || request.job_id != authority.job_id()
        || request.expected_job_revision != source.revision
        || request.expected_job_digest != source.job_digest
        || request.reserved_capacity != authority.reserved_capacity()
        || !expiry_matches
    {
        bail!("Delivery Allocation Broker request differs from its sealed exercise authority");
    }
    Ok(())
}

fn same_utc_instant(label: &str, left: &str, right: &str) -> Result<bool> {
    let left = DateTime::parse_from_rfc3339(left)
        .with_context(|| format!("{label} left value is not RFC3339"))?;
    let right = DateTime::parse_from_rfc3339(right)
        .with_context(|| format!("{label} right value is not RFC3339"))?;
    if left.offset().local_minus_utc() != 0 || right.offset().local_minus_utc() != 0 {
        bail!("{label} must use UTC")
    }
    Ok(left == right)
}

use anyhow::{bail, Result};

use crate::compute_federation::{
    capacity::{ComputeCapacityClaim, ComputeCapacityClaimState},
    execution::{ComputeJob, ComputeReservation},
    market::ComputePriceSnapshot,
    offer::ComputeOffer,
};
use crate::store::compute_delivery_allocations::DeliveryAllocationReservationAuthority;

use super::parse_utc;

pub(super) fn validate_delivery_allocation_authority(
    reservation: &ComputeReservation,
    job: &ComputeJob,
    offer: &ComputeOffer,
    snapshot: &ComputePriceSnapshot,
    claim: &ComputeCapacityClaim,
    authority: &DeliveryAllocationReservationAuthority,
) -> Result<()> {
    let transfer = authority.transfer();
    let grant = transfer.grant();
    let parent = authority.parent_claim();
    let parent_result = authority.parent_result_claim();
    let child = authority.child_claim();
    let source_job = authority.source_job_binding();
    let grant_created = parse_utc("Delivery Allocation Grant 创建时间", &grant.created_at)?;
    let snapshot_expires = parse_utc("Price Snapshot 失效时间", &snapshot.expires_at)?;
    let window_start = parse_utc(
        "Price Snapshot 窗口开始时间",
        &snapshot.delivery_window.starts_at_utc,
    )?;
    let exercise_at = parse_utc(
        "Delivery Allocation 行权时间",
        authority.exercise_occurred_at(),
    )?;
    let expected_parent_result_revision = parent
        .revision
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("Delivery Allocation parent revision overflow"))?;
    if authority.reservation_id() != reservation.reservation_id
        || authority.consumer_account_id() != job.consumer_account_id
        || authority.job_id() != job.job_id
        || source_job.job_id != job.job_id
        || grant.job != source_job
        || authority.snapshot_id() != snapshot.snapshot_id
        || authority.snapshot_digest() != snapshot.snapshot_digest
        || authority.offer_binding() != &reservation.offer
        || authority.offer_binding().offer_id != offer.offer_id
        || authority.offer_binding().offer_version != offer.offer_version
        || authority.offer_binding().offer_digest != offer.offer_digest
        || authority.pool_binding() != &claim.pool
        || authority.delivery_window() != &claim.delivery_window
        || authority.reservation_expires_at() != reservation.expires_at
        || reservation.expires_at != job.workload.deadline_at
        || !same_claim_lineage(child, claim)
        || child.parent_claim_id.as_deref() != Some(parent.claim_id.as_str())
        || child.lines != parent.lines
        || parent_result.claim_id != parent.claim_id
        || parent_result.revision != expected_parent_result_revision
        || parent_result.state != ComputeCapacityClaimState::Released
        || parent_result.lines != parent.lines
        || grant_created >= snapshot_expires
        || exercise_at >= window_start
    {
        bail!("Reservation 的 Delivery Allocation 私有授权绑定不一致");
    }
    Ok(())
}

fn same_claim_lineage(initial: &ComputeCapacityClaim, version: &ComputeCapacityClaim) -> bool {
    version.claim_id == initial.claim_id
        && version.revision >= initial.revision
        && version.pool == initial.pool
        && version.delivery_window == initial.delivery_window
        && version.claim_kind == initial.claim_kind
        && version.parent_claim_id == initial.parent_claim_id
        && version.subject_kind == initial.subject_kind
        && version.subject_id == initial.subject_id
        && version.idempotency_scope == initial.idempotency_scope
        && version.idempotency_key == initial.idempotency_key
        && version.request_digest == initial.request_digest
        && version.lines == initial.lines
        && version.created_at == initial.created_at
        && version.expires_at == initial.expires_at
}

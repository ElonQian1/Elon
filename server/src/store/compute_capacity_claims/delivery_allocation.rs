use anyhow::{anyhow, bail, Result};
use rusqlite::Transaction;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_federation::capacity::{ComputeCapacityClaimKind, ComputeCapacityClaimState};

use super::{
    hold_compute_capacity_claim_with_lineage_on, HoldComputeCapacityClaim,
    HoldComputeCapacityClaimLine, HoldComputeCapacityClaimLineage, HoldComputeCapacityClaimReceipt,
};
use crate::store::{
    compute_capacity_claim_rows::stored_claim_on,
    compute_capacity_claim_transitions::FinishComputeCapacityClaimReceipt,
    compute_capacity_posting::reservation_capacity_causal_binding,
    compute_capacity_request_digest::hold_claim_request_digest,
    compute_delivery_allocations::DeliveryAllocationClaimTransferAuthority,
};

const PARENTED_HOLD_REQUEST_SCHEMA: &str =
    "compute_federation.delivery_allocation_parented_reservation_hold_request.v1";

#[derive(Serialize)]
struct ParentedHoldRequestDigest<'a> {
    schema: &'static str,
    base_hold_request_digest: &'a str,
    parent_claim_id: &'a str,
    parent_claim_revision: i64,
    parent_claim_digest: &'a str,
    parent_release_transaction_id: &'a str,
    parent_release_transaction_digest: &'a str,
    parent_release_ledger_sequence: i64,
    reservation_id: &'a str,
    job_id: &'a str,
    offer_id: &'a str,
    offer_version: i64,
    offer_digest: &'a str,
}

pub(in crate::store) fn hold_parented_delivery_reservation_claim_on(
    tx: &Transaction<'_>,
    authority: &DeliveryAllocationClaimTransferAuthority,
    parent_release: &FinishComputeCapacityClaimReceipt,
) -> Result<HoldComputeCapacityClaimReceipt> {
    let parent = authority.parent_claim();
    ensure_exact_parent_release_on(tx, parent, parent_release)?;

    let offer = authority.offer_binding();
    let reservation_id = authority.reservation_id();
    let job_id = authority.job_id();
    let input = HoldComputeCapacityClaim {
        pool: parent.pool.clone(),
        delivery_window: parent.delivery_window.clone(),
        claim_kind: ComputeCapacityClaimKind::Reservation,
        subject_kind: "compute_reservation".to_string(),
        subject_id: reservation_id.to_string(),
        idempotency_scope: authority.child_hold_idempotency_scope().to_string(),
        idempotency_key: authority.child_hold_idempotency_key().to_string(),
        lines: parent
            .lines
            .iter()
            .map(|line| HoldComputeCapacityClaimLine {
                bucket_id: line.bucket.bucket_id.clone(),
                quantity_units: line.quantity_units,
            })
            .collect(),
        expires_at: Some(authority.reservation_expires_at().to_string()),
        occurred_at: authority.exercise_occurred_at().to_string(),
        causal_binding: reservation_capacity_causal_binding(offer.clone(), job_id, reservation_id)?,
    };
    let base_hold_request_digest = hold_claim_request_digest(&input)?;
    let request_digest = parented_hold_request_digest(
        &base_hold_request_digest,
        parent,
        parent_release,
        reservation_id,
        job_id,
        offer,
    )?;
    let held = hold_compute_capacity_claim_with_lineage_on(
        tx,
        input,
        Some(HoldComputeCapacityClaimLineage {
            parent_claim_id: parent.claim_id.clone(),
            causal_transaction_id: parent_release.ledger.transaction_id.clone(),
            request_digest,
        }),
    )?;
    if held.replayed
        || held.claim_kind != "reservation"
        || held.state != "held"
        || held.revision != 1
        || held.ledger.event_kind != "reservation_held"
        || held.ledger.replayed
    {
        bail!("Delivery Allocation child Hold returned an invalid or replayed result");
    }
    Ok(held)
}

fn ensure_exact_parent_release_on(
    tx: &Transaction<'_>,
    parent: &crate::compute_federation::capacity::ComputeCapacityClaim,
    release: &FinishComputeCapacityClaimReceipt,
) -> Result<()> {
    let current = stored_claim_on(tx, &parent.claim_id)?
        .ok_or_else(|| anyhow!("Delivery Allocation parent Claim is missing after release"))?;
    let expected_revision = parent
        .revision
        .checked_add(1)
        .ok_or_else(|| anyhow!("Delivery Allocation parent Claim revision overflow"))?;
    if parent.claim_kind != ComputeCapacityClaimKind::CapacityCommitment
        || parent.state != ComputeCapacityClaimState::Held
        || parent.parent_claim_id.is_some()
        || release.replayed
        || release.claim_id != parent.claim_id
        || release.revision != expected_revision
        || release.state != "released"
        || release.ledger.replayed
        || release.ledger.event_kind != "reservation_released"
        || current.claim_id != release.claim_id
        || current.revision != release.revision
        || current.claim_digest != release.claim_digest
        || current.state != ComputeCapacityClaimState::Released
        || current.parent_claim_id.is_some()
        || current.lines != parent.lines
        || current.pool != parent.pool
        || current.delivery_window != parent.delivery_window
    {
        bail!("Delivery Allocation parent release is not the exact Commitment Claim transition");
    }
    Ok(())
}

fn parented_hold_request_digest(
    base_hold_request_digest: &str,
    parent: &crate::compute_federation::capacity::ComputeCapacityClaim,
    release: &FinishComputeCapacityClaimReceipt,
    reservation_id: &str,
    job_id: &str,
    offer: &crate::compute_federation::capacity::ComputeCapacityOfferBinding,
) -> Result<String> {
    let payload = ParentedHoldRequestDigest {
        schema: PARENTED_HOLD_REQUEST_SCHEMA,
        base_hold_request_digest,
        parent_claim_id: &parent.claim_id,
        parent_claim_revision: parent.revision,
        parent_claim_digest: &parent.claim_digest,
        parent_release_transaction_id: &release.ledger.transaction_id,
        parent_release_transaction_digest: &release.ledger.transaction_digest,
        parent_release_ledger_sequence: release.ledger.ledger_sequence,
        reservation_id,
        job_id,
        offer_id: &offer.offer_id,
        offer_version: offer.offer_version,
        offer_digest: &offer.offer_digest,
    };
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&payload)?)))
}

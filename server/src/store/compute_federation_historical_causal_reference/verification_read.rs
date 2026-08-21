use anyhow::{anyhow, bail, Result};
use rusqlite::Connection;

use crate::compute_federation::execution::ATTEMPT_STATUS_RUNNING;
use crate::store::{
    compute_attempt_leases::audited_compute_attempt_lease_version_on,
    compute_attempt_terminals::{
        compute_attempt_historical_terminal_candidate_on, ComputeAttemptTerminalCandidateReceipt,
    },
    compute_attempt_verifications::ComputeAttemptVerificationDecisionReceipt,
    compute_capacity_claim_rows::stored_claim_version_on,
    compute_job_registry::registered_historical_job_version_on,
    compute_offer_registry::registered_historical_offer_version_on,
    compute_provider_registry::registered_provider_version_on,
    compute_reservation_registry::registered_historical_reservation_version_on,
};

use super::{
    ComputeAttemptRetainedVerificationAccessScope, ValidatedComputeAttemptRetainedVerification,
};

pub(super) fn validate_retained_verification_on(
    conn: &Connection,
    requested_lease_id: &str,
    receipt: ComputeAttemptVerificationDecisionReceipt,
) -> Result<ValidatedComputeAttemptRetainedVerification> {
    if receipt.lease_id != requested_lease_id {
        bail!("retained Verification does not match the requested Attempt Lease");
    }
    let candidate = compute_attempt_historical_terminal_candidate_on(conn, requested_lease_id)?
        .ok_or_else(|| anyhow!("retained Verification historical terminal candidate is absent"))?;
    validate_receipt_candidate_bindings(&receipt, &candidate)?;

    let job = registered_historical_job_version_on(conn, &receipt.job_id, receipt.job_revision)?
        .ok_or_else(|| anyhow!("retained Verification historical Job is absent"))?;
    if job.job.job_id != receipt.job_id
        || job.revision != receipt.job_revision
        || job.job_digest != receipt.job_digest
        || job.job.consumer_account_id != receipt.consumer_account_id
    {
        bail!("retained Verification and historical Job bindings differ");
    }

    let selected_offer = job
        .job
        .selected_offer
        .as_ref()
        .ok_or_else(|| anyhow!("retained Verification historical Job has no selected Offer"))?;
    let offer = registered_historical_offer_version_on(
        conn,
        &selected_offer.offer_id,
        selected_offer.offer_version,
    )?
    .ok_or_else(|| anyhow!("retained Verification historical Offer is absent"))?;
    if offer.offer.offer_id != selected_offer.offer_id
        || offer.offer.offer_version != selected_offer.offer_version
        || offer.offer.offer_digest != selected_offer.offer_digest
        || offer.offer.provider_id != selected_offer.provider_id
        || selected_offer.provider_id != receipt.provider_id
    {
        bail!("retained Verification historical Job and Offer bindings differ");
    }

    let provider = registered_provider_version_on(
        conn,
        &offer.offer.provider_id,
        offer.provider_policy_revision,
    )?
    .ok_or_else(|| anyhow!("retained Verification historical Provider is absent"))?;
    if provider.provider.provider_id != receipt.provider_id
        || provider.provider.policy_revision != offer.provider_policy_revision
        || provider.provider_digest != offer.provider_digest
    {
        bail!("retained Verification historical Offer and Provider bindings differ");
    }

    let reservation = registered_historical_reservation_version_on(
        conn,
        &receipt.reservation_id,
        receipt.reservation_revision,
    )?
    .ok_or_else(|| anyhow!("retained Verification historical Reservation is absent"))?;
    let reservation_body = &reservation.reservation;
    if reservation_body.reservation_id != receipt.reservation_id
        || reservation.revision != receipt.reservation_revision
        || reservation.reservation_digest != receipt.reservation_digest
        || reservation_body.job.job_id != receipt.job_id
        || reservation_body.job.job_revision != receipt.job_revision
        || reservation_body.job.job_digest != receipt.job_digest
        || reservation_body.offer.provider_id != receipt.provider_id
        || reservation_body.offer.provider_id != selected_offer.provider_id
        || reservation_body.offer.offer_id != selected_offer.offer_id
        || reservation_body.offer.offer_version != selected_offer.offer_version
        || reservation_body.offer.offer_digest != selected_offer.offer_digest
        || reservation_body.offer.offer_id != offer.offer.offer_id
        || reservation_body.offer.offer_version != offer.offer.offer_version
        || reservation_body.offer.offer_digest != offer.offer.offer_digest
        || reservation_body.capacity_claim.claim_id != receipt.capacity_claim_id
        || reservation_body.capacity_claim.claim_revision != receipt.capacity_claim_revision
        || reservation_body.capacity_claim.claim_digest != receipt.capacity_claim_digest
    {
        bail!("retained Verification historical Reservation owner chain differs");
    }

    let claim = stored_claim_version_on(
        conn,
        &receipt.capacity_claim_id,
        receipt.capacity_claim_revision,
    )?
    .ok_or_else(|| anyhow!("retained Verification historical Capacity Claim is absent"))?;
    if claim.claim_id != receipt.capacity_claim_id
        || claim.revision != receipt.capacity_claim_revision
        || claim.claim_digest != receipt.capacity_claim_digest
        || claim.claim_id != reservation_body.capacity_claim.claim_id
        || claim.revision != reservation_body.capacity_claim.claim_revision
        || claim.claim_digest != reservation_body.capacity_claim.claim_digest
        || claim.subject_kind != "compute_reservation"
        || claim.subject_id != reservation_body.reservation_id
    {
        bail!("retained Verification historical Capacity Claim owner chain differs");
    }

    let source_lease = audited_compute_attempt_lease_version_on(
        conn,
        &receipt.lease_id,
        receipt.source_lease_revision,
    )?
    .ok_or_else(|| anyhow!("retained Verification historical source Lease is absent"))?;
    if source_lease.lease.lease_id != receipt.lease_id
        || source_lease.lease_revision != receipt.source_lease_revision
        || source_lease.lease_digest != receipt.source_lease_digest
        || source_lease.lease.fencing_generation != receipt.fencing_generation
        || source_lease.lease.job_id != receipt.job_id
        || source_lease.lease.reservation_id != receipt.reservation_id
        || source_lease.lease.provider_id != receipt.provider_id
        || source_lease.lease.status != ATTEMPT_STATUS_RUNNING
        || source_lease.lease.last_heartbeat_at.is_none()
        || source_lease.consumer_account_id != receipt.consumer_account_id
    {
        bail!("retained Verification historical source Lease owner chain differs");
    }

    let access_scope =
        ComputeAttemptRetainedVerificationAccessScope::from_historical_job_and_provider(
            &job.job.consumer_account_id,
            job.job.project_id.as_deref(),
            &provider.provider.owner_account_id,
        )?;
    Ok(ValidatedComputeAttemptRetainedVerification::from_historical_receipt(receipt, access_scope))
}

fn validate_receipt_candidate_bindings(
    receipt: &ComputeAttemptVerificationDecisionReceipt,
    candidate: &ComputeAttemptTerminalCandidateReceipt,
) -> Result<()> {
    if receipt.terminal_candidate_id != candidate.terminal_candidate_id
        || receipt.terminal_candidate_event_digest != candidate.event_digest
        || receipt.lease_id != candidate.lease_id
        || receipt.provider_id != candidate.provider_id
        || receipt.consumer_account_id != candidate.consumer_account_id
        || receipt.source_lease_revision != candidate.source_lease_revision
        || receipt.source_lease_digest != candidate.source_lease_digest
        || receipt.fencing_generation != candidate.fencing_generation
        || receipt.job_id != candidate.job_id
        || receipt.job_revision != candidate.job_revision
        || receipt.job_digest != candidate.job_digest
        || receipt.reservation_id != candidate.reservation_id
        || receipt.reservation_revision != candidate.reservation_revision
        || receipt.reservation_digest != candidate.reservation_digest
        || receipt.capacity_claim_id != candidate.capacity_claim_id
        || receipt.capacity_claim_revision != candidate.capacity_claim_revision
        || receipt.capacity_claim_digest != candidate.capacity_claim_digest
        || receipt.final_usage_snapshot_id != candidate.final_usage_snapshot_id
        || receipt.final_usage_sequence_no != candidate.final_usage_sequence_no
        || receipt.final_provider_usage_digest != candidate.final_cumulative_usage_digest
        || receipt.candidate_outcome != candidate.outcome
    {
        bail!("retained Verification and historical terminal candidate bindings differ");
    }
    Ok(())
}

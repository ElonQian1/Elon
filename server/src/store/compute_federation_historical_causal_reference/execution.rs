use anyhow::{anyhow, bail, Result};
use rusqlite::Connection;

use crate::{
    compute_federation::{
        capacity::ComputeCapacityClaimBinding,
        execution::{ComputeJobVersionBinding, ComputeOfferBinding, ATTEMPT_STATUS_RUNNING},
        federation_historical_causal_reference::{
            build_execution_source_carrier, ExecutionSourceLineageV1,
        },
    },
    store::{
        compute_attempt_activations::compute_attempt_historical_activation_sources_on,
        compute_attempt_execution_receipts::compute_attempt_execution_receipt_by_id_on,
        compute_attempt_leases::audited_compute_attempt_lease_version_on,
        compute_attempt_terminals::compute_attempt_historical_terminal_candidate_on,
        compute_attempt_verifications::compute_attempt_historical_verification_decision_on,
        compute_capacity_claim_rows::stored_claim_version_on,
        compute_capacity_pool_queries::audited_compute_capacity_pool_version_on,
        compute_job_registry::registered_historical_job_version_on,
        compute_offer_registry::registered_historical_offer_version_on,
        compute_price_snapshot_registry::registered_historical_price_snapshot_on,
        compute_provider_registry::registered_provider_version_on,
        compute_reservation_registry::registered_historical_reservation_version_on,
    },
};

use super::{
    source_refs::{
        claim_ref, execution_receipt_ref, job_ref, lease_ref, offer_ref, pool_ref, positive_u64,
        price_snapshot_ref, provider_ref, reservation_ref, validate_execution_source_links,
        ExecutionSourceLinkFacts,
    },
    FederationHistoricalLineageAccessScope, ValidatedFederationHistoricalLineage,
};

pub(super) fn resolve_execution_source_lineage_on(
    conn: &Connection,
    execution_receipt_id: &str,
    execution_receipt_digest: &str,
) -> Result<ValidatedFederationHistoricalLineage> {
    validate_root_pair(
        "Execution Receipt ID",
        execution_receipt_id,
        "Execution Receipt digest",
        execution_receipt_digest,
    )?;
    let execution = compute_attempt_execution_receipt_by_id_on(conn, execution_receipt_id)?
        .ok_or_else(|| anyhow!("Execution Receipt exact root does not exist"))?;
    if execution.receipt.receipt_id != execution_receipt_id
        || execution.receipt.receipt_digest != execution_receipt_digest
    {
        bail!("Execution Receipt exact root pair does not match retained v193 owner data");
    }

    let candidate = compute_attempt_historical_terminal_candidate_on(
        conn,
        &execution.receipt.attempt_lease_id,
    )?
    .ok_or_else(|| anyhow!("Execution source v189 terminal candidate does not exist"))?;
    let verification = compute_attempt_historical_verification_decision_on(
        conn,
        &execution.receipt.attempt_lease_id,
    )?
    .ok_or_else(|| anyhow!("Execution source v192 verification does not exist"))?;
    let activation = compute_attempt_historical_activation_sources_on(
        conn,
        &execution.receipt.attempt_lease_id,
    )?;
    let job =
        registered_historical_job_version_on(conn, &candidate.job_id, candidate.job_revision)?
            .ok_or_else(|| anyhow!("Execution source historical Job does not exist"))?;
    let reservation = registered_historical_reservation_version_on(
        conn,
        &candidate.reservation_id,
        candidate.reservation_revision,
    )?
    .ok_or_else(|| anyhow!("Execution source historical Reservation does not exist"))?;
    let claim = stored_claim_version_on(
        conn,
        &candidate.capacity_claim_id,
        candidate.capacity_claim_revision,
    )?
    .ok_or_else(|| anyhow!("Execution source historical Capacity Claim does not exist"))?;
    let snapshot = registered_historical_price_snapshot_on(
        conn,
        &reservation.reservation.price_snapshot.snapshot_id,
    )?
    .ok_or_else(|| anyhow!("Execution source Price Snapshot does not exist"))?;
    let offer = registered_historical_offer_version_on(
        conn,
        &reservation.reservation.offer.offer_id,
        reservation.reservation.offer.offer_version,
    )?
    .ok_or_else(|| anyhow!("Execution source historical Offer does not exist"))?;
    let provider = registered_provider_version_on(
        conn,
        &offer.offer.provider_id,
        offer.provider_policy_revision,
    )?
    .ok_or_else(|| anyhow!("Execution source historical Provider does not exist"))?;
    let pool = audited_compute_capacity_pool_version_on(
        conn,
        &offer.offer.capacity_pool.pool_id,
        offer.offer.capacity_pool.capacity_epoch,
        offer.offer.capacity_pool.pool_revision,
    )?
    .ok_or_else(|| anyhow!("Execution source historical Capacity Pool does not exist"))?;
    let source_lease = audited_compute_attempt_lease_version_on(
        conn,
        &candidate.lease_id,
        candidate.source_lease_revision,
    )?
    .ok_or_else(|| anyhow!("Execution source historical Attempt Lease does not exist"))?;
    let access_scope = FederationHistoricalLineageAccessScope::from_historical_job_and_provider(
        &job.job.consumer_account_id,
        job.job.project_id.as_deref(),
        &provider.provider.owner_account_id,
    )?;

    let selected_offer = job
        .job
        .selected_offer
        .as_ref()
        .ok_or_else(|| anyhow!("Execution source Job has no selected Offer"))?;
    let job_snapshot_id = job
        .job
        .price_snapshot_id
        .as_deref()
        .ok_or_else(|| anyhow!("Execution source Job has no Price Snapshot"))?;
    if provider.provider.policy_revision != offer.provider_policy_revision
        || provider.provider_digest != offer.provider_digest
        || pool.binding != offer.offer.capacity_pool
        || pool.provider_id != offer.offer.provider_id
        || pool.resource_profile_digest != offer.offer.resource_profile.declared_profile_digest
        || pool.region_or_data_zone != offer.offer.sku.region_or_data_zone
        || snapshot != reservation.reservation.price_snapshot
        || snapshot.sku != offer.offer.sku
        || claim.pool != offer.offer.capacity_pool
        || source_lease.lease.status != ATTEMPT_STATUS_RUNNING
        || source_lease.lease.job_id != candidate.job_id
        || source_lease.lease.reservation_id != candidate.reservation_id
        || source_lease.lease.provider_id != candidate.provider_id
        || source_lease.lease.fencing_generation != candidate.fencing_generation
        || source_lease.lease.attempt_no != execution.receipt.attempt_no
        || source_lease.lease.executor_id != execution.receipt.executor_id
    {
        bail!("Execution source retained owner bodies do not form one exact historical chain");
    }

    let execution_receipt = execution_receipt_ref(
        &execution.receipt.receipt_id,
        &execution.receipt.receipt_digest,
    );
    let provider_lineage = provider_ref(
        &offer.offer.provider_id,
        offer.provider_policy_revision,
        &offer.provider_digest,
    )?;
    let capacity_pool = pool_ref(&offer.offer.capacity_pool)?;
    let offer_lineage = offer_ref(&reservation.reservation.offer)?;
    let price_snapshot = price_snapshot_ref(&snapshot.snapshot_id, &snapshot.snapshot_digest);
    let job_lineage = job_ref(&ComputeJobVersionBinding {
        job_id: job.job.job_id.clone(),
        job_revision: job.revision,
        job_digest: job.job_digest.clone(),
    })?;
    let reservation_lineage = reservation_ref(
        &reservation.reservation.reservation_id,
        reservation.revision,
        &reservation.reservation_digest,
    )?;
    let capacity_claim = claim_ref(&ComputeCapacityClaimBinding {
        claim_id: claim.claim_id.clone(),
        claim_revision: claim.revision,
        claim_digest: claim.claim_digest.clone(),
    })?;
    let attempt_lease_source = lease_ref(
        &candidate.lease_id,
        candidate.source_lease_revision,
        &candidate.source_lease_digest,
        candidate.fencing_generation,
    )?;
    let lineage = ExecutionSourceLineageV1 {
        execution_receipt,
        provider: provider_lineage,
        capacity_pool,
        offer: offer_lineage,
        price_snapshot,
        job: job_lineage,
        reservation: reservation_lineage,
        capacity_claim,
        attempt_lease_source,
    };

    let facts = ExecutionSourceLinkFacts {
        audited_execution_receipt: execution_receipt_ref(
            &execution.receipt.receipt_id,
            &execution.receipt.receipt_digest,
        ),
        audited_provider: provider_ref(
            &provider.provider.provider_id,
            provider.provider.policy_revision,
            &provider.provider_digest,
        )?,
        offer_provider: provider_ref(
            &offer.offer.provider_id,
            offer.provider_policy_revision,
            &offer.provider_digest,
        )?,
        audited_pool: pool_ref(&pool.binding)?,
        pool_from_offer: pool_ref(&offer.offer.capacity_pool)?,
        pool_from_claim: pool_ref(&claim.pool)?,
        pool_provider_id: pool.provider_id,
        snapshot_provider_id: snapshot.provider_id.clone(),
        audited_offer: offer_ref(&ComputeOfferBinding {
            provider_id: offer.offer.provider_id.clone(),
            offer_id: offer.offer.offer_id.clone(),
            offer_version: offer.offer.offer_version,
            offer_digest: offer.offer.offer_digest.clone(),
        })?,
        snapshot_offer: offer_ref(&ComputeOfferBinding {
            provider_id: snapshot.provider_id.clone(),
            offer_id: snapshot.offer_id.clone(),
            offer_version: snapshot.offer_version,
            offer_digest: snapshot.offer_digest.clone(),
        })?,
        job_offer: offer_ref(selected_offer)?,
        job_price_snapshot_id: job_snapshot_id.to_string(),
        reservation_job: job_ref(&reservation.reservation.job)?,
        reservation_offer: offer_ref(&reservation.reservation.offer)?,
        reservation_snapshot: price_snapshot_ref(
            &reservation.reservation.price_snapshot.snapshot_id,
            &reservation.reservation.price_snapshot.snapshot_digest,
        ),
        reservation_claim: claim_ref(&reservation.reservation.capacity_claim)?,
        claim_delivery_window: claim.delivery_window.clone(),
        snapshot_delivery_window: snapshot.delivery_window.binding.clone(),
        offer_delivery_windows: offer
            .offer
            .delivery_windows
            .iter()
            .map(|window| window.binding.clone())
            .collect(),
        candidate_provider_id: candidate.provider_id.clone(),
        candidate_job: job_ref(&ComputeJobVersionBinding {
            job_id: candidate.job_id.clone(),
            job_revision: candidate.job_revision,
            job_digest: candidate.job_digest.clone(),
        })?,
        candidate_reservation: reservation_ref(
            &candidate.reservation_id,
            candidate.reservation_revision,
            &candidate.reservation_digest,
        )?,
        candidate_claim: claim_ref(&ComputeCapacityClaimBinding {
            claim_id: candidate.capacity_claim_id.clone(),
            claim_revision: candidate.capacity_claim_revision,
            claim_digest: candidate.capacity_claim_digest.clone(),
        })?,
        candidate_lease: lease_ref(
            &candidate.lease_id,
            candidate.source_lease_revision,
            &candidate.source_lease_digest,
            candidate.fencing_generation,
        )?,
        verification_provider_id: verification.provider_id.clone(),
        verification_job: job_ref(&ComputeJobVersionBinding {
            job_id: verification.job_id.clone(),
            job_revision: verification.job_revision,
            job_digest: verification.job_digest.clone(),
        })?,
        verification_reservation: reservation_ref(
            &verification.reservation_id,
            verification.reservation_revision,
            &verification.reservation_digest,
        )?,
        verification_claim: claim_ref(&ComputeCapacityClaimBinding {
            claim_id: verification.capacity_claim_id.clone(),
            claim_revision: verification.capacity_claim_revision,
            claim_digest: verification.capacity_claim_digest.clone(),
        })?,
        verification_lease: lease_ref(
            &verification.lease_id,
            verification.source_lease_revision,
            &verification.source_lease_digest,
            verification.fencing_generation,
        )?,
        audited_lease: lease_ref(
            &source_lease.lease.lease_id,
            source_lease.lease_revision,
            &source_lease.lease_digest,
            source_lease.lease.fencing_generation,
        )?,
        receipt_job_id: execution.receipt.job_id.clone(),
        receipt_reservation_id: execution.receipt.reservation_id.clone(),
        receipt_lease_id: execution.receipt.attempt_lease_id.clone(),
        receipt_provider_id: execution.receipt.provider_id.clone(),
        receipt_offer: offer_ref(&ComputeOfferBinding {
            provider_id: execution.receipt.provider_id.clone(),
            offer_id: execution.receipt.offer_id.clone(),
            offer_version: execution.receipt.offer_version,
            offer_digest: execution.receipt.offer_digest.clone(),
        })?,
        receipt_attempt_no: positive_u64(
            "Execution Receipt attempt number",
            execution.receipt.attempt_no,
        )?,
        receipt_fencing_generation: positive_u64(
            "Execution Receipt fencing generation",
            execution.receipt.fencing_generation,
        )?,
        receipt_executor_id: execution.receipt.executor_id.clone(),
        activation_job_id: activation.lease.job_id.clone(),
        activation_job: job_ref(&activation.running_job)?,
        activation_reservation_id: activation.lease.reservation_id.clone(),
        activation_reservation: reservation_ref(
            &activation.lease.reservation_id,
            activation.active_reservation_revision,
            &activation.active_reservation_digest,
        )?,
        activation_claim: claim_ref(&activation.active_claim)?,
        activation_provider_id: activation.lease.provider_id.clone(),
        activation_attempt_no: positive_u64(
            "Activation attempt number",
            activation.lease.attempt_no,
        )?,
        activation_fencing_generation: positive_u64(
            "Activation fencing generation",
            activation.lease.fencing_generation,
        )?,
        activation_executor_id: activation.lease.executor_id.clone(),
        lineage,
    };
    validate_execution_source_links(&facts)?;
    ValidatedFederationHistoricalLineage::from_carrier(
        build_execution_source_carrier(facts.lineage)?,
        access_scope,
    )
}

fn validate_root_pair(id_label: &str, id: &str, digest_label: &str, digest: &str) -> Result<()> {
    if id.trim().is_empty() || id != id.trim() || id.len() > 240 || id.chars().any(char::is_control)
    {
        bail!("{id_label} is invalid");
    }
    if digest.len() != 64
        || digest != digest.to_ascii_lowercase()
        || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        bail!("{digest_label} must be a lowercase SHA-256 digest");
    }
    Ok(())
}

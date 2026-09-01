use anyhow::{ensure, Context, Result};
use chrono::DateTime;

use super::super::{
    capacity_commitment::{
        CAPACITY_COMMITMENT_STATUS_COMMITTED, COMPUTE_CAPACITY_COMMITMENT_SCHEMA,
    },
    capacity_instrument::{
        validate_compute_capacity_instrument,
        validate_compute_capacity_instrument_activation_receipt,
        validate_compute_capacity_instrument_offer_adoption_receipt,
        COMPUTE_CAPACITY_INSTRUMENT_REVISION,
    },
    delivery_allocation::{
        COMPUTE_DELIVERY_ALLOCATION_GRANT_SCHEMA,
        COMPUTE_DELIVERY_ALLOCATION_TERMINAL_RECEIPT_SCHEMA, DELIVERY_ALLOCATION_ACTOR_CONSUMER,
        DELIVERY_ALLOCATION_STATUS_EXERCISED, DELIVERY_ALLOCATION_STATUS_GRANTED,
    },
    market::{COMPUTE_PRICE_SNAPSHOT_SCHEMA, PRICING_MODE_CAPACITY_FUTURE},
    receipts::{COMPUTE_EXECUTION_RECEIPT_SCHEMA, VERIFICATION_STATUS_ACCEPTED},
};

use super::{
    canonical::canonical_compute_capacity_future_settlement_lineage_json_and_digest,
    settlement_equations::validate_settlement_source_equations,
    source_inputs::{
        ComputeCapacityFutureSettlementLineageSources, ComputeCapacityFutureSettlementStageSources,
    },
    source_support::{
        canonical_source_lineages, claim_ref, job_ref, positive_u64, release_source_carrier,
        reservation_ref, settlement_source_carrier, CanonicalSourceLineages,
    },
    types::*,
    validation::validate_compute_capacity_future_settlement_lineage,
};

pub(crate) fn build_compute_capacity_future_settlement_lineage(
    sources: &ComputeCapacityFutureSettlementLineageSources<'_>,
) -> Result<ProjectedComputeCapacityFutureSettlementLineageV1> {
    let lineage = project_source_lineage(sources)?;
    let mut envelope = UntrustedComputeCapacityFutureSettlementLineageEnvelopeV1 {
        schema: COMPUTE_CAPACITY_FUTURE_SETTLEMENT_LINEAGE_SCHEMA.to_string(),
        lineage_kind: COMPUTE_CAPACITY_FUTURE_SETTLEMENT_LINEAGE_KIND.to_string(),
        lineage_digest: String::new(),
        canonicalization: COMPUTE_CAPACITY_FUTURE_SETTLEMENT_LINEAGE_CANONICALIZATION.to_string(),
        digest_algorithm: COMPUTE_CAPACITY_FUTURE_SETTLEMENT_LINEAGE_DIGEST_ALGORITHM.to_string(),
        lineage,
    };
    envelope.lineage_digest =
        canonical_compute_capacity_future_settlement_lineage_json_and_digest(&envelope)?.1;
    validate_compute_capacity_future_settlement_lineage(&envelope)?;
    Ok(ProjectedComputeCapacityFutureSettlementLineageV1 { envelope })
}

pub(crate) fn validate_compute_capacity_future_settlement_lineage_against_sources(
    envelope: &UntrustedComputeCapacityFutureSettlementLineageEnvelopeV1,
    sources: &ComputeCapacityFutureSettlementLineageSources<'_>,
) -> Result<ProjectedComputeCapacityFutureSettlementLineageV1> {
    validate_compute_capacity_future_settlement_lineage(envelope)?;
    let expected = build_compute_capacity_future_settlement_lineage(sources)?;
    ensure!(
        expected.envelope() == envelope,
        "capacity-future settlement lineage does not equal its retained source projection"
    );
    Ok(ProjectedComputeCapacityFutureSettlementLineageV1 {
        envelope: envelope.clone(),
    })
}

fn project_source_lineage(
    sources: &ComputeCapacityFutureSettlementLineageSources<'_>,
) -> Result<ComputeCapacityFutureSettlementLineageV1> {
    let canonical_sources = canonical_source_lineages(sources)?;
    validate_capacity_source_equations(sources)?;
    validate_execution_source_equations(sources, &canonical_sources)?;
    validate_settlement_source_equations(sources, &canonical_sources)?;

    let exercise = sources
        .delivery_allocation_exercise
        .exercise
        .as_ref()
        .context("capacity-future settlement lineage lacks exercise evidence")?;
    let economic_lineage = match (&sources.settlement_stage, canonical_sources.release) {
        (ComputeCapacityFutureSettlementStageSources::PendingSettlementSource { .. }, None) => {
            CapacityFutureSettlementEconomicLineageV1::PendingSettlementSourceV1 {
                attempt_settlement: canonical_sources.settlement.attempt_settlement.clone(),
                settlement_lineage_digest: settlement_source_carrier(sources)
                    .lineage_digest()
                    .to_string(),
            }
        }
        (
            ComputeCapacityFutureSettlementStageSources::AvailableReleaseSource { .. },
            Some(release),
        ) => CapacityFutureSettlementEconomicLineageV1::AvailableReleaseSourceV1 {
            attempt_settlement: canonical_sources.settlement.attempt_settlement.clone(),
            settlement_lineage_digest: settlement_source_carrier(sources)
                .lineage_digest()
                .to_string(),
            settlement_release: release.settlement_release.clone(),
            settlement_release_lineage_digest: release_source_carrier(sources)
                .context("available release source carrier is missing")?
                .lineage_digest()
                .to_string(),
        },
        _ => unreachable!("canonical source lineage stage extraction is exact"),
    };

    Ok(ComputeCapacityFutureSettlementLineageV1 {
        pricing_mode: COMPUTE_CAPACITY_FUTURE_SETTLEMENT_PRICING_MODE.to_string(),
        settlement_currency: COMPUTE_CAPACITY_FUTURE_SETTLEMENT_CURRENCY.to_string(),
        price_snapshot: canonical_sources.execution.price_snapshot.clone(),
        reference_price_binding: sources.commitment.reference_binding.clone(),
        delivery_window: CapacityFutureDeliveryWindowRef {
            window_id: sources.commitment.delivery_window.binding.window_id.clone(),
            window_digest: sources
                .commitment
                .delivery_window
                .binding
                .window_digest
                .clone(),
            starts_at_utc: sources.commitment.delivery_window.starts_at_utc.clone(),
            ends_at_utc: sources.commitment.delivery_window.ends_at_utc.clone(),
        },
        capacity_instrument: CapacityFutureInstrumentRef {
            instrument_id: sources.instrument.instrument_id.clone(),
            instrument_revision: positive_u64(
                sources.instrument.instrument_revision,
                "capacity instrument revision",
            )?,
            instrument_digest: sources.instrument.instrument_digest.clone(),
        },
        instrument_activation: CapacityFutureInstrumentActivationRef {
            activation_receipt_id: sources.instrument_activation.activation_receipt_id.clone(),
            activation_receipt_digest: sources
                .instrument_activation
                .activation_receipt_digest
                .clone(),
        },
        instrument_offer_adoption: CapacityFutureInstrumentOfferAdoptionRef {
            adoption_receipt_id: sources
                .instrument_offer_adoption
                .adoption_receipt_id
                .clone(),
            adoption_receipt_digest: sources
                .instrument_offer_adoption
                .adoption_receipt_digest
                .clone(),
            offer: canonical_sources.execution.offer.clone(),
            publication_id: sources.instrument_offer_adoption.publication_id.clone(),
            publication_digest: sources.instrument_offer_adoption.publication_digest.clone(),
        },
        capacity_commitment: CapacityFutureCommitmentRef {
            commitment_id: sources.commitment.commitment_id.clone(),
            commitment_revision: positive_u64(
                sources.commitment.commitment_revision,
                "capacity commitment revision",
            )?,
            commitment_digest: sources.commitment.commitment_digest.clone(),
            capacity_claim: claim_ref(
                &sources.commitment.claim.claim_id,
                sources.commitment.claim.claim_revision,
                &sources.commitment.claim.claim_digest,
            )?,
        },
        delivery_allocation_grant: CapacityFutureDeliveryAllocationGrantRef {
            grant_id: sources.delivery_allocation_grant.grant_id.clone(),
            grant_revision: positive_u64(
                sources.delivery_allocation_grant.grant_revision,
                "delivery-allocation Grant revision",
            )?,
            grant_digest: sources.delivery_allocation_grant.grant_digest.clone(),
            job: job_ref(
                &sources.delivery_allocation_grant.job.job_id,
                sources.delivery_allocation_grant.job.job_revision,
                &sources.delivery_allocation_grant.job.job_digest,
            )?,
        },
        delivery_allocation_exercise: CapacityFutureDeliveryAllocationExerciseRef {
            terminal_receipt_id: sources
                .delivery_allocation_exercise
                .terminal_receipt_id
                .clone(),
            terminal_revision: positive_u64(
                sources.delivery_allocation_exercise.terminal_revision,
                "delivery-allocation exercise revision",
            )?,
            terminal_receipt_digest: sources
                .delivery_allocation_exercise
                .terminal_receipt_digest
                .clone(),
            parent_released_claim: claim_ref(
                &exercise.parent_claim_id,
                exercise.parent_result_claim_revision,
                &exercise.parent_result_claim_digest,
            )?,
            reservation_claim: claim_ref(
                &exercise.reservation_claim.claim_id,
                exercise.reservation_claim.claim_revision,
                &exercise.reservation_claim.claim_digest,
            )?,
            exercise_reservation: reservation_ref(
                &exercise.reservation.reservation_id,
                exercise.reservation.reservation_revision,
                &exercise.reservation.reservation_digest,
            )?,
            reserved_job: job_ref(
                &sources.delivery_allocation_grant.job.job_id,
                exercise.reserved_job_revision,
                &exercise.reserved_job_digest,
            )?,
        },
        terminal_reservation: canonical_sources.settlement.terminal_reservation.clone(),
        execution_source_lineage_digest: sources.execution_source.lineage_digest().to_string(),
        execution_receipt: canonical_sources.execution.execution_receipt.clone(),
        execution_verification_lineage_digest: sources
            .execution_verification_source
            .lineage_digest()
            .to_string(),
        verification_decision: canonical_sources.verification.verification_decision.clone(),
        settlement_usage_digests: CapacityFutureSettlementUsageDigestRefsV1 {
            verified_usage_digest: sources
                .attempt_settlement
                .settlement
                .verified_usage_digest
                .clone(),
            compensable_usage_digest: sources
                .attempt_settlement
                .settlement
                .compensable_usage_digest
                .clone(),
        },
        economic_lineage,
        effects: CapacityFutureSettlementLineageEffectsV1 {
            reference_effect: COMPUTE_CAPACITY_FUTURE_SETTLEMENT_REFERENCE_EFFECT.to_string(),
            capacity_effect: COMPUTE_CAPACITY_FUTURE_SETTLEMENT_NO_EFFECT.to_string(),
            verification_effect: COMPUTE_CAPACITY_FUTURE_SETTLEMENT_NO_EFFECT.to_string(),
            settlement_effect: COMPUTE_CAPACITY_FUTURE_SETTLEMENT_NO_EFFECT.to_string(),
            money_effect: COMPUTE_CAPACITY_FUTURE_SETTLEMENT_NO_EFFECT.to_string(),
            withdrawal_effect: COMPUTE_CAPACITY_FUTURE_SETTLEMENT_NO_EFFECT.to_string(),
        },
    })
}

fn validate_capacity_source_equations(
    sources: &ComputeCapacityFutureSettlementLineageSources<'_>,
) -> Result<()> {
    validate_compute_capacity_instrument(sources.instrument)?;
    validate_compute_capacity_instrument_activation_receipt(sources.instrument_activation)?;
    validate_compute_capacity_instrument_offer_adoption_receipt(sources.instrument_offer_adoption)?;
    let commitment = sources.commitment;
    let grant = sources.delivery_allocation_grant;
    let terminal = sources.delivery_allocation_exercise;
    let exercise = terminal
        .exercise
        .as_ref()
        .context("capacity-future bridge requires exercised DeliveryAllocation")?;
    ensure!(
        sources.instrument.instrument_revision == COMPUTE_CAPACITY_INSTRUMENT_REVISION
            && sources.instrument_activation.instrument_id == sources.instrument.instrument_id
            && sources.instrument_activation.instrument_revision
                == sources.instrument.instrument_revision
            && sources.instrument_activation.instrument_digest
                == sources.instrument.instrument_digest
            && sources.instrument_offer_adoption.instrument_id == sources.instrument.instrument_id
            && sources.instrument_offer_adoption.instrument_revision
                == sources.instrument.instrument_revision
            && sources.instrument_offer_adoption.instrument_digest
                == sources.instrument.instrument_digest,
        "capacity-future instrument, activation, and adoption roots differ"
    );
    let activated_at = DateTime::parse_from_rfc3339(&sources.instrument_activation.activated_at)?;
    let adopted_at = DateTime::parse_from_rfc3339(&sources.instrument_offer_adoption.adopted_at)?;
    ensure!(
        activated_at <= adopted_at,
        "capacity-future instrument adoption predates activation"
    );
    ensure!(
        commitment.schema == COMPUTE_CAPACITY_COMMITMENT_SCHEMA
            && commitment.commitment_revision == 1
            && commitment.commitment_status == CAPACITY_COMMITMENT_STATUS_COMMITTED
            && commitment.instrument_id == sources.instrument.instrument_id
            && commitment.delivery_window == sources.instrument.delivery_window
            && commitment.expires_at == commitment.delivery_window.ends_at_utc,
        "capacity-future Commitment root is not exact"
    );
    ensure!(
        sources.price_snapshot.schema == COMPUTE_PRICE_SNAPSHOT_SCHEMA
            && sources.price_snapshot.pricing_mode == PRICING_MODE_CAPACITY_FUTURE
            && sources.price_snapshot.currency == COMPUTE_CAPACITY_FUTURE_SETTLEMENT_CURRENCY
            && sources.price_snapshot.instrument_id.as_deref()
                == Some(sources.instrument.instrument_id.as_str())
            && sources.price_snapshot.snapshot_id == commitment.price_snapshot_id
            && sources.price_snapshot.snapshot_digest == commitment.price_snapshot_digest
            && sources.price_snapshot.delivery_window == commitment.delivery_window
            && sources.price_snapshot.sku.sku_id == sources.instrument.sku_id
            && sources.price_snapshot.sku.sku_digest == sources.instrument.sku_digest,
        "capacity-future Price Snapshot does not equal Instrument or Commitment"
    );
    ensure!(
        sources.instrument_offer_adoption.offer_id == commitment.offer.offer_id
            && sources.instrument_offer_adoption.offer_version == commitment.offer.offer_version
            && sources.instrument_offer_adoption.offer_digest == commitment.offer.offer_digest
            && sources.price_snapshot.offer_id == commitment.offer.offer_id
            && sources.price_snapshot.offer_version == commitment.offer.offer_version
            && sources.price_snapshot.offer_digest == commitment.offer.offer_digest
            && sources.price_snapshot.provider_id == commitment.provider.provider_id,
        "capacity-future Offer adoption, Commitment, and Snapshot differ"
    );
    ensure!(
        grant.schema == COMPUTE_DELIVERY_ALLOCATION_GRANT_SCHEMA
            && grant.grant_revision == 1
            && grant.grant_status == DELIVERY_ALLOCATION_STATUS_GRANTED
            && grant.provider_owner_account_id == commitment.owner_account_id
            && grant.exercise_expires_at == commitment.delivery_window.starts_at_utc
            && grant.commitment.commitment_id == commitment.commitment_id
            && grant.commitment.commitment_revision == commitment.commitment_revision
            && grant.commitment.commitment_digest == commitment.commitment_digest
            && terminal.schema == COMPUTE_DELIVERY_ALLOCATION_TERMINAL_RECEIPT_SCHEMA
            && terminal.terminal_revision == 2
            && terminal.terminal_status == DELIVERY_ALLOCATION_STATUS_EXERCISED
            && terminal.grant_id == grant.grant_id
            && terminal.grant_digest == grant.grant_digest
            && terminal.commitment == grant.commitment
            && terminal.actor_kind == DELIVERY_ALLOCATION_ACTOR_CONSUMER
            && terminal.actor_id == grant.consumer_account_id
            && terminal.occurred_at == terminal.recorded_at,
        "capacity-future Commitment, Grant, and exercise roots differ"
    );
    let exercised_at = DateTime::parse_from_rfc3339(&terminal.occurred_at)?;
    let exercise_expires_at = DateTime::parse_from_rfc3339(&grant.exercise_expires_at)?;
    ensure!(
        exercised_at < exercise_expires_at,
        "capacity-future DeliveryAllocation exercise is outside its authorization window"
    );
    ensure!(
        exercise.parent_claim_id == commitment.claim.claim_id
            && commitment.claim.claim_revision == 1
            && exercise.parent_prior_claim_revision == 1
            && exercise.parent_prior_claim_digest == commitment.claim.claim_digest
            && exercise.parent_result_claim_revision == 2
            && exercise.parent_result_claim_state == "released"
            && exercise.reservation_claim.parent_claim_id == commitment.claim.claim_id
            && exercise.reservation_claim.claim_revision == 1
            && exercise.reservation.reservation_revision == 2
            && exercise.parent_release_ledger.event_kind == "reservation_released"
            && exercise.reservation_hold_ledger.event_kind == "reservation_held"
            && exercise.parent_release_ledger.causal_transaction_id
                == commitment.creation_ledger.transaction_id
            && exercise.reservation_hold_ledger.causal_transaction_id
                == exercise.parent_release_ledger.transaction_id
            && exercise.source_job_revision == grant.job.job_revision
            && exercise.source_job_digest == grant.job.job_digest
            && exercise.source_job_revision.checked_add(1) == Some(exercise.reserved_job_revision),
        "capacity-future DeliveryAllocation exercise evidence differs from its roots"
    );
    Ok(())
}

fn validate_execution_source_equations(
    sources: &ComputeCapacityFutureSettlementLineageSources<'_>,
    canonical_sources: &CanonicalSourceLineages<'_>,
) -> Result<()> {
    let commitment = sources.commitment;
    let grant = sources.delivery_allocation_grant;
    let exercise = sources
        .delivery_allocation_exercise
        .exercise
        .as_ref()
        .context("capacity-future bridge requires exercise evidence")?;
    let execution = canonical_sources.execution;
    let execution_receipt = sources.execution_receipt;
    let reserved_job_revision =
        positive_u64(exercise.reserved_job_revision, "reserved Job revision")?;
    let exercise_reservation_revision = positive_u64(
        exercise.reservation.reservation_revision,
        "exercise Reservation revision",
    )?;
    let reservation_claim_revision = positive_u64(
        exercise.reservation_claim.claim_revision,
        "reservation Claim revision",
    )?;
    ensure!(
        execution.provider.provider_id == commitment.provider.provider_id
            && execution.provider.policy_revision
                == positive_u64(commitment.provider.policy_revision, "Provider revision")?
            && execution.provider.provider_digest == commitment.provider.provider_digest
            && execution.capacity_pool.pool_id == commitment.pool.pool_id
            && execution.capacity_pool.capacity_epoch
                == positive_u64(commitment.pool.capacity_epoch, "Pool epoch")?
            && execution.capacity_pool.pool_revision
                == positive_u64(commitment.pool.pool_revision, "Pool revision")?
            && execution.capacity_pool.pool_digest == commitment.pool.pool_digest,
        "capacity-future Provider or Pool differs from execution source"
    );
    ensure!(
        execution.offer.provider_id == commitment.provider.provider_id
            && execution.offer.offer_id == commitment.offer.offer_id
            && execution.offer.offer_version
                == positive_u64(commitment.offer.offer_version, "Offer version")?
            && execution.offer.offer_digest == commitment.offer.offer_digest
            && execution.price_snapshot.price_snapshot_id == commitment.price_snapshot_id
            && execution.price_snapshot.price_snapshot_digest == commitment.price_snapshot_digest,
        "capacity-future Offer or Price Snapshot differs from execution source"
    );
    ensure!(
        execution.job.job_id == grant.job.job_id
            && reserved_job_revision.checked_add(1) == Some(execution.job.job_revision)
            && execution.reservation.reservation_id == exercise.reservation.reservation_id
            && exercise_reservation_revision.checked_add(1)
                == Some(execution.reservation.reservation_revision)
            && execution.capacity_claim.claim_id == exercise.reservation_claim.claim_id
            && reservation_claim_revision.checked_add(1)
                == Some(execution.capacity_claim.claim_revision),
        "capacity-future Job, Reservation, or Claim differs from execution source"
    );
    ensure!(
        execution_receipt.schema == COMPUTE_EXECUTION_RECEIPT_SCHEMA
            && execution_receipt.receipt_id == execution.execution_receipt.execution_receipt_id
            && execution_receipt.receipt_digest
                == execution.execution_receipt.execution_receipt_digest
            && execution_receipt.job_id == execution.job.job_id
            && execution_receipt.reservation_id == execution.reservation.reservation_id
            && execution_receipt.attempt_lease_id == execution.attempt_lease_source.lease_id
            && positive_u64(
                execution_receipt.fencing_generation,
                "Execution Receipt fencing generation",
            )? == execution.attempt_lease_source.fencing_generation
            && execution_receipt.provider_id == execution.provider.provider_id
            && execution_receipt.offer_id == execution.offer.offer_id
            && positive_u64(
                execution_receipt.offer_version,
                "Execution Receipt Offer version"
            )? == execution.offer.offer_version
            && execution_receipt.offer_digest == execution.offer.offer_digest
            && execution_receipt.verification.status == VERIFICATION_STATUS_ACCEPTED
            && execution_receipt.verification.decision_digest
                == canonical_sources
                    .verification
                    .verification_decision
                    .verification_event_digest,
        "capacity-future v193 Execution Receipt differs from execution source"
    );
    ensure!(
        canonical_sources.verification.execution_receipt == execution.execution_receipt
            && canonical_sources.verification.execution_lineage_digest
                == sources.execution_source.lineage_digest(),
        "capacity-future verification source does not close the execution source"
    );
    Ok(())
}

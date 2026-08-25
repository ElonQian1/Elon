use anyhow::{ensure, Result};
use chrono::{DateTime, SecondsFormat};

use super::{
    canonical::canonical_compute_capacity_future_settlement_lineage_json_and_digest,
    types::{
        CapacityFutureSettlementEconomicLineageV1,
        UntrustedComputeCapacityFutureSettlementLineageEnvelopeV1,
        COMPUTE_CAPACITY_FUTURE_SETTLEMENT_CURRENCY,
        COMPUTE_CAPACITY_FUTURE_SETTLEMENT_LINEAGE_CANONICALIZATION,
        COMPUTE_CAPACITY_FUTURE_SETTLEMENT_LINEAGE_DIGEST_ALGORITHM,
        COMPUTE_CAPACITY_FUTURE_SETTLEMENT_LINEAGE_KIND,
        COMPUTE_CAPACITY_FUTURE_SETTLEMENT_LINEAGE_SCHEMA,
        COMPUTE_CAPACITY_FUTURE_SETTLEMENT_NO_EFFECT,
        COMPUTE_CAPACITY_FUTURE_SETTLEMENT_PRICING_MODE,
        COMPUTE_CAPACITY_FUTURE_SETTLEMENT_REFERENCE_EFFECT,
    },
};

const MAX_IJSON_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

pub(crate) fn validate_compute_capacity_future_settlement_lineage(
    envelope: &UntrustedComputeCapacityFutureSettlementLineageEnvelopeV1,
) -> Result<()> {
    ensure!(
        envelope.schema == COMPUTE_CAPACITY_FUTURE_SETTLEMENT_LINEAGE_SCHEMA
            && envelope.lineage_kind == COMPUTE_CAPACITY_FUTURE_SETTLEMENT_LINEAGE_KIND
            && envelope.canonicalization
                == COMPUTE_CAPACITY_FUTURE_SETTLEMENT_LINEAGE_CANONICALIZATION
            && envelope.digest_algorithm
                == COMPUTE_CAPACITY_FUTURE_SETTLEMENT_LINEAGE_DIGEST_ALGORITHM,
        "capacity-future settlement lineage metadata is unsupported"
    );
    digest(&envelope.lineage_digest, "lineage digest")?;
    validate_lineage(envelope)?;
    let (_, computed_digest) =
        canonical_compute_capacity_future_settlement_lineage_json_and_digest(envelope)?;
    ensure!(
        envelope.lineage_digest == computed_digest,
        "capacity-future settlement lineage digest does not match its canonical projection"
    );
    Ok(())
}

fn validate_lineage(
    envelope: &UntrustedComputeCapacityFutureSettlementLineageEnvelopeV1,
) -> Result<()> {
    let lineage = &envelope.lineage;
    ensure!(
        lineage.pricing_mode == COMPUTE_CAPACITY_FUTURE_SETTLEMENT_PRICING_MODE
            && lineage.settlement_currency == COMPUTE_CAPACITY_FUTURE_SETTLEMENT_CURRENCY,
        "capacity-future settlement lineage pricing mode or currency is not exact"
    );

    identifier(
        &lineage.price_snapshot.price_snapshot_id,
        "price snapshot ID",
    )?;
    digest(
        &lineage.price_snapshot.price_snapshot_digest,
        "price snapshot digest",
    )?;
    identifier(
        &lineage.reference_price_binding.binding_id,
        "reference price binding ID",
    )?;
    digest(
        &lineage.reference_price_binding.binding_digest,
        "reference price binding digest",
    )?;
    validate_delivery_window(lineage)?;
    validate_capacity_roots(lineage)?;
    validate_execution_and_verification(lineage)?;
    digest(
        &lineage.settlement_usage_digests.verified_usage_digest,
        "v195 verified usage digest",
    )?;
    digest(
        &lineage.settlement_usage_digests.compensable_usage_digest,
        "v195 compensable usage digest",
    )?;
    validate_economic_lineage(&lineage.economic_lineage)?;

    let effects = &lineage.effects;
    ensure!(
        effects.reference_effect == COMPUTE_CAPACITY_FUTURE_SETTLEMENT_REFERENCE_EFFECT
            && [
                effects.capacity_effect.as_str(),
                effects.verification_effect.as_str(),
                effects.settlement_effect.as_str(),
                effects.money_effect.as_str(),
                effects.withdrawal_effect.as_str(),
            ]
            .into_iter()
            .all(|effect| effect == COMPUTE_CAPACITY_FUTURE_SETTLEMENT_NO_EFFECT),
        "capacity-future settlement lineage effects are not reference-only"
    );
    Ok(())
}

fn validate_delivery_window(
    lineage: &super::types::ComputeCapacityFutureSettlementLineageV1,
) -> Result<()> {
    let window = &lineage.delivery_window;
    identifier(&window.window_id, "delivery window ID")?;
    digest(&window.window_digest, "delivery window digest")?;
    let starts = canonical_utc_nanos(&window.starts_at_utc, "delivery window start")?;
    let ends = canonical_utc_nanos(&window.ends_at_utc, "delivery window end")?;
    ensure!(
        starts < ends,
        "capacity-future delivery window must be a positive half-open interval"
    );
    Ok(())
}

fn validate_capacity_roots(
    lineage: &super::types::ComputeCapacityFutureSettlementLineageV1,
) -> Result<()> {
    for (value, label) in [
        (
            lineage.capacity_instrument.instrument_id.as_str(),
            "capacity instrument ID",
        ),
        (
            lineage.instrument_activation.activation_receipt_id.as_str(),
            "instrument activation receipt ID",
        ),
        (
            lineage
                .instrument_offer_adoption
                .adoption_receipt_id
                .as_str(),
            "instrument adoption receipt ID",
        ),
        (
            lineage.instrument_offer_adoption.publication_id.as_str(),
            "Offer publication ID",
        ),
        (
            lineage.capacity_commitment.commitment_id.as_str(),
            "capacity commitment ID",
        ),
        (
            lineage.delivery_allocation_grant.grant_id.as_str(),
            "delivery-allocation Grant ID",
        ),
        (
            lineage
                .delivery_allocation_exercise
                .terminal_receipt_id
                .as_str(),
            "delivery-allocation exercise receipt ID",
        ),
    ] {
        identifier(value, label)?;
    }
    for (value, label) in [
        (
            lineage.capacity_instrument.instrument_digest.as_str(),
            "capacity instrument digest",
        ),
        (
            lineage
                .instrument_activation
                .activation_receipt_digest
                .as_str(),
            "instrument activation receipt digest",
        ),
        (
            lineage
                .instrument_offer_adoption
                .adoption_receipt_digest
                .as_str(),
            "instrument adoption receipt digest",
        ),
        (
            lineage
                .instrument_offer_adoption
                .publication_digest
                .as_str(),
            "Offer publication digest",
        ),
        (
            lineage.capacity_commitment.commitment_digest.as_str(),
            "capacity commitment digest",
        ),
        (
            lineage.delivery_allocation_grant.grant_digest.as_str(),
            "delivery-allocation Grant digest",
        ),
        (
            lineage
                .delivery_allocation_exercise
                .terminal_receipt_digest
                .as_str(),
            "delivery-allocation exercise receipt digest",
        ),
    ] {
        digest(value, label)?;
    }
    for (value, label) in [
        (
            lineage.capacity_instrument.instrument_revision,
            "capacity instrument revision",
        ),
        (
            lineage.capacity_commitment.commitment_revision,
            "capacity commitment revision",
        ),
        (
            lineage.delivery_allocation_grant.grant_revision,
            "delivery-allocation Grant revision",
        ),
        (
            lineage.delivery_allocation_exercise.terminal_revision,
            "delivery-allocation exercise revision",
        ),
    ] {
        positive(value, label)?;
    }
    ensure!(
        lineage.capacity_instrument.instrument_revision == 1
            && lineage.capacity_commitment.commitment_revision == 1
            && lineage.delivery_allocation_grant.grant_revision == 1
            && lineage.delivery_allocation_exercise.terminal_revision == 2,
        "capacity-future settlement lineage capacity revisions are not exact"
    );
    validate_offer_ref(&lineage.instrument_offer_adoption.offer)?;
    validate_claim_ref(&lineage.capacity_commitment.capacity_claim)?;
    validate_job_ref(&lineage.delivery_allocation_grant.job)?;
    validate_claim_ref(&lineage.delivery_allocation_exercise.parent_released_claim)?;
    validate_claim_ref(&lineage.delivery_allocation_exercise.reservation_claim)?;
    validate_reservation_ref(&lineage.delivery_allocation_exercise.exercise_reservation)?;
    validate_job_ref(&lineage.delivery_allocation_exercise.reserved_job)?;
    validate_reservation_ref(&lineage.terminal_reservation)?;
    ensure!(
        lineage.capacity_commitment.capacity_claim.claim_revision == 1
            && lineage
                .delivery_allocation_exercise
                .parent_released_claim
                .claim_id
                == lineage.capacity_commitment.capacity_claim.claim_id
            && lineage
                .capacity_commitment
                .capacity_claim
                .claim_revision
                .checked_add(1)
                == Some(
                    lineage
                        .delivery_allocation_exercise
                        .parent_released_claim
                        .claim_revision
                )
            && lineage
                .delivery_allocation_exercise
                .reservation_claim
                .claim_revision
                == 1
            && lineage
                .delivery_allocation_exercise
                .exercise_reservation
                .reservation_revision
                == 2
            && lineage.terminal_reservation.reservation_id
                == lineage
                    .delivery_allocation_exercise
                    .exercise_reservation
                    .reservation_id
            && lineage
                .delivery_allocation_exercise
                .exercise_reservation
                .reservation_revision
                .checked_add(2)
                == Some(lineage.terminal_reservation.reservation_revision)
            && lineage.delivery_allocation_grant.job.job_id
                == lineage.delivery_allocation_exercise.reserved_job.job_id
            && lineage
                .delivery_allocation_grant
                .job
                .job_revision
                .checked_add(1)
                == Some(
                    lineage
                        .delivery_allocation_exercise
                        .reserved_job
                        .job_revision
                ),
        "capacity-future settlement lineage allocation revisions or identities differ"
    );
    Ok(())
}

fn validate_execution_and_verification(
    lineage: &super::types::ComputeCapacityFutureSettlementLineageV1,
) -> Result<()> {
    digest(
        &lineage.execution_source_lineage_digest,
        "execution source lineage digest",
    )?;
    digest(
        &lineage.execution_verification_lineage_digest,
        "execution verification lineage digest",
    )?;
    identifier(
        &lineage.execution_receipt.execution_receipt_id,
        "execution receipt ID",
    )?;
    digest(
        &lineage.execution_receipt.execution_receipt_digest,
        "execution receipt digest",
    )?;
    let decision = &lineage.verification_decision;
    identifier(
        &decision.verification_decision_id,
        "verification decision ID",
    )?;
    for (value, label) in [
        (
            decision.verification_event_digest.as_str(),
            "verification event digest",
        ),
        (
            decision.verified_usage_digest.as_str(),
            "verified usage digest",
        ),
        (
            decision.compensable_usage_digest.as_str(),
            "compensable usage digest",
        ),
    ] {
        digest(value, label)?;
    }
    Ok(())
}

fn validate_economic_lineage(value: &CapacityFutureSettlementEconomicLineageV1) -> Result<()> {
    let (settlement, settlement_digest) = match value {
        CapacityFutureSettlementEconomicLineageV1::PendingSettlementSourceV1 {
            attempt_settlement,
            settlement_lineage_digest,
        } => (attempt_settlement, settlement_lineage_digest),
        CapacityFutureSettlementEconomicLineageV1::AvailableReleaseSourceV1 {
            attempt_settlement,
            settlement_lineage_digest,
            settlement_release,
            settlement_release_lineage_digest,
        } => {
            identifier(
                &settlement_release.settlement_release_id,
                "settlement release ID",
            )?;
            digest(
                &settlement_release.settlement_release_event_digest,
                "settlement release event digest",
            )?;
            digest(
                settlement_release_lineage_digest,
                "settlement release lineage digest",
            )?;
            (attempt_settlement, settlement_lineage_digest)
        }
    };
    identifier(&settlement.settlement_receipt_id, "settlement receipt ID")?;
    digest(
        &settlement.settlement_receipt_digest,
        "settlement receipt digest",
    )?;
    digest(
        &settlement.settlement_event_digest,
        "settlement event digest",
    )?;
    digest(settlement_digest, "settlement lineage digest")
}

fn validate_offer_ref(
    value: &super::super::federation_historical_causal_reference::OfferVersionRef,
) -> Result<()> {
    identifier(&value.provider_id, "Offer Provider ID")?;
    identifier(&value.offer_id, "Offer ID")?;
    positive(value.offer_version, "Offer version")?;
    digest(&value.offer_digest, "Offer digest")
}

fn validate_job_ref(
    value: &super::super::federation_historical_causal_reference::JobVersionRef,
) -> Result<()> {
    identifier(&value.job_id, "Job ID")?;
    positive(value.job_revision, "Job revision")?;
    digest(&value.job_digest, "Job digest")
}

fn validate_reservation_ref(
    value: &super::super::federation_historical_causal_reference::ReservationVersionRef,
) -> Result<()> {
    identifier(&value.reservation_id, "Reservation ID")?;
    positive(value.reservation_revision, "Reservation revision")?;
    digest(&value.reservation_digest, "Reservation digest")
}

fn validate_claim_ref(
    value: &super::super::federation_historical_causal_reference::CapacityClaimVersionRef,
) -> Result<()> {
    identifier(&value.claim_id, "Capacity Claim ID")?;
    positive(value.claim_revision, "Capacity Claim revision")?;
    digest(&value.claim_digest, "Capacity Claim digest")
}

fn identifier(value: &str, label: &str) -> Result<()> {
    ensure!(
        !value.is_empty()
            && value.trim() == value
            && value.chars().count() <= 200
            && !value.chars().any(char::is_control),
        "capacity-future settlement lineage {label} is invalid"
    );
    Ok(())
}

fn digest(value: &str, label: &str) -> Result<()> {
    ensure!(
        value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "capacity-future settlement lineage {label} is invalid"
    );
    Ok(())
}

fn positive(value: u64, label: &str) -> Result<()> {
    ensure!(
        (1..=MAX_IJSON_SAFE_INTEGER).contains(&value),
        "capacity-future settlement lineage {label} must be an I-JSON safe positive integer"
    );
    Ok(())
}

fn canonical_utc_nanos(value: &str, label: &str) -> Result<DateTime<chrono::FixedOffset>> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    ensure!(
        parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) == value,
        "capacity-future settlement lineage {label} is not canonical UTC nanoseconds"
    );
    Ok(parsed)
}

use anyhow::{ensure, Result};

use super::canonical::canonical_federation_historical_causal_reference_json_and_digest;
use super::types::{
    ExecutionSourceLineageV1, FederationHistoricalLineageKindV1, FederationHistoricalLineageV1,
    SettlementChallengeRef, SettlementChallengeResolutionRef, SettlementCorrectionRef,
    SettlementReleaseGateV1, SettlementReleaseSourceLineageV1, SettlementSourceLineageV1,
    UntrustedFederationHistoricalCausalReferenceEnvelopeV1,
    FEDERATION_HISTORICAL_CAUSAL_REFERENCE_CANONICALIZATION,
    FEDERATION_HISTORICAL_CAUSAL_REFERENCE_DIGEST_ALGORITHM,
    FEDERATION_HISTORICAL_CAUSAL_REFERENCE_SCHEMA,
};

const MAX_IJSON_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

pub(crate) fn validate_federation_historical_causal_reference(
    envelope: &UntrustedFederationHistoricalCausalReferenceEnvelopeV1,
) -> Result<()> {
    ensure!(
        envelope.schema == FEDERATION_HISTORICAL_CAUSAL_REFERENCE_SCHEMA,
        "federation historical causal reference schema is unsupported"
    );
    ensure!(
        envelope.canonicalization == FEDERATION_HISTORICAL_CAUSAL_REFERENCE_CANONICALIZATION,
        "federation historical causal reference canonicalization is unsupported"
    );
    ensure!(
        envelope.digest_algorithm == FEDERATION_HISTORICAL_CAUSAL_REFERENCE_DIGEST_ALGORITHM,
        "federation historical causal reference digest algorithm is unsupported"
    );
    ensure!(
        is_lowercase_sha256(&envelope.lineage_digest),
        "federation historical causal reference lineage digest is invalid"
    );

    match (&envelope.lineage_kind, &envelope.lineage) {
        (
            FederationHistoricalLineageKindV1::ExecutionSourceV1,
            FederationHistoricalLineageV1::ExecutionSource(lineage),
        ) => validate_execution_lineage(lineage)?,
        (
            FederationHistoricalLineageKindV1::SettlementSourceV1,
            FederationHistoricalLineageV1::SettlementSource(lineage),
        ) => validate_settlement_lineage(lineage)?,
        (
            FederationHistoricalLineageKindV1::SettlementReleaseSourceV1,
            FederationHistoricalLineageV1::SettlementReleaseSource(lineage),
        ) => validate_settlement_release_lineage(lineage)?,
        _ => {
            ensure!(
                false,
                "federation historical causal reference kind and lineage shape differ"
            );
        }
    }

    let (_, computed_digest) =
        canonical_federation_historical_causal_reference_json_and_digest(envelope)?;
    ensure!(
        envelope.lineage_digest == computed_digest,
        "federation historical causal reference lineage digest does not match its canonical projection"
    );
    Ok(())
}

fn validate_execution_lineage(lineage: &ExecutionSourceLineageV1) -> Result<()> {
    safe_positive(lineage.provider.policy_revision, "provider policy revision")?;
    safe_positive(lineage.capacity_pool.capacity_epoch, "capacity pool epoch")?;
    safe_positive(
        lineage.capacity_pool.pool_revision,
        "capacity pool revision",
    )?;
    safe_positive(lineage.offer.offer_version, "offer version")?;
    safe_positive(lineage.job.job_revision, "job revision")?;
    safe_positive(
        lineage.reservation.reservation_revision,
        "reservation revision",
    )?;
    safe_positive(
        lineage.capacity_claim.claim_revision,
        "capacity claim revision",
    )?;
    safe_positive(
        lineage.attempt_lease_source.lease_revision,
        "source lease revision",
    )?;
    safe_positive(
        lineage.attempt_lease_source.fencing_generation,
        "source lease fencing generation",
    )?;
    Ok(())
}

fn validate_settlement_lineage(lineage: &SettlementSourceLineageV1) -> Result<()> {
    ensure!(
        is_lowercase_sha256(&lineage.execution_lineage_digest),
        "settlement execution lineage digest is invalid"
    );
    safe_positive(lineage.provider.policy_revision, "provider policy revision")?;
    safe_positive(lineage.source_job.job_revision, "source job revision")?;
    safe_positive(lineage.terminal_job.job_revision, "terminal job revision")?;
    safe_positive(
        lineage.terminal_reservation.reservation_revision,
        "terminal reservation revision",
    )?;
    Ok(())
}

fn validate_settlement_release_lineage(lineage: &SettlementReleaseSourceLineageV1) -> Result<()> {
    validate_reference_id(
        &lineage.attempt_settlement.settlement_receipt_id,
        "settlement receipt ID",
    )?;
    for (digest, field) in [
        (
            lineage
                .attempt_settlement
                .settlement_receipt_digest
                .as_str(),
            "settlement receipt digest",
        ),
        (
            lineage.attempt_settlement.settlement_event_digest.as_str(),
            "settlement event digest",
        ),
        (
            lineage.settlement_lineage_digest.as_str(),
            "settlement lineage digest",
        ),
        (
            lineage
                .source_settlement_posting
                .settlement_posting_digest
                .as_str(),
            "settlement posting digest",
        ),
        (
            lineage
                .settlement_release
                .settlement_release_event_digest
                .as_str(),
            "settlement release event digest",
        ),
        (
            lineage
                .release_posting
                .settlement_release_posting_digest
                .as_str(),
            "settlement release posting digest",
        ),
    ] {
        validate_reference_digest(digest, field)?;
    }
    validate_reference_id(
        &lineage.source_settlement_posting.settlement_posting_id,
        "settlement posting ID",
    )?;
    validate_reference_id(
        &lineage.settlement_release.settlement_release_id,
        "settlement release ID",
    )?;
    validate_reference_id(
        &lineage.release_posting.settlement_release_posting_id,
        "settlement release posting ID",
    )?;

    let challenge_gate_digest = match &lineage.release_gate {
        SettlementReleaseGateV1::NoChallenge {
            challenge_gate_digest,
        } => challenge_gate_digest,
        SettlementReleaseGateV1::ResolvedChallenge {
            challenge_gate_digest,
            challenge,
            resolution,
            ..
        } => {
            validate_challenge_ref(challenge)?;
            validate_resolution_ref(resolution)?;
            challenge_gate_digest
        }
        SettlementReleaseGateV1::AcceptedCorrected {
            challenge_gate_digest,
            challenge,
            resolution,
            correction,
            correction_posting,
            ..
        } => {
            validate_challenge_ref(challenge)?;
            validate_resolution_ref(resolution)?;
            validate_correction_ref(correction)?;
            validate_reference_id(
                &correction_posting.settlement_correction_posting_id,
                "settlement correction posting ID",
            )?;
            validate_reference_digest(
                &correction_posting.settlement_correction_posting_digest,
                "settlement correction posting digest",
            )?;
            challenge_gate_digest
        }
    };
    validate_reference_digest(
        challenge_gate_digest,
        "settlement release challenge gate digest",
    )?;
    Ok(())
}

fn validate_challenge_ref(reference: &SettlementChallengeRef) -> Result<()> {
    validate_reference_id(
        &reference.settlement_challenge_id,
        "settlement challenge ID",
    )?;
    validate_reference_digest(
        &reference.settlement_challenge_event_digest,
        "settlement challenge event digest",
    )
}

fn validate_resolution_ref(reference: &SettlementChallengeResolutionRef) -> Result<()> {
    validate_reference_id(
        &reference.settlement_challenge_resolution_id,
        "settlement challenge resolution ID",
    )?;
    validate_reference_digest(
        &reference.settlement_challenge_resolution_event_digest,
        "settlement challenge resolution event digest",
    )
}

fn validate_correction_ref(reference: &SettlementCorrectionRef) -> Result<()> {
    validate_reference_id(
        &reference.settlement_correction_id,
        "settlement correction ID",
    )?;
    validate_reference_digest(
        &reference.settlement_correction_event_digest,
        "settlement correction event digest",
    )
}

fn validate_reference_id(value: &str, field: &str) -> Result<()> {
    ensure!(
        !value.is_empty() && value.trim() == value && !value.chars().any(char::is_control),
        "federation historical causal reference {field} is invalid"
    );
    Ok(())
}

fn validate_reference_digest(value: &str, field: &str) -> Result<()> {
    ensure!(
        is_lowercase_sha256(value),
        "federation historical causal reference {field} is invalid"
    );
    Ok(())
}

fn safe_positive(value: u64, field: &str) -> Result<()> {
    ensure!(
        (1..=MAX_IJSON_SAFE_INTEGER).contains(&value),
        "federation historical causal reference {field} must be a positive I-JSON safe integer"
    );
    Ok(())
}

fn is_lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
}

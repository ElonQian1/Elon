use anyhow::{ensure, Result};

use super::canonical::canonical_federation_historical_causal_reference_json_and_digest;
use super::types::{
    ExecutionSourceLineageV1, FederationHistoricalLineageKindV1, FederationHistoricalLineageV1,
    SettlementSourceLineageV1, UntrustedFederationHistoricalCausalReferenceEnvelopeV1,
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

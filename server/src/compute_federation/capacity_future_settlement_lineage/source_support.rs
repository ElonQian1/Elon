use anyhow::{ensure, Context, Result};

use super::super::federation_historical_causal_reference::{
    validate_federation_historical_causal_reference, CapacityClaimVersionRef,
    ExecutionSourceLineageV1, ExecutionVerificationSourceLineageV1, FederationHistoricalLineageV1,
    JobVersionRef, ReservationVersionRef, SettlementReleaseSourceLineageV1,
    SettlementSourceLineageV1, UntrustedFederationHistoricalCausalReferenceEnvelopeV1,
};

use super::source_inputs::{
    ComputeCapacityFutureSettlementLineageSources, ComputeCapacityFutureSettlementStageSources,
};

pub(super) struct CanonicalSourceLineages<'a> {
    pub(super) execution: &'a ExecutionSourceLineageV1,
    pub(super) verification: &'a ExecutionVerificationSourceLineageV1,
    pub(super) settlement: &'a SettlementSourceLineageV1,
    pub(super) release: Option<&'a SettlementReleaseSourceLineageV1>,
}

pub(super) fn canonical_source_lineages<'a>(
    sources: &ComputeCapacityFutureSettlementLineageSources<'a>,
) -> Result<CanonicalSourceLineages<'a>> {
    for carrier in [
        sources.execution_source,
        sources.execution_verification_source,
    ] {
        validate_federation_historical_causal_reference(carrier)?;
    }
    validate_federation_historical_causal_reference(settlement_source_carrier(sources))?;
    if let Some(release) = release_source_carrier(sources) {
        validate_federation_historical_causal_reference(release)?;
    }
    let execution = match sources.execution_source.lineage() {
        FederationHistoricalLineageV1::ExecutionSource(value) => value,
        _ => anyhow::bail!("capacity-future bridge requires execution_source_v1"),
    };
    let verification = match sources.execution_verification_source.lineage() {
        FederationHistoricalLineageV1::ExecutionVerificationSource(value) => value,
        _ => anyhow::bail!("capacity-future bridge requires execution_verification_source_v1"),
    };
    let settlement = match settlement_source_carrier(sources).lineage() {
        FederationHistoricalLineageV1::SettlementSource(value) => value,
        _ => anyhow::bail!("capacity-future bridge requires settlement_source_v1"),
    };
    let release = match release_source_carrier(sources) {
        Some(carrier) => match carrier.lineage() {
            FederationHistoricalLineageV1::SettlementReleaseSource(value) => Some(value),
            _ => anyhow::bail!(
                "capacity-future available branch requires settlement_release_source_v1"
            ),
        },
        None => None,
    };
    Ok(CanonicalSourceLineages {
        execution,
        verification,
        settlement,
        release,
    })
}

pub(super) fn settlement_source_carrier<'a>(
    sources: &ComputeCapacityFutureSettlementLineageSources<'a>,
) -> &'a UntrustedFederationHistoricalCausalReferenceEnvelopeV1 {
    match &sources.settlement_stage {
        ComputeCapacityFutureSettlementStageSources::PendingSettlementSource {
            settlement_source,
        }
        | ComputeCapacityFutureSettlementStageSources::AvailableReleaseSource {
            settlement_source,
            ..
        } => settlement_source,
    }
}

pub(super) fn release_source_carrier<'a>(
    sources: &ComputeCapacityFutureSettlementLineageSources<'a>,
) -> Option<&'a UntrustedFederationHistoricalCausalReferenceEnvelopeV1> {
    match &sources.settlement_stage {
        ComputeCapacityFutureSettlementStageSources::PendingSettlementSource { .. } => None,
        ComputeCapacityFutureSettlementStageSources::AvailableReleaseSource {
            settlement_release_source,
            ..
        } => Some(settlement_release_source),
    }
}

pub(super) fn claim_ref(id: &str, revision: i64, digest: &str) -> Result<CapacityClaimVersionRef> {
    Ok(CapacityClaimVersionRef {
        claim_id: id.to_string(),
        claim_revision: positive_u64(revision, "Capacity Claim revision")?,
        claim_digest: digest.to_string(),
    })
}

pub(super) fn job_ref(id: &str, revision: i64, digest: &str) -> Result<JobVersionRef> {
    Ok(JobVersionRef {
        job_id: id.to_string(),
        job_revision: positive_u64(revision, "Job revision")?,
        job_digest: digest.to_string(),
    })
}

pub(super) fn reservation_ref(
    id: &str,
    revision: i64,
    digest: &str,
) -> Result<ReservationVersionRef> {
    Ok(ReservationVersionRef {
        reservation_id: id.to_string(),
        reservation_revision: positive_u64(revision, "Reservation revision")?,
        reservation_digest: digest.to_string(),
    })
}

pub(super) fn positive_u64(value: i64, label: &str) -> Result<u64> {
    let value = u64::try_from(value).with_context(|| format!("{label} is negative"))?;
    ensure!(
        (1..=9_007_199_254_740_991).contains(&value),
        "{label} is not an I-JSON safe positive integer"
    );
    Ok(value)
}

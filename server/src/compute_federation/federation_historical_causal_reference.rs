//! Canonical, provider-neutral causal references for retained federation history.
//!
//! Values parsed here are intentionally untrusted. Historical owner resolution and any
//! writer-safe view remain the responsibility of the Store integration layer.

mod canonical;
mod types;
mod validation;

#[cfg(test)]
mod release_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod verification_tests;

pub(crate) use canonical::{
    build_execution_source_carrier, build_execution_verification_source_carrier,
    build_settlement_release_source_carrier, build_settlement_source_carrier,
    canonical_federation_historical_causal_reference_json_and_digest,
    federation_historical_causal_reference_from_json,
    federation_historical_causal_reference_from_json_bytes,
};
pub(crate) use types::{
    AttemptLeaseSourceRef, AttemptSettlementRef, CapacityClaimVersionRef, CapacityPoolVersionRef,
    ConsumerReviewRef, ExecutionReceiptRef, ExecutionSourceLineageV1,
    ExecutionVerificationSourceLineageV1, FederationHistoricalLineageKindV1,
    FederationHistoricalLineageV1, FinalizationRef, JobVersionRef, OfferVersionRef,
    PlatformObservationRef, PriceSnapshotRef, ProviderDeclaredUsageRef, ProviderVersionRef,
    ReservationVersionRef, SettlementChallengeRef, SettlementChallengeResolutionActionV1,
    SettlementChallengeResolutionRef, SettlementCorrectionPostingRef, SettlementCorrectionRef,
    SettlementReleaseGateV1, SettlementReleasePostingRef, SettlementReleaseRef,
    SettlementReleaseSourceLineageV1, SettlementSourceLineageV1, SettlementSourcePostingRef,
    TerminalCandidateRef, UntrustedFederationHistoricalCausalReferenceEnvelopeV1,
    VerificationDecisionRef, FEDERATION_HISTORICAL_CAUSAL_REFERENCE_CANONICALIZATION,
    FEDERATION_HISTORICAL_CAUSAL_REFERENCE_DIGEST_ALGORITHM,
    FEDERATION_HISTORICAL_CAUSAL_REFERENCE_DIGEST_DOMAIN,
    FEDERATION_HISTORICAL_CAUSAL_REFERENCE_MAX_JSON_BYTES,
    FEDERATION_HISTORICAL_CAUSAL_REFERENCE_SCHEMA,
};
pub(crate) use validation::validate_federation_historical_causal_reference;

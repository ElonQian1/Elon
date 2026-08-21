//! Canonical, provider-neutral causal references for retained federation history.
//!
//! Values parsed here are intentionally untrusted. Historical owner resolution and any
//! writer-safe view remain the responsibility of the Store integration layer.

mod canonical;
mod types;
mod validation;

#[cfg(test)]
mod tests;

pub(crate) use canonical::{
    build_execution_source_carrier, build_settlement_source_carrier,
    canonical_federation_historical_causal_reference_json_and_digest,
    federation_historical_causal_reference_from_json,
    federation_historical_causal_reference_from_json_bytes,
};
pub(crate) use types::{
    AttemptLeaseSourceRef, AttemptSettlementRef, CapacityClaimVersionRef, CapacityPoolVersionRef,
    ExecutionReceiptRef, ExecutionSourceLineageV1, FederationHistoricalLineageKindV1,
    FederationHistoricalLineageV1, FinalizationRef, JobVersionRef, OfferVersionRef,
    PriceSnapshotRef, ProviderVersionRef, ReservationVersionRef, SettlementSourceLineageV1,
    UntrustedFederationHistoricalCausalReferenceEnvelopeV1,
    FEDERATION_HISTORICAL_CAUSAL_REFERENCE_CANONICALIZATION,
    FEDERATION_HISTORICAL_CAUSAL_REFERENCE_DIGEST_ALGORITHM,
    FEDERATION_HISTORICAL_CAUSAL_REFERENCE_DIGEST_DOMAIN,
    FEDERATION_HISTORICAL_CAUSAL_REFERENCE_MAX_JSON_BYTES,
    FEDERATION_HISTORICAL_CAUSAL_REFERENCE_SCHEMA,
};
pub(crate) use validation::validate_federation_historical_causal_reference;

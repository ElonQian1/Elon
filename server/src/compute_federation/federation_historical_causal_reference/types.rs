use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::canonical::canonical_federation_historical_causal_reference_json_and_digest;
use super::validation::validate_federation_historical_causal_reference;

pub(crate) const FEDERATION_HISTORICAL_CAUSAL_REFERENCE_SCHEMA: &str =
    "compute_federation.core_historical_causal_reference.v1";
pub(crate) const FEDERATION_HISTORICAL_CAUSAL_REFERENCE_DIGEST_DOMAIN: &str =
    "ELON-COMPUTE-CORE-HISTORICAL-LINEAGE-V1";
pub(crate) const FEDERATION_HISTORICAL_CAUSAL_REFERENCE_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const FEDERATION_HISTORICAL_CAUSAL_REFERENCE_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const FEDERATION_HISTORICAL_CAUSAL_REFERENCE_MAX_JSON_BYTES: usize = 262_144;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProviderVersionRef {
    pub(crate) provider_id: String,
    pub(crate) policy_revision: u64,
    pub(crate) provider_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct CapacityPoolVersionRef {
    pub(crate) pool_id: String,
    pub(crate) capacity_epoch: u64,
    pub(crate) pool_revision: u64,
    pub(crate) pool_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct OfferVersionRef {
    pub(crate) provider_id: String,
    pub(crate) offer_id: String,
    pub(crate) offer_version: u64,
    pub(crate) offer_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PriceSnapshotRef {
    pub(crate) price_snapshot_id: String,
    pub(crate) price_snapshot_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct JobVersionRef {
    pub(crate) job_id: String,
    pub(crate) job_revision: u64,
    pub(crate) job_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReservationVersionRef {
    pub(crate) reservation_id: String,
    pub(crate) reservation_revision: u64,
    pub(crate) reservation_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct CapacityClaimVersionRef {
    pub(crate) claim_id: String,
    pub(crate) claim_revision: u64,
    pub(crate) claim_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct AttemptLeaseSourceRef {
    pub(crate) lease_id: String,
    pub(crate) lease_revision: u64,
    pub(crate) lease_digest: String,
    pub(crate) fencing_generation: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExecutionReceiptRef {
    pub(crate) execution_receipt_id: String,
    pub(crate) execution_receipt_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct FinalizationRef {
    pub(crate) finalization_id: String,
    pub(crate) finalization_event_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct AttemptSettlementRef {
    pub(crate) settlement_receipt_id: String,
    pub(crate) settlement_receipt_digest: String,
    pub(crate) settlement_event_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExecutionSourceLineageV1 {
    pub(crate) execution_receipt: ExecutionReceiptRef,
    pub(crate) provider: ProviderVersionRef,
    pub(crate) capacity_pool: CapacityPoolVersionRef,
    pub(crate) offer: OfferVersionRef,
    pub(crate) price_snapshot: PriceSnapshotRef,
    pub(crate) job: JobVersionRef,
    pub(crate) reservation: ReservationVersionRef,
    pub(crate) capacity_claim: CapacityClaimVersionRef,
    pub(crate) attempt_lease_source: AttemptLeaseSourceRef,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct SettlementSourceLineageV1 {
    pub(crate) attempt_settlement: AttemptSettlementRef,
    pub(crate) execution_receipt: ExecutionReceiptRef,
    pub(crate) execution_lineage_digest: String,
    pub(crate) finalization: FinalizationRef,
    pub(crate) price_snapshot: PriceSnapshotRef,
    pub(crate) provider: ProviderVersionRef,
    pub(crate) source_job: JobVersionRef,
    pub(crate) terminal_job: JobVersionRef,
    pub(crate) terminal_reservation: ReservationVersionRef,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FederationHistoricalLineageKindV1 {
    ExecutionSourceV1,
    SettlementSourceV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub(crate) enum FederationHistoricalLineageV1 {
    ExecutionSource(ExecutionSourceLineageV1),
    SettlementSource(SettlementSourceLineageV1),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct UntrustedFederationHistoricalCausalReferenceEnvelopeV1 {
    pub(crate) schema: String,
    pub(crate) lineage_kind: FederationHistoricalLineageKindV1,
    pub(crate) lineage_digest: String,
    pub(crate) canonicalization: String,
    pub(crate) digest_algorithm: String,
    pub(crate) lineage: FederationHistoricalLineageV1,
}

impl UntrustedFederationHistoricalCausalReferenceEnvelopeV1 {
    pub(crate) fn lineage_kind(&self) -> FederationHistoricalLineageKindV1 {
        self.lineage_kind
    }

    pub(crate) fn lineage_digest(&self) -> &str {
        &self.lineage_digest
    }

    pub(crate) fn lineage(&self) -> &FederationHistoricalLineageV1 {
        &self.lineage
    }

    pub(crate) fn canonical_json(&self) -> Result<String> {
        validate_federation_historical_causal_reference(self)?;
        canonical_federation_historical_causal_reference_json_and_digest(self).map(|(json, _)| json)
    }
}

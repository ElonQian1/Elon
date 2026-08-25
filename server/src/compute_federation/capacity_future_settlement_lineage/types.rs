use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::super::{
    capacity_commitment::ComputeCapacityCommitmentReferenceBinding,
    federation_historical_causal_reference::{
        AttemptSettlementRef, CapacityClaimVersionRef, ExecutionReceiptRef, JobVersionRef,
        OfferVersionRef, PriceSnapshotRef, ReservationVersionRef, SettlementReleaseRef,
        VerificationDecisionRef,
    },
};

pub(crate) const COMPUTE_CAPACITY_FUTURE_SETTLEMENT_LINEAGE_SCHEMA: &str =
    "compute_federation.capacity_future_settlement_lineage_bridge.v1";
pub(crate) const COMPUTE_CAPACITY_FUTURE_SETTLEMENT_LINEAGE_KIND: &str =
    "capacity_future_settlement_bridge_v1";
pub(crate) const COMPUTE_CAPACITY_FUTURE_SETTLEMENT_LINEAGE_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const COMPUTE_CAPACITY_FUTURE_SETTLEMENT_LINEAGE_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const COMPUTE_CAPACITY_FUTURE_SETTLEMENT_LINEAGE_DIGEST_DOMAIN: &str =
    "ELON-COMPUTE-CAPACITY-FUTURE-SETTLEMENT-LINEAGE-BRIDGE-V1";
pub(crate) const COMPUTE_CAPACITY_FUTURE_SETTLEMENT_LINEAGE_MAX_JSON_BYTES: usize = 262_144;
pub(crate) const COMPUTE_CAPACITY_FUTURE_SETTLEMENT_PRICING_MODE: &str = "capacity_future";
pub(crate) const COMPUTE_CAPACITY_FUTURE_SETTLEMENT_CURRENCY: &str = "CNY";
pub(crate) const COMPUTE_CAPACITY_FUTURE_SETTLEMENT_REFERENCE_EFFECT: &str =
    "retained_references_only";
pub(crate) const COMPUTE_CAPACITY_FUTURE_SETTLEMENT_NO_EFFECT: &str = "none";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CapacityFutureDeliveryWindowRef {
    pub(crate) window_id: String,
    pub(crate) window_digest: String,
    pub(crate) starts_at_utc: String,
    pub(crate) ends_at_utc: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CapacityFutureInstrumentRef {
    pub(crate) instrument_id: String,
    pub(crate) instrument_revision: u64,
    pub(crate) instrument_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CapacityFutureInstrumentActivationRef {
    pub(crate) activation_receipt_id: String,
    pub(crate) activation_receipt_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CapacityFutureInstrumentOfferAdoptionRef {
    pub(crate) adoption_receipt_id: String,
    pub(crate) adoption_receipt_digest: String,
    pub(crate) offer: OfferVersionRef,
    pub(crate) publication_id: String,
    pub(crate) publication_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CapacityFutureCommitmentRef {
    pub(crate) commitment_id: String,
    pub(crate) commitment_revision: u64,
    pub(crate) commitment_digest: String,
    pub(crate) capacity_claim: CapacityClaimVersionRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CapacityFutureDeliveryAllocationGrantRef {
    pub(crate) grant_id: String,
    pub(crate) grant_revision: u64,
    pub(crate) grant_digest: String,
    pub(crate) job: JobVersionRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CapacityFutureDeliveryAllocationExerciseRef {
    pub(crate) terminal_receipt_id: String,
    pub(crate) terminal_revision: u64,
    pub(crate) terminal_receipt_digest: String,
    pub(crate) parent_released_claim: CapacityClaimVersionRef,
    pub(crate) reservation_claim: CapacityClaimVersionRef,
    pub(crate) exercise_reservation: ReservationVersionRef,
    pub(crate) reserved_job: JobVersionRef,
}

/// v195 settlement-role digests. These are deliberately distinct from the v192
/// verification-role digests retained in `VerificationDecisionRef`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CapacityFutureSettlementUsageDigestRefsV1 {
    pub(crate) verified_usage_digest: String,
    pub(crate) compensable_usage_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "economic_stage", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum CapacityFutureSettlementEconomicLineageV1 {
    PendingSettlementSourceV1 {
        attempt_settlement: AttemptSettlementRef,
        settlement_lineage_digest: String,
    },
    AvailableReleaseSourceV1 {
        attempt_settlement: AttemptSettlementRef,
        settlement_lineage_digest: String,
        settlement_release: SettlementReleaseRef,
        settlement_release_lineage_digest: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CapacityFutureSettlementLineageEffectsV1 {
    pub(crate) reference_effect: String,
    pub(crate) capacity_effect: String,
    pub(crate) verification_effect: String,
    pub(crate) settlement_effect: String,
    pub(crate) money_effect: String,
    pub(crate) withdrawal_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeCapacityFutureSettlementLineageV1 {
    pub(crate) pricing_mode: String,
    pub(crate) settlement_currency: String,
    pub(crate) price_snapshot: PriceSnapshotRef,
    pub(crate) reference_price_binding: ComputeCapacityCommitmentReferenceBinding,
    pub(crate) delivery_window: CapacityFutureDeliveryWindowRef,
    pub(crate) capacity_instrument: CapacityFutureInstrumentRef,
    pub(crate) instrument_activation: CapacityFutureInstrumentActivationRef,
    pub(crate) instrument_offer_adoption: CapacityFutureInstrumentOfferAdoptionRef,
    pub(crate) capacity_commitment: CapacityFutureCommitmentRef,
    pub(crate) delivery_allocation_grant: CapacityFutureDeliveryAllocationGrantRef,
    pub(crate) delivery_allocation_exercise: CapacityFutureDeliveryAllocationExerciseRef,
    pub(crate) terminal_reservation: ReservationVersionRef,
    pub(crate) execution_source_lineage_digest: String,
    pub(crate) execution_receipt: ExecutionReceiptRef,
    pub(crate) execution_verification_lineage_digest: String,
    pub(crate) verification_decision: VerificationDecisionRef,
    pub(crate) settlement_usage_digests: CapacityFutureSettlementUsageDigestRefsV1,
    pub(crate) economic_lineage: CapacityFutureSettlementEconomicLineageV1,
    pub(crate) effects: CapacityFutureSettlementLineageEffectsV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UntrustedComputeCapacityFutureSettlementLineageEnvelopeV1 {
    pub(crate) schema: String,
    pub(crate) lineage_kind: String,
    pub(crate) lineage_digest: String,
    pub(crate) canonicalization: String,
    pub(crate) digest_algorithm: String,
    pub(crate) lineage: ComputeCapacityFutureSettlementLineageV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProjectedComputeCapacityFutureSettlementLineageV1 {
    pub(super) envelope: UntrustedComputeCapacityFutureSettlementLineageEnvelopeV1,
}

impl ProjectedComputeCapacityFutureSettlementLineageV1 {
    pub(crate) fn envelope(&self) -> &UntrustedComputeCapacityFutureSettlementLineageEnvelopeV1 {
        &self.envelope
    }

    pub(crate) fn lineage_digest(&self) -> &str {
        &self.envelope.lineage_digest
    }

    pub(crate) fn lineage(&self) -> &ComputeCapacityFutureSettlementLineageV1 {
        &self.envelope.lineage
    }

    pub(crate) fn canonical_json(&self) -> Result<String> {
        super::canonical::canonical_compute_capacity_future_settlement_lineage_json_and_digest(
            &self.envelope,
        )
        .map(|(json, _)| json)
    }
}

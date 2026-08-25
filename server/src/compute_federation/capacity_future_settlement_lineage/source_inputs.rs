use super::super::{
    capacity_commitment::ComputeCapacityCommitment,
    capacity_instrument::{
        ComputeCapacityInstrument, ComputeCapacityInstrumentActivationReceipt,
        ComputeCapacityInstrumentOfferAdoptionReceipt,
    },
    delivery_allocation::{
        ComputeDeliveryAllocationGrant, ComputeDeliveryAllocationTerminalReceipt,
    },
    execution::ComputeJobVersionBinding,
    federation_historical_causal_reference::UntrustedFederationHistoricalCausalReferenceEnvelopeV1,
    market::ComputePriceSnapshot,
    receipts::{ComputeExecutionReceipt, ComputeSettlementReceipt},
};

pub(crate) enum ComputeCapacityFutureSettlementStageSources<'a> {
    PendingSettlementSource {
        settlement_source: &'a UntrustedFederationHistoricalCausalReferenceEnvelopeV1,
    },
    AvailableReleaseSource {
        settlement_source: &'a UntrustedFederationHistoricalCausalReferenceEnvelopeV1,
        settlement_release_source: &'a UntrustedFederationHistoricalCausalReferenceEnvelopeV1,
    },
}

/// Read-only projection of fields owned by the outer v195 receipt. This is caller-supplied and
/// untrusted; a future trusted resolver must construct it from an audited owner body.
pub(crate) struct UntrustedCapacityFutureAttemptSettlementAuditView<'a> {
    pub(crate) settlement: &'a ComputeSettlementReceipt,
    pub(crate) settlement_event_digest: &'a str,
    pub(crate) lease_id: &'a str,
    pub(crate) finalization_id: &'a str,
    pub(crate) finalization_event_digest: &'a str,
    pub(crate) budget_reservation_id: &'a str,
    pub(crate) budget_reserved_fen: i64,
    pub(crate) provider_policy_revision: i64,
    pub(crate) provider_digest: &'a str,
    pub(crate) source_job: &'a ComputeJobVersionBinding,
    pub(crate) terminal_job: &'a ComputeJobVersionBinding,
}

pub(crate) struct ComputeCapacityFutureSettlementLineageSources<'a> {
    pub(crate) instrument: &'a ComputeCapacityInstrument,
    pub(crate) instrument_activation: &'a ComputeCapacityInstrumentActivationReceipt,
    pub(crate) instrument_offer_adoption: &'a ComputeCapacityInstrumentOfferAdoptionReceipt,
    pub(crate) commitment: &'a ComputeCapacityCommitment,
    pub(crate) delivery_allocation_grant: &'a ComputeDeliveryAllocationGrant,
    pub(crate) delivery_allocation_exercise: &'a ComputeDeliveryAllocationTerminalReceipt,
    pub(crate) price_snapshot: &'a ComputePriceSnapshot,
    pub(crate) execution_receipt: &'a ComputeExecutionReceipt,
    pub(crate) attempt_settlement: UntrustedCapacityFutureAttemptSettlementAuditView<'a>,
    pub(crate) execution_source: &'a UntrustedFederationHistoricalCausalReferenceEnvelopeV1,
    pub(crate) execution_verification_source:
        &'a UntrustedFederationHistoricalCausalReferenceEnvelopeV1,
    pub(crate) settlement_stage: ComputeCapacityFutureSettlementStageSources<'a>,
}

use serde::{Deserialize, Serialize};

use super::{
    capacity::{
        ComputeCapacityClaimBinding, ComputeCapacityOfferBinding, ComputeCapacityPoolBinding,
    },
    market::ComputeDeliveryWindow,
};

pub(crate) const COMPUTE_CAPACITY_COMMITMENT_SCHEMA: &str =
    "compute_federation.capacity_commitment.v1";
pub(crate) const COMPUTE_CAPACITY_COMMITMENT_TERMINAL_RECEIPT_SCHEMA: &str =
    "compute_federation.capacity_commitment_terminal_receipt.v1";

pub(crate) const CAPACITY_COMMITMENT_STATUS_COMMITTED: &str = "committed";
pub(crate) const CAPACITY_COMMITMENT_STATUS_CANCELED: &str = "canceled";
pub(crate) const CAPACITY_COMMITMENT_STATUS_EXPIRED: &str = "expired";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeCapacityCommitmentProviderBinding {
    pub provider_id: String,
    pub policy_revision: i64,
    pub provider_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeCapacityCommitmentReferenceBinding {
    pub binding_id: String,
    pub binding_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeCapacityCommitmentLedgerBinding {
    pub transaction_id: String,
    pub transaction_digest: String,
    pub ledger_sequence: i64,
    pub event_kind: String,
    pub causal_transaction_id: Option<String>,
}

/// Immutable revision-one fact. Meter quantities remain authoritative in the bound Claim lines.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeCapacityCommitment {
    pub schema: String,
    pub commitment_id: String,
    pub commitment_revision: i64,
    pub commitment_digest: String,
    pub commitment_status: String,
    pub owner_account_id: String,
    pub provider: ComputeCapacityCommitmentProviderBinding,
    pub offer: ComputeCapacityOfferBinding,
    pub pool: ComputeCapacityPoolBinding,
    pub delivery_window: ComputeDeliveryWindow,
    pub price_snapshot_id: String,
    pub price_snapshot_digest: String,
    pub reference_binding: ComputeCapacityCommitmentReferenceBinding,
    pub instrument_id: String,
    pub claim: ComputeCapacityClaimBinding,
    pub creation_ledger: ComputeCapacityCommitmentLedgerBinding,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub request_digest: String,
    pub created_at: String,
    pub expires_at: String,
}

/// The only possible revision-two fact. A commitment has at most one terminal receipt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeCapacityCommitmentTerminalReceipt {
    pub schema: String,
    pub terminal_receipt_id: String,
    pub terminal_revision: i64,
    pub terminal_receipt_digest: String,
    pub terminal_status: String,
    pub commitment_id: String,
    pub commitment_digest: String,
    pub claim_id: String,
    pub prior_claim_revision: i64,
    pub prior_claim_digest: String,
    pub result_claim_revision: i64,
    pub result_claim_digest: String,
    pub result_claim_state: String,
    pub ledger: ComputeCapacityCommitmentLedgerBinding,
    pub actor_kind: String,
    pub actor_id: String,
    pub reason: Option<String>,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub request_digest: String,
    pub occurred_at: String,
    pub recorded_at: String,
}

/// API callers name meters, never bucket IDs. The Store resolves exact-window buckets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeCapacityCommitmentQuantity {
    pub meter: String,
    pub quantity_units: i64,
}

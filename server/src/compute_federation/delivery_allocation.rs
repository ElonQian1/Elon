//! Immutable whole-only DeliveryAllocation contracts.
//!
//! A Grant binds one v225 CapacityCommitment to one exact quoted Job. Exercising it does not
//! create an allocation Claim: the persisted evidence proves an atomic transfer from the parent
//! Commitment Claim to a standard parented Reservation Claim and the existing Broker result.

use serde::{Deserialize, Serialize};

use super::execution::ComputeJobVersionBinding;

pub(crate) const COMPUTE_DELIVERY_ALLOCATION_GRANT_SCHEMA: &str =
    "compute_federation.delivery_allocation_grant.v1";
pub(crate) const COMPUTE_DELIVERY_ALLOCATION_TERMINAL_RECEIPT_SCHEMA: &str =
    "compute_federation.delivery_allocation_terminal_receipt.v1";
pub(crate) const COMPUTE_DELIVERY_ALLOCATION_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const COMPUTE_DELIVERY_ALLOCATION_DIGEST_ALGORITHM: &str = "sha256";

pub(crate) const DELIVERY_ALLOCATION_STATUS_GRANTED: &str = "granted";
pub(crate) const DELIVERY_ALLOCATION_STATUS_EXERCISED: &str = "exercised";
pub(crate) const DELIVERY_ALLOCATION_STATUS_DECLINED: &str = "declined";
pub(crate) const DELIVERY_ALLOCATION_STATUS_EXPIRED: &str = "expired";
pub(crate) const DELIVERY_ALLOCATION_ACTOR_CONSUMER: &str = "consumer";
pub(crate) const DELIVERY_ALLOCATION_ACTOR_ADMIN: &str = "admin";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeDeliveryAllocationCommitmentBinding {
    pub commitment_id: String,
    pub commitment_revision: i64,
    pub commitment_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeDeliveryAllocationLedgerEvidence {
    pub transaction_id: String,
    pub transaction_digest: String,
    pub ledger_sequence: i64,
    pub event_kind: String,
    pub causal_transaction_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeDeliveryAllocationReservationClaimEvidence {
    pub claim_id: String,
    pub claim_revision: i64,
    pub claim_digest: String,
    pub parent_claim_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeDeliveryAllocationReservationEvidence {
    pub reservation_id: String,
    pub reservation_revision: i64,
    pub reservation_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeDeliveryAllocationExerciseEvidence {
    pub parent_claim_id: String,
    pub parent_prior_claim_revision: i64,
    pub parent_prior_claim_digest: String,
    pub parent_result_claim_revision: i64,
    pub parent_result_claim_digest: String,
    pub parent_result_claim_state: String,
    pub parent_release_ledger: ComputeDeliveryAllocationLedgerEvidence,
    pub reservation_claim: ComputeDeliveryAllocationReservationClaimEvidence,
    pub reservation_hold_ledger: ComputeDeliveryAllocationLedgerEvidence,
    pub reservation: ComputeDeliveryAllocationReservationEvidence,
    pub source_job_revision: i64,
    pub source_job_digest: String,
    pub reserved_job_revision: i64,
    pub reserved_job_digest: String,
    pub budget_reservation_id: String,
    pub reserved_amount_fen: i64,
    pub broker_reserve_request_digest: String,
}

/// Immutable revision-one bilateral authorization. Quantities remain only in the parent Claim.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeDeliveryAllocationGrant {
    pub schema: String,
    pub grant_id: String,
    pub grant_revision: i64,
    pub grant_digest: String,
    pub grant_status: String,
    pub commitment: ComputeDeliveryAllocationCommitmentBinding,
    pub provider_owner_account_id: String,
    pub consumer_account_id: String,
    pub project_id: Option<String>,
    pub job: ComputeJobVersionBinding,
    pub exercise_expires_at: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub request_digest: String,
    pub created_at: String,
}

/// The only revision-two fact for a Grant. Non-exercise terminals carry no economic evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeDeliveryAllocationTerminalReceipt {
    pub schema: String,
    pub terminal_receipt_id: String,
    pub terminal_revision: i64,
    pub terminal_receipt_digest: String,
    pub terminal_status: String,
    pub grant_id: String,
    pub grant_digest: String,
    pub commitment: ComputeDeliveryAllocationCommitmentBinding,
    pub actor_kind: String,
    pub actor_id: String,
    pub exercise: Option<ComputeDeliveryAllocationExerciseEvidence>,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub request_digest: String,
    pub occurred_at: String,
    pub recorded_at: String,
}

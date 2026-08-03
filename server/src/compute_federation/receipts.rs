use serde::{Deserialize, Serialize};

use super::workload::ComputeArtifactRef;

pub(crate) const COMPUTE_EXECUTION_RECEIPT_SCHEMA: &str = "compute_federation.execution_receipt.v1";
pub(crate) const COMPUTE_SETTLEMENT_RECEIPT_SCHEMA: &str =
    "compute_federation.settlement_receipt.v1";

pub(crate) const VERIFICATION_STATUS_PENDING: &str = "pending";
pub(crate) const VERIFICATION_STATUS_ACCEPTED: &str = "accepted";
pub(crate) const VERIFICATION_STATUS_REJECTED: &str = "rejected";
pub(crate) const VERIFICATION_STATUS_DISPUTED: &str = "disputed";

pub(crate) const BALANCE_STATE_PENDING: &str = "pending";
pub(crate) const BALANCE_STATE_DISPUTED: &str = "disputed";
pub(crate) const BALANCE_STATE_AVAILABLE: &str = "available";
pub(crate) const BALANCE_STATE_VOIDED: &str = "voided";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeMeterReading {
    pub meter: String,
    pub quantity: i64,
    pub source_kind: String,
    pub source_id: String,
    pub reading_digest: String,
    pub observed_at: String,
}

/// Provider claims, platform observations and accepted facts are deliberately separate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeExecutionUsage {
    pub declared_usage: Vec<ComputeMeterReading>,
    pub observed_usage: Vec<ComputeMeterReading>,
    pub verified_usage: Vec<ComputeMeterReading>,
    pub compensable_usage: Vec<ComputeMeterReading>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeAttestationEvidence {
    pub evidence_kind: String,
    pub issuer: String,
    pub evidence_digest: String,
    pub artifact_ref: Option<String>,
    pub observed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeVerificationDecision {
    pub status: String,
    pub policy_id: String,
    pub policy_version: i64,
    pub reason_codes: Vec<String>,
    pub duplicate_receipt_ids: Vec<String>,
    pub challenge_receipt_ids: Vec<String>,
    pub decision_digest: String,
    pub decided_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeExecutionReceipt {
    pub schema: String,
    pub receipt_id: String,
    pub receipt_digest: String,
    pub job_id: String,
    pub reservation_id: String,
    pub attempt_lease_id: String,
    pub attempt_no: i64,
    pub fencing_generation: i64,
    pub provider_id: String,
    pub executor_id: String,
    pub offer_id: String,
    pub offer_version: i64,
    pub offer_digest: String,
    pub plugin_digest: Option<String>,
    pub runner_digest: String,
    pub model_digest: Option<String>,
    pub tokenizer_digest: Option<String>,
    pub input_digest: String,
    pub output_digest: Option<String>,
    pub result_artifacts: Vec<ComputeArtifactRef>,
    pub execution_status: String,
    pub usage: ComputeExecutionUsage,
    pub attestations: Vec<ComputeAttestationEvidence>,
    pub verification: ComputeVerificationDecision,
    pub started_at: String,
    pub finished_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeSettlementAmounts {
    pub consumer_charge_micros: i64,
    pub provider_payable_micros: i64,
    pub platform_margin_micros: i64,
    pub third_party_cost_micros: i64,
    pub transfer_fee_micros: i64,
    pub storage_fee_micros: i64,
    pub verification_fee_micros: i64,
    pub availability_bonus_micros: i64,
    pub acceptance_bonus_micros: i64,
    pub delivery_penalty_micros: i64,
    pub refund_micros: i64,
}

/// Append-only settlement result. Corrections reference, but never replace, an earlier receipt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeSettlementReceipt {
    pub schema: String,
    pub settlement_receipt_id: String,
    pub settlement_receipt_digest: String,
    pub execution_receipt_id: String,
    pub execution_receipt_digest: String,
    pub reservation_id: String,
    pub price_snapshot_id: String,
    pub price_snapshot_digest: String,
    pub consumer_account_id: String,
    pub provider_account_id: String,
    pub currency: String,
    pub amounts: ComputeSettlementAmounts,
    pub verified_usage_digest: String,
    pub compensable_usage_digest: String,
    pub balance_state: String,
    pub correction_of_receipt_id: Option<String>,
    pub ledger_posting_ref: Option<String>,
    pub reason_codes: Vec<String>,
    pub created_at: String,
    pub available_at: Option<String>,
}

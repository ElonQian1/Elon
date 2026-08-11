use serde::Serialize;

use crate::compute_federation::{
    capacity::ComputeCapacityPoolBinding,
    capacity_commitment::{
        ComputeCapacityCommitment, ComputeCapacityCommitmentQuantity,
        ComputeCapacityCommitmentTerminalReceipt,
    },
    market::ComputeDeliveryWindowBinding,
};

pub(crate) const COMPUTE_CAPACITY_COMMITMENT_CREATE_CONFIRMATION: &str =
    "confirm_compute_capacity_commitment_create";
pub(crate) const COMPUTE_CAPACITY_COMMITMENT_CANCEL_CONFIRMATION: &str =
    "confirm_compute_capacity_commitment_cancel";
pub(crate) const COMPUTE_CAPACITY_COMMITMENT_EXPIRE_DUE_CONFIRMATION: &str =
    "confirm_compute_capacity_commitment_expire_due";

#[derive(Debug, Clone)]
pub(crate) struct CreateComputeCapacityCommitment {
    pub owner_account_id: String,
    pub provider_id: String,
    pub provider_policy_revision: i64,
    pub provider_digest: String,
    pub offer_id: String,
    pub offer_version: i64,
    pub offer_digest: String,
    pub pool: ComputeCapacityPoolBinding,
    pub delivery_window: ComputeDeliveryWindowBinding,
    pub price_snapshot_id: String,
    pub price_snapshot_digest: String,
    pub reference_binding_id: String,
    pub reference_binding_digest: String,
    pub instrument_id: String,
    pub quantities: Vec<ComputeCapacityCommitmentQuantity>,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub confirmation: String,
}

#[derive(Debug, Clone)]
pub(crate) struct CancelComputeCapacityCommitment {
    pub owner_account_id: String,
    pub provider_id: String,
    pub pool_id: String,
    pub commitment_id: String,
    pub expected_commitment_revision: i64,
    pub expected_commitment_digest: String,
    pub reason: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub confirmation: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ExpireDueComputeCapacityCommitments {
    pub admin_user_id: String,
    pub limit: usize,
    pub confirmation: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeCapacityCommitmentCreateReceipt {
    pub commitment: ComputeCapacityCommitment,
    pub quantities: Vec<ComputeCapacityCommitmentQuantity>,
    pub replayed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeCapacityCommitmentTerminalWriteReceipt {
    pub commitment: ComputeCapacityCommitment,
    pub terminal_receipt: ComputeCapacityCommitmentTerminalReceipt,
    pub quantities: Vec<ComputeCapacityCommitmentQuantity>,
    pub replayed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeCapacityCommitmentDetail {
    pub commitment: ComputeCapacityCommitment,
    pub terminal_receipt: Option<ComputeCapacityCommitmentTerminalReceipt>,
    pub current_status: String,
    pub quantities: Vec<ComputeCapacityCommitmentQuantity>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeCapacityCommitmentExpiryItem {
    pub commitment_id: String,
    pub status: String,
    pub replayed: bool,
    pub terminal_receipt: Option<ComputeCapacityCommitmentTerminalReceipt>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeCapacityCommitmentExpiryReport {
    pub recovery_started_at: String,
    pub selected_count: usize,
    pub expired_count: usize,
    pub replayed_count: usize,
    pub failed_count: usize,
    pub items: Vec<ComputeCapacityCommitmentExpiryItem>,
}

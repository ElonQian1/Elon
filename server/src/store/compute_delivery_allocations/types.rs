use serde::Serialize;

use crate::compute_federation::{
    capacity::{
        ComputeCapacityClaim, ComputeCapacityClaimBinding, ComputeCapacityOfferBinding,
        ComputeCapacityPoolBinding,
    },
    capacity_commitment::ComputeCapacityCommitment,
    delivery_allocation::{
        ComputeDeliveryAllocationGrant, ComputeDeliveryAllocationLedgerEvidence,
        ComputeDeliveryAllocationTerminalReceipt, DELIVERY_ALLOCATION_STATUS_DECLINED,
        DELIVERY_ALLOCATION_STATUS_EXERCISED, DELIVERY_ALLOCATION_STATUS_EXPIRED,
        DELIVERY_ALLOCATION_STATUS_GRANTED,
    },
    execution::{ComputeJobVersionBinding, ComputeReservedCapacity},
    market::ComputeDeliveryWindowBinding,
};

use super::super::compute_job_registry::ComputeJobRegistrationReceipt;

pub(crate) const COMPUTE_DELIVERY_ALLOCATION_GRANT_CONFIRMATION: &str =
    "confirm_compute_delivery_allocation_grant";
pub(crate) const COMPUTE_DELIVERY_ALLOCATION_EXERCISE_CONFIRMATION: &str =
    "confirm_compute_delivery_allocation_exercise";
pub(crate) const COMPUTE_DELIVERY_ALLOCATION_DECLINE_CONFIRMATION: &str =
    "confirm_compute_delivery_allocation_decline";
pub(crate) const COMPUTE_DELIVERY_ALLOCATION_EXPIRE_DUE_CONFIRMATION: &str =
    "confirm_compute_delivery_allocation_expire_due";

#[derive(Debug, Clone)]
pub(crate) struct CreateComputeDeliveryAllocationGrant {
    pub provider_owner_account_id: String,
    pub provider_id: String,
    pub pool_id: String,
    pub commitment_id: String,
    pub expected_commitment_revision: i64,
    pub expected_commitment_digest: String,
    pub consumer_account_id: String,
    pub job_id: String,
    pub expected_job_revision: i64,
    pub expected_job_digest: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub confirmation: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ExerciseComputeDeliveryAllocationGrant {
    pub consumer_account_id: String,
    pub grant_id: String,
    pub reservation_id: String,
    pub expected_grant_revision: i64,
    pub expected_grant_digest: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub confirmation: String,
}

#[derive(Debug, Clone)]
pub(crate) struct DeclineComputeDeliveryAllocationGrant {
    pub consumer_account_id: String,
    pub grant_id: String,
    pub expected_grant_revision: i64,
    pub expected_grant_digest: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub confirmation: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ExpireDueComputeDeliveryAllocationGrants {
    pub admin_user_id: String,
    pub limit: usize,
    pub confirmation: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeDeliveryAllocationGrantWriteReceipt {
    pub grant: ComputeDeliveryAllocationGrant,
    pub replayed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeDeliveryAllocationExerciseWriteReceipt {
    pub grant: ComputeDeliveryAllocationGrant,
    pub terminal_receipt: ComputeDeliveryAllocationTerminalReceipt,
    pub replayed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeDeliveryAllocationTerminalWriteReceipt {
    pub grant: ComputeDeliveryAllocationGrant,
    pub terminal_receipt: ComputeDeliveryAllocationTerminalReceipt,
    pub replayed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeDeliveryAllocationDetail {
    pub grant: ComputeDeliveryAllocationGrant,
    pub terminal_receipt: Option<ComputeDeliveryAllocationTerminalReceipt>,
    pub current_status: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeDeliveryAllocationExpiryItem {
    pub grant_id: String,
    pub status: String,
    pub replayed: bool,
    pub terminal_receipt: Option<ComputeDeliveryAllocationTerminalReceipt>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeDeliveryAllocationExpiryReport {
    pub recovery_started_at: String,
    pub selected_count: usize,
    pub expired_count: usize,
    pub replayed_count: usize,
    pub failed_count: usize,
    pub items: Vec<ComputeDeliveryAllocationExpiryItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::store) enum DeliveryAllocationCommitmentState {
    Granted,
    Exercised,
    Declined,
    Expired,
}

impl DeliveryAllocationCommitmentState {
    pub(in crate::store) fn as_str(self) -> &'static str {
        match self {
            Self::Granted => DELIVERY_ALLOCATION_STATUS_GRANTED,
            Self::Exercised => DELIVERY_ALLOCATION_STATUS_EXERCISED,
            Self::Declined => DELIVERY_ALLOCATION_STATUS_DECLINED,
            Self::Expired => DELIVERY_ALLOCATION_STATUS_EXPIRED,
        }
    }

    pub(in crate::store) fn blocks_commitment_terminal(self) -> bool {
        matches!(self, Self::Granted | Self::Exercised)
    }
}

#[derive(Debug, Clone)]
pub(in crate::store) struct DeliveryAllocationCommitmentStatus {
    pub(in crate::store) grant_id: String,
    pub(in crate::store) grant_digest: String,
    pub(in crate::store) state: DeliveryAllocationCommitmentState,
    pub(in crate::store) terminal_receipt_id: Option<String>,
    pub(in crate::store) terminal_receipt_digest: Option<String>,
}

impl DeliveryAllocationCommitmentStatus {
    pub(in crate::store) fn current_status(&self) -> &'static str {
        self.state.as_str()
    }

    pub(in crate::store) fn blocks_commitment_terminal(&self) -> bool {
        self.state.blocks_commitment_terminal()
    }
}

/// Sealed, fresh exercise authority. Only the DeliveryAllocation Store can construct it.
#[derive(Debug, Clone)]
pub(in crate::store) struct DeliveryAllocationClaimTransferAuthority {
    grant: ComputeDeliveryAllocationGrant,
    commitment: ComputeCapacityCommitment,
    parent_claim: ComputeCapacityClaim,
    source_job: ComputeJobRegistrationReceipt,
    reservation_id: String,
    reservation_idempotency_key: String,
    child_hold_idempotency_scope: String,
    child_hold_idempotency_key: String,
    reservation_expires_at: String,
    exercise_occurred_at: String,
}

impl DeliveryAllocationClaimTransferAuthority {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        grant: ComputeDeliveryAllocationGrant,
        commitment: ComputeCapacityCommitment,
        parent_claim: ComputeCapacityClaim,
        source_job: ComputeJobRegistrationReceipt,
        reservation_id: String,
        reservation_idempotency_key: String,
        reservation_expires_at: String,
        exercise_occurred_at: String,
    ) -> Self {
        let child_hold_idempotency_scope =
            format!("delivery-allocation:child-hold:{}", grant.grant_id);
        let child_hold_idempotency_key = reservation_idempotency_key.clone();
        Self {
            grant,
            commitment,
            parent_claim,
            source_job,
            reservation_id,
            reservation_idempotency_key,
            child_hold_idempotency_scope,
            child_hold_idempotency_key,
            reservation_expires_at,
            exercise_occurred_at,
        }
    }

    pub(in crate::store) fn grant(&self) -> &ComputeDeliveryAllocationGrant {
        &self.grant
    }

    pub(in crate::store) fn commitment(&self) -> &ComputeCapacityCommitment {
        &self.commitment
    }

    pub(in crate::store) fn parent_claim(&self) -> &ComputeCapacityClaim {
        &self.parent_claim
    }

    pub(in crate::store) fn offer_binding(&self) -> &ComputeCapacityOfferBinding {
        &self.commitment.offer
    }

    pub(in crate::store) fn pool_binding(&self) -> &ComputeCapacityPoolBinding {
        &self.commitment.pool
    }

    pub(in crate::store) fn delivery_window(&self) -> &ComputeDeliveryWindowBinding {
        &self.commitment.delivery_window.binding
    }

    pub(in crate::store) fn consumer_account_id(&self) -> &str {
        &self.grant.consumer_account_id
    }

    pub(in crate::store) fn project_id(&self) -> Option<&str> {
        self.grant.project_id.as_deref()
    }

    pub(in crate::store) fn job_id(&self) -> &str {
        &self.grant.job.job_id
    }

    pub(in crate::store) fn source_job(&self) -> &ComputeJobRegistrationReceipt {
        &self.source_job
    }

    pub(in crate::store) fn snapshot_id(&self) -> &str {
        &self.commitment.price_snapshot_id
    }

    pub(in crate::store) fn snapshot_digest(&self) -> &str {
        &self.commitment.price_snapshot_digest
    }

    pub(in crate::store) fn reservation_id(&self) -> &str {
        &self.reservation_id
    }

    pub(in crate::store) fn reservation_idempotency_key(&self) -> &str {
        &self.reservation_idempotency_key
    }

    pub(in crate::store) fn child_hold_idempotency_scope(&self) -> &str {
        &self.child_hold_idempotency_scope
    }

    pub(in crate::store) fn child_hold_idempotency_key(&self) -> &str {
        &self.child_hold_idempotency_key
    }

    pub(in crate::store) fn reservation_expires_at(&self) -> &str {
        &self.reservation_expires_at
    }

    pub(in crate::store) fn exercise_occurred_at(&self) -> &str {
        &self.exercise_occurred_at
    }

    pub(in crate::store) fn reserved_capacity(&self) -> Vec<ComputeReservedCapacity> {
        let mut values = self
            .parent_claim
            .lines
            .iter()
            .map(|line| ComputeReservedCapacity {
                meter: line.bucket.meter.clone(),
                quantity: line.quantity_units,
            })
            .collect::<Vec<_>>();
        values.sort_by(|left, right| left.meter.cmp(&right.meter));
        values
    }
}

/// Sealed proof passed to the stale-Snapshot Reservation and pre-held Broker seams.
#[derive(Debug, Clone)]
pub(in crate::store) struct DeliveryAllocationReservationAuthority {
    transfer: DeliveryAllocationClaimTransferAuthority,
    parent_result_claim: ComputeCapacityClaim,
    parent_release_ledger: ComputeDeliveryAllocationLedgerEvidence,
    child_claim: ComputeCapacityClaim,
    child_hold_ledger: ComputeDeliveryAllocationLedgerEvidence,
}

impl DeliveryAllocationReservationAuthority {
    pub(super) fn new(
        transfer: DeliveryAllocationClaimTransferAuthority,
        parent_result_claim: ComputeCapacityClaim,
        parent_release_ledger: ComputeDeliveryAllocationLedgerEvidence,
        child_claim: ComputeCapacityClaim,
        child_hold_ledger: ComputeDeliveryAllocationLedgerEvidence,
    ) -> Self {
        Self {
            transfer,
            parent_result_claim,
            parent_release_ledger,
            child_claim,
            child_hold_ledger,
        }
    }

    pub(in crate::store) fn transfer(&self) -> &DeliveryAllocationClaimTransferAuthority {
        &self.transfer
    }

    pub(in crate::store) fn grant_id(&self) -> &str {
        &self.transfer.grant.grant_id
    }

    pub(in crate::store) fn grant_digest(&self) -> &str {
        &self.transfer.grant.grant_digest
    }

    pub(in crate::store) fn reservation_id(&self) -> &str {
        self.transfer.reservation_id()
    }

    pub(in crate::store) fn consumer_account_id(&self) -> &str {
        self.transfer.consumer_account_id()
    }

    pub(in crate::store) fn job_id(&self) -> &str {
        self.transfer.job_id()
    }

    pub(in crate::store) fn source_job_binding(&self) -> ComputeJobVersionBinding {
        ComputeJobVersionBinding {
            job_id: self.transfer.source_job.job.job_id.clone(),
            job_revision: self.transfer.source_job.revision,
            job_digest: self.transfer.source_job.job_digest.clone(),
        }
    }

    pub(in crate::store) fn snapshot_id(&self) -> &str {
        self.transfer.snapshot_id()
    }

    pub(in crate::store) fn snapshot_digest(&self) -> &str {
        self.transfer.snapshot_digest()
    }

    pub(in crate::store) fn offer_binding(&self) -> &ComputeCapacityOfferBinding {
        self.transfer.offer_binding()
    }

    pub(in crate::store) fn pool_binding(&self) -> &ComputeCapacityPoolBinding {
        self.transfer.pool_binding()
    }

    pub(in crate::store) fn delivery_window(&self) -> &ComputeDeliveryWindowBinding {
        self.transfer.delivery_window()
    }

    pub(in crate::store) fn parent_claim(&self) -> &ComputeCapacityClaim {
        self.transfer.parent_claim()
    }

    pub(in crate::store) fn parent_result_claim(&self) -> &ComputeCapacityClaim {
        &self.parent_result_claim
    }

    pub(in crate::store) fn parent_release_ledger(
        &self,
    ) -> &ComputeDeliveryAllocationLedgerEvidence {
        &self.parent_release_ledger
    }

    pub(in crate::store) fn child_claim(&self) -> &ComputeCapacityClaim {
        &self.child_claim
    }

    pub(in crate::store) fn child_claim_binding(&self) -> ComputeCapacityClaimBinding {
        ComputeCapacityClaimBinding {
            claim_id: self.child_claim.claim_id.clone(),
            claim_revision: self.child_claim.revision,
            claim_digest: self.child_claim.claim_digest.clone(),
        }
    }

    pub(in crate::store) fn child_hold_ledger(&self) -> &ComputeDeliveryAllocationLedgerEvidence {
        &self.child_hold_ledger
    }

    pub(in crate::store) fn reservation_expires_at(&self) -> &str {
        self.transfer.reservation_expires_at()
    }

    pub(in crate::store) fn exercise_occurred_at(&self) -> &str {
        self.transfer.exercise_occurred_at()
    }
}

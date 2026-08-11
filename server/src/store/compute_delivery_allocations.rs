//! Whole-only bilateral DeliveryAllocation Store authority.

mod canonical;
mod exercise;
mod grant;
mod read;
mod terminal;
mod types;
mod validation;

pub(crate) use types::{
    ComputeDeliveryAllocationDetail, ComputeDeliveryAllocationExerciseWriteReceipt,
    ComputeDeliveryAllocationExpiryItem, ComputeDeliveryAllocationExpiryReport,
    ComputeDeliveryAllocationGrantWriteReceipt, ComputeDeliveryAllocationTerminalWriteReceipt,
    CreateComputeDeliveryAllocationGrant, DeclineComputeDeliveryAllocationGrant,
    ExerciseComputeDeliveryAllocationGrant, ExpireDueComputeDeliveryAllocationGrants,
    COMPUTE_DELIVERY_ALLOCATION_DECLINE_CONFIRMATION,
    COMPUTE_DELIVERY_ALLOCATION_EXERCISE_CONFIRMATION,
    COMPUTE_DELIVERY_ALLOCATION_EXPIRE_DUE_CONFIRMATION,
    COMPUTE_DELIVERY_ALLOCATION_GRANT_CONFIRMATION,
};

pub(in crate::store) use read::{
    delivery_allocation_commitment_status_on,
    persisted_delivery_allocation_reservation_authority_on,
};
pub(in crate::store) use types::{
    DeliveryAllocationClaimTransferAuthority, DeliveryAllocationCommitmentState,
    DeliveryAllocationCommitmentStatus, DeliveryAllocationReservationAuthority,
};

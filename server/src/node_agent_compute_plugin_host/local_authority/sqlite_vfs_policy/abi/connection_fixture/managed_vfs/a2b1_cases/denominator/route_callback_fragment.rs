//! Map route and operation-callback normal-return fragment.
//!
//! This local quotient starts after the raw-state gate admitted the installed typed file and the
//! outer callback-fault wrapper passed with a live inner file. It looks only at the Result algebra
//! around route preparation and the registry SHM callback: one route rejection, one callback-
//! admission rejection, and the admitted operation / callback-completion `2 x 2` product. Outer
//! controller rejection/selection/inner-missing, panics, promotion and fault-install internals,
//! managed Map provenance, prestate, mutation, quarantine custody, and adapter payload validation
//! remain outside this fragment.

use super::{
    abi_map_fragment::AbiNullWriteOutcome,
    case_key::Path,
    projection::ExpectedStatus,
    raw_state_fragment::{
        RawCleanupEffect, RawPostOperationOutcome, RawSlotRetention, INSTALLED_RAW_VALUES,
    },
    typed_map_fragment::ReviewedTypedMapOutputFragment,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedMapOuterFaultIngressFragment {
    PassedWithLiveInner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedMapRoutePreparationFragment {
    Rejected,
    Accepted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedMapOperationAdmissionFragment {
    NotReached,
    Rejected,
    Accepted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedMapOperationResultFragment {
    NotRun,
    Error,
    Ok,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedMapCallbackCompletionFragment {
    NotRun,
    Error,
    Ok,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedMapRouteCallbackProjectionFragment {
    TypedFailure,
    AdapterProjectionPending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedMapRouteCallbackPendingAxis {
    NotReached,
    Pending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ReviewedMapRouteCallbackProvenanceFragment {
    pub(super) route_promotion_fault_internals_and_custody: ReviewedMapRouteCallbackPendingAxis,
    pub(super) callback_owner_and_route_custody: ReviewedMapRouteCallbackPendingAxis,
    pub(super) managed_cause_prestate_and_retention: ReviewedMapRouteCallbackPendingAxis,
    pub(super) adapter_projection_control_flow: ReviewedMapRouteCallbackPendingAxis,
    pub(super) adapter_payload_custody: ReviewedMapRouteCallbackPendingAxis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ReviewedMapRouteCallbackBranchFragment {
    pub(super) candidate_path: Path,
    pub(super) raw_post_operation: RawPostOperationOutcome,
    pub(super) outer_fault_ingress: ReviewedMapOuterFaultIngressFragment,
    pub(super) route_preparation: ReviewedMapRoutePreparationFragment,
    pub(super) operation_admission: ReviewedMapOperationAdmissionFragment,
    pub(super) operation_result: ReviewedMapOperationResultFragment,
    pub(super) callback_completion: ReviewedMapCallbackCompletionFragment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ReviewedMapRouteCallbackExpectedFragment {
    pub(super) null_write_at_entry: AbiNullWriteOutcome,
    pub(super) output_at_cut: ReviewedTypedMapOutputFragment,
    pub(super) raw_slots_at_entry: RawSlotRetention,
    pub(super) raw_slots_at_cut: RawSlotRetention,
    pub(super) cleanup: RawCleanupEffect,
    pub(super) callback_completion_attempts: u8,
    pub(super) pointer_writes: u8,
    pub(super) projection: ReviewedMapRouteCallbackProjectionFragment,
    pub(super) provenance: ReviewedMapRouteCallbackProvenanceFragment,
    pub(super) expected_status: ExpectedStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ReviewedMapRouteCallbackFragmentCell {
    pub(super) branch: ReviewedMapRouteCallbackBranchFragment,
    pub(super) expected: ReviewedMapRouteCallbackExpectedFragment,
}

const fn cell(
    route_preparation: ReviewedMapRoutePreparationFragment,
    operation_admission: ReviewedMapOperationAdmissionFragment,
    operation_result: ReviewedMapOperationResultFragment,
    callback_completion: ReviewedMapCallbackCompletionFragment,
    callback_completion_attempts: u8,
    projection: ReviewedMapRouteCallbackProjectionFragment,
    provenance: ReviewedMapRouteCallbackProvenanceFragment,
) -> ReviewedMapRouteCallbackFragmentCell {
    ReviewedMapRouteCallbackFragmentCell {
        branch: ReviewedMapRouteCallbackBranchFragment {
            candidate_path: Path::Map,
            raw_post_operation: RawPostOperationOutcome::AcceptedNormalReturn,
            outer_fault_ingress: ReviewedMapOuterFaultIngressFragment::PassedWithLiveInner,
            route_preparation,
            operation_admission,
            operation_result,
            callback_completion,
        },
        expected: ReviewedMapRouteCallbackExpectedFragment {
            null_write_at_entry: AbiNullWriteOutcome::NullWritten,
            output_at_cut: ReviewedTypedMapOutputFragment::NullRetained,
            raw_slots_at_entry: INSTALLED_RAW_VALUES,
            raw_slots_at_cut: INSTALLED_RAW_VALUES,
            cleanup: RawCleanupEffect::None,
            callback_completion_attempts,
            pointer_writes: 0,
            projection,
            provenance,
            expected_status: ExpectedStatus::PendingSourceAndRedTeamReview,
        },
    }
}

/// One route rejection, one callback-admission rejection, and the admitted operation/completion
/// product. The final cell stops before adapter payload projection; none writes an ABI pointer.
pub(super) const REVIEWED_MAP_ROUTE_CALLBACK_FRAGMENTS: &[ReviewedMapRouteCallbackFragmentCell] = &[
    cell(
        ReviewedMapRoutePreparationFragment::Rejected,
        ReviewedMapOperationAdmissionFragment::NotReached,
        ReviewedMapOperationResultFragment::NotRun,
        ReviewedMapCallbackCompletionFragment::NotRun,
        0,
        ReviewedMapRouteCallbackProjectionFragment::TypedFailure,
        route_only_provenance(),
    ),
    cell(
        ReviewedMapRoutePreparationFragment::Accepted,
        ReviewedMapOperationAdmissionFragment::Rejected,
        ReviewedMapOperationResultFragment::NotRun,
        ReviewedMapCallbackCompletionFragment::NotRun,
        0,
        ReviewedMapRouteCallbackProjectionFragment::TypedFailure,
        route_callback_provenance(),
    ),
    admitted(
        ReviewedMapOperationResultFragment::Error,
        ReviewedMapCallbackCompletionFragment::Ok,
        ReviewedMapRouteCallbackProjectionFragment::TypedFailure,
        full_pending_provenance(),
    ),
    admitted(
        ReviewedMapOperationResultFragment::Error,
        ReviewedMapCallbackCompletionFragment::Error,
        ReviewedMapRouteCallbackProjectionFragment::TypedFailure,
        full_pending_provenance(),
    ),
    admitted(
        ReviewedMapOperationResultFragment::Ok,
        ReviewedMapCallbackCompletionFragment::Error,
        ReviewedMapRouteCallbackProjectionFragment::TypedFailure,
        full_pending_provenance(),
    ),
    admitted(
        ReviewedMapOperationResultFragment::Ok,
        ReviewedMapCallbackCompletionFragment::Ok,
        ReviewedMapRouteCallbackProjectionFragment::AdapterProjectionPending,
        full_pending_provenance(),
    ),
];

const fn admitted(
    operation_result: ReviewedMapOperationResultFragment,
    callback_completion: ReviewedMapCallbackCompletionFragment,
    projection: ReviewedMapRouteCallbackProjectionFragment,
    provenance: ReviewedMapRouteCallbackProvenanceFragment,
) -> ReviewedMapRouteCallbackFragmentCell {
    cell(
        ReviewedMapRoutePreparationFragment::Accepted,
        ReviewedMapOperationAdmissionFragment::Accepted,
        operation_result,
        callback_completion,
        1,
        projection,
        provenance,
    )
}

const fn route_only_provenance() -> ReviewedMapRouteCallbackProvenanceFragment {
    provenance(
        ReviewedMapRouteCallbackPendingAxis::NotReached,
        ReviewedMapRouteCallbackPendingAxis::NotReached,
        ReviewedMapRouteCallbackPendingAxis::NotReached,
        ReviewedMapRouteCallbackPendingAxis::NotReached,
    )
}

const fn route_callback_provenance() -> ReviewedMapRouteCallbackProvenanceFragment {
    provenance(
        ReviewedMapRouteCallbackPendingAxis::Pending,
        ReviewedMapRouteCallbackPendingAxis::NotReached,
        ReviewedMapRouteCallbackPendingAxis::Pending,
        ReviewedMapRouteCallbackPendingAxis::Pending,
    )
}

const fn full_pending_provenance() -> ReviewedMapRouteCallbackProvenanceFragment {
    provenance(
        ReviewedMapRouteCallbackPendingAxis::Pending,
        ReviewedMapRouteCallbackPendingAxis::Pending,
        ReviewedMapRouteCallbackPendingAxis::Pending,
        ReviewedMapRouteCallbackPendingAxis::Pending,
    )
}

const fn provenance(
    callback_owner_and_route_custody: ReviewedMapRouteCallbackPendingAxis,
    managed_cause_prestate_and_retention: ReviewedMapRouteCallbackPendingAxis,
    adapter_projection_control_flow: ReviewedMapRouteCallbackPendingAxis,
    adapter_payload_custody: ReviewedMapRouteCallbackPendingAxis,
) -> ReviewedMapRouteCallbackProvenanceFragment {
    ReviewedMapRouteCallbackProvenanceFragment {
        route_promotion_fault_internals_and_custody: ReviewedMapRouteCallbackPendingAxis::Pending,
        callback_owner_and_route_custody,
        managed_cause_prestate_and_retention,
        adapter_projection_control_flow,
        adapter_payload_custody,
    }
}

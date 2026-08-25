//! Map adapter-outcome and ABI-projection fragment.
//!
//! This local quotient composes the six route/callback cells with the adapter result immediately
//! below their normal-return cut. The five existing failures keep their ABI-unavailable result;
//! the sole operation-Ok/completion-Ok continuation contributes `NotPresent` and guard-pass typed
//! `Mapped` cells. Three defensive payload guards are tracked separately, so this inventory does
//! not exhaust guard rejection paths: region/length review remains pending while the null guard is
//! excluded only by the reviewed `NonNull` type envelope. Managed prestate, route/callback custody,
//! dropped/mapped payload lifetime, caught unwind and both reviewed open frontiers remain outside.

use super::{
    abi_map_fragment::AbiNullWriteOutcome,
    case_key::Path,
    projection::ExpectedStatus,
    raw_state_fragment::{RawCleanupEffect, RawSlotRetention, INSTALLED_RAW_VALUES},
    route_callback_fragment::{
        ReviewedMapCallbackCompletionFragment, ReviewedMapOperationAdmissionFragment,
        ReviewedMapOperationResultFragment, ReviewedMapOuterFaultIngressFragment,
        ReviewedMapRouteCallbackBranchFragment, ReviewedMapRoutePreparationFragment,
    },
    typed_map_fragment::{ReviewedTypedMapExitFragment, ReviewedTypedMapOutputFragment},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedMapAdapterOperationScopeFragment {
    ObserveOnly,
    ObserveOrExtend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedMapAdapterDispositionFragment {
    NotReached,
    CallbackAdmissionRejectionDropped,
    OperationRejectionDropped,
    CallbackCompletionRejectionDropped,
    NotPresent,
    MappedAfterDefensiveGuards,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedMapAdapterPayloadDispositionFragment {
    NotReached,
    NoOperationPayload,
    SuccessPayloadDroppedBeforeAdapter,
    NoPointerPayload,
    NonOwningPointerCarried,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedMapAdapterProjectionFragment {
    TypedFailure,
    TypedNotPresent,
    TypedMapped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedMapAdapterReviewState {
    NotReached,
    Reviewed,
    Pending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ReviewedMapAdapterProvenanceFragment {
    pub(super) route_promotion_fault_internals_and_custody: ReviewedMapAdapterReviewState,
    pub(super) callback_owner_and_route_custody: ReviewedMapAdapterReviewState,
    pub(super) managed_cause_prestate_and_retention: ReviewedMapAdapterReviewState,
    pub(super) adapter_projection_control_flow: ReviewedMapAdapterReviewState,
    pub(super) adapter_payload_custody: ReviewedMapAdapterReviewState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ReviewedMapAdapterProjectionBranchFragment {
    pub(super) candidate_path: Path,
    pub(super) route_callback: ReviewedMapRouteCallbackBranchFragment,
    pub(super) operation_scope: ReviewedMapAdapterOperationScopeFragment,
    pub(super) adapter_disposition: ReviewedMapAdapterDispositionFragment,
    pub(super) payload_disposition: ReviewedMapAdapterPayloadDispositionFragment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ReviewedMapAdapterProjectionExpectedFragment {
    pub(super) null_write_at_entry: AbiNullWriteOutcome,
    pub(super) output_at_cut: ReviewedTypedMapOutputFragment,
    pub(super) raw_slots_at_entry: RawSlotRetention,
    pub(super) raw_slots_at_cut: RawSlotRetention,
    pub(super) cleanup: RawCleanupEffect,
    pub(super) callback_completion_attempts: u8,
    pub(super) pointer_writes: u8,
    pub(super) projection: ReviewedMapAdapterProjectionFragment,
    pub(super) sqlite_exit: ReviewedTypedMapExitFragment,
    pub(super) provenance: ReviewedMapAdapterProvenanceFragment,
    pub(super) expected_status: ExpectedStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ReviewedMapAdapterProjectionFragmentCell {
    pub(super) branch: ReviewedMapAdapterProjectionBranchFragment,
    pub(super) expected: ReviewedMapAdapterProjectionExpectedFragment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedMapAdapterPayloadGuardFragment {
    RegionMismatch,
    LengthMismatch,
    NullPointer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedMapAdapterPayloadGuardDispositionFragment {
    PendingSourceReview,
    ExcludedByNonNullTypeEnvelope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ReviewedMapAdapterPayloadGuardReviewFragment {
    pub(super) guard: ReviewedMapAdapterPayloadGuardFragment,
    pub(super) disposition: ReviewedMapAdapterPayloadGuardDispositionFragment,
}

/// Five inherited failures plus two reviewed adapter outcomes below the only parent continuation.
/// The Mapped cell begins after all defensive guards pass; guard leaves remain a separate review
/// inventory. None of these cells performs cleanup or clears the installed raw-state slots.
pub(super) const REVIEWED_MAP_ADAPTER_PROJECTION_FRAGMENTS:
    &[ReviewedMapAdapterProjectionFragmentCell] = &[
    failure(
        ReviewedMapRoutePreparationFragment::Rejected,
        ReviewedMapOperationAdmissionFragment::NotReached,
        ReviewedMapOperationResultFragment::NotRun,
        ReviewedMapCallbackCompletionFragment::NotRun,
        0,
        ReviewedMapAdapterReviewState::NotReached,
        ReviewedMapAdapterReviewState::NotReached,
        ReviewedMapAdapterDispositionFragment::NotReached,
        ReviewedMapAdapterPayloadDispositionFragment::NotReached,
        ReviewedMapAdapterReviewState::NotReached,
        ReviewedMapAdapterReviewState::NotReached,
    ),
    failure(
        ReviewedMapRoutePreparationFragment::Accepted,
        ReviewedMapOperationAdmissionFragment::Rejected,
        ReviewedMapOperationResultFragment::NotRun,
        ReviewedMapCallbackCompletionFragment::NotRun,
        0,
        ReviewedMapAdapterReviewState::Pending,
        ReviewedMapAdapterReviewState::NotReached,
        ReviewedMapAdapterDispositionFragment::CallbackAdmissionRejectionDropped,
        ReviewedMapAdapterPayloadDispositionFragment::NoOperationPayload,
        ReviewedMapAdapterReviewState::Reviewed,
        ReviewedMapAdapterReviewState::Reviewed,
    ),
    operation_failure(
        ReviewedMapOperationResultFragment::Error,
        ReviewedMapCallbackCompletionFragment::Ok,
    ),
    operation_failure(
        ReviewedMapOperationResultFragment::Error,
        ReviewedMapCallbackCompletionFragment::Error,
    ),
    failure(
        ReviewedMapRoutePreparationFragment::Accepted,
        ReviewedMapOperationAdmissionFragment::Accepted,
        ReviewedMapOperationResultFragment::Ok,
        ReviewedMapCallbackCompletionFragment::Error,
        1,
        ReviewedMapAdapterReviewState::Pending,
        ReviewedMapAdapterReviewState::Pending,
        ReviewedMapAdapterDispositionFragment::CallbackCompletionRejectionDropped,
        ReviewedMapAdapterPayloadDispositionFragment::SuccessPayloadDroppedBeforeAdapter,
        ReviewedMapAdapterReviewState::Reviewed,
        ReviewedMapAdapterReviewState::Pending,
    ),
    success(
        ReviewedMapAdapterOperationScopeFragment::ObserveOnly,
        ReviewedMapAdapterDispositionFragment::NotPresent,
        ReviewedMapAdapterPayloadDispositionFragment::NoPointerPayload,
        ReviewedTypedMapOutputFragment::NullRetained,
        0,
        ReviewedMapAdapterProjectionFragment::TypedNotPresent,
        ReviewedTypedMapExitFragment::SqliteOkNotPresent,
        ReviewedMapAdapterReviewState::Reviewed,
    ),
    success(
        ReviewedMapAdapterOperationScopeFragment::ObserveOrExtend,
        ReviewedMapAdapterDispositionFragment::MappedAfterDefensiveGuards,
        ReviewedMapAdapterPayloadDispositionFragment::NonOwningPointerCarried,
        ReviewedTypedMapOutputFragment::MappedPointerWritten,
        1,
        ReviewedMapAdapterProjectionFragment::TypedMapped,
        ReviewedTypedMapExitFragment::SqliteOkMapped,
        ReviewedMapAdapterReviewState::Pending,
    ),
];

pub(super) const REVIEWED_MAP_ADAPTER_PAYLOAD_GUARD_REVIEWS:
    &[ReviewedMapAdapterPayloadGuardReviewFragment] = &[
    guard_review(
        ReviewedMapAdapterPayloadGuardFragment::RegionMismatch,
        ReviewedMapAdapterPayloadGuardDispositionFragment::PendingSourceReview,
    ),
    guard_review(
        ReviewedMapAdapterPayloadGuardFragment::LengthMismatch,
        ReviewedMapAdapterPayloadGuardDispositionFragment::PendingSourceReview,
    ),
    guard_review(
        ReviewedMapAdapterPayloadGuardFragment::NullPointer,
        ReviewedMapAdapterPayloadGuardDispositionFragment::ExcludedByNonNullTypeEnvelope,
    ),
];

const fn operation_failure(
    operation_result: ReviewedMapOperationResultFragment,
    callback_completion: ReviewedMapCallbackCompletionFragment,
) -> ReviewedMapAdapterProjectionFragmentCell {
    failure(
        ReviewedMapRoutePreparationFragment::Accepted,
        ReviewedMapOperationAdmissionFragment::Accepted,
        operation_result,
        callback_completion,
        1,
        ReviewedMapAdapterReviewState::Pending,
        ReviewedMapAdapterReviewState::Pending,
        ReviewedMapAdapterDispositionFragment::OperationRejectionDropped,
        ReviewedMapAdapterPayloadDispositionFragment::NoOperationPayload,
        ReviewedMapAdapterReviewState::Reviewed,
        ReviewedMapAdapterReviewState::Reviewed,
    )
}

#[allow(clippy::too_many_arguments)]
const fn failure(
    route_preparation: ReviewedMapRoutePreparationFragment,
    operation_admission: ReviewedMapOperationAdmissionFragment,
    operation_result: ReviewedMapOperationResultFragment,
    callback_completion: ReviewedMapCallbackCompletionFragment,
    callback_completion_attempts: u8,
    callback_custody: ReviewedMapAdapterReviewState,
    managed_provenance: ReviewedMapAdapterReviewState,
    adapter_disposition: ReviewedMapAdapterDispositionFragment,
    payload_disposition: ReviewedMapAdapterPayloadDispositionFragment,
    adapter_control_flow_review: ReviewedMapAdapterReviewState,
    adapter_payload_custody_review: ReviewedMapAdapterReviewState,
) -> ReviewedMapAdapterProjectionFragmentCell {
    cell(
        route_branch(
            route_preparation,
            operation_admission,
            operation_result,
            callback_completion,
        ),
        ReviewedMapAdapterOperationScopeFragment::ObserveOrExtend,
        adapter_disposition,
        payload_disposition,
        ReviewedTypedMapOutputFragment::NullRetained,
        callback_completion_attempts,
        0,
        ReviewedMapAdapterProjectionFragment::TypedFailure,
        ReviewedTypedMapExitFragment::ShmMapUnavailable,
        provenance(
            callback_custody,
            managed_provenance,
            adapter_control_flow_review,
            adapter_payload_custody_review,
        ),
    )
}

#[allow(clippy::too_many_arguments)]
const fn success(
    operation_scope: ReviewedMapAdapterOperationScopeFragment,
    adapter_disposition: ReviewedMapAdapterDispositionFragment,
    payload_disposition: ReviewedMapAdapterPayloadDispositionFragment,
    output_at_cut: ReviewedTypedMapOutputFragment,
    pointer_writes: u8,
    projection: ReviewedMapAdapterProjectionFragment,
    sqlite_exit: ReviewedTypedMapExitFragment,
    payload_custody: ReviewedMapAdapterReviewState,
) -> ReviewedMapAdapterProjectionFragmentCell {
    cell(
        route_branch(
            ReviewedMapRoutePreparationFragment::Accepted,
            ReviewedMapOperationAdmissionFragment::Accepted,
            ReviewedMapOperationResultFragment::Ok,
            ReviewedMapCallbackCompletionFragment::Ok,
        ),
        operation_scope,
        adapter_disposition,
        payload_disposition,
        output_at_cut,
        1,
        pointer_writes,
        projection,
        sqlite_exit,
        provenance(
            ReviewedMapAdapterReviewState::Pending,
            ReviewedMapAdapterReviewState::Pending,
            ReviewedMapAdapterReviewState::Reviewed,
            payload_custody,
        ),
    )
}

#[allow(clippy::too_many_arguments)]
const fn cell(
    route_callback: ReviewedMapRouteCallbackBranchFragment,
    operation_scope: ReviewedMapAdapterOperationScopeFragment,
    adapter_disposition: ReviewedMapAdapterDispositionFragment,
    payload_disposition: ReviewedMapAdapterPayloadDispositionFragment,
    output_at_cut: ReviewedTypedMapOutputFragment,
    callback_completion_attempts: u8,
    pointer_writes: u8,
    projection: ReviewedMapAdapterProjectionFragment,
    sqlite_exit: ReviewedTypedMapExitFragment,
    provenance: ReviewedMapAdapterProvenanceFragment,
) -> ReviewedMapAdapterProjectionFragmentCell {
    ReviewedMapAdapterProjectionFragmentCell {
        branch: ReviewedMapAdapterProjectionBranchFragment {
            candidate_path: Path::Map,
            route_callback,
            operation_scope,
            adapter_disposition,
            payload_disposition,
        },
        expected: ReviewedMapAdapterProjectionExpectedFragment {
            null_write_at_entry: AbiNullWriteOutcome::NullWritten,
            output_at_cut,
            raw_slots_at_entry: INSTALLED_RAW_VALUES,
            raw_slots_at_cut: INSTALLED_RAW_VALUES,
            cleanup: RawCleanupEffect::None,
            callback_completion_attempts,
            pointer_writes,
            projection,
            sqlite_exit,
            provenance,
            expected_status: ExpectedStatus::PendingSourceAndRedTeamReview,
        },
    }
}

const fn route_branch(
    route_preparation: ReviewedMapRoutePreparationFragment,
    operation_admission: ReviewedMapOperationAdmissionFragment,
    operation_result: ReviewedMapOperationResultFragment,
    callback_completion: ReviewedMapCallbackCompletionFragment,
) -> ReviewedMapRouteCallbackBranchFragment {
    ReviewedMapRouteCallbackBranchFragment {
        candidate_path: Path::Map,
        raw_post_operation:
            super::raw_state_fragment::RawPostOperationOutcome::AcceptedNormalReturn,
        outer_fault_ingress: ReviewedMapOuterFaultIngressFragment::PassedWithLiveInner,
        route_preparation,
        operation_admission,
        operation_result,
        callback_completion,
    }
}

const fn provenance(
    callback_owner_and_route_custody: ReviewedMapAdapterReviewState,
    managed_cause_prestate_and_retention: ReviewedMapAdapterReviewState,
    adapter_projection_control_flow: ReviewedMapAdapterReviewState,
    adapter_payload_custody: ReviewedMapAdapterReviewState,
) -> ReviewedMapAdapterProvenanceFragment {
    ReviewedMapAdapterProvenanceFragment {
        route_promotion_fault_internals_and_custody: ReviewedMapAdapterReviewState::Pending,
        callback_owner_and_route_custody,
        managed_cause_prestate_and_retention,
        adapter_projection_control_flow,
        adapter_payload_custody,
    }
}

const fn guard_review(
    guard: ReviewedMapAdapterPayloadGuardFragment,
    disposition: ReviewedMapAdapterPayloadGuardDispositionFragment,
) -> ReviewedMapAdapterPayloadGuardReviewFragment {
    ReviewedMapAdapterPayloadGuardReviewFragment { guard, disposition }
}

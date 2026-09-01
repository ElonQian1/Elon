//! Linear envelope for future A2 Windows dynamic evidence.
//!
//! The types in this module are only reachable from the Windows test target. A record can be
//! created only after a parent process has revalidated a sanitized child observation, consumed a
//! real child exit status, and removed the same child-bound root. Merely compiling this
//! module or finding an A2 source case never creates Windows dynamic evidence.

mod capture;
mod child;
mod cleanup;
mod environment;
mod joint_close_family;
mod lock_runner;
mod map_runner;
mod record;
#[cfg(test)]
mod tests;
mod unmap_family;

pub(super) use child::{
    BoundDynamicChild, ChildLaunchIdentity, DynamicChildFailure, SanitizedActualPayloadCommitment,
    SanitizedChildReport, ValidatedChildProcessReceipt, A2_DYNAMIC_CHILD_NONCE_ENV,
};
pub(super) use cleanup::ValidatedParentCleanupReceipt;
pub(super) use environment::WindowsDynamicEnvironment;
pub(super) use joint_close_family::{
    JointCloseCandidateReportView, JointCloseFamilyCohort, RenderedJointCloseFamilyReport,
    ValidatedJointCloseCandidateRecord, ValidatedJointCloseCleanCheckoutReceipt,
    ValidatedJointCloseFamily, ValidatedJointCloseFamilyMemberReceipt,
};
#[cfg(all(test, windows))]
pub(super) use lock_runner::{
    lock_abi_scalar_rejection_selector_for_test, lock_callback_route_unknown_selector_for_test,
    lock_local_protocol_rejection_selector_for_test,
    lock_local_sibling_contention_selector_for_test, lock_native_acquire_busy_selector_for_test,
    lock_pre_managed_rejection_selector_for_test, lock_raw_state_rejection_selector_for_test,
    lock_stored_poison_selector_for_test,
    selected_lock_abi_scalar_rejection_selector_for_test,
    selected_lock_callback_route_unknown_selector_for_test,
    selected_lock_local_protocol_rejection_selector_for_test,
    selected_lock_local_sibling_contention_selector_for_test,
    selected_lock_native_acquire_busy_selector_for_test,
    selected_lock_pre_managed_rejection_selector_for_test,
    selected_lock_raw_state_rejection_selector_for_test,
    selected_lock_stored_poison_selector_for_test,
};
pub(super) use lock_runner::{
    run_lock_abi_scalar_rejection_program_isolated,
    run_lock_callback_route_unknown_program_isolated, run_lock_lifecycle_program_isolated,
    run_lock_local_protocol_rejection_program_isolated,
    run_lock_local_sibling_contention_program_isolated,
    run_lock_native_acquire_busy_program_isolated, run_lock_pre_managed_rejection_program_isolated,
    run_lock_program_isolated, run_lock_raw_state_rejection_program_isolated,
    run_lock_stored_poison_program_isolated,
    LocalProtocolRejectionPathV1, LockRunnerAbiScalarRejectionBindingV1,
    LockRunnerAbiScalarValidityV1, LockRunnerActionV1, LockRunnerCallbackRouteUnknownBindingV1,
    LockRunnerCallbackRouteUnknownPathV1, LockRunnerEvidenceReceiptV1,
    LockRunnerIsolatedEvidenceV1, LockRunnerLifecycleBindingV1, LockRunnerLifecyclePathV1,
    LockRunnerLocalProtocolRejectionBindingV1, LockRunnerLocalSiblingContentionBindingV1,
    LockRunnerNativeAcquireBusyBindingV1, LockRunnerPreManagedCompletionV1,
    LockRunnerPreManagedRejectionBindingV1, LockRunnerPreManagedRejectionV1,
    LockRunnerProgramBindingV1, LockRunnerRequestValidationV1,
    LockRunnerRawStateRejectionBindingV1, LockRunnerRawStateRejectionV1,
    LockRunnerStoredPoisonBindingV1,
    LockRunnerStoredPoisonCompletionV1, LockRunnerStoredPoisonProfileV1,
};
pub(super) use map_runner::{
    run_map_lifecycle_program_isolated, run_map_program_isolated,
    run_map_region_loop_program_isolated, MapRunnerEvidenceReceiptV1, MapRunnerIsolatedEvidenceV1,
    MapRunnerLifecycleBindingV1, MapRunnerLifecyclePathV1, MapRunnerModeV1,
    MapRunnerProgramBindingV1, MapRunnerRegionLoopBindingV1, MapRunnerRegionLoopFamilyV1,
    MapRunnerRequestBudgetV1,
};
pub(super) use record::{
    UnmapCandidateReportView, ValidatedUnmapCandidateRecord, ValidatedWindowsDynamicRecord,
    WindowsDynamicReportView,
};
pub(super) use unmap_family::{
    RenderedUnmapFamilyReport, UnmapFamilyCohort, ValidatedUnmapCleanCheckoutReceipt,
    ValidatedUnmapFamily, ValidatedUnmapFamilyMemberReceipt,
};

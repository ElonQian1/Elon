//! Sealed admission bridge for the executable Lock dynamic-quotient programs.

mod abi_scalar_rejection;
mod callback_completion_route_unknown;
mod execution_receipt;
mod lifecycle;
mod local_protocol_rejection;
mod local_sibling_contention;
mod native_acquire_busy;
mod native_acquire_created_first_exclusive_release_error;
mod native_acquire_created_first_truncate_error_release_failed;
mod native_acquire_created_first_truncate_error_release_succeeded;
mod native_acquire_existing_first_exclusive_release_error;
mod native_acquire_existing_first_truncate_error_release_succeeded;
mod pre_managed_callback_rejection;
mod raw_state_rejection;
mod request_validation;
mod source_program;
mod stored_poison;

#[cfg(windows)]
use std::fmt;

#[cfg(windows)]
use super::super::super::terminal_descriptor::LockActionV1;
use super::super::super::{
    source_leaf_authority::Digest32, terminal_descriptor::RunnerCapabilityV1,
};
use super::super::{DynamicClassKeyV1, StaticMemberSealV1};
use super::CompiledRunnerPlanV1;
#[cfg(windows)]
use abi_scalar_rejection::LockAbiScalarRejectionProgramSpecV1;
pub(super) use abi_scalar_rejection::ABI_SCALAR_REJECTION_PROJECTOR_DELTA_V1;
#[cfg(windows)]
use callback_completion_route_unknown::LockCallbackCompletionRouteUnknownProgramSpecV1;
use execution_receipt::digest_execution_receipt_v1;
#[cfg(windows)]
use execution_receipt::seal_execution_receipt;
#[cfg(windows)]
use lifecycle::{LockLifecyclePathSpecV1, LockLifecycleProgramSpecV1};
#[cfg(windows)]
use local_protocol_rejection::LockLocalProtocolRejectionProgramSpecV1;
#[cfg(windows)]
use local_sibling_contention::LockLocalSiblingContentionProgramSpecV1;
#[cfg(windows)]
use native_acquire_busy::LockNativeAcquireBusyProgramSpecV1;
#[cfg(windows)]
use native_acquire_created_first_exclusive_release_error::LockNativeAcquireCreatedFirstExclusiveReleaseErrorProgramSpecV1;
pub(super) use native_acquire_created_first_exclusive_release_error::NATIVE_ACQUIRE_CREATED_FIRST_EXCLUSIVE_RELEASE_ERROR_PROJECTOR_DELTA_V1;
#[cfg(windows)]
use native_acquire_created_first_truncate_error_release_failed::LockNativeAcquireCreatedFirstTruncateErrorReleaseFailedProgramSpecV1;
pub(super) use native_acquire_created_first_truncate_error_release_failed::NATIVE_ACQUIRE_CREATED_FIRST_TRUNCATE_ERROR_RELEASE_FAILED_PROJECTOR_DELTA_V1;
#[cfg(windows)]
use native_acquire_created_first_truncate_error_release_succeeded::LockNativeAcquireCreatedFirstTruncateErrorReleaseSucceededProgramSpecV1;
pub(super) use native_acquire_created_first_truncate_error_release_succeeded::NATIVE_ACQUIRE_CREATED_FIRST_TRUNCATE_ERROR_RELEASE_SUCCEEDED_PROJECTOR_DELTA_V1;
#[cfg(windows)]
use native_acquire_existing_first_exclusive_release_error::LockNativeAcquireExistingFirstExclusiveReleaseErrorProgramSpecV1;
pub(super) use native_acquire_existing_first_exclusive_release_error::NATIVE_ACQUIRE_EXISTING_FIRST_EXCLUSIVE_RELEASE_ERROR_PROJECTOR_DELTA_V1;
#[cfg(windows)]
use native_acquire_existing_first_truncate_error_release_succeeded::LockNativeAcquireExistingFirstTruncateErrorReleaseSucceededProgramSpecV1;
pub(super) use native_acquire_existing_first_truncate_error_release_succeeded::NATIVE_ACQUIRE_EXISTING_FIRST_TRUNCATE_ERROR_RELEASE_SUCCEEDED_PROJECTOR_DELTA_V1;
pub(super) use pre_managed_callback_rejection::PRE_MANAGED_CALLBACK_REJECTION_PROJECTOR_DELTA_V1;
pub(super) use raw_state_rejection::RAW_STATE_REJECTION_PROJECTOR_DELTA_V1;
#[cfg(windows)]
use request_validation::LockRequestValidationGuardV1;
use source_program::program_spec_v1 as source_program_spec_v1;
#[cfg(windows)]
use stored_poison::{
    LockStoredPoisonCompletionV1, LockStoredPoisonProfileV1, LockStoredPoisonProgramSpecV1,
};

#[cfg(windows)]
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2_dynamic_evidence::{
    run_lock_lifecycle_program_isolated, run_lock_local_sibling_contention_program_isolated,
    run_lock_native_acquire_busy_program_isolated, run_lock_program_isolated,
    run_lock_stored_poison_program_isolated, LockRunnerActionV1, LockRunnerIsolatedEvidenceV1,
    LockRunnerLifecycleBindingV1, LockRunnerLifecyclePathV1,
    LockRunnerLocalSiblingContentionBindingV1, LockRunnerNativeAcquireBusyBindingV1,
    LockRunnerProgramBindingV1, LockRunnerRequestValidationV1,
    LockRunnerStoredPoisonBindingV1, LockRunnerStoredPoisonCompletionV1,
    LockRunnerStoredPoisonProfileV1,
};

/// A real execution receipt. Private fields and the absent public constructor prevent digest-only
/// callers from converting a capability declaration or compiled plan into execution authority.
pub(in super::super) struct LockRunnerExecutionReceiptV1 {
    normalized_descriptor_sha256: Digest32,
    member: StaticMemberSealV1,
    plan_sha256: Digest32,
    implementation_sha256: Digest32,
    root_commitment_sha256: Digest32,
    child_fingerprint_sha256: Digest32,
    registration_commitment_sha256: Digest32,
    payload_commitment_sha256: Digest32,
    environment_sha256: Digest32,
    cleanup_sha256: Digest32,
    native_receipt_sha256: Digest32,
    child_exit_code: i32,
    execution_sha256: Digest32,
}

pub(super) struct ValidatedLockRunnerExecutionV1 {
    implementation_sha256: Digest32,
    execution_sha256: Digest32,
}

impl ValidatedLockRunnerExecutionV1 {
    pub(super) const fn implementation_sha256(&self) -> Digest32 {
        self.implementation_sha256
    }

    pub(super) const fn execution_sha256(&self) -> Digest32 {
        self.execution_sha256
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LockRunnerExecutionViolationV1 {
    UnsupportedProgram,
    PlanBindingMismatch,
    MemberCatalogInvalid,
    MemberSealMismatch,
    ReceiptBindingMismatch,
    ExecutionSealMismatch,
}

#[cfg(windows)]
pub(in super::super) enum LockRunnerIsolatedOutcomeV1 {
    ParentReceipt(LockRunnerExecutionReceiptV1),
    ChildReported,
}

#[cfg(windows)]
#[derive(Debug)]
pub(in super::super) struct LockRunnerExecutionErrorV1(String);

#[cfg(windows)]
impl fmt::Display for LockRunnerExecutionErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(windows)]
impl std::error::Error for LockRunnerExecutionErrorV1 {}

pub(super) fn validate_execution_receipt_v1(
    key: &DynamicClassKeyV1,
    member: StaticMemberSealV1,
    plan: CompiledRunnerPlanV1,
    receipt: LockRunnerExecutionReceiptV1,
) -> Result<ValidatedLockRunnerExecutionV1, LockRunnerExecutionViolationV1> {
    let program = program_v1(key, member, plan)?;
    if receipt.normalized_descriptor_sha256 != program.normalized_descriptor_sha256
        || receipt.member != program.member
        || receipt.plan_sha256 != program.plan_sha256
        || receipt.implementation_sha256 != program.implementation_sha256
        || receipt.root_commitment_sha256 == Digest32::ZERO
        || receipt.child_fingerprint_sha256 == Digest32::ZERO
        || receipt.registration_commitment_sha256 == Digest32::ZERO
        || receipt.payload_commitment_sha256 == Digest32::ZERO
        || receipt.environment_sha256 == Digest32::ZERO
        || receipt.cleanup_sha256 == Digest32::ZERO
        || receipt.native_receipt_sha256 == Digest32::ZERO
        || receipt.child_exit_code != 0
    {
        return Err(LockRunnerExecutionViolationV1::ReceiptBindingMismatch);
    }
    let execution_sha256 = digest_execution_receipt_v1(&receipt);
    if receipt.execution_sha256 != execution_sha256 {
        return Err(LockRunnerExecutionViolationV1::ExecutionSealMismatch);
    }
    Ok(ValidatedLockRunnerExecutionV1 {
        implementation_sha256: receipt.implementation_sha256,
        execution_sha256,
    })
}

#[cfg(windows)]
pub(in super::super) fn run_lock_isolated_for_test(
    exact_test: &str,
    key: &DynamicClassKeyV1,
    member: StaticMemberSealV1,
    plan: CompiledRunnerPlanV1,
) -> Result<LockRunnerIsolatedOutcomeV1, LockRunnerExecutionErrorV1> {
    let program = program_v1(key, member, plan)
        .map_err(|violation| LockRunnerExecutionErrorV1(format!("{violation:?}")))?;
    let evidence = match program.case {
        LockProgramCaseV1::AbiScalarRejection(rejection) => {
            abi_scalar_rejection::run_isolated_v1(exact_test, rejection, member)
        }
        LockProgramCaseV1::RequestValidation { action, guard } => run_lock_program_isolated(
            exact_test,
            LockRunnerProgramBindingV1 {
                action: runner_action_v1(action),
                request_validation: match guard {
                    LockRequestValidationGuardV1::RangeOverflow => {
                        LockRunnerRequestValidationV1::RangeOverflow
                    }
                    LockRequestValidationGuardV1::EndPastEight => {
                        LockRunnerRequestValidationV1::EndPastEight
                    }
                    LockRequestValidationGuardV1::SharedMultiSlot => {
                        LockRunnerRequestValidationV1::SharedMultiSlot
                    }
                },
                normalized_descriptor_sha256: program.normalized_descriptor_sha256.0,
                case_key_sha256: member.case_key_sha256.0,
                full_record_sha256: member.full_record_sha256.0,
                plan_sha256: program.plan_sha256.0,
                implementation_sha256: program.implementation_sha256.0,
            },
        ),
        LockProgramCaseV1::CallbackCompletionRouteUnknown(route_unknown) => {
            callback_completion_route_unknown::run_isolated_v1(exact_test, route_unknown, member)
        }
        LockProgramCaseV1::Lifecycle(lifecycle) => run_lock_lifecycle_program_isolated(
            exact_test,
            LockRunnerLifecycleBindingV1 {
                path: match lifecycle.path {
                    LockLifecyclePathSpecV1::NativeAcquire => {
                        LockRunnerLifecyclePathV1::NativeAcquire
                    }
                    LockLifecyclePathSpecV1::NativeRelease => {
                        LockRunnerLifecyclePathV1::NativeRelease
                    }
                    LockLifecyclePathSpecV1::SharedLocalAcquire => {
                        LockRunnerLifecyclePathV1::SharedLocalAcquire
                    }
                    LockLifecyclePathSpecV1::SharedLocalRelease => {
                        LockRunnerLifecyclePathV1::SharedLocalRelease
                    }
                },
                action: runner_action_v1(lifecycle.action),
                first: lifecycle.first,
                count: lifecycle.count,
                mask: lifecycle.mask,
                normalized_descriptor_sha256: program.normalized_descriptor_sha256.0,
                case_key_sha256: member.case_key_sha256.0,
                full_record_sha256: member.full_record_sha256.0,
                plan_sha256: program.plan_sha256.0,
                implementation_sha256: program.implementation_sha256.0,
            },
        ),
        LockProgramCaseV1::LocalProtocolRejection(rejection) => {
            local_protocol_rejection::run_isolated_v1(exact_test, rejection, member)
        }
        LockProgramCaseV1::LocalSiblingContention(contention) => {
            run_lock_local_sibling_contention_program_isolated(
                exact_test,
                LockRunnerLocalSiblingContentionBindingV1 {
                    action: runner_action_v1(contention.action),
                    first: contention.first,
                    count: contention.count,
                    mask: contention.mask,
                    normalized_descriptor_sha256: program.normalized_descriptor_sha256.0,
                    case_key_sha256: member.case_key_sha256.0,
                    full_record_sha256: member.full_record_sha256.0,
                    plan_sha256: program.plan_sha256.0,
                    implementation_sha256: program.implementation_sha256.0,
                },
            )
        }
        LockProgramCaseV1::NativeAcquireBusy(busy) => {
            run_lock_native_acquire_busy_program_isolated(
                exact_test,
                LockRunnerNativeAcquireBusyBindingV1 {
                    action: runner_action_v1(busy.action),
                    first: busy.first,
                    count: busy.count,
                    mask: busy.mask,
                    normalized_descriptor_sha256: program.normalized_descriptor_sha256.0,
                    case_key_sha256: member.case_key_sha256.0,
                    full_record_sha256: member.full_record_sha256.0,
                    plan_sha256: program.plan_sha256.0,
                    implementation_sha256: program.implementation_sha256.0,
                },
            )
        }
        LockProgramCaseV1::NativeAcquireCreatedFirstExclusiveReleaseError(initialization) => {
            native_acquire_created_first_exclusive_release_error::run_isolated_v1(
                exact_test,
                initialization,
                member,
            )
        }
        LockProgramCaseV1::NativeAcquireCreatedFirstTruncateErrorReleaseSucceeded(
            initialization,
        ) => native_acquire_created_first_truncate_error_release_succeeded::run_isolated_v1(
            exact_test,
            initialization,
            member,
        ),
        LockProgramCaseV1::NativeAcquireExistingFirstExclusiveReleaseError(initialization) => {
            native_acquire_existing_first_exclusive_release_error::run_isolated_v1(
                exact_test,
                initialization,
                member,
            )
        }
        LockProgramCaseV1::NativeAcquireExistingFirstTruncateErrorReleaseSucceeded(
            initialization,
        ) => native_acquire_existing_first_truncate_error_release_succeeded::run_isolated_v1(
            exact_test,
            initialization,
            member,
        ),
        LockProgramCaseV1::NativeAcquireCreatedFirstTruncateErrorReleaseFailed(initialization) => {
            native_acquire_created_first_truncate_error_release_failed::run_isolated_v1(
                exact_test,
                initialization,
                member,
            )
        }
        LockProgramCaseV1::PreManagedCallbackRejection(rejection) => {
            pre_managed_callback_rejection::run_isolated_v1(exact_test, rejection, member)
        }
        LockProgramCaseV1::RawStateRejection(rejection) => {
            raw_state_rejection::run_isolated_v1(exact_test, rejection, member)
        }
        LockProgramCaseV1::StoredPoison(stored) => run_lock_stored_poison_program_isolated(
            exact_test,
            LockRunnerStoredPoisonBindingV1 {
                action: runner_action_v1(stored.action),
                first: stored.first,
                count: stored.count,
                mask: stored.mask,
                profile: runner_stored_poison_profile_v1(stored.profile),
                completion: match stored.completion {
                    LockStoredPoisonCompletionV1::RetentionSucceeded => {
                        LockRunnerStoredPoisonCompletionV1::RetentionSucceeded
                    }
                    LockStoredPoisonCompletionV1::RetentionRouteUnknown => {
                        LockRunnerStoredPoisonCompletionV1::RetentionRouteUnknown
                    }
                },
                normalized_descriptor_sha256: program.normalized_descriptor_sha256.0,
                case_key_sha256: member.case_key_sha256.0,
                full_record_sha256: member.full_record_sha256.0,
                plan_sha256: program.plan_sha256.0,
                implementation_sha256: program.implementation_sha256.0,
            },
        ),
    };
    match evidence.map_err(|error| LockRunnerExecutionErrorV1(error.to_string()))? {
        LockRunnerIsolatedEvidenceV1::ChildReported => {
            Ok(LockRunnerIsolatedOutcomeV1::ChildReported)
        }
        LockRunnerIsolatedEvidenceV1::ParentReceipt(evidence) => Ok(
            LockRunnerIsolatedOutcomeV1::ParentReceipt(seal_execution_receipt(program, evidence)),
        ),
    }
}

#[cfg(windows)]
const fn runner_stored_poison_profile_v1(
    profile: LockStoredPoisonProfileV1,
) -> LockRunnerStoredPoisonProfileV1 {
    match profile {
        LockStoredPoisonProfileV1::GateNoMutation => {
            LockRunnerStoredPoisonProfileV1::GateNoMutation
        }
        LockStoredPoisonProfileV1::FileCloseNoMutation => {
            LockRunnerStoredPoisonProfileV1::FileCloseNoMutation
        }
        LockStoredPoisonProfileV1::ExactSiblingDeleteNoMutation => {
            LockRunnerStoredPoisonProfileV1::ExactSiblingDeleteNoMutation
        }
        LockStoredPoisonProfileV1::ExactSiblingOpenUncertain => {
            LockRunnerStoredPoisonProfileV1::ExactSiblingOpenUncertain
        }
        LockStoredPoisonProfileV1::DmsTruncateUncertain => {
            LockRunnerStoredPoisonProfileV1::DmsTruncateUncertain
        }
        LockStoredPoisonProfileV1::FileCloseUncertain => {
            LockRunnerStoredPoisonProfileV1::FileCloseUncertain
        }
        LockStoredPoisonProfileV1::ExactSiblingDeleteUncertain => {
            LockRunnerStoredPoisonProfileV1::ExactSiblingDeleteUncertain
        }
        LockStoredPoisonProfileV1::FileGrowUncertain => {
            LockRunnerStoredPoisonProfileV1::FileGrowUncertain
        }
        LockStoredPoisonProfileV1::MappingCloseUncertain => {
            LockRunnerStoredPoisonProfileV1::MappingCloseUncertain
        }
        LockStoredPoisonProfileV1::ViewUnmapUncertain => {
            LockRunnerStoredPoisonProfileV1::ViewUnmapUncertain
        }
        LockStoredPoisonProfileV1::LockReleaseUncertain => {
            LockRunnerStoredPoisonProfileV1::LockReleaseUncertain
        }
        LockStoredPoisonProfileV1::ConnectionDetachUncertain => {
            LockRunnerStoredPoisonProfileV1::ConnectionDetachUncertain
        }
        LockStoredPoisonProfileV1::DeleteAuthorizationUncertain => {
            LockRunnerStoredPoisonProfileV1::DeleteAuthorizationUncertain
        }
        LockStoredPoisonProfileV1::DmsExclusiveReleaseUncertain => {
            LockRunnerStoredPoisonProfileV1::DmsExclusiveReleaseUncertain
        }
        LockStoredPoisonProfileV1::DmsSharedReleaseUncertain => {
            LockRunnerStoredPoisonProfileV1::DmsSharedReleaseUncertain
        }
    }
}

#[cfg(windows)]
const fn runner_action_v1(action: LockActionV1) -> LockRunnerActionV1 {
    match action {
        LockActionV1::LockShared => LockRunnerActionV1::LockShared,
        LockActionV1::LockExclusive => LockRunnerActionV1::LockExclusive,
        LockActionV1::UnlockShared => LockRunnerActionV1::UnlockShared,
        LockActionV1::UnlockExclusive => LockRunnerActionV1::UnlockExclusive,
    }
}

#[cfg(all(test, windows))]
pub(in super::super) fn tamper_lock_implementation_digest_for_test(
    receipt: &mut LockRunnerExecutionReceiptV1,
    digest: Digest32,
) {
    receipt.implementation_sha256 = digest;
}

#[derive(Clone, Copy)]
struct LockProgramV1 {
    #[cfg(windows)]
    case: LockProgramCaseV1,
    normalized_descriptor_sha256: Digest32,
    member: StaticMemberSealV1,
    plan_sha256: Digest32,
    implementation_sha256: Digest32,
}

#[cfg(windows)]
#[derive(Clone, Copy)]
enum LockProgramCaseV1 {
    AbiScalarRejection(LockAbiScalarRejectionProgramSpecV1),
    RequestValidation {
        action: LockActionV1,
        guard: LockRequestValidationGuardV1,
    },
    CallbackCompletionRouteUnknown(LockCallbackCompletionRouteUnknownProgramSpecV1),
    Lifecycle(LockLifecycleProgramSpecV1),
    LocalProtocolRejection(LockLocalProtocolRejectionProgramSpecV1),
    LocalSiblingContention(LockLocalSiblingContentionProgramSpecV1),
    NativeAcquireBusy(LockNativeAcquireBusyProgramSpecV1),
    NativeAcquireCreatedFirstExclusiveReleaseError(
        LockNativeAcquireCreatedFirstExclusiveReleaseErrorProgramSpecV1,
    ),
    NativeAcquireCreatedFirstTruncateErrorReleaseSucceeded(
        LockNativeAcquireCreatedFirstTruncateErrorReleaseSucceededProgramSpecV1,
    ),
    NativeAcquireCreatedFirstTruncateErrorReleaseFailed(
        LockNativeAcquireCreatedFirstTruncateErrorReleaseFailedProgramSpecV1,
    ),
    NativeAcquireExistingFirstExclusiveReleaseError(
        LockNativeAcquireExistingFirstExclusiveReleaseErrorProgramSpecV1,
    ),
    NativeAcquireExistingFirstTruncateErrorReleaseSucceeded(
        LockNativeAcquireExistingFirstTruncateErrorReleaseSucceededProgramSpecV1,
    ),
    PreManagedCallbackRejection(
        pre_managed_callback_rejection::LockPreManagedCallbackRejectionProgramSpecV1,
    ),
    RawStateRejection(raw_state_rejection::LockRawStateRejectionProgramSpecV1),
    StoredPoison(LockStoredPoisonProgramSpecV1),
}

pub(super) fn implementation_for_inventory_v1(
    key: &DynamicClassKeyV1,
    plan: CompiledRunnerPlanV1,
) -> Result<Option<Digest32>, LockRunnerExecutionViolationV1> {
    match source_program_spec_v1(key, plan) {
        Ok(program) => Ok(Some(program.implementation_sha256)),
        Err(LockRunnerExecutionViolationV1::UnsupportedProgram) => Ok(None),
        Err(error) => Err(error),
    }
}

fn program_v1(
    key: &DynamicClassKeyV1,
    member: StaticMemberSealV1,
    plan: CompiledRunnerPlanV1,
) -> Result<LockProgramV1, LockRunnerExecutionViolationV1> {
    let program = source_program_spec_v1(key, plan)?;
    if key.recipe.capability != RunnerCapabilityV1::Supported {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    }
    if program
        .expected_member
        .is_some_and(|expected| expected != member)
    {
        return Err(LockRunnerExecutionViolationV1::MemberSealMismatch);
    }
    Ok(LockProgramV1 {
        #[cfg(windows)]
        case: program.case,
        normalized_descriptor_sha256: program.normalized_descriptor_sha256,
        member,
        plan_sha256: program.plan_sha256,
        implementation_sha256: program.implementation_sha256,
    })
}

#[cfg(test)]
pub(super) fn native_acquire_busy_catalog_row_count_for_test() -> usize {
    native_acquire_busy::catalog_row_count_for_test()
}

#[cfg(test)]
pub(super) fn native_acquire_created_first_exclusive_release_error_catalog_row_count_for_test(
) -> usize {
    native_acquire_created_first_exclusive_release_error::catalog_row_count_for_test()
}

#[cfg(test)]
pub(super) fn native_acquire_created_first_truncate_error_release_succeeded_catalog_row_count_for_test(
) -> usize {
    native_acquire_created_first_truncate_error_release_succeeded::catalog_row_count_for_test()
}

#[cfg(test)]
pub(super) fn native_acquire_created_first_truncate_error_release_failed_catalog_row_count_for_test(
) -> usize {
    native_acquire_created_first_truncate_error_release_failed::catalog_row_count_for_test()
}

#[cfg(test)]
pub(super) fn native_acquire_existing_first_exclusive_release_error_catalog_row_count_for_test(
) -> usize {
    native_acquire_existing_first_exclusive_release_error::catalog_row_count_for_test()
}

#[cfg(test)]
pub(super) fn native_acquire_existing_first_truncate_error_release_succeeded_catalog_row_count_for_test(
) -> usize {
    native_acquire_existing_first_truncate_error_release_succeeded::catalog_row_count_for_test()
}

#[cfg(test)]
pub(super) fn abi_scalar_rejection_catalog_row_count_for_test() -> usize {
    abi_scalar_rejection::catalog_row_count_for_test()
}

#[cfg(test)]
pub(super) fn callback_completion_route_unknown_catalog_row_count_for_test() -> usize {
    callback_completion_route_unknown::catalog_row_count_for_test()
}

#[cfg(test)]
pub(super) fn local_sibling_contention_catalog_row_count_for_test() -> usize {
    local_sibling_contention::catalog_row_count_for_test()
}

#[cfg(test)]
pub(super) fn local_protocol_rejection_catalog_row_count_for_test() -> usize {
    local_protocol_rejection::catalog_row_count_for_test()
}

#[cfg(test)]
pub(super) fn pre_managed_callback_rejection_catalog_row_count_for_test() -> usize {
    pre_managed_callback_rejection::catalog_row_count_for_test()
}

#[cfg(test)]
pub(super) fn raw_state_rejection_catalog_row_count_for_test() -> usize {
    raw_state_rejection::catalog_row_count_for_test()
}

#[cfg(test)]
pub(super) fn stored_poison_catalog_row_count_for_test() -> usize {
    stored_poison::catalog_row_count_for_test()
}

#[cfg(test)]
pub(super) fn validate_program_for_test(
    key: &DynamicClassKeyV1,
    member: StaticMemberSealV1,
    plan: CompiledRunnerPlanV1,
) -> Result<(), LockRunnerExecutionViolationV1> {
    program_v1(key, member, plan).map(|_| ())
}

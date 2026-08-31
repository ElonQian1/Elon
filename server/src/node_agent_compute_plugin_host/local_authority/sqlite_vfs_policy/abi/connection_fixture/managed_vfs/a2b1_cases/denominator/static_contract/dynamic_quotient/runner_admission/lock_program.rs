//! Sealed admission bridge for the executable Lock dynamic-quotient programs.

mod lifecycle;
mod request_validation;
mod stored_poison;

#[cfg(windows)]
use std::fmt;

use sha2::{Digest, Sha256};

#[cfg(windows)]
use super::super::super::terminal_descriptor::LockActionV1;
use super::super::super::{
    source_leaf_authority::Digest32, terminal_descriptor::RunnerCapabilityV1,
};
use super::super::{DynamicClassKeyV1, StaticMemberSealV1};
use super::CompiledRunnerPlanV1;
#[cfg(windows)]
use lifecycle::LockLifecyclePathSpecV1;
use lifecycle::{program_spec_v1 as lifecycle_program_spec_v1, LockLifecycleProgramSpecV1};
use request_validation::{
    program_spec_v1 as request_validation_program_spec_v1, LockProgramSpecV1,
};
use stored_poison::{
    program_spec_v1 as stored_poison_program_spec_v1, LockStoredPoisonProgramSpecV1,
};
#[cfg(windows)]
use stored_poison::{LockStoredPoisonCompletionV1, LockStoredPoisonProfileV1};

#[cfg(windows)]
use request_validation::LockRequestValidationGuardV1;

#[cfg(windows)]
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2_dynamic_evidence::{
    run_lock_lifecycle_program_isolated, run_lock_program_isolated, LockRunnerActionV1,
    LockRunnerEvidenceReceiptV1, LockRunnerIsolatedEvidenceV1, LockRunnerLifecycleBindingV1,
    LockRunnerLifecyclePathV1, LockRunnerProgramBindingV1, LockRunnerRequestValidationV1,
    LockRunnerStoredPoisonBindingV1, LockRunnerStoredPoisonCompletionV1,
    LockRunnerStoredPoisonProfileV1,
    run_lock_stored_poison_program_isolated,
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
    RequestValidation {
        action: LockActionV1,
        guard: LockRequestValidationGuardV1,
    },
    Lifecycle(LockLifecycleProgramSpecV1),
    StoredPoison(LockStoredPoisonProgramSpecV1),
}

#[derive(Clone, Copy)]
struct SourceLockProgramSpecV1 {
    #[cfg(windows)]
    case: LockProgramCaseV1,
    normalized_descriptor_sha256: Digest32,
    expected_member: Option<StaticMemberSealV1>,
    plan_sha256: Digest32,
    implementation_sha256: Digest32,
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

fn source_program_spec_v1(
    key: &DynamicClassKeyV1,
    plan: CompiledRunnerPlanV1,
) -> Result<SourceLockProgramSpecV1, LockRunnerExecutionViolationV1> {
    match request_validation_program_spec_v1(key, plan) {
        Ok(program) => Ok(from_request_validation_v1(program)),
        Err(LockRunnerExecutionViolationV1::UnsupportedProgram) => {
            match lifecycle_program_spec_v1(key, plan) {
                Ok(program) => Ok(from_lifecycle_v1(program)),
                Err(LockRunnerExecutionViolationV1::UnsupportedProgram) => {
                    stored_poison_program_spec_v1(key, plan).map(from_stored_poison_v1)
                }
                Err(error) => Err(error),
            }
        }
        Err(error) => Err(error),
    }
}

fn from_request_validation_v1(program: LockProgramSpecV1) -> SourceLockProgramSpecV1 {
    #[cfg(not(windows))]
    let _ = program.action;
    SourceLockProgramSpecV1 {
        #[cfg(windows)]
        case: LockProgramCaseV1::RequestValidation {
            action: program.action,
            guard: program.guard,
        },
        normalized_descriptor_sha256: program.normalized_descriptor_sha256,
        expected_member: None,
        plan_sha256: program.plan_sha256,
        implementation_sha256: program.implementation_sha256,
    }
}

fn from_lifecycle_v1(program: LockLifecycleProgramSpecV1) -> SourceLockProgramSpecV1 {
    SourceLockProgramSpecV1 {
        #[cfg(windows)]
        case: LockProgramCaseV1::Lifecycle(program),
        normalized_descriptor_sha256: program.normalized_descriptor_sha256,
        expected_member: None,
        plan_sha256: program.plan_sha256,
        implementation_sha256: program.implementation_sha256,
    }
}

fn from_stored_poison_v1(program: LockStoredPoisonProgramSpecV1) -> SourceLockProgramSpecV1 {
    SourceLockProgramSpecV1 {
        #[cfg(windows)]
        case: LockProgramCaseV1::StoredPoison(program),
        normalized_descriptor_sha256: program.normalized_descriptor_sha256,
        expected_member: Some(program.member),
        plan_sha256: program.plan_sha256,
        implementation_sha256: program.implementation_sha256,
    }
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

#[cfg(windows)]
fn seal_execution_receipt(
    program: LockProgramV1,
    evidence: LockRunnerEvidenceReceiptV1,
) -> LockRunnerExecutionReceiptV1 {
    let (root, child, registration, payload, environment, cleanup, native, exit_code) =
        evidence.into_bindings();
    let mut receipt = LockRunnerExecutionReceiptV1 {
        normalized_descriptor_sha256: program.normalized_descriptor_sha256,
        member: program.member,
        plan_sha256: program.plan_sha256,
        implementation_sha256: program.implementation_sha256,
        root_commitment_sha256: Digest32(root),
        child_fingerprint_sha256: Digest32(child),
        registration_commitment_sha256: Digest32(registration),
        payload_commitment_sha256: Digest32(payload),
        environment_sha256: Digest32(environment),
        cleanup_sha256: Digest32(cleanup),
        native_receipt_sha256: Digest32(native),
        child_exit_code: exit_code,
        execution_sha256: Digest32::ZERO,
    };
    receipt.execution_sha256 = digest_execution_receipt_v1(&receipt);
    receipt
}

fn digest_execution_receipt_v1(receipt: &LockRunnerExecutionReceiptV1) -> Digest32 {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-lock-quotient-real-execution-v1\0");
    for digest in [
        receipt.normalized_descriptor_sha256,
        receipt.member.case_key_sha256,
        receipt.member.full_record_sha256,
        receipt.plan_sha256,
        receipt.implementation_sha256,
        receipt.root_commitment_sha256,
        receipt.child_fingerprint_sha256,
        receipt.registration_commitment_sha256,
        receipt.payload_commitment_sha256,
        receipt.environment_sha256,
        receipt.cleanup_sha256,
        receipt.native_receipt_sha256,
    ] {
        hasher.update(digest.0);
    }
    hasher.update(receipt.child_exit_code.to_le_bytes());
    Digest32(hasher.finalize().into())
}

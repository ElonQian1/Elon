//! Sealed admission bridge for the executable Lock managed-request-validation programs.

mod request_validation;

#[cfg(windows)]
use std::fmt;

use sha2::{Digest, Sha256};

use super::super::super::{
    source_leaf_authority::Digest32,
    terminal_descriptor::{LockActionV1, RunnerCapabilityV1},
};
use super::super::{DynamicClassKeyV1, StaticMemberSealV1};
use super::CompiledRunnerPlanV1;
use request_validation::program_spec_v1;

#[cfg(windows)]
use request_validation::LockRequestValidationGuardV1;

#[cfg(windows)]
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2_dynamic_evidence::{
    run_lock_program_isolated, LockRunnerActionV1, LockRunnerEvidenceReceiptV1,
    LockRunnerIsolatedEvidenceV1, LockRunnerProgramBindingV1, LockRunnerRequestValidationV1,
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
    let binding = LockRunnerProgramBindingV1 {
        action: match program.action {
            LockActionV1::LockShared => LockRunnerActionV1::LockShared,
            LockActionV1::LockExclusive => LockRunnerActionV1::LockExclusive,
            LockActionV1::UnlockShared => LockRunnerActionV1::UnlockShared,
            LockActionV1::UnlockExclusive => LockRunnerActionV1::UnlockExclusive,
        },
        request_validation: match program.guard {
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
    };
    match run_lock_program_isolated(exact_test, binding)
        .map_err(|error| LockRunnerExecutionErrorV1(error.to_string()))?
    {
        LockRunnerIsolatedEvidenceV1::ChildReported => {
            Ok(LockRunnerIsolatedOutcomeV1::ChildReported)
        }
        LockRunnerIsolatedEvidenceV1::ParentReceipt(evidence) => Ok(
            LockRunnerIsolatedOutcomeV1::ParentReceipt(seal_execution_receipt(program, evidence)),
        ),
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
    action: LockActionV1,
    #[cfg(windows)]
    guard: LockRequestValidationGuardV1,
    normalized_descriptor_sha256: Digest32,
    member: StaticMemberSealV1,
    plan_sha256: Digest32,
    implementation_sha256: Digest32,
}

pub(super) fn implementation_for_inventory_v1(
    key: &DynamicClassKeyV1,
    plan: CompiledRunnerPlanV1,
) -> Result<Option<Digest32>, LockRunnerExecutionViolationV1> {
    match program_spec_v1(key, plan) {
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
    let program = program_spec_v1(key, plan)?;
    if key.recipe.capability != RunnerCapabilityV1::Supported {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    }
    Ok(LockProgramV1 {
        action: program.action,
        #[cfg(windows)]
        guard: program.guard,
        normalized_descriptor_sha256: program.normalized_descriptor_sha256,
        member,
        plan_sha256: program.plan_sha256,
        implementation_sha256: program.implementation_sha256,
    })
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

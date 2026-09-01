//! Ordered source-program classification for the sealed Lock runner.

use super::super::super::{DynamicClassKeyV1, StaticMemberSealV1};
use super::super::CompiledRunnerPlanV1;
#[cfg(windows)]
use super::LockProgramCaseV1;
use super::{
    callback_completion_route_unknown::{
        program_spec_v1 as callback_completion_route_unknown_program_spec_v1,
        LockCallbackCompletionRouteUnknownProgramSpecV1,
    },
    lifecycle::{program_spec_v1 as lifecycle_program_spec_v1, LockLifecycleProgramSpecV1},
    local_sibling_contention::{
        program_spec_v1 as local_sibling_contention_program_spec_v1,
        LockLocalSiblingContentionProgramSpecV1,
    },
    native_acquire_busy::{
        program_spec_v1 as native_acquire_busy_program_spec_v1, LockNativeAcquireBusyProgramSpecV1,
    },
    request_validation::{
        program_spec_v1 as request_validation_program_spec_v1, LockProgramSpecV1,
    },
    stored_poison::{
        program_spec_v1 as stored_poison_program_spec_v1, LockStoredPoisonProgramSpecV1,
    },
    LockRunnerExecutionViolationV1,
};

#[derive(Clone, Copy)]
pub(super) struct SourceLockProgramSpecV1 {
    #[cfg(windows)]
    pub(super) case: LockProgramCaseV1,
    pub(super) normalized_descriptor_sha256:
        super::super::super::super::source_leaf_authority::Digest32,
    pub(super) expected_member: Option<StaticMemberSealV1>,
    pub(super) plan_sha256: super::super::super::super::source_leaf_authority::Digest32,
    pub(super) implementation_sha256: super::super::super::super::source_leaf_authority::Digest32,
}

pub(super) fn program_spec_v1(
    key: &DynamicClassKeyV1,
    plan: CompiledRunnerPlanV1,
) -> Result<SourceLockProgramSpecV1, LockRunnerExecutionViolationV1> {
    match request_validation_program_spec_v1(key, plan) {
        Ok(program) => Ok(from_request_validation_v1(program)),
        Err(LockRunnerExecutionViolationV1::UnsupportedProgram) => {
            match lifecycle_program_spec_v1(key, plan) {
                Ok(program) => Ok(from_lifecycle_v1(program)),
                Err(LockRunnerExecutionViolationV1::UnsupportedProgram) => {
                    match callback_completion_route_unknown_program_spec_v1(key, plan) {
                        Ok(program) => Ok(from_callback_completion_route_unknown_v1(program)),
                        Err(LockRunnerExecutionViolationV1::UnsupportedProgram) => {
                            match local_sibling_contention_program_spec_v1(key, plan) {
                                Ok(program) => Ok(from_local_sibling_contention_v1(program)),
                                Err(LockRunnerExecutionViolationV1::UnsupportedProgram) => {
                                    match native_acquire_busy_program_spec_v1(key, plan) {
                                        Ok(program) => Ok(from_native_acquire_busy_v1(program)),
                                        Err(LockRunnerExecutionViolationV1::UnsupportedProgram) => {
                                            stored_poison_program_spec_v1(key, plan)
                                                .map(from_stored_poison_v1)
                                        }
                                        Err(error) => Err(error),
                                    }
                                }
                                Err(error) => Err(error),
                            }
                        }
                        Err(error) => Err(error),
                    }
                }
                Err(error) => Err(error),
            }
        }
        Err(error) => Err(error),
    }
}

fn from_callback_completion_route_unknown_v1(
    program: LockCallbackCompletionRouteUnknownProgramSpecV1,
) -> SourceLockProgramSpecV1 {
    SourceLockProgramSpecV1 {
        #[cfg(windows)]
        case: LockProgramCaseV1::CallbackCompletionRouteUnknown(program),
        normalized_descriptor_sha256: program.normalized_descriptor_sha256,
        expected_member: Some(program.member),
        plan_sha256: program.plan_sha256,
        implementation_sha256: program.implementation_sha256,
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

fn from_local_sibling_contention_v1(
    program: LockLocalSiblingContentionProgramSpecV1,
) -> SourceLockProgramSpecV1 {
    SourceLockProgramSpecV1 {
        #[cfg(windows)]
        case: LockProgramCaseV1::LocalSiblingContention(program),
        normalized_descriptor_sha256: program.normalized_descriptor_sha256,
        expected_member: Some(program.member),
        plan_sha256: program.plan_sha256,
        implementation_sha256: program.implementation_sha256,
    }
}

fn from_native_acquire_busy_v1(
    program: LockNativeAcquireBusyProgramSpecV1,
) -> SourceLockProgramSpecV1 {
    SourceLockProgramSpecV1 {
        #[cfg(windows)]
        case: LockProgramCaseV1::NativeAcquireBusy(program),
        normalized_descriptor_sha256: program.normalized_descriptor_sha256,
        expected_member: Some(program.member),
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

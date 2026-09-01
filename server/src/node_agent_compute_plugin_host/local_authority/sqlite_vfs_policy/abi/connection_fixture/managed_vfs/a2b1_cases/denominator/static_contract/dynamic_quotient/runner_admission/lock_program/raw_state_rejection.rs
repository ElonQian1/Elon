//! Exact source-bound programs for the eleven memory-safe Lock raw-state rejections.
//!
//! Classification consumes the complete typed semantic key and Expected vector. Frozen leaf ids,
//! branch labels, and display strings never participate in admission. The two unsafe pointer
//! domains remain static safety-premise exclusions and therefore have no executable program here.

mod case;
mod catalog;
mod expected;
#[cfg(windows)]
mod runtime;
mod source_scope;

#[cfg(windows)]
pub(super) use runtime::run_isolated_v1;
pub(super) use source_scope::RAW_STATE_REJECTION_PROJECTOR_DELTA_V1;

use super::super::super::super::{
    source_leaf_authority::{Digest32, RootOperationV1},
    terminal_descriptor::{
        CallbackV1, CapabilityGapV1, CleanupV1, FaultSeamV1, FixtureV1, LockPrestateV1, ObserverV1,
        OccurrenceV1, PrestateV1, ReachabilityV1, RunnerCapabilityV1, StimulusV1,
    },
};
use super::super::super::{
    DynamicAxesV1, DynamicClassKeyV1, DynamicOperationV1, StaticMemberSealV1,
    DYNAMIC_PROJECTOR_SCHEMA_V1,
};
use super::super::CompiledRunnerPlanV1;
use super::LockRunnerExecutionViolationV1;
use case::LockRawStateRejectionCaseV1;
use catalog::exact_member_v1;
use expected::expected_v1;
use source_scope::digest_implementation_v1;

pub(super) const RAW_STATE_REJECTION_MEMBER_COUNT: usize = 11;

#[derive(Clone, Copy)]
pub(super) struct LockRawStateRejectionProgramSpecV1 {
    #[cfg(windows)]
    pub(super) rejection: LockRawStateRejectionCaseV1,
    pub(super) member: StaticMemberSealV1,
    pub(super) normalized_descriptor_sha256: Digest32,
    pub(super) plan_sha256: Digest32,
    pub(super) implementation_sha256: Digest32,
}

pub(super) fn program_spec_v1(
    key: &DynamicClassKeyV1,
    plan: CompiledRunnerPlanV1,
) -> Result<LockRawStateRejectionProgramSpecV1, LockRunnerExecutionViolationV1> {
    if plan != super::super::compile_v1(key) {
        return Err(LockRunnerExecutionViolationV1::PlanBindingMismatch);
    }
    let StimulusV1::LockRaw(raw_state) = key.stimulus else {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    };
    let DynamicAxesV1::Lock(axes) = key.axes else {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    };
    let ReachabilityV1::Reached(completion) = axes.completion else {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    };
    let Some(rejection) = LockRawStateRejectionCaseV1::from_typed_v1(raw_state, completion) else {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    };
    if key.schema_version != DYNAMIC_PROJECTOR_SCHEMA_V1
        || key.root != RootOperationV1::Lock
        || key.source_site != rejection.source_site_v1()
        || key.prestate != PrestateV1::Lock(LockPrestateV1::NotReached)
        || key.operation != DynamicOperationV1::Lock(rejection.operation_v1())
        || key.phase != rejection.phase_v1()
        || key.timing != rejection.timing_v1()
        || key.occurrence != OccurrenceV1::Natural
        || key.recipe.fixture != FixtureV1::AbiRawOnly
        || key.recipe.callback != CallbackV1::XShmLock
        || key.recipe.fault_seam != FaultSeamV1::RawState
        || key.recipe.observer != rejection.observer_v1()
        || key.recipe.cleanup != CleanupV1::ParentOwnedRoot
        || !matches!(
            key.recipe.capability,
            RunnerCapabilityV1::Supported
                | RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete)
        )
        || axes != rejection.axes_v1()
        || key.expected != expected_v1(rejection)
    {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    }
    Ok(LockRawStateRejectionProgramSpecV1 {
        #[cfg(windows)]
        rejection,
        member: exact_member_v1(rejection)?,
        normalized_descriptor_sha256: plan.normalized_descriptor_sha256,
        plan_sha256: plan.plan_sha256,
        implementation_sha256: digest_implementation_v1(rejection),
    })
}

#[cfg(test)]
pub(super) fn catalog_row_count_for_test() -> usize {
    catalog::catalog_row_count_for_test()
}

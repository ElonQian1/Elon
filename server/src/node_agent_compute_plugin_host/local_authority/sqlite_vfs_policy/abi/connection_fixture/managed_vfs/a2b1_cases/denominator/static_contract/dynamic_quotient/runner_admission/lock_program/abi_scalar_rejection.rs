//! Exact source-bound programs for Lock ABI scalar rejection terminals.
//!
//! Classification consumes the complete typed semantic key and Expected vector. Frozen leaf ids,
//! branch labels, and display strings never participate in admission.

mod catalog;
#[cfg(windows)]
mod runtime;
mod source_scope;

#[cfg(windows)]
pub(super) use runtime::run_isolated_v1;
pub(super) use source_scope::ABI_SCALAR_REJECTION_PROJECTOR_DELTA_V1;

use super::super::super::super::{
    source_leaf_authority::{
        CustodyStateV1, Digest32, DmsLockCustodyV1, FailureClassV1, LockEffectV1,
        MutationStateV1, ObservableCountsV1, RootOperationV1, SqliteResultV1,
        TerminalDispositionV1,
    },
    terminal_descriptor::{
        CallbackV1, CapabilityGapV1, CleanupV1, FaultSeamV1, FixtureV1, LockAbiScalarV1,
        LockAxesV1, LockCompletionV1, LockOperationV1, LockPrestateV1, ObserverV1,
        OccurrenceV1, PhaseV1, PrestateV1, ReachabilityV1, RunnerCapabilityV1, SourceSiteV1,
        StimulusV1, TimingV1, ValidityV1,
    },
};
use super::super::super::{
    DynamicAxesV1, DynamicClassKeyV1, DynamicExpectedV1, DynamicOperationV1,
    StaticMemberSealV1, DYNAMIC_PROJECTOR_SCHEMA_V1,
};
use super::super::CompiledRunnerPlanV1;
use super::LockRunnerExecutionViolationV1;
use catalog::exact_member_v1;
use source_scope::digest_implementation_v1;

pub(super) const ABI_SCALAR_REJECTION_MEMBER_COUNT: usize = 7;

#[derive(Clone, Copy)]
pub(super) struct LockAbiScalarRejectionProgramSpecV1 {
    #[cfg(windows)]
    pub(super) scalar: LockAbiScalarV1,
    pub(super) member: StaticMemberSealV1,
    pub(super) normalized_descriptor_sha256: Digest32,
    pub(super) plan_sha256: Digest32,
    pub(super) implementation_sha256: Digest32,
}

pub(super) fn program_spec_v1(
    key: &DynamicClassKeyV1,
    plan: CompiledRunnerPlanV1,
) -> Result<LockAbiScalarRejectionProgramSpecV1, LockRunnerExecutionViolationV1> {
    if plan != super::super::compile_v1(key) {
        return Err(LockRunnerExecutionViolationV1::PlanBindingMismatch);
    }
    let StimulusV1::LockAbi(scalar) = key.stimulus else {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    };
    let DynamicAxesV1::Lock(axes) = key.axes else {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    };
    if !is_rejection_scalar_v1(scalar)
        || key.schema_version != DYNAMIC_PROJECTOR_SCHEMA_V1
        || key.root != RootOperationV1::Lock
        || key.source_site != SourceSiteV1::LockAbiBoundary
        || key.prestate != PrestateV1::Lock(LockPrestateV1::NotReached)
        || key.operation != DynamicOperationV1::Lock(LockOperationV1::AbiValidation)
        || key.phase != PhaseV1::AbiValidation
        || key.timing != TimingV1::BeforeCall
        || key.occurrence != OccurrenceV1::Natural
        || key.recipe.fixture != FixtureV1::AbiRawOnly
        || key.recipe.callback != CallbackV1::XShmLock
        || key.recipe.fault_seam != FaultSeamV1::AbiBoundary
        || key.recipe.observer != ObserverV1::LockCallbackAndSnapshot
        || key.recipe.cleanup != CleanupV1::ParentOwnedRoot
        || !matches!(
            key.recipe.capability,
            RunnerCapabilityV1::Supported
                | RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete)
        )
        || axes != expected_axes_v1()
        || key.expected != expected_v1()
    {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    }
    Ok(LockAbiScalarRejectionProgramSpecV1 {
        #[cfg(windows)]
        scalar,
        member: exact_member_v1(scalar)?,
        normalized_descriptor_sha256: plan.normalized_descriptor_sha256,
        plan_sha256: plan.plan_sha256,
        implementation_sha256: digest_implementation_v1(scalar),
    })
}

pub(super) const fn is_rejection_scalar_v1(scalar: LockAbiScalarV1) -> bool {
    !(matches!(scalar.offset, ValidityV1::Valid)
        && matches!(scalar.count, ValidityV1::Valid)
        && matches!(scalar.flags, ValidityV1::Valid))
}

const fn expected_axes_v1() -> LockAxesV1 {
    LockAxesV1 {
        completion: ReachabilityV1::Reached(LockCompletionV1::Direct),
        ..LockAxesV1::NOT_REACHED
    }
}

fn expected_v1() -> DynamicExpectedV1 {
    DynamicExpectedV1 {
        sqlite: SqliteResultV1::LockUnavailable,
        disposition: TerminalDispositionV1::Returned,
        phase: PhaseV1::AbiValidation,
        failure: FailureClassV1::ProtocolViolation,
        mutation: MutationStateV1::None,
        lock_outcome_uncertain: false,
        lock_effect: LockEffectV1::NotReached,
        dms_lock: DmsLockCustodyV1::NotReached,
        raw_slots: CustodyStateV1::NotReached,
        route: CustodyStateV1::NotReached,
        callback: CustodyStateV1::NotReached,
        file: CustodyStateV1::NotReached,
        mapping: CustodyStateV1::NotReached,
        view: CustodyStateV1::NotReached,
        payload: CustodyStateV1::NotReached,
        counts: ObservableCountsV1::default(),
    }
}

#[cfg(test)]
pub(super) fn catalog_row_count_for_test() -> usize {
    catalog::catalog_row_count_for_test()
}

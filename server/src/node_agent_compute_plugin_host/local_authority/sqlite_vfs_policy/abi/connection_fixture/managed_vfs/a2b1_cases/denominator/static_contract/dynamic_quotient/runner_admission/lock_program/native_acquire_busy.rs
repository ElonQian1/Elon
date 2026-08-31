//! Exact source-bound programs for completed node-live native-acquire contention.
//!
//! Classification uses only the typed dynamic key and its complete expected vector. Exact frozen
//! member seals come from the separately committed catalog; leaf ids are never admission inputs.

mod catalog;
mod source_scope;

use super::super::super::super::{
    source_leaf_authority::{
        CustodyStateV1, Digest32, DmsLockCustodyV1, FailureClassV1, LockEffectV1, MutationStateV1,
        ObservableCountsV1, RootOperationV1, SqliteResultV1, TerminalDispositionV1,
    },
    terminal_descriptor::{
        CallbackV1, CleanupV1, FaultSeamV1, FixtureV1, InitializationProfileV1, LockActionV1,
        LockAxesV1, LockCompletionV1, LockManagedStimulusV1, LockOperationV1, LockPrestateV1,
        ObserverV1, OccurrenceV1, PhaseV1, PrestateV1, ReachabilityV1, SourceSiteV1, StimulusV1,
        TimingV1,
    },
};
use super::super::super::{
    DynamicAxesV1, DynamicClassKeyV1, DynamicExpectedV1, DynamicOperationV1, StaticMemberSealV1,
    DYNAMIC_PROJECTOR_SCHEMA_V1,
};
use super::super::CompiledRunnerPlanV1;
use super::LockRunnerExecutionViolationV1;
use catalog::exact_member_v1;
use source_scope::digest_implementation_v1;

pub(super) const NATIVE_ACQUIRE_BUSY_MEMBER_COUNT: usize = 44;

#[derive(Clone, Copy)]
pub(super) struct LockNativeAcquireBusyProgramSpecV1 {
    #[cfg(windows)]
    pub(super) action: LockActionV1,
    #[cfg(windows)]
    pub(super) first: u8,
    #[cfg(windows)]
    pub(super) count: u8,
    #[cfg(windows)]
    pub(super) mask: u8,
    pub(super) member: StaticMemberSealV1,
    pub(super) normalized_descriptor_sha256: Digest32,
    pub(super) plan_sha256: Digest32,
    pub(super) implementation_sha256: Digest32,
}

pub(super) fn program_spec_v1(
    key: &DynamicClassKeyV1,
    plan: CompiledRunnerPlanV1,
) -> Result<LockNativeAcquireBusyProgramSpecV1, LockRunnerExecutionViolationV1> {
    if plan != super::super::compile_v1(key) {
        return Err(LockRunnerExecutionViolationV1::PlanBindingMismatch);
    }
    let DynamicAxesV1::Lock(axes) = key.axes else {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    };
    let (
        ReachabilityV1::Reached(action),
        ReachabilityV1::Reached(first),
        ReachabilityV1::Reached(count),
        ReachabilityV1::Reached(mask),
    ) = (axes.action, axes.first, axes.count, axes.mask)
    else {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    };
    let Some(expected_mask) = range_mask_v1(action, first, count) else {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    };
    if key.schema_version != DYNAMIC_PROJECTOR_SCHEMA_V1
        || key.root != RootOperationV1::Lock
        || key.source_site != SourceSiteV1::LockNativeAcquire
        || key.stimulus != StimulusV1::LockManaged(LockManagedStimulusV1::NativeAcquire)
        || key.prestate != PrestateV1::Lock(LockPrestateV1::NoHeldLocks)
        || key.operation != DynamicOperationV1::Lock(LockOperationV1::NativeAcquire)
        || key.phase != PhaseV1::LockAcquire
        || key.timing != TimingV1::AtCall
        || key.occurrence != OccurrenceV1::Natural
        || key.recipe.fixture != FixtureV1::ManagedWalMainSingleConnection
        || key.recipe.callback != CallbackV1::XShmLock
        || key.recipe.fault_seam != FaultSeamV1::NativeOperation
        || key.recipe.observer != ObserverV1::LockCallbackAndSnapshot
        || key.recipe.cleanup != CleanupV1::ParentOwnedRoot
        || mask != expected_mask
        || axes != expected_axes_v1(action, first, count, mask)
        || key.expected != expected_v1()
    {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    }
    Ok(LockNativeAcquireBusyProgramSpecV1 {
        #[cfg(windows)]
        action,
        #[cfg(windows)]
        first,
        #[cfg(windows)]
        count,
        #[cfg(windows)]
        mask,
        member: exact_member_v1(action, first, count, mask)?,
        normalized_descriptor_sha256: plan.normalized_descriptor_sha256,
        plan_sha256: plan.plan_sha256,
        implementation_sha256: digest_implementation_v1(action, first, count, mask),
    })
}

fn expected_axes_v1(action: LockActionV1, first: u8, count: u8, mask: u8) -> LockAxesV1 {
    LockAxesV1 {
        action: ReachabilityV1::Reached(action),
        first: ReachabilityV1::Reached(first),
        count: ReachabilityV1::Reached(count),
        mask: ReachabilityV1::Reached(mask),
        initialization: ReachabilityV1::Reached(InitializationProfileV1::NodeLive),
        held_shared_mask: ReachabilityV1::Reached(0),
        held_exclusive_mask: ReachabilityV1::Reached(0),
        sibling_shared_mask: ReachabilityV1::Reached(0),
        sibling_exclusive_mask: ReachabilityV1::Reached(0),
        completion: ReachabilityV1::Reached(LockCompletionV1::Completed),
    }
}

fn expected_v1() -> DynamicExpectedV1 {
    DynamicExpectedV1 {
        sqlite: SqliteResultV1::Busy,
        disposition: TerminalDispositionV1::Returned,
        phase: PhaseV1::LockAcquire,
        failure: FailureClassV1::BusyNoMutation,
        mutation: MutationStateV1::None,
        lock_outcome_uncertain: false,
        lock_effect: LockEffectV1::Unchanged,
        dms_lock: DmsLockCustodyV1::ExistingShared,
        raw_slots: CustodyStateV1::Unchanged,
        route: CustodyStateV1::Unchanged,
        callback: CustodyStateV1::Released,
        file: CustodyStateV1::Unchanged,
        mapping: CustodyStateV1::NotReached,
        view: CustodyStateV1::NotReached,
        payload: CustodyStateV1::NotReached,
        counts: ObservableCountsV1 {
            callback_begin: 1,
            callback_complete: 1,
            native_lock: 1,
            ..ObservableCountsV1::default()
        },
    }
}

pub(super) const fn range_mask_v1(action: LockActionV1, first: u8, count: u8) -> Option<u8> {
    let end = match first.checked_add(count) {
        Some(end) => end,
        None => return None,
    };
    if first >= 8 || count == 0 || end > 8 {
        return None;
    }
    match action {
        LockActionV1::LockShared if count == 1 => {}
        LockActionV1::LockExclusive => {}
        LockActionV1::LockShared | LockActionV1::UnlockShared | LockActionV1::UnlockExclusive => {
            return None
        }
    }
    Some(((((1_u16 << count) - 1) << first) & 0xff) as u8)
}

#[cfg(test)]
pub(super) fn catalog_row_count_for_test() -> usize {
    catalog::catalog_row_count_for_test()
}

//! Exact q18 source programs for created-first DMS shared-acquire contention after known
//! initialization mutation with successful file close.
//!
//! Admission consumes the complete typed dynamic key and expected vector. Frozen leaf ids,
//! decision labels, and display text never participate in classification; the committed catalog
//! binds each typed request/completion coordinate to its exact frozen member seal.

mod catalog;
#[cfg(windows)]
mod runtime;
mod source_scope;

#[cfg(windows)]
pub(super) use runtime::run_isolated_v1;
pub(super) use source_scope::NATIVE_ACQUIRE_CREATED_FIRST_SHARED_BUSY_CLOSE_SUCCEEDED_PROJECTOR_DELTA_V1;

use super::super::super::super::{
    source_leaf_authority::{
        CustodyStateV1, Digest32, DmsLockCustodyV1, FailureClassV1, LockEffectV1, MutationStateV1,
        ObservableCountsV1, RootOperationV1, SqliteResultV1, TerminalDispositionV1,
    },
    terminal_descriptor::{
        CallbackV1, CapabilityGapV1, CleanupV1, FaultSeamV1, FixtureV1, InitializationFaultSiteV1,
        InitializationPathV1, InitializationStimulusV1, LockActionV1, LockAxesV1, LockCompletionV1,
        LockOperationV1, LockPrestateV1, ObserverV1, OccurrenceV1, PhaseV1, PrestateV1,
        ReachabilityV1, RunnerCapabilityV1, SourceSiteV1, StimulusV1, TimingV1,
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

pub(super) const CREATED_FIRST_SHARED_BUSY_CLOSE_SUCCEEDED_MEMBER_COUNT: usize = 88;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LockCreatedFirstSharedBusyCloseSucceededCompletionV1 {
    RetentionSucceeded,
    RetentionRouteUnknown,
}

impl LockCreatedFirstSharedBusyCloseSucceededCompletionV1 {
    const fn axis(self) -> LockCompletionV1 {
        match self {
            Self::RetentionSucceeded => LockCompletionV1::UnsafeRetentionSucceededThenRouteUnknown,
            Self::RetentionRouteUnknown => {
                LockCompletionV1::UnsafeRetentionRouteUnknownThenRouteUnknown
            }
        }
    }

    const fn implementation_tag_v1(self) -> u8 {
        match self {
            Self::RetentionSucceeded => 1,
            Self::RetentionRouteUnknown => 2,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct LockNativeAcquireCreatedFirstSharedBusyCloseSucceededProgramSpecV1 {
    #[cfg(windows)]
    pub(super) action: LockActionV1,
    #[cfg(windows)]
    pub(super) first: u8,
    #[cfg(windows)]
    pub(super) count: u8,
    #[cfg(windows)]
    pub(super) mask: u8,
    #[cfg(windows)]
    pub(super) completion: LockCreatedFirstSharedBusyCloseSucceededCompletionV1,
    pub(super) member: StaticMemberSealV1,
    pub(super) normalized_descriptor_sha256: Digest32,
    pub(super) plan_sha256: Digest32,
    pub(super) implementation_sha256: Digest32,
}

pub(super) fn program_spec_v1(
    key: &DynamicClassKeyV1,
    plan: CompiledRunnerPlanV1,
) -> Result<
    LockNativeAcquireCreatedFirstSharedBusyCloseSucceededProgramSpecV1,
    LockRunnerExecutionViolationV1,
> {
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
    let Some(completion) = completion_v1(axes.completion) else {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    };
    let Some(expected_mask) = range_mask_v1(action, first, count) else {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    };
    if key.schema_version != DYNAMIC_PROJECTOR_SCHEMA_V1
        || key.root != RootOperationV1::Lock
        || key.source_site != SourceSiteV1::InitializationDms
        || key.stimulus
            != StimulusV1::Initialization(InitializationStimulusV1 {
                fault_site: InitializationFaultSiteV1::DmsSharedAcquire,
                path: InitializationPathV1::CreatedFirst,
                cleanup_rewrite: false,
            })
        || key.prestate != PrestateV1::Lock(LockPrestateV1::NoHeldLocks)
        || key.operation != DynamicOperationV1::Lock(LockOperationV1::Initialization)
        || key.phase != PhaseV1::DmsSharedAcquire
        || key.timing != TimingV1::AtCall
        || key.occurrence != OccurrenceV1::Natural
        || key.recipe.fixture != FixtureV1::ManagedWalMainSingleConnection
        || key.recipe.callback != CallbackV1::XShmLock
        || key.recipe.fault_seam != FaultSeamV1::Initialization
        || key.recipe.observer != ObserverV1::LockCallbackAndSnapshot
        || key.recipe.cleanup != CleanupV1::RetainUnsafeCustodyThenParentCleanup
        || !matches!(
            key.recipe.capability,
            RunnerCapabilityV1::Supported
                | RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete)
        )
        || mask != expected_mask
        || axes != expected_axes_v1(action, first, count, mask, completion)
        || key.expected != expected_v1()
    {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    }
    Ok(
        LockNativeAcquireCreatedFirstSharedBusyCloseSucceededProgramSpecV1 {
            #[cfg(windows)]
            action,
            #[cfg(windows)]
            first,
            #[cfg(windows)]
            count,
            #[cfg(windows)]
            mask,
            #[cfg(windows)]
            completion,
            member: exact_member_v1(action, first, count, mask, completion)?,
            normalized_descriptor_sha256: plan.normalized_descriptor_sha256,
            plan_sha256: plan.plan_sha256,
            implementation_sha256: digest_implementation_v1(
                action,
                first,
                count,
                mask,
                completion.implementation_tag_v1(),
            ),
        },
    )
}

fn completion_v1(
    value: ReachabilityV1<LockCompletionV1>,
) -> Option<LockCreatedFirstSharedBusyCloseSucceededCompletionV1> {
    match value {
        ReachabilityV1::Reached(LockCompletionV1::UnsafeRetentionSucceededThenRouteUnknown) => {
            Some(LockCreatedFirstSharedBusyCloseSucceededCompletionV1::RetentionSucceeded)
        }
        ReachabilityV1::Reached(LockCompletionV1::UnsafeRetentionRouteUnknownThenRouteUnknown) => {
            Some(LockCreatedFirstSharedBusyCloseSucceededCompletionV1::RetentionRouteUnknown)
        }
        _ => None,
    }
}

fn expected_axes_v1(
    action: LockActionV1,
    first: u8,
    count: u8,
    mask: u8,
    completion: LockCreatedFirstSharedBusyCloseSucceededCompletionV1,
) -> LockAxesV1 {
    LockAxesV1 {
        action: ReachabilityV1::Reached(action),
        first: ReachabilityV1::Reached(first),
        count: ReachabilityV1::Reached(count),
        mask: ReachabilityV1::Reached(mask),
        initialization: ReachabilityV1::NotReached,
        held_shared_mask: ReachabilityV1::Reached(0),
        held_exclusive_mask: ReachabilityV1::Reached(0),
        sibling_shared_mask: ReachabilityV1::Reached(0),
        sibling_exclusive_mask: ReachabilityV1::Reached(0),
        completion: ReachabilityV1::Reached(completion.axis()),
    }
}

fn expected_v1() -> DynamicExpectedV1 {
    DynamicExpectedV1 {
        sqlite: SqliteResultV1::LockUnavailable,
        disposition: TerminalDispositionV1::Quarantined,
        phase: PhaseV1::DmsSharedAcquire,
        failure: FailureClassV1::BusyAfterKnownMutation,
        mutation: MutationStateV1::Known,
        lock_outcome_uncertain: false,
        lock_effect: LockEffectV1::Unchanged,
        dms_lock: DmsLockCustodyV1::Released,
        raw_slots: CustodyStateV1::Unchanged,
        route: CustodyStateV1::Quarantined,
        callback: CustodyStateV1::Retained,
        file: CustodyStateV1::Released,
        mapping: CustodyStateV1::NotReached,
        view: CustodyStateV1::NotReached,
        payload: CustodyStateV1::Retained,
        counts: ObservableCountsV1 {
            callback_begin: 1,
            callback_complete: 1,
            native_lock: 2,
            native_unlock: 1,
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
            return None;
        }
    }
    Some(((((1_u16 << count) - 1) << first) & 0xff) as u8)
}

#[cfg(test)]
pub(super) fn catalog_row_count_for_test() -> usize {
    catalog::catalog_row_count_for_test()
}

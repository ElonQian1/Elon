//! Exact source-bound programs for callback-completion route-unknown outcomes.
//!
//! Classification uses only the typed dynamic key and its complete expected vector. Exact frozen
//! member seals come from the separately committed catalog; leaf ids are never admission inputs.

mod catalog;
#[cfg(windows)]
mod runtime;
mod source_scope;

#[cfg(windows)]
pub(super) use runtime::run_isolated_v1;

use super::super::super::super::{
    source_leaf_authority::{
        CustodyStateV1, Digest32, DmsLockCustodyV1, FailureClassV1, LockEffectV1, LockModeV1,
        MutationStateV1, ObservableCountsV1, RootOperationV1, SqliteResultV1,
        TerminalDispositionV1,
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

pub(super) const CALLBACK_COMPLETION_ROUTE_UNKNOWN_MEMBER_COUNT: usize = 192;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum LockCallbackCompletionRouteUnknownPathV1 {
    LocalSiblingContention,
    NativeRelease,
    NativeAcquireAcquired,
    NativeAcquireBusy,
    SharedLocalAcquire,
    SharedLocalRelease,
}

#[derive(Clone, Copy)]
pub(super) struct LockCallbackCompletionRouteUnknownProgramSpecV1 {
    #[cfg(windows)]
    pub(super) path: LockCallbackCompletionRouteUnknownPathV1,
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
) -> Result<LockCallbackCompletionRouteUnknownProgramSpecV1, LockRunnerExecutionViolationV1> {
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
    let Some(path) = path_v1(key) else {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    };
    let Some(expected_mask) = range_mask_v1(path, action, first, count) else {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    };
    if key.schema_version != DYNAMIC_PROJECTOR_SCHEMA_V1
        || key.root != RootOperationV1::Lock
        || key.prestate != PrestateV1::Lock(expected_prestate_v1(path, action))
        || key.occurrence != OccurrenceV1::Natural
        || key.recipe.callback != CallbackV1::XShmLock
        || key.recipe.observer != ObserverV1::LockCallbackAndSnapshot
        || key.recipe.cleanup != CleanupV1::ParentOwnedRoot
        || mask != expected_mask
        || axes != expected_axes_v1(path, action, first, count, mask)
        || key.expected != expected_v1(path, action, mask)
    {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    }
    Ok(LockCallbackCompletionRouteUnknownProgramSpecV1 {
        #[cfg(windows)]
        path,
        #[cfg(windows)]
        action,
        #[cfg(windows)]
        first,
        #[cfg(windows)]
        count,
        #[cfg(windows)]
        mask,
        member: exact_member_v1(path, action, first, count, mask)?,
        normalized_descriptor_sha256: plan.normalized_descriptor_sha256,
        plan_sha256: plan.plan_sha256,
        implementation_sha256: digest_implementation_v1(path, action, first, count, mask),
    })
}

fn expected_prestate_v1(
    path: LockCallbackCompletionRouteUnknownPathV1,
    action: LockActionV1,
) -> LockPrestateV1 {
    match path {
        LockCallbackCompletionRouteUnknownPathV1::LocalSiblingContention => match action {
            LockActionV1::LockShared => LockPrestateV1::SiblingExclusiveContention,
            LockActionV1::LockExclusive => LockPrestateV1::SiblingAnyContention,
            LockActionV1::UnlockShared | LockActionV1::UnlockExclusive => unreachable!(),
        },
        LockCallbackCompletionRouteUnknownPathV1::NativeRelease => match action {
            LockActionV1::UnlockShared => LockPrestateV1::OwnSharedHeld,
            LockActionV1::UnlockExclusive => LockPrestateV1::OwnExclusiveHeld,
            LockActionV1::LockShared | LockActionV1::LockExclusive => unreachable!(),
        },
        LockCallbackCompletionRouteUnknownPathV1::NativeAcquireAcquired
        | LockCallbackCompletionRouteUnknownPathV1::NativeAcquireBusy => {
            LockPrestateV1::NoHeldLocks
        }
        LockCallbackCompletionRouteUnknownPathV1::SharedLocalAcquire
        | LockCallbackCompletionRouteUnknownPathV1::SharedLocalRelease => {
            LockPrestateV1::SiblingSharedCoalesced
        }
    }
}

fn path_v1(key: &DynamicClassKeyV1) -> Option<LockCallbackCompletionRouteUnknownPathV1> {
    match (
        key.source_site,
        key.stimulus,
        key.prestate,
        key.operation,
        key.phase,
        key.timing,
        key.recipe.fixture,
        key.recipe.fault_seam,
    ) {
        (
            SourceSiteV1::LockLocalState,
            StimulusV1::LockManaged(LockManagedStimulusV1::LocalState),
            PrestateV1::Lock(
                LockPrestateV1::SiblingExclusiveContention | LockPrestateV1::SiblingAnyContention,
            ),
            DynamicOperationV1::Lock(LockOperationV1::LocalAcquire),
            PhaseV1::LockAcquire,
            TimingV1::Natural,
            FixtureV1::ManagedWalMainTwoConnections,
            FaultSeamV1::Natural,
        ) => Some(LockCallbackCompletionRouteUnknownPathV1::LocalSiblingContention),
        (
            SourceSiteV1::LockNativeRelease,
            StimulusV1::LockManaged(LockManagedStimulusV1::NativeRelease),
            PrestateV1::Lock(LockPrestateV1::OwnSharedHeld | LockPrestateV1::OwnExclusiveHeld),
            DynamicOperationV1::Lock(LockOperationV1::NativeRelease),
            PhaseV1::Success,
            TimingV1::AfterSuccess,
            FixtureV1::ManagedWalMainSingleConnection,
            FaultSeamV1::NativeOperation,
        ) => Some(LockCallbackCompletionRouteUnknownPathV1::NativeRelease),
        (
            SourceSiteV1::LockNativeAcquire,
            StimulusV1::LockManaged(LockManagedStimulusV1::NativeAcquire),
            PrestateV1::Lock(LockPrestateV1::NoHeldLocks),
            DynamicOperationV1::Lock(LockOperationV1::NativeAcquire),
            PhaseV1::Success,
            TimingV1::AfterSuccess,
            FixtureV1::ManagedWalMainSingleConnection,
            FaultSeamV1::NativeOperation,
        ) => Some(LockCallbackCompletionRouteUnknownPathV1::NativeAcquireAcquired),
        (
            SourceSiteV1::LockNativeAcquire,
            StimulusV1::LockManaged(LockManagedStimulusV1::NativeAcquire),
            PrestateV1::Lock(LockPrestateV1::NoHeldLocks),
            DynamicOperationV1::Lock(LockOperationV1::NativeAcquire),
            PhaseV1::LockAcquire,
            TimingV1::AtCall,
            FixtureV1::ManagedWalMainSingleConnection,
            FaultSeamV1::NativeOperation,
        ) => Some(LockCallbackCompletionRouteUnknownPathV1::NativeAcquireBusy),
        (
            SourceSiteV1::LockLocalState,
            StimulusV1::LockManaged(LockManagedStimulusV1::LocalState),
            PrestateV1::Lock(LockPrestateV1::SiblingSharedCoalesced),
            DynamicOperationV1::Lock(LockOperationV1::LocalAcquire),
            PhaseV1::Success,
            TimingV1::Natural,
            FixtureV1::ManagedWalMainTwoConnections,
            FaultSeamV1::Natural,
        ) => Some(LockCallbackCompletionRouteUnknownPathV1::SharedLocalAcquire),
        (
            SourceSiteV1::LockLocalState,
            StimulusV1::LockManaged(LockManagedStimulusV1::LocalState),
            PrestateV1::Lock(LockPrestateV1::SiblingSharedCoalesced),
            DynamicOperationV1::Lock(LockOperationV1::LocalRelease),
            PhaseV1::Success,
            TimingV1::Natural,
            FixtureV1::ManagedWalMainTwoConnections,
            FaultSeamV1::Natural,
        ) => Some(LockCallbackCompletionRouteUnknownPathV1::SharedLocalRelease),
        _ => None,
    }
}

fn expected_axes_v1(
    path: LockCallbackCompletionRouteUnknownPathV1,
    action: LockActionV1,
    first: u8,
    count: u8,
    mask: u8,
) -> LockAxesV1 {
    let (initialization, held_shared, held_exclusive, sibling_shared, sibling_exclusive) =
        match path {
            LockCallbackCompletionRouteUnknownPathV1::LocalSiblingContention => match action {
                LockActionV1::LockShared => (ReachabilityV1::NotReached, 0, 0, 0, mask),
                LockActionV1::LockExclusive => (ReachabilityV1::NotReached, 0, 0, mask, 0),
                LockActionV1::UnlockShared | LockActionV1::UnlockExclusive => unreachable!(),
            },
            LockCallbackCompletionRouteUnknownPathV1::NativeRelease => match action {
                LockActionV1::UnlockShared => (ReachabilityV1::NotReached, mask, 0, 0, 0),
                LockActionV1::UnlockExclusive => (ReachabilityV1::NotReached, 0, mask, 0, 0),
                LockActionV1::LockShared | LockActionV1::LockExclusive => unreachable!(),
            },
            LockCallbackCompletionRouteUnknownPathV1::NativeAcquireAcquired
            | LockCallbackCompletionRouteUnknownPathV1::NativeAcquireBusy => (
                ReachabilityV1::Reached(InitializationProfileV1::NodeLive),
                0,
                0,
                0,
                0,
            ),
            LockCallbackCompletionRouteUnknownPathV1::SharedLocalAcquire => {
                (ReachabilityV1::NotReached, 0, 0, mask, 0)
            }
            LockCallbackCompletionRouteUnknownPathV1::SharedLocalRelease => {
                (ReachabilityV1::NotReached, mask, 0, mask, 0)
            }
        };
    LockAxesV1 {
        action: ReachabilityV1::Reached(action),
        first: ReachabilityV1::Reached(first),
        count: ReachabilityV1::Reached(count),
        mask: ReachabilityV1::Reached(mask),
        initialization,
        held_shared_mask: ReachabilityV1::Reached(held_shared),
        held_exclusive_mask: ReachabilityV1::Reached(held_exclusive),
        sibling_shared_mask: ReachabilityV1::Reached(sibling_shared),
        sibling_exclusive_mask: ReachabilityV1::Reached(sibling_exclusive),
        completion: ReachabilityV1::Reached(LockCompletionV1::RouteUnknown),
    }
}

fn expected_v1(
    path: LockCallbackCompletionRouteUnknownPathV1,
    action: LockActionV1,
    mask: u8,
) -> DynamicExpectedV1 {
    let phase = match path {
        LockCallbackCompletionRouteUnknownPathV1::LocalSiblingContention
        | LockCallbackCompletionRouteUnknownPathV1::NativeAcquireBusy => PhaseV1::LockAcquire,
        LockCallbackCompletionRouteUnknownPathV1::NativeRelease
        | LockCallbackCompletionRouteUnknownPathV1::NativeAcquireAcquired
        | LockCallbackCompletionRouteUnknownPathV1::SharedLocalAcquire
        | LockCallbackCompletionRouteUnknownPathV1::SharedLocalRelease => PhaseV1::Success,
    };
    let mutation = match path {
        LockCallbackCompletionRouteUnknownPathV1::LocalSiblingContention
        | LockCallbackCompletionRouteUnknownPathV1::NativeAcquireBusy => MutationStateV1::None,
        LockCallbackCompletionRouteUnknownPathV1::NativeRelease
        | LockCallbackCompletionRouteUnknownPathV1::NativeAcquireAcquired
        | LockCallbackCompletionRouteUnknownPathV1::SharedLocalAcquire
        | LockCallbackCompletionRouteUnknownPathV1::SharedLocalRelease => MutationStateV1::Known,
    };
    let lock_effect = match path {
        LockCallbackCompletionRouteUnknownPathV1::LocalSiblingContention
        | LockCallbackCompletionRouteUnknownPathV1::NativeAcquireBusy => LockEffectV1::Unchanged,
        LockCallbackCompletionRouteUnknownPathV1::NativeAcquireAcquired => LockEffectV1::Acquired {
            mode: mode_v1(action),
            mask,
            native: true,
        },
        LockCallbackCompletionRouteUnknownPathV1::NativeRelease => LockEffectV1::Released {
            mode: mode_v1(action),
            mask,
            native: true,
        },
        LockCallbackCompletionRouteUnknownPathV1::SharedLocalAcquire => LockEffectV1::Acquired {
            mode: LockModeV1::Shared,
            mask,
            native: false,
        },
        LockCallbackCompletionRouteUnknownPathV1::SharedLocalRelease => LockEffectV1::Released {
            mode: LockModeV1::Shared,
            mask,
            native: false,
        },
    };
    DynamicExpectedV1 {
        sqlite: SqliteResultV1::LockUnavailable,
        disposition: TerminalDispositionV1::Quarantined,
        phase,
        failure: FailureClassV1::RegistryRejected,
        mutation,
        lock_outcome_uncertain: false,
        lock_effect,
        dms_lock: DmsLockCustodyV1::ExistingShared,
        raw_slots: CustodyStateV1::Unchanged,
        route: CustodyStateV1::Quarantined,
        callback: CustodyStateV1::Retained,
        file: CustodyStateV1::Unchanged,
        mapping: CustodyStateV1::NotReached,
        view: CustodyStateV1::NotReached,
        payload: CustodyStateV1::NotReached,
        counts: ObservableCountsV1 {
            callback_begin: 1,
            callback_complete: 1,
            native_lock: u16::from(matches!(
                path,
                LockCallbackCompletionRouteUnknownPathV1::NativeAcquireAcquired
                    | LockCallbackCompletionRouteUnknownPathV1::NativeAcquireBusy
            )),
            native_unlock: u16::from(matches!(
                path,
                LockCallbackCompletionRouteUnknownPathV1::NativeRelease
            )),
            ..ObservableCountsV1::default()
        },
    }
}

const fn mode_v1(action: LockActionV1) -> LockModeV1 {
    match action {
        LockActionV1::LockShared | LockActionV1::UnlockShared => LockModeV1::Shared,
        LockActionV1::LockExclusive | LockActionV1::UnlockExclusive => LockModeV1::Exclusive,
    }
}

pub(super) const fn range_mask_v1(
    path: LockCallbackCompletionRouteUnknownPathV1,
    action: LockActionV1,
    first: u8,
    count: u8,
) -> Option<u8> {
    let end = match first.checked_add(count) {
        Some(end) => end,
        None => return None,
    };
    if first >= 8 || count == 0 || end > 8 {
        return None;
    }
    match path {
        LockCallbackCompletionRouteUnknownPathV1::LocalSiblingContention
        | LockCallbackCompletionRouteUnknownPathV1::NativeAcquireAcquired
        | LockCallbackCompletionRouteUnknownPathV1::NativeAcquireBusy => match action {
            LockActionV1::LockShared if count == 1 => {}
            LockActionV1::LockExclusive => {}
            LockActionV1::LockShared
            | LockActionV1::UnlockShared
            | LockActionV1::UnlockExclusive => return None,
        },
        LockCallbackCompletionRouteUnknownPathV1::NativeRelease => match action {
            LockActionV1::UnlockShared if count == 1 => {}
            LockActionV1::UnlockExclusive => {}
            LockActionV1::LockShared | LockActionV1::LockExclusive | LockActionV1::UnlockShared => {
                return None
            }
        },
        LockCallbackCompletionRouteUnknownPathV1::SharedLocalAcquire => {
            if !matches!(action, LockActionV1::LockShared) || count != 1 {
                return None;
            }
        }
        LockCallbackCompletionRouteUnknownPathV1::SharedLocalRelease => {
            if !matches!(action, LockActionV1::UnlockShared) || count != 1 {
                return None;
            }
        }
    }
    Some(((((1_u16 << count) - 1) << first) & 0xff) as u8)
}

#[cfg(test)]
pub(super) fn catalog_row_count_for_test() -> usize {
    catalog::catalog_row_count_for_test()
}

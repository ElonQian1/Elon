//! Exact source-bound programs for pre-managed callback rejection terminals.
//!
//! Admission consumes the complete typed semantic key and Expected vector. Frozen leaf ids,
//! branches, and display strings never participate in classification.

mod catalog;
#[cfg(windows)]
mod runtime;
mod source_scope;

#[cfg(windows)]
pub(super) use runtime::run_isolated_v1;
pub(super) use source_scope::PRE_MANAGED_CALLBACK_REJECTION_PROJECTOR_DELTA_V1;

use super::super::super::super::{
    source_leaf_authority::{
        CustodyStateV1, Digest32, DmsLockCustodyV1, FailureClassV1, LockEffectV1, MutationStateV1,
        ObservableCountsV1, RootOperationV1, SqliteResultV1, TerminalDispositionV1,
    },
    terminal_descriptor::{
        CallbackV1, CleanupV1, FaultSeamV1, FixtureV1, LockActionV1, LockAxesV1, LockCompletionV1,
        LockManagedStimulusV1, LockOperationV1, LockPrestateV1, ObserverV1, OccurrenceV1, PhaseV1,
        PrestateV1, ReachabilityV1, SourceSiteV1, StimulusV1, TimingV1,
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

pub(super) const PRE_MANAGED_CALLBACK_REJECTION_MEMBER_COUNT: usize = 528;
pub(super) const PRE_MANAGED_CALLBACK_REJECTION_FAMILY_MEMBER_COUNT: usize = 88;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum LockPreManagedCallbackRejectionFamilyV1 {
    AdmissionRouteUnknownDirect,
    AdmissionCounterOverflowDirect,
    UnsupportedFileRoleCompleted,
    UnsupportedFileRoleRouteUnknown,
    ShmDetachedCompleted,
    ShmDetachedRouteUnknown,
}

#[derive(Clone, Copy)]
pub(super) struct LockPreManagedCallbackRejectionProgramSpecV1 {
    #[cfg(windows)]
    pub(super) family: LockPreManagedCallbackRejectionFamilyV1,
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
) -> Result<LockPreManagedCallbackRejectionProgramSpecV1, LockRunnerExecutionViolationV1> {
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
    let Some(family) = family_v1(key, axes.completion) else {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    };
    if key.schema_version != DYNAMIC_PROJECTOR_SCHEMA_V1
        || key.root != RootOperationV1::Lock
        || key.prestate != PrestateV1::Lock(LockPrestateV1::NotReached)
        || key.operation != DynamicOperationV1::Lock(LockOperationV1::CallbackAdmission)
        || key.phase != PhaseV1::CallbackAdmission
        || key.timing != TimingV1::BeforeCall
        || key.occurrence != OccurrenceV1::Natural
        || key.recipe.fixture != FixtureV1::ManagedWalMainSingleConnection
        || key.recipe.callback != CallbackV1::XShmLock
        || key.recipe.fault_seam != FaultSeamV1::RegistryAdmission
        || key.recipe.observer != ObserverV1::LockCallbackAndSnapshot
        || key.recipe.cleanup != CleanupV1::ParentOwnedRoot
        || mask != expected_mask
        || axes != expected_axes_v1(family, action, first, count, mask)
        || key.expected != expected_v1(family)
    {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    }
    Ok(LockPreManagedCallbackRejectionProgramSpecV1 {
        #[cfg(windows)]
        family,
        #[cfg(windows)]
        action,
        #[cfg(windows)]
        first,
        #[cfg(windows)]
        count,
        #[cfg(windows)]
        mask,
        member: exact_member_v1(family, action, first, count, mask)?,
        normalized_descriptor_sha256: plan.normalized_descriptor_sha256,
        plan_sha256: plan.plan_sha256,
        implementation_sha256: digest_implementation_v1(family, action, first, count, mask),
    })
}

fn family_v1(
    key: &DynamicClassKeyV1,
    completion: ReachabilityV1<LockCompletionV1>,
) -> Option<LockPreManagedCallbackRejectionFamilyV1> {
    use LockPreManagedCallbackRejectionFamilyV1 as F;
    match (key.source_site, key.stimulus, completion) {
        (
            SourceSiteV1::RegistryCallbackAdmission,
            StimulusV1::LockManaged(LockManagedStimulusV1::AdmissionRouteUnknown),
            ReachabilityV1::Reached(LockCompletionV1::Direct),
        ) => Some(F::AdmissionRouteUnknownDirect),
        (
            SourceSiteV1::RegistryCallbackAdmission,
            StimulusV1::LockManaged(LockManagedStimulusV1::AdmissionCounterOverflow),
            ReachabilityV1::Reached(LockCompletionV1::Direct),
        ) => Some(F::AdmissionCounterOverflowDirect),
        (
            SourceSiteV1::AdapterDispatch,
            StimulusV1::LockManaged(LockManagedStimulusV1::UnsupportedFileRole),
            ReachabilityV1::Reached(LockCompletionV1::Completed),
        ) => Some(F::UnsupportedFileRoleCompleted),
        (
            SourceSiteV1::AdapterDispatch,
            StimulusV1::LockManaged(LockManagedStimulusV1::UnsupportedFileRole),
            ReachabilityV1::Reached(LockCompletionV1::RouteUnknown),
        ) => Some(F::UnsupportedFileRoleRouteUnknown),
        (
            SourceSiteV1::AdapterDispatch,
            StimulusV1::LockManaged(LockManagedStimulusV1::ShmDetached),
            ReachabilityV1::Reached(LockCompletionV1::Completed),
        ) => Some(F::ShmDetachedCompleted),
        (
            SourceSiteV1::AdapterDispatch,
            StimulusV1::LockManaged(LockManagedStimulusV1::ShmDetached),
            ReachabilityV1::Reached(LockCompletionV1::RouteUnknown),
        ) => Some(F::ShmDetachedRouteUnknown),
        _ => None,
    }
}

fn expected_axes_v1(
    family: LockPreManagedCallbackRejectionFamilyV1,
    action: LockActionV1,
    first: u8,
    count: u8,
    mask: u8,
) -> LockAxesV1 {
    LockAxesV1 {
        action: ReachabilityV1::Reached(action),
        first: ReachabilityV1::Reached(first),
        count: ReachabilityV1::Reached(count),
        mask: ReachabilityV1::Reached(mask),
        initialization: ReachabilityV1::NotReached,
        held_shared_mask: ReachabilityV1::NotReached,
        held_exclusive_mask: ReachabilityV1::NotReached,
        sibling_shared_mask: ReachabilityV1::NotReached,
        sibling_exclusive_mask: ReachabilityV1::NotReached,
        completion: ReachabilityV1::Reached(completion_v1(family)),
    }
}

fn expected_v1(family: LockPreManagedCallbackRejectionFamilyV1) -> DynamicExpectedV1 {
    use LockPreManagedCallbackRejectionFamilyV1 as F;
    let direct = matches!(
        family,
        F::AdmissionRouteUnknownDirect | F::AdmissionCounterOverflowDirect
    );
    let route_unknown = matches!(
        family,
        F::AdmissionRouteUnknownDirect
            | F::AdmissionCounterOverflowDirect
            | F::UnsupportedFileRoleRouteUnknown
            | F::ShmDetachedRouteUnknown
    );
    DynamicExpectedV1 {
        sqlite: SqliteResultV1::LockUnavailable,
        disposition: if matches!(
            family,
            F::AdmissionCounterOverflowDirect
                | F::UnsupportedFileRoleRouteUnknown
                | F::ShmDetachedRouteUnknown
        ) {
            TerminalDispositionV1::Quarantined
        } else {
            TerminalDispositionV1::Returned
        },
        phase: PhaseV1::CallbackAdmission,
        failure: FailureClassV1::RegistryRejected,
        mutation: MutationStateV1::None,
        lock_outcome_uncertain: false,
        lock_effect: if direct {
            LockEffectV1::Unchanged
        } else {
            LockEffectV1::NotReached
        },
        dms_lock: DmsLockCustodyV1::NotReached,
        raw_slots: CustodyStateV1::Unchanged,
        route: if route_unknown {
            CustodyStateV1::Quarantined
        } else {
            CustodyStateV1::Unchanged
        },
        callback: if direct {
            CustodyStateV1::NotReached
        } else if route_unknown {
            CustodyStateV1::Retained
        } else {
            CustodyStateV1::Released
        },
        file: CustodyStateV1::Unchanged,
        mapping: CustodyStateV1::NotReached,
        view: CustodyStateV1::NotReached,
        payload: CustodyStateV1::NotReached,
        counts: ObservableCountsV1 {
            callback_begin: 1,
            callback_complete: u16::from(!direct),
            ..ObservableCountsV1::default()
        },
    }
}

const fn completion_v1(family: LockPreManagedCallbackRejectionFamilyV1) -> LockCompletionV1 {
    use LockPreManagedCallbackRejectionFamilyV1 as F;
    match family {
        F::AdmissionRouteUnknownDirect | F::AdmissionCounterOverflowDirect => {
            LockCompletionV1::Direct
        }
        F::UnsupportedFileRoleCompleted | F::ShmDetachedCompleted => LockCompletionV1::Completed,
        F::UnsupportedFileRoleRouteUnknown | F::ShmDetachedRouteUnknown => {
            LockCompletionV1::RouteUnknown
        }
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
        LockActionV1::LockShared | LockActionV1::UnlockShared if count == 1 => {}
        LockActionV1::LockExclusive | LockActionV1::UnlockExclusive => {}
        LockActionV1::LockShared | LockActionV1::UnlockShared => return None,
    }
    Some(((((1_u16 << count) - 1) << first) & 0xff) as u8)
}

#[cfg(test)]
pub(super) fn catalog_row_count_for_test() -> usize {
    catalog::catalog_row_count_for_test()
}

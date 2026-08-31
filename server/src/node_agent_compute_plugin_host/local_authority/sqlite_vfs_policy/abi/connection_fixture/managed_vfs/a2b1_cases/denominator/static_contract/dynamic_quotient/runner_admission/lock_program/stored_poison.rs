//! Exact source-bound programs for both stored-poison Lock quarantine retention completions.
//!
//! The successful-retention family preserves the q3 wire shape and native-receipt domain. Its
//! source-bound implementation seal intentionally moves with the expanded source scope. The exact
//! route-already-unknown sibling is domain-separated behind the additive q4 receipt and a test-only
//! one-shot route preemption after the real installed `xShmLock` has returned its unsafe failure.

mod catalog;
mod source_scope;

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

pub(super) const STORED_POISON_COMPLETION_MEMBER_COUNT: usize = 1_320;
pub(super) const STORED_POISON_MEMBER_COUNT: usize = 2_640;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LockStoredPoisonCompletionV1 {
    RetentionSucceeded,
    RetentionRouteUnknown,
}

impl LockStoredPoisonCompletionV1 {
    pub(super) const fn axis(self) -> LockCompletionV1 {
        match self {
            Self::RetentionSucceeded => LockCompletionV1::UnsafeRetentionSucceededThenRouteUnknown,
            Self::RetentionRouteUnknown => {
                LockCompletionV1::UnsafeRetentionRouteUnknownThenRouteUnknown
            }
        }
    }

    pub(super) const fn ordinal(self) -> u8 {
        match self {
            Self::RetentionSucceeded => 4,
            Self::RetentionRouteUnknown => 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LockStoredPoisonProfileV1 {
    GateNoMutation,
    FileCloseNoMutation,
    ExactSiblingDeleteNoMutation,
    ExactSiblingOpenUncertain,
    DmsTruncateUncertain,
    FileCloseUncertain,
    ExactSiblingDeleteUncertain,
    FileGrowUncertain,
    MappingCloseUncertain,
    ViewUnmapUncertain,
    LockReleaseUncertain,
    ConnectionDetachUncertain,
    DeleteAuthorizationUncertain,
    DmsExclusiveReleaseUncertain,
    DmsSharedReleaseUncertain,
}

pub(super) const STORED_POISON_PROFILES: [LockStoredPoisonProfileV1; 15] = [
    LockStoredPoisonProfileV1::GateNoMutation,
    LockStoredPoisonProfileV1::FileCloseNoMutation,
    LockStoredPoisonProfileV1::ExactSiblingDeleteNoMutation,
    LockStoredPoisonProfileV1::ExactSiblingOpenUncertain,
    LockStoredPoisonProfileV1::DmsTruncateUncertain,
    LockStoredPoisonProfileV1::FileCloseUncertain,
    LockStoredPoisonProfileV1::ExactSiblingDeleteUncertain,
    LockStoredPoisonProfileV1::FileGrowUncertain,
    LockStoredPoisonProfileV1::MappingCloseUncertain,
    LockStoredPoisonProfileV1::ViewUnmapUncertain,
    LockStoredPoisonProfileV1::LockReleaseUncertain,
    LockStoredPoisonProfileV1::ConnectionDetachUncertain,
    LockStoredPoisonProfileV1::DeleteAuthorizationUncertain,
    LockStoredPoisonProfileV1::DmsExclusiveReleaseUncertain,
    LockStoredPoisonProfileV1::DmsSharedReleaseUncertain,
];

impl LockStoredPoisonProfileV1 {
    pub(super) const fn tag(self) -> &'static str {
        match self {
            Self::GateNoMutation => "gate-none-lock-certain",
            Self::FileCloseNoMutation => "file-close-none-lock-certain",
            Self::ExactSiblingDeleteNoMutation => "exact-sibling-delete-none-lock-certain",
            Self::ExactSiblingOpenUncertain => "exact-sibling-open-uncertain-lock-certain",
            Self::DmsTruncateUncertain => "dms-truncate-uncertain-lock-certain",
            Self::FileCloseUncertain => "file-close-uncertain-lock-certain",
            Self::ExactSiblingDeleteUncertain => "exact-sibling-delete-uncertain-lock-certain",
            Self::FileGrowUncertain => "file-grow-uncertain-lock-certain",
            Self::MappingCloseUncertain => "mapping-close-uncertain-lock-certain",
            Self::ViewUnmapUncertain => "view-unmap-uncertain-lock-certain",
            Self::LockReleaseUncertain => "lock-release-none-lock-uncertain",
            Self::ConnectionDetachUncertain => "connection-detach-none-lock-uncertain",
            Self::DeleteAuthorizationUncertain => "delete-authorization-none-lock-uncertain",
            Self::DmsExclusiveReleaseUncertain => "dms-exclusive-release-uncertain-lock-uncertain",
            Self::DmsSharedReleaseUncertain => "dms-shared-release-uncertain-lock-uncertain",
        }
    }

    pub(super) const fn phase(self) -> PhaseV1 {
        match self {
            Self::GateNoMutation => PhaseV1::Gate,
            Self::FileCloseNoMutation | Self::FileCloseUncertain => PhaseV1::FileClose,
            Self::ExactSiblingDeleteNoMutation | Self::ExactSiblingDeleteUncertain => {
                PhaseV1::ExactSiblingDelete
            }
            Self::ExactSiblingOpenUncertain => PhaseV1::ExactSiblingOpen,
            Self::DmsTruncateUncertain => PhaseV1::DmsTruncate,
            Self::FileGrowUncertain => PhaseV1::FileGrow,
            Self::MappingCloseUncertain => PhaseV1::MappingClose,
            Self::ViewUnmapUncertain => PhaseV1::ViewUnmap,
            Self::LockReleaseUncertain => PhaseV1::LockRelease,
            Self::ConnectionDetachUncertain => PhaseV1::ConnectionDetach,
            Self::DeleteAuthorizationUncertain => PhaseV1::DeleteAuthorization,
            Self::DmsExclusiveReleaseUncertain => PhaseV1::DmsExclusiveRelease,
            Self::DmsSharedReleaseUncertain => PhaseV1::DmsSharedRelease,
        }
    }

    pub(super) const fn mutation(self) -> MutationStateV1 {
        match self {
            Self::GateNoMutation
            | Self::FileCloseNoMutation
            | Self::ExactSiblingDeleteNoMutation
            | Self::LockReleaseUncertain
            | Self::ConnectionDetachUncertain
            | Self::DeleteAuthorizationUncertain => MutationStateV1::None,
            Self::ExactSiblingOpenUncertain
            | Self::DmsTruncateUncertain
            | Self::FileCloseUncertain
            | Self::ExactSiblingDeleteUncertain
            | Self::FileGrowUncertain
            | Self::MappingCloseUncertain
            | Self::ViewUnmapUncertain
            | Self::DmsExclusiveReleaseUncertain
            | Self::DmsSharedReleaseUncertain => MutationStateV1::Uncertain,
        }
    }

    pub(super) const fn lock_outcome_uncertain(self) -> bool {
        matches!(
            self,
            Self::LockReleaseUncertain
                | Self::ConnectionDetachUncertain
                | Self::DeleteAuthorizationUncertain
                | Self::DmsExclusiveReleaseUncertain
                | Self::DmsSharedReleaseUncertain
        )
    }

    pub(super) const fn ordinal(self) -> u8 {
        match self {
            Self::GateNoMutation => 0,
            Self::FileCloseNoMutation => 1,
            Self::ExactSiblingDeleteNoMutation => 2,
            Self::ExactSiblingOpenUncertain => 3,
            Self::DmsTruncateUncertain => 4,
            Self::FileCloseUncertain => 5,
            Self::ExactSiblingDeleteUncertain => 6,
            Self::FileGrowUncertain => 7,
            Self::MappingCloseUncertain => 8,
            Self::ViewUnmapUncertain => 9,
            Self::LockReleaseUncertain => 10,
            Self::ConnectionDetachUncertain => 11,
            Self::DeleteAuthorizationUncertain => 12,
            Self::DmsExclusiveReleaseUncertain => 13,
            Self::DmsSharedReleaseUncertain => 14,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct LockStoredPoisonProgramSpecV1 {
    #[cfg(windows)]
    pub(super) action: LockActionV1,
    #[cfg(windows)]
    pub(super) first: u8,
    #[cfg(windows)]
    pub(super) count: u8,
    #[cfg(windows)]
    pub(super) mask: u8,
    #[cfg(windows)]
    pub(super) profile: LockStoredPoisonProfileV1,
    #[cfg(windows)]
    pub(super) completion: LockStoredPoisonCompletionV1,
    pub(super) member: StaticMemberSealV1,
    pub(super) normalized_descriptor_sha256: Digest32,
    pub(super) plan_sha256: Digest32,
    pub(super) implementation_sha256: Digest32,
}

pub(super) fn program_spec_v1(
    key: &DynamicClassKeyV1,
    plan: CompiledRunnerPlanV1,
) -> Result<LockStoredPoisonProgramSpecV1, LockRunnerExecutionViolationV1> {
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
    let Some(profile) = classify_profile_v1(key) else {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    };
    let Some(completion) = classify_completion_v1(axes.completion) else {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    };
    let Some(expected_mask) = range_mask_v1(action, first, count) else {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    };
    if mask != expected_mask
        || axes != expected_axes_v1(action, first, count, mask, completion)
        || key.expected != expected_v1(profile)
    {
        return Err(LockRunnerExecutionViolationV1::UnsupportedProgram);
    }
    Ok(LockStoredPoisonProgramSpecV1 {
        #[cfg(windows)]
        action,
        #[cfg(windows)]
        first,
        #[cfg(windows)]
        count,
        #[cfg(windows)]
        mask,
        #[cfg(windows)]
        profile,
        #[cfg(windows)]
        completion,
        member: exact_member_v1(action, first, count, mask, profile, completion)?,
        normalized_descriptor_sha256: plan.normalized_descriptor_sha256,
        plan_sha256: plan.plan_sha256,
        implementation_sha256: digest_implementation_v1(action, first, count, profile, completion),
    })
}

fn classify_profile_v1(key: &DynamicClassKeyV1) -> Option<LockStoredPoisonProfileV1> {
    if key.schema_version != DYNAMIC_PROJECTOR_SCHEMA_V1
        || key.root != RootOperationV1::Lock
        || key.source_site != SourceSiteV1::CoordinatorState
        || key.stimulus != StimulusV1::LockManaged(LockManagedStimulusV1::StoredPoison)
        || key.prestate != PrestateV1::Lock(LockPrestateV1::StoredPoison)
        || key.operation != DynamicOperationV1::Lock(LockOperationV1::Quarantine)
        || key.timing != TimingV1::BeforeCall
        || key.occurrence != OccurrenceV1::Natural
        || key.recipe.fixture != FixtureV1::ManagedWalMainSingleConnection
        || key.recipe.callback != CallbackV1::XShmLock
        || key.recipe.fault_seam != FaultSeamV1::Natural
        || key.recipe.observer != ObserverV1::LockCallbackAndSnapshot
        || key.recipe.cleanup != CleanupV1::RetainUnsafeCustodyThenParentCleanup
    {
        return None;
    }
    STORED_POISON_PROFILES.into_iter().find(|profile| {
        key.phase == profile.phase()
            && key.expected.mutation == profile.mutation()
            && key.expected.lock_outcome_uncertain == profile.lock_outcome_uncertain()
    })
}

fn classify_completion_v1(
    completion: ReachabilityV1<LockCompletionV1>,
) -> Option<LockStoredPoisonCompletionV1> {
    match completion {
        ReachabilityV1::Reached(LockCompletionV1::UnsafeRetentionSucceededThenRouteUnknown) => {
            Some(LockStoredPoisonCompletionV1::RetentionSucceeded)
        }
        ReachabilityV1::Reached(LockCompletionV1::UnsafeRetentionRouteUnknownThenRouteUnknown) => {
            Some(LockStoredPoisonCompletionV1::RetentionRouteUnknown)
        }
        _ => None,
    }
}

fn expected_axes_v1(
    action: LockActionV1,
    first: u8,
    count: u8,
    mask: u8,
    completion: LockStoredPoisonCompletionV1,
) -> LockAxesV1 {
    LockAxesV1 {
        action: ReachabilityV1::Reached(action),
        first: ReachabilityV1::Reached(first),
        count: ReachabilityV1::Reached(count),
        mask: ReachabilityV1::Reached(mask),
        completion: ReachabilityV1::Reached(completion.axis()),
        ..LockAxesV1::NOT_REACHED
    }
}

fn expected_v1(profile: LockStoredPoisonProfileV1) -> DynamicExpectedV1 {
    DynamicExpectedV1 {
        sqlite: SqliteResultV1::LockUnavailable,
        disposition: TerminalDispositionV1::Quarantined,
        phase: profile.phase(),
        failure: FailureClassV1::OutcomeUncertainPoisoned,
        mutation: profile.mutation(),
        lock_outcome_uncertain: profile.lock_outcome_uncertain(),
        lock_effect: LockEffectV1::Unchanged,
        dms_lock: DmsLockCustodyV1::UnobservedRetained,
        raw_slots: CustodyStateV1::Unchanged,
        route: CustodyStateV1::Quarantined,
        callback: CustodyStateV1::Retained,
        file: CustodyStateV1::Unchanged,
        mapping: CustodyStateV1::NotReached,
        view: CustodyStateV1::NotReached,
        payload: CustodyStateV1::Retained,
        counts: ObservableCountsV1 {
            callback_begin: 1,
            callback_complete: 1,
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
    if matches!(
        action,
        LockActionV1::LockShared | LockActionV1::UnlockShared
    ) && count != 1
    {
        return None;
    }
    Some(((((1_u16 << count) - 1) << first) & 0xff) as u8)
}

#[cfg(test)]
pub(super) fn catalog_row_count_for_test() -> usize {
    catalog::catalog_row_count_for_test()
}

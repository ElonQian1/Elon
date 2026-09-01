//! Independent typed fixtures for the 88 q14 created-first truncate-error release-ok terminals.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use super::super::super::terminal_descriptor::{
    InitializationFaultSiteV1, InitializationPathV1, InitializationStimulusV1,
};
use super::*;

pub(super) const LOCK_CREATED_FIRST_TRUNCATE_ERROR_RELEASE_SUCCEEDED_MEMBER_COUNT: usize = 88;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum FrozenCreatedFirstTruncateErrorReleaseSucceededCompletionV1 {
    RetentionSucceeded,
    RetentionRouteUnknown,
}

impl FrozenCreatedFirstTruncateErrorReleaseSucceededCompletionV1 {
    pub(super) const fn axis(self) -> LockCompletionV1 {
        match self {
            Self::RetentionSucceeded => LockCompletionV1::UnsafeRetentionSucceededThenRouteUnknown,
            Self::RetentionRouteUnknown => {
                LockCompletionV1::UnsafeRetentionRouteUnknownThenRouteUnknown
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct FrozenLockCreatedFirstTruncateErrorReleaseSucceededCaseV1 {
    pub(super) action: LockActionV1,
    pub(super) first: u8,
    pub(super) count: u8,
    pub(super) mask: u8,
    pub(super) completion: FrozenCreatedFirstTruncateErrorReleaseSucceededCompletionV1,
}

#[derive(Clone)]
pub(super) struct FrozenLockCreatedFirstTruncateErrorReleaseSucceededLeafV1 {
    pub(super) record: LeafRecordV1,
    pub(super) descriptor: TerminalDescriptorV1,
    pub(super) member: StaticMemberSealV1,
}

pub(super) fn frozen_lock_created_first_truncate_error_release_succeeded_leaves_v1(
) -> &'static BTreeMap<
    FrozenLockCreatedFirstTruncateErrorReleaseSucceededCaseV1,
    FrozenLockCreatedFirstTruncateErrorReleaseSucceededLeafV1,
> {
    static LEAVES: OnceLock<
        BTreeMap<
            FrozenLockCreatedFirstTruncateErrorReleaseSucceededCaseV1,
            FrozenLockCreatedFirstTruncateErrorReleaseSucceededLeafV1,
        >,
    > = OnceLock::new();
    LEAVES.get_or_init(|| {
        let graph = super::super::super::lock::graph();
        let mut leaves = BTreeMap::new();
        super::super::super::source_leaf_authority::validate_lock_graph_with_records(
            &graph,
            |leaf| {
                let StreamedLeafV1::Terminal {
                    record,
                    descriptor,
                    seal,
                } = leaf
                else {
                    return Ok(());
                };
                let Some(case) =
                    created_first_truncate_error_release_succeeded_v1(record, descriptor)
                else {
                    return Ok(());
                };
                let previous = leaves.insert(
                    case,
                    FrozenLockCreatedFirstTruncateErrorReleaseSucceededLeafV1 {
                        record: record.clone(),
                        descriptor: *descriptor,
                        member: StaticMemberSealV1 {
                            case_key_sha256: seal.case_key_sha256,
                            full_record_sha256: seal.full_record_sha256,
                        },
                    },
                );
                if previous.is_some() {
                    return Err(format!("duplicate frozen q14 member {case:?}"));
                }
                Ok(())
            },
        )
        .expect("the frozen Lock authority must validate before q14 source tests");
        assert_eq!(
            leaves.len(),
            LOCK_CREATED_FIRST_TRUNCATE_ERROR_RELEASE_SUCCEEDED_MEMBER_COUNT
        );
        leaves
    })
}

pub(super) fn lock_created_first_truncate_error_release_succeeded_descriptor_v1(
    case: FrozenLockCreatedFirstTruncateErrorReleaseSucceededCaseV1,
    capability: RunnerCapabilityV1,
) -> TerminalDescriptorV1 {
    let mut descriptor =
        frozen_lock_created_first_truncate_error_release_succeeded_leaves_v1()[&case].descriptor;
    let TerminalDescriptorV1::Lock(value) = &mut descriptor else {
        unreachable!("a q14 fixture must have a Lock descriptor")
    };
    value.recipe.capability = capability;
    descriptor
}

pub(super) fn lock_created_first_truncate_error_release_succeeded_expected_groups_v1(
) -> BTreeSet<(DynamicClassKeyV1, StaticMemberSealV1)> {
    frozen_lock_created_first_truncate_error_release_succeeded_leaves_v1()
        .values()
        .map(|leaf| {
            (
                prepare_dynamic_terminal_v1(&leaf.record, &leaf.descriptor)
                    .expect("a frozen q14 descriptor must prepare")
                    .key,
                leaf.member,
            )
        })
        .collect()
}

fn created_first_truncate_error_release_succeeded_v1(
    record: &LeafRecordV1,
    descriptor: &TerminalDescriptorV1,
) -> Option<FrozenLockCreatedFirstTruncateErrorReleaseSucceededCaseV1> {
    let TerminalDescriptorV1::Lock(value) = descriptor else {
        return None;
    };
    let StimulusV1::Initialization(stimulus) = value.stimulus else {
        return None;
    };
    let (
        ReachabilityV1::Reached(action),
        ReachabilityV1::Reached(first),
        ReachabilityV1::Reached(count),
        ReachabilityV1::Reached(mask),
        ReachabilityV1::Reached(completion),
    ) = (
        value.axes.action,
        value.axes.first,
        value.axes.count,
        value.axes.mask,
        value.axes.completion,
    )
    else {
        return None;
    };
    let completion = match completion {
        LockCompletionV1::UnsafeRetentionSucceededThenRouteUnknown => {
            FrozenCreatedFirstTruncateErrorReleaseSucceededCompletionV1::RetentionSucceeded
        }
        LockCompletionV1::UnsafeRetentionRouteUnknownThenRouteUnknown => {
            FrozenCreatedFirstTruncateErrorReleaseSucceededCompletionV1::RetentionRouteUnknown
        }
        _ => return None,
    };
    let case = FrozenLockCreatedFirstTruncateErrorReleaseSucceededCaseV1 {
        action,
        first,
        count,
        mask,
        completion,
    };
    if range_mask_v1(action, first, count) != Some(mask)
        || value.source_site != SourceSiteV1::InitializationDms
        || stimulus
            != (InitializationStimulusV1 {
                fault_site: InitializationFaultSiteV1::DmsTruncate,
                path: InitializationPathV1::CreatedFirst,
                cleanup_rewrite: false,
            })
        || value.prestate != PrestateV1::Lock(LockPrestateV1::NoHeldLocks)
        || value.operation != LockOperationV1::Initialization
        || value.phase != PhaseV1::DmsTruncate
        || value.timing != TimingV1::AtCall
        || value.occurrence != OccurrenceV1::Natural
        || value.recipe.fixture != FixtureV1::ManagedWalMainSingleConnection
        || value.recipe.callback != CallbackV1::XShmLock
        || value.recipe.fault_seam != FaultSeamV1::Initialization
        || value.recipe.observer != ObserverV1::LockCallbackAndSnapshot
        || value.recipe.cleanup != CleanupV1::RetainUnsafeCustodyThenParentCleanup
        || value.recipe.capability
            != RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete)
        || value.axes != expected_axes_v1(case)
    {
        return None;
    }
    let LeafOutcomeV1::Terminal(expected) = &record.outcome else {
        return None;
    };
    (expected == &expected_v1()).then_some(case)
}

const fn expected_axes_v1(
    case: FrozenLockCreatedFirstTruncateErrorReleaseSucceededCaseV1,
) -> LockAxesV1 {
    LockAxesV1 {
        action: ReachabilityV1::Reached(case.action),
        first: ReachabilityV1::Reached(case.first),
        count: ReachabilityV1::Reached(case.count),
        mask: ReachabilityV1::Reached(case.mask),
        initialization: ReachabilityV1::NotReached,
        held_shared_mask: ReachabilityV1::Reached(0),
        held_exclusive_mask: ReachabilityV1::Reached(0),
        sibling_shared_mask: ReachabilityV1::Reached(0),
        sibling_exclusive_mask: ReachabilityV1::Reached(0),
        completion: ReachabilityV1::Reached(case.completion.axis()),
    }
}

fn expected_v1() -> ExpectedV1 {
    ExpectedV1 {
        sqlite: SqliteResultV1::LockUnavailable,
        disposition: TerminalDispositionV1::Quarantined,
        phase: "DmsTruncate".to_owned(),
        failure: FailureClassV1::OutcomeUncertainPoisoned,
        mutation: MutationStateV1::Uncertain,
        lock_outcome_uncertain: false,
        lock_effect: LockEffectV1::Unchanged,
        dms_lock: DmsLockCustodyV1::Released,
        raw_slots: CustodyStateV1::Unchanged,
        route: CustodyStateV1::Quarantined,
        callback: CustodyStateV1::Retained,
        file: CustodyStateV1::Retained,
        mapping: CustodyStateV1::NotReached,
        view: CustodyStateV1::NotReached,
        payload: CustodyStateV1::Retained,
        counts: ObservableCountsV1 {
            callback_begin: 1,
            callback_complete: 1,
            native_lock: 1,
            native_unlock: 1,
            ..ObservableCountsV1::default()
        },
    }
}

const fn range_mask_v1(action: LockActionV1, first: u8, count: u8) -> Option<u8> {
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

#[test]
fn frozen_q14_family_is_exact_unique_and_only_contains_acquire_requests() {
    let leaves = frozen_lock_created_first_truncate_error_release_succeeded_leaves_v1();
    assert_eq!(
        leaves.len(),
        LOCK_CREATED_FIRST_TRUNCATE_ERROR_RELEASE_SUCCEEDED_MEMBER_COUNT
    );
    assert_eq!(
        leaves
            .values()
            .map(|leaf| leaf.member)
            .collect::<BTreeSet<_>>()
            .len(),
        LOCK_CREATED_FIRST_TRUNCATE_ERROR_RELEASE_SUCCEEDED_MEMBER_COUNT
    );
    let shared = leaves
        .keys()
        .filter(|case| case.action == LockActionV1::LockShared)
        .count();
    let exclusive = leaves
        .keys()
        .filter(|case| case.action == LockActionV1::LockExclusive)
        .count();
    assert_eq!((shared, exclusive), (16, 72));
    assert!(leaves.keys().all(|case| matches!(
        case.action,
        LockActionV1::LockShared | LockActionV1::LockExclusive
    )));
}

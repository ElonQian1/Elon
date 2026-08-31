//! Frozen exact fixtures for both stored-poison Lock retention completions.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};

use super::*;

pub(super) const LOCK_STORED_POISON_COMPLETION_MEMBER_COUNT: usize = 1_320;
pub(super) const LOCK_STORED_POISON_MEMBER_COUNT: usize = 2_640;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum LockStoredPoisonActionV1 {
    LockShared,
    LockExclusive,
    UnlockShared,
    UnlockExclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum LockStoredPoisonCompletionV1 {
    RetentionSucceeded,
    RetentionRouteUnknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

pub(super) const LOCK_STORED_POISON_PROFILES: [LockStoredPoisonProfileV1; 15] = [
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct LockStoredPoisonKeyV1 {
    pub(super) action: LockStoredPoisonActionV1,
    pub(super) first: u8,
    pub(super) count: u8,
    pub(super) profile: LockStoredPoisonProfileV1,
    pub(super) completion: LockStoredPoisonCompletionV1,
}

#[derive(Clone)]
pub(super) struct FrozenLockStoredPoisonLeafV1 {
    pub(super) record: LeafRecordV1,
    pub(super) descriptor: TerminalDescriptorV1,
    pub(super) member: StaticMemberSealV1,
}

pub(super) fn frozen_lock_stored_poison_leaves_v1(
) -> &'static BTreeMap<LockStoredPoisonKeyV1, FrozenLockStoredPoisonLeafV1> {
    static LEAVES: OnceLock<BTreeMap<LockStoredPoisonKeyV1, FrozenLockStoredPoisonLeafV1>> =
        OnceLock::new();
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
                let Some(key) = stored_poison_key_v1(record, descriptor) else {
                    return Ok(());
                };
                let previous = leaves.insert(
                    key,
                    FrozenLockStoredPoisonLeafV1 {
                        record: record.clone(),
                        descriptor: *descriptor,
                        member: StaticMemberSealV1 {
                            case_key_sha256: seal.case_key_sha256,
                            full_record_sha256: seal.full_record_sha256,
                        },
                    },
                );
                if previous.is_some() {
                    return Err(format!(
                        "duplicate frozen Lock stored-poison member {key:?}"
                    ));
                }
                Ok(())
            },
        )
        .expect("the frozen Lock authority must validate before stored-poison program tests");
        assert_eq!(leaves.len(), LOCK_STORED_POISON_MEMBER_COUNT);
        leaves
    })
}

pub(super) fn lock_stored_poison_leaf_v1(
    key: LockStoredPoisonKeyV1,
) -> FrozenLockStoredPoisonLeafV1 {
    frozen_lock_stored_poison_leaves_v1()
        .get(&key)
        .unwrap_or_else(|| panic!("missing frozen Lock stored-poison member {key:?}"))
        .clone()
}

pub(super) fn lock_stored_poison_descriptor_v1(
    key: LockStoredPoisonKeyV1,
    capability: RunnerCapabilityV1,
) -> TerminalDescriptorV1 {
    let mut descriptor = lock_stored_poison_leaf_v1(key).descriptor;
    let TerminalDescriptorV1::Lock(value) = &mut descriptor else {
        unreachable!("a Lock frozen program must have a Lock descriptor")
    };
    value.recipe.capability = capability;
    descriptor
}

fn stored_poison_key_v1(
    record: &LeafRecordV1,
    descriptor: &TerminalDescriptorV1,
) -> Option<LockStoredPoisonKeyV1> {
    let TerminalDescriptorV1::Lock(value) = descriptor else {
        return None;
    };
    if value.source_site != SourceSiteV1::CoordinatorState
        || value.stimulus != StimulusV1::LockManaged(LockManagedStimulusV1::StoredPoison)
        || value.prestate != PrestateV1::Lock(LockPrestateV1::StoredPoison)
        || value.operation != LockOperationV1::Quarantine
        || value.timing != TimingV1::BeforeCall
        || value.occurrence != OccurrenceV1::Natural
        || value.recipe.fixture != FixtureV1::ManagedWalMainSingleConnection
        || value.recipe.callback != CallbackV1::XShmLock
        || value.recipe.fault_seam != FaultSeamV1::Natural
        || value.recipe.observer != ObserverV1::LockCallbackAndSnapshot
        || value.recipe.cleanup != CleanupV1::RetainUnsafeCustodyThenParentCleanup
        || value.recipe.capability
            != RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete)
    {
        return None;
    }
    let completion = match value.axes.completion {
        ReachabilityV1::Reached(LockCompletionV1::UnsafeRetentionSucceededThenRouteUnknown) => {
            LockStoredPoisonCompletionV1::RetentionSucceeded
        }
        ReachabilityV1::Reached(LockCompletionV1::UnsafeRetentionRouteUnknownThenRouteUnknown) => {
            LockStoredPoisonCompletionV1::RetentionRouteUnknown
        }
        _ => return None,
    };
    let (
        ReachabilityV1::Reached(action),
        ReachabilityV1::Reached(first),
        ReachabilityV1::Reached(count),
        ReachabilityV1::Reached(mask),
    ) = (
        value.axes.action,
        value.axes.first,
        value.axes.count,
        value.axes.mask,
    )
    else {
        return None;
    };
    if mask != range_mask(action, first, count)? {
        return None;
    }
    let LeafOutcomeV1::Terminal(expected) = &record.outcome else {
        return None;
    };
    let profile = profile_v1(
        value.phase,
        expected.mutation,
        expected.lock_outcome_uncertain,
    )?;
    Some(LockStoredPoisonKeyV1 {
        action: action_v1(action),
        first,
        count,
        profile,
        completion,
    })
}

fn action_v1(action: LockActionV1) -> LockStoredPoisonActionV1 {
    match action {
        LockActionV1::LockShared => LockStoredPoisonActionV1::LockShared,
        LockActionV1::LockExclusive => LockStoredPoisonActionV1::LockExclusive,
        LockActionV1::UnlockShared => LockStoredPoisonActionV1::UnlockShared,
        LockActionV1::UnlockExclusive => LockStoredPoisonActionV1::UnlockExclusive,
    }
}

fn range_mask(action: LockActionV1, first: u8, count: u8) -> Option<u8> {
    let end = first.checked_add(count)?;
    if count == 0 || first >= 8 || end > 8 {
        return None;
    }
    if matches!(
        action,
        LockActionV1::LockShared | LockActionV1::UnlockShared
    ) && count != 1
    {
        return None;
    }
    Some((((1_u16 << count) - 1) << first) as u8)
}

fn profile_v1(
    phase: PhaseV1,
    mutation: MutationStateV1,
    lock_uncertain: bool,
) -> Option<LockStoredPoisonProfileV1> {
    use LockStoredPoisonProfileV1 as Profile;
    match (phase, mutation, lock_uncertain) {
        (PhaseV1::Gate, MutationStateV1::None, false) => Some(Profile::GateNoMutation),
        (PhaseV1::FileClose, MutationStateV1::None, false) => Some(Profile::FileCloseNoMutation),
        (PhaseV1::ExactSiblingDelete, MutationStateV1::None, false) => {
            Some(Profile::ExactSiblingDeleteNoMutation)
        }
        (PhaseV1::ExactSiblingOpen, MutationStateV1::Uncertain, false) => {
            Some(Profile::ExactSiblingOpenUncertain)
        }
        (PhaseV1::DmsTruncate, MutationStateV1::Uncertain, false) => {
            Some(Profile::DmsTruncateUncertain)
        }
        (PhaseV1::FileClose, MutationStateV1::Uncertain, false) => {
            Some(Profile::FileCloseUncertain)
        }
        (PhaseV1::ExactSiblingDelete, MutationStateV1::Uncertain, false) => {
            Some(Profile::ExactSiblingDeleteUncertain)
        }
        (PhaseV1::FileGrow, MutationStateV1::Uncertain, false) => Some(Profile::FileGrowUncertain),
        (PhaseV1::MappingClose, MutationStateV1::Uncertain, false) => {
            Some(Profile::MappingCloseUncertain)
        }
        (PhaseV1::ViewUnmap, MutationStateV1::Uncertain, false) => {
            Some(Profile::ViewUnmapUncertain)
        }
        (PhaseV1::LockRelease, MutationStateV1::None, true) => Some(Profile::LockReleaseUncertain),
        (PhaseV1::ConnectionDetach, MutationStateV1::None, true) => {
            Some(Profile::ConnectionDetachUncertain)
        }
        (PhaseV1::DeleteAuthorization, MutationStateV1::None, true) => {
            Some(Profile::DeleteAuthorizationUncertain)
        }
        (PhaseV1::DmsExclusiveRelease, MutationStateV1::Uncertain, true) => {
            Some(Profile::DmsExclusiveReleaseUncertain)
        }
        (PhaseV1::DmsSharedRelease, MutationStateV1::Uncertain, true) => {
            Some(Profile::DmsSharedReleaseUncertain)
        }
        _ => None,
    }
}

#[test]
fn frozen_stored_poison_families_are_exact_unique_and_completion_partitioned() {
    let leaves = frozen_lock_stored_poison_leaves_v1();
    assert_eq!(leaves.len(), LOCK_STORED_POISON_MEMBER_COUNT);
    assert_eq!(
        leaves
            .values()
            .map(|leaf| leaf.member)
            .collect::<BTreeSet<_>>()
            .len(),
        LOCK_STORED_POISON_MEMBER_COUNT
    );
    for profile in LOCK_STORED_POISON_PROFILES {
        assert_eq!(
            leaves.keys().filter(|key| key.profile == profile).count(),
            176
        );
    }
    for completion in [
        LockStoredPoisonCompletionV1::RetentionSucceeded,
        LockStoredPoisonCompletionV1::RetentionRouteUnknown,
    ] {
        assert_eq!(
            leaves
                .keys()
                .filter(|key| key.completion == completion)
                .count(),
            LOCK_STORED_POISON_COMPLETION_MEMBER_COUNT
        );
    }
    for (action, expected) in [
        (LockStoredPoisonActionV1::LockShared, 240),
        (LockStoredPoisonActionV1::LockExclusive, 1_080),
        (LockStoredPoisonActionV1::UnlockShared, 240),
        (LockStoredPoisonActionV1::UnlockExclusive, 1_080),
    ] {
        assert_eq!(
            leaves.keys().filter(|key| key.action == action).count(),
            expected
        );
    }
}

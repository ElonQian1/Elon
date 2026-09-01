//! Independent frozen-member fixtures for completed own-overlap and not-held rejections.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use super::*;

pub(super) const LOCK_LOCAL_PROTOCOL_REJECTION_MEMBER_COUNT: usize = 88;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum LockLocalProtocolRejectionPathV1 {
    OwnOverlap,
    NotHeld,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct LockLocalProtocolRejectionKeyV1 {
    pub(super) path: LockLocalProtocolRejectionPathV1,
    pub(super) action: LockActionV1,
    pub(super) first: u8,
    pub(super) count: u8,
    pub(super) mask: u8,
}

#[derive(Clone)]
pub(super) struct FrozenLockLocalProtocolRejectionLeafV1 {
    pub(super) record: LeafRecordV1,
    pub(super) descriptor: TerminalDescriptorV1,
    pub(super) member: StaticMemberSealV1,
}

pub(super) fn frozen_lock_local_protocol_rejection_leaves_v1() -> &'static BTreeMap<
    LockLocalProtocolRejectionKeyV1,
    FrozenLockLocalProtocolRejectionLeafV1,
> {
    static LEAVES: OnceLock<
        BTreeMap<LockLocalProtocolRejectionKeyV1, FrozenLockLocalProtocolRejectionLeafV1>,
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
                let Some(key) = local_protocol_rejection_key_v1(record, descriptor) else {
                    return Ok(());
                };
                let previous = leaves.insert(
                    key,
                    FrozenLockLocalProtocolRejectionLeafV1 {
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
                        "duplicate frozen Lock local protocol-rejection member {key:?}"
                    ));
                }
                Ok(())
            },
        )
        .expect("the frozen Lock authority must validate before q8 program tests");
        assert_eq!(leaves.len(), LOCK_LOCAL_PROTOCOL_REJECTION_MEMBER_COUNT);
        leaves
    })
}

pub(super) fn lock_local_protocol_rejection_leaf_v1(
    key: LockLocalProtocolRejectionKeyV1,
) -> FrozenLockLocalProtocolRejectionLeafV1 {
    frozen_lock_local_protocol_rejection_leaves_v1()
        .get(&key)
        .unwrap_or_else(|| panic!("missing frozen Lock local protocol-rejection member {key:?}"))
        .clone()
}

pub(super) fn lock_local_protocol_rejection_descriptor_v1(
    key: LockLocalProtocolRejectionKeyV1,
    capability: RunnerCapabilityV1,
) -> TerminalDescriptorV1 {
    let mut descriptor = lock_local_protocol_rejection_leaf_v1(key).descriptor;
    let TerminalDescriptorV1::Lock(value) = &mut descriptor else {
        unreachable!("a Lock frozen program must have a Lock descriptor")
    };
    value.recipe.capability = capability;
    descriptor
}

fn local_protocol_rejection_key_v1(
    record: &LeafRecordV1,
    descriptor: &TerminalDescriptorV1,
) -> Option<LockLocalProtocolRejectionKeyV1> {
    let TerminalDescriptorV1::Lock(value) = descriptor else {
        return None;
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
    let expected_mask = range_mask_v1(action, first, count)?;
    let (path, prestate, operation) = expected_path_v1(action);
    if value.source_site != SourceSiteV1::LockLocalState
        || value.stimulus != StimulusV1::LockManaged(LockManagedStimulusV1::LocalState)
        || value.prestate != PrestateV1::Lock(prestate)
        || value.operation != operation
        || value.phase != PhaseV1::RequestValidation
        || value.timing != TimingV1::Natural
        || value.occurrence != OccurrenceV1::Natural
        || value.recipe.fixture != FixtureV1::ManagedWalMainSingleConnection
        || value.recipe.callback != CallbackV1::XShmLock
        || value.recipe.fault_seam != FaultSeamV1::Natural
        || value.recipe.observer != ObserverV1::LockCallbackAndSnapshot
        || value.recipe.cleanup != CleanupV1::ParentOwnedRoot
        || value.recipe.capability
            != RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete)
        || mask != expected_mask
        || value.axes != expected_axes_v1(action, first, count, mask)
    {
        return None;
    }
    let LeafOutcomeV1::Terminal(expected) = &record.outcome else {
        return None;
    };
    if expected != &expected_v1() {
        return None;
    }
    Some(LockLocalProtocolRejectionKeyV1 {
        path,
        action,
        first,
        count,
        mask,
    })
}

const fn expected_path_v1(
    action: LockActionV1,
) -> (
    LockLocalProtocolRejectionPathV1,
    LockPrestateV1,
    LockOperationV1,
) {
    match action {
        LockActionV1::LockShared | LockActionV1::LockExclusive => (
            LockLocalProtocolRejectionPathV1::OwnOverlap,
            LockPrestateV1::OwnOverlap,
            LockOperationV1::LocalAcquire,
        ),
        LockActionV1::UnlockShared | LockActionV1::UnlockExclusive => (
            LockLocalProtocolRejectionPathV1::NotHeld,
            LockPrestateV1::NoHeldLocks,
            LockOperationV1::LocalRelease,
        ),
    }
}

fn expected_axes_v1(action: LockActionV1, first: u8, count: u8, mask: u8) -> LockAxesV1 {
    let (held_shared_mask, held_exclusive_mask) = match action {
        LockActionV1::LockShared => (mask, 0),
        LockActionV1::LockExclusive => (0, mask),
        LockActionV1::UnlockShared | LockActionV1::UnlockExclusive => (0, 0),
    };
    LockAxesV1 {
        action: ReachabilityV1::Reached(action),
        first: ReachabilityV1::Reached(first),
        count: ReachabilityV1::Reached(count),
        mask: ReachabilityV1::Reached(mask),
        initialization: ReachabilityV1::NotReached,
        held_shared_mask: ReachabilityV1::Reached(held_shared_mask),
        held_exclusive_mask: ReachabilityV1::Reached(held_exclusive_mask),
        sibling_shared_mask: ReachabilityV1::Reached(0),
        sibling_exclusive_mask: ReachabilityV1::Reached(0),
        completion: ReachabilityV1::Reached(LockCompletionV1::Completed),
    }
}

fn expected_v1() -> ExpectedV1 {
    ExpectedV1 {
        sqlite: SqliteResultV1::LockUnavailable,
        disposition: TerminalDispositionV1::Returned,
        phase: "RequestValidation".to_owned(),
        failure: FailureClassV1::ProtocolViolation,
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
            ..ObservableCountsV1::default()
        },
    }
}

fn range_mask_v1(action: LockActionV1, first: u8, count: u8) -> Option<u8> {
    let end = first.checked_add(count)?;
    if first >= 8 || count == 0 || end > 8 {
        return None;
    }
    match action {
        LockActionV1::LockShared | LockActionV1::UnlockShared if count == 1 => {}
        LockActionV1::LockExclusive | LockActionV1::UnlockExclusive => {}
        _ => return None,
    }
    Some(((((1_u16 << count) - 1) << first) & 0xff) as u8)
}

#[test]
fn frozen_local_protocol_rejection_family_is_exact_unique_and_partitioned() {
    let leaves = frozen_lock_local_protocol_rejection_leaves_v1();
    assert_eq!(leaves.len(), LOCK_LOCAL_PROTOCOL_REJECTION_MEMBER_COUNT);
    assert_eq!(
        leaves
            .values()
            .map(|leaf| leaf.member)
            .collect::<BTreeSet<_>>()
            .len(),
        LOCK_LOCAL_PROTOCOL_REJECTION_MEMBER_COUNT
    );
    for (action, count) in [
        (LockActionV1::LockShared, 8),
        (LockActionV1::LockExclusive, 36),
        (LockActionV1::UnlockShared, 8),
        (LockActionV1::UnlockExclusive, 36),
    ] {
        assert_eq!(leaves.keys().filter(|key| key.action == action).count(), count);
    }
    assert_eq!(
        leaves
            .keys()
            .filter(|key| key.path == LockLocalProtocolRejectionPathV1::OwnOverlap)
            .count(),
        44
    );
    assert_eq!(
        leaves
            .keys()
            .filter(|key| key.path == LockLocalProtocolRejectionPathV1::NotHeld)
            .count(),
        44
    );
}

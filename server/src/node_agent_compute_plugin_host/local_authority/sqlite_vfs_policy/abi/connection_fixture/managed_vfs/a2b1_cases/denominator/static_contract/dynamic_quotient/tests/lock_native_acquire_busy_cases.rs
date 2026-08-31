//! Independent frozen-member fixtures for completed node-live native-acquire contention.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use super::*;

pub(super) const LOCK_NATIVE_ACQUIRE_BUSY_MEMBER_COUNT: usize = 44;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct LockNativeAcquireBusyKeyV1 {
    pub(super) action: LockActionV1,
    pub(super) first: u8,
    pub(super) count: u8,
    pub(super) mask: u8,
}

#[derive(Clone)]
pub(super) struct FrozenLockNativeAcquireBusyLeafV1 {
    pub(super) record: LeafRecordV1,
    pub(super) descriptor: TerminalDescriptorV1,
    pub(super) member: StaticMemberSealV1,
}

pub(super) fn frozen_lock_native_acquire_busy_leaves_v1(
) -> &'static BTreeMap<LockNativeAcquireBusyKeyV1, FrozenLockNativeAcquireBusyLeafV1> {
    static LEAVES: OnceLock<
        BTreeMap<LockNativeAcquireBusyKeyV1, FrozenLockNativeAcquireBusyLeafV1>,
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
                let Some(key) = native_acquire_busy_key_v1(record, descriptor) else {
                    return Ok(());
                };
                let previous = leaves.insert(
                    key,
                    FrozenLockNativeAcquireBusyLeafV1 {
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
                        "duplicate frozen Lock native-acquire busy member {key:?}"
                    ));
                }
                Ok(())
            },
        )
        .expect("the frozen Lock authority must validate before native-busy program tests");
        assert_eq!(leaves.len(), LOCK_NATIVE_ACQUIRE_BUSY_MEMBER_COUNT);
        leaves
    })
}

pub(super) fn lock_native_acquire_busy_leaf_v1(
    key: LockNativeAcquireBusyKeyV1,
) -> FrozenLockNativeAcquireBusyLeafV1 {
    frozen_lock_native_acquire_busy_leaves_v1()
        .get(&key)
        .unwrap_or_else(|| panic!("missing frozen Lock native-acquire busy member {key:?}"))
        .clone()
}

pub(super) fn lock_native_acquire_busy_descriptor_v1(
    key: LockNativeAcquireBusyKeyV1,
    capability: RunnerCapabilityV1,
) -> TerminalDescriptorV1 {
    let mut descriptor = lock_native_acquire_busy_leaf_v1(key).descriptor;
    let TerminalDescriptorV1::Lock(value) = &mut descriptor else {
        unreachable!("a Lock frozen program must have a Lock descriptor")
    };
    value.recipe.capability = capability;
    descriptor
}

fn native_acquire_busy_key_v1(
    record: &LeafRecordV1,
    descriptor: &TerminalDescriptorV1,
) -> Option<LockNativeAcquireBusyKeyV1> {
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
    if value.source_site != SourceSiteV1::LockNativeAcquire
        || value.stimulus != StimulusV1::LockManaged(LockManagedStimulusV1::NativeAcquire)
        || value.prestate != PrestateV1::Lock(LockPrestateV1::NoHeldLocks)
        || value.operation != LockOperationV1::NativeAcquire
        || value.phase != PhaseV1::LockAcquire
        || value.timing != TimingV1::AtCall
        || value.occurrence != OccurrenceV1::Natural
        || value.recipe.fixture != FixtureV1::ManagedWalMainSingleConnection
        || value.recipe.callback != CallbackV1::XShmLock
        || value.recipe.fault_seam != FaultSeamV1::NativeOperation
        || value.recipe.observer != ObserverV1::LockCallbackAndSnapshot
        || value.recipe.cleanup != CleanupV1::ParentOwnedRoot
        || value.recipe.capability
            != RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete)
        || mask != expected_mask
        || value.axes
            != (LockAxesV1 {
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
            })
    {
        return None;
    }
    let LeafOutcomeV1::Terminal(expected) = &record.outcome else {
        return None;
    };
    if expected != &expected_v1() {
        return None;
    }
    Some(LockNativeAcquireBusyKeyV1 {
        action,
        first,
        count,
        mask,
    })
}

fn expected_v1() -> ExpectedV1 {
    ExpectedV1 {
        sqlite: SqliteResultV1::Busy,
        disposition: TerminalDispositionV1::Returned,
        phase: "LockAcquire".to_owned(),
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

fn range_mask_v1(action: LockActionV1, first: u8, count: u8) -> Option<u8> {
    let end = first.checked_add(count)?;
    if first >= 8 || count == 0 || end > 8 {
        return None;
    }
    match action {
        LockActionV1::LockShared if count == 1 => {}
        LockActionV1::LockExclusive => {}
        _ => return None,
    }
    Some(((((1_u16 << count) - 1) << first) & 0xff) as u8)
}

#[test]
fn frozen_native_acquire_busy_family_is_exact_and_unique() {
    let leaves = frozen_lock_native_acquire_busy_leaves_v1();
    assert_eq!(leaves.len(), LOCK_NATIVE_ACQUIRE_BUSY_MEMBER_COUNT);
    assert_eq!(
        leaves
            .values()
            .map(|leaf| leaf.member)
            .collect::<BTreeSet<_>>()
            .len(),
        LOCK_NATIVE_ACQUIRE_BUSY_MEMBER_COUNT
    );
    assert_eq!(
        leaves
            .keys()
            .filter(|key| key.action == LockActionV1::LockShared)
            .count(),
        8
    );
    assert_eq!(
        leaves
            .keys()
            .filter(|key| key.action == LockActionV1::LockExclusive)
            .count(),
        36
    );
}

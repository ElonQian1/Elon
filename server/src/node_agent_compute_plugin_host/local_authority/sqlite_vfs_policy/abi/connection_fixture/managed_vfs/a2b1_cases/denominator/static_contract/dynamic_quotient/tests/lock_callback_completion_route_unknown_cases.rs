//! Independent frozen-member fixtures for callback-completion route-unknown outcomes.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use super::*;

pub(super) const LOCK_CALLBACK_COMPLETION_ROUTE_UNKNOWN_MEMBER_COUNT: usize = 192;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum LockCallbackCompletionRouteUnknownFixturePathV1 {
    LocalSiblingContention,
    NativeRelease,
    NativeAcquireAcquired,
    NativeAcquireBusy,
    SharedLocalAcquire,
    SharedLocalRelease,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct LockCallbackCompletionRouteUnknownKeyV1 {
    pub(super) path: LockCallbackCompletionRouteUnknownFixturePathV1,
    pub(super) action: LockActionV1,
    pub(super) first: u8,
    pub(super) count: u8,
    pub(super) mask: u8,
}

#[derive(Clone)]
pub(super) struct FrozenLockCallbackCompletionRouteUnknownLeafV1 {
    pub(super) record: LeafRecordV1,
    pub(super) descriptor: TerminalDescriptorV1,
    pub(super) member: StaticMemberSealV1,
}

pub(super) fn frozen_lock_callback_completion_route_unknown_leaves_v1() -> &'static BTreeMap<
    LockCallbackCompletionRouteUnknownKeyV1,
    FrozenLockCallbackCompletionRouteUnknownLeafV1,
> {
    static LEAVES: OnceLock<
        BTreeMap<
            LockCallbackCompletionRouteUnknownKeyV1,
            FrozenLockCallbackCompletionRouteUnknownLeafV1,
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
                let Some(key) = callback_completion_route_unknown_key_v1(record, descriptor) else {
                    return Ok(());
                };
                let previous = leaves.insert(
                    key,
                    FrozenLockCallbackCompletionRouteUnknownLeafV1 {
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
                        "duplicate frozen Lock callback route-unknown member {key:?}"
                    ));
                }
                Ok(())
            },
        )
        .expect("the frozen Lock authority must validate before callback route-unknown tests");
        assert_eq!(
            leaves.len(),
            LOCK_CALLBACK_COMPLETION_ROUTE_UNKNOWN_MEMBER_COUNT
        );
        leaves
    })
}

pub(super) fn lock_callback_completion_route_unknown_leaf_v1(
    key: LockCallbackCompletionRouteUnknownKeyV1,
) -> FrozenLockCallbackCompletionRouteUnknownLeafV1 {
    frozen_lock_callback_completion_route_unknown_leaves_v1()
        .get(&key)
        .unwrap_or_else(|| panic!("missing frozen Lock callback route-unknown member {key:?}"))
        .clone()
}

pub(super) fn lock_callback_completion_route_unknown_descriptor_v1(
    key: LockCallbackCompletionRouteUnknownKeyV1,
    capability: RunnerCapabilityV1,
) -> TerminalDescriptorV1 {
    let mut descriptor = lock_callback_completion_route_unknown_leaf_v1(key).descriptor;
    let TerminalDescriptorV1::Lock(value) = &mut descriptor else {
        unreachable!("a Lock frozen program must have a Lock descriptor")
    };
    value.recipe.capability = capability;
    descriptor
}

fn callback_completion_route_unknown_key_v1(
    record: &LeafRecordV1,
    descriptor: &TerminalDescriptorV1,
) -> Option<LockCallbackCompletionRouteUnknownKeyV1> {
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
    let path = path_v1(value)?;
    let expected_mask = range_mask_v1(path, action, first, count)?;
    if value.prestate != PrestateV1::Lock(expected_prestate_v1(path, action))
        || value.occurrence != OccurrenceV1::Natural
        || value.recipe.callback != CallbackV1::XShmLock
        || value.recipe.observer != ObserverV1::LockCallbackAndSnapshot
        || value.recipe.cleanup != CleanupV1::ParentOwnedRoot
        || value.recipe.capability
            != RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete)
        || mask != expected_mask
        || value.axes != expected_axes_v1(path, action, first, count, mask)
    {
        return None;
    }
    let LeafOutcomeV1::Terminal(expected) = &record.outcome else {
        return None;
    };
    if expected != &expected_v1(path, action, mask) {
        return None;
    }
    Some(LockCallbackCompletionRouteUnknownKeyV1 {
        path,
        action,
        first,
        count,
        mask,
    })
}

fn path_v1(
    value: &super::super::super::terminal_descriptor::LockTerminalDescriptorV1,
) -> Option<LockCallbackCompletionRouteUnknownFixturePathV1> {
    match (
        value.source_site,
        value.stimulus,
        value.operation,
        value.phase,
        value.timing,
        value.recipe.fixture,
        value.recipe.fault_seam,
    ) {
        (
            SourceSiteV1::LockLocalState,
            StimulusV1::LockManaged(LockManagedStimulusV1::LocalState),
            LockOperationV1::LocalAcquire,
            PhaseV1::LockAcquire,
            TimingV1::Natural,
            FixtureV1::ManagedWalMainTwoConnections,
            FaultSeamV1::Natural,
        ) => Some(LockCallbackCompletionRouteUnknownFixturePathV1::LocalSiblingContention),
        (
            SourceSiteV1::LockNativeRelease,
            StimulusV1::LockManaged(LockManagedStimulusV1::NativeRelease),
            LockOperationV1::NativeRelease,
            PhaseV1::Success,
            TimingV1::AfterSuccess,
            FixtureV1::ManagedWalMainSingleConnection,
            FaultSeamV1::NativeOperation,
        ) => Some(LockCallbackCompletionRouteUnknownFixturePathV1::NativeRelease),
        (
            SourceSiteV1::LockNativeAcquire,
            StimulusV1::LockManaged(LockManagedStimulusV1::NativeAcquire),
            LockOperationV1::NativeAcquire,
            PhaseV1::Success,
            TimingV1::AfterSuccess,
            FixtureV1::ManagedWalMainSingleConnection,
            FaultSeamV1::NativeOperation,
        ) => Some(LockCallbackCompletionRouteUnknownFixturePathV1::NativeAcquireAcquired),
        (
            SourceSiteV1::LockNativeAcquire,
            StimulusV1::LockManaged(LockManagedStimulusV1::NativeAcquire),
            LockOperationV1::NativeAcquire,
            PhaseV1::LockAcquire,
            TimingV1::AtCall,
            FixtureV1::ManagedWalMainSingleConnection,
            FaultSeamV1::NativeOperation,
        ) => Some(LockCallbackCompletionRouteUnknownFixturePathV1::NativeAcquireBusy),
        (
            SourceSiteV1::LockLocalState,
            StimulusV1::LockManaged(LockManagedStimulusV1::LocalState),
            LockOperationV1::LocalAcquire,
            PhaseV1::Success,
            TimingV1::Natural,
            FixtureV1::ManagedWalMainTwoConnections,
            FaultSeamV1::Natural,
        ) => Some(LockCallbackCompletionRouteUnknownFixturePathV1::SharedLocalAcquire),
        (
            SourceSiteV1::LockLocalState,
            StimulusV1::LockManaged(LockManagedStimulusV1::LocalState),
            LockOperationV1::LocalRelease,
            PhaseV1::Success,
            TimingV1::Natural,
            FixtureV1::ManagedWalMainTwoConnections,
            FaultSeamV1::Natural,
        ) => Some(LockCallbackCompletionRouteUnknownFixturePathV1::SharedLocalRelease),
        _ => None,
    }
}

fn expected_prestate_v1(
    path: LockCallbackCompletionRouteUnknownFixturePathV1,
    action: LockActionV1,
) -> LockPrestateV1 {
    match path {
        LockCallbackCompletionRouteUnknownFixturePathV1::LocalSiblingContention => match action {
            LockActionV1::LockShared => LockPrestateV1::SiblingExclusiveContention,
            LockActionV1::LockExclusive => LockPrestateV1::SiblingAnyContention,
            LockActionV1::UnlockShared | LockActionV1::UnlockExclusive => unreachable!(),
        },
        LockCallbackCompletionRouteUnknownFixturePathV1::NativeRelease => match action {
            LockActionV1::UnlockShared => LockPrestateV1::OwnSharedHeld,
            LockActionV1::UnlockExclusive => LockPrestateV1::OwnExclusiveHeld,
            LockActionV1::LockShared | LockActionV1::LockExclusive => unreachable!(),
        },
        LockCallbackCompletionRouteUnknownFixturePathV1::NativeAcquireAcquired
        | LockCallbackCompletionRouteUnknownFixturePathV1::NativeAcquireBusy => {
            LockPrestateV1::NoHeldLocks
        }
        LockCallbackCompletionRouteUnknownFixturePathV1::SharedLocalAcquire
        | LockCallbackCompletionRouteUnknownFixturePathV1::SharedLocalRelease => {
            LockPrestateV1::SiblingSharedCoalesced
        }
    }
}

fn expected_axes_v1(
    path: LockCallbackCompletionRouteUnknownFixturePathV1,
    action: LockActionV1,
    first: u8,
    count: u8,
    mask: u8,
) -> LockAxesV1 {
    let (initialization, held_shared, held_exclusive, sibling_shared, sibling_exclusive) =
        match path {
            LockCallbackCompletionRouteUnknownFixturePathV1::LocalSiblingContention => {
                if action == LockActionV1::LockShared {
                    (ReachabilityV1::NotReached, 0, 0, 0, mask)
                } else {
                    (ReachabilityV1::NotReached, 0, 0, mask, 0)
                }
            }
            LockCallbackCompletionRouteUnknownFixturePathV1::NativeRelease => {
                if action == LockActionV1::UnlockShared {
                    (ReachabilityV1::NotReached, mask, 0, 0, 0)
                } else {
                    (ReachabilityV1::NotReached, 0, mask, 0, 0)
                }
            }
            LockCallbackCompletionRouteUnknownFixturePathV1::NativeAcquireAcquired
            | LockCallbackCompletionRouteUnknownFixturePathV1::NativeAcquireBusy => (
                ReachabilityV1::Reached(InitializationProfileV1::NodeLive),
                0,
                0,
                0,
                0,
            ),
            LockCallbackCompletionRouteUnknownFixturePathV1::SharedLocalAcquire => {
                (ReachabilityV1::NotReached, 0, 0, mask, 0)
            }
            LockCallbackCompletionRouteUnknownFixturePathV1::SharedLocalRelease => {
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
    path: LockCallbackCompletionRouteUnknownFixturePathV1,
    action: LockActionV1,
    mask: u8,
) -> ExpectedV1 {
    let busy = matches!(
        path,
        LockCallbackCompletionRouteUnknownFixturePathV1::LocalSiblingContention
            | LockCallbackCompletionRouteUnknownFixturePathV1::NativeAcquireBusy
    );
    let lock_effect = match path {
        LockCallbackCompletionRouteUnknownFixturePathV1::LocalSiblingContention
        | LockCallbackCompletionRouteUnknownFixturePathV1::NativeAcquireBusy => {
            LockEffectV1::Unchanged
        }
        LockCallbackCompletionRouteUnknownFixturePathV1::NativeAcquireAcquired => {
            LockEffectV1::Acquired {
                mode: mode_v1(action),
                mask,
                native: true,
            }
        }
        LockCallbackCompletionRouteUnknownFixturePathV1::NativeRelease => LockEffectV1::Released {
            mode: mode_v1(action),
            mask,
            native: true,
        },
        LockCallbackCompletionRouteUnknownFixturePathV1::SharedLocalAcquire => {
            LockEffectV1::Acquired {
                mode: LockModeV1::Shared,
                mask,
                native: false,
            }
        }
        LockCallbackCompletionRouteUnknownFixturePathV1::SharedLocalRelease => {
            LockEffectV1::Released {
                mode: LockModeV1::Shared,
                mask,
                native: false,
            }
        }
    };
    ExpectedV1 {
        sqlite: SqliteResultV1::LockUnavailable,
        disposition: TerminalDispositionV1::Quarantined,
        phase: if busy { "LockAcquire" } else { "Success" }.to_owned(),
        failure: FailureClassV1::RegistryRejected,
        mutation: if busy {
            MutationStateV1::None
        } else {
            MutationStateV1::Known
        },
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
                LockCallbackCompletionRouteUnknownFixturePathV1::NativeAcquireAcquired
                    | LockCallbackCompletionRouteUnknownFixturePathV1::NativeAcquireBusy
            )),
            native_unlock: u16::from(matches!(
                path,
                LockCallbackCompletionRouteUnknownFixturePathV1::NativeRelease
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

fn range_mask_v1(
    path: LockCallbackCompletionRouteUnknownFixturePathV1,
    action: LockActionV1,
    first: u8,
    count: u8,
) -> Option<u8> {
    let end = first.checked_add(count)?;
    if first >= 8 || count == 0 || end > 8 {
        return None;
    }
    match path {
        LockCallbackCompletionRouteUnknownFixturePathV1::LocalSiblingContention
        | LockCallbackCompletionRouteUnknownFixturePathV1::NativeAcquireAcquired
        | LockCallbackCompletionRouteUnknownFixturePathV1::NativeAcquireBusy => match action {
            LockActionV1::LockShared if count == 1 => {}
            LockActionV1::LockExclusive => {}
            _ => return None,
        },
        LockCallbackCompletionRouteUnknownFixturePathV1::NativeRelease => match action {
            LockActionV1::UnlockShared if count == 1 => {}
            LockActionV1::UnlockExclusive => {}
            _ => return None,
        },
        LockCallbackCompletionRouteUnknownFixturePathV1::SharedLocalAcquire => {
            if action != LockActionV1::LockShared || count != 1 {
                return None;
            }
        }
        LockCallbackCompletionRouteUnknownFixturePathV1::SharedLocalRelease => {
            if action != LockActionV1::UnlockShared || count != 1 {
                return None;
            }
        }
    }
    Some(((((1_u16 << count) - 1) << first) & 0xff) as u8)
}

#[test]
fn frozen_callback_completion_route_unknown_family_is_exact_unique_and_partitioned() {
    let leaves = frozen_lock_callback_completion_route_unknown_leaves_v1();
    assert_eq!(
        leaves.len(),
        LOCK_CALLBACK_COMPLETION_ROUTE_UNKNOWN_MEMBER_COUNT
    );
    assert_eq!(
        leaves
            .values()
            .map(|leaf| leaf.member)
            .collect::<BTreeSet<_>>()
            .len(),
        LOCK_CALLBACK_COMPLETION_ROUTE_UNKNOWN_MEMBER_COUNT
    );
    for (path, count) in [
        (
            LockCallbackCompletionRouteUnknownFixturePathV1::LocalSiblingContention,
            44,
        ),
        (
            LockCallbackCompletionRouteUnknownFixturePathV1::NativeRelease,
            44,
        ),
        (
            LockCallbackCompletionRouteUnknownFixturePathV1::NativeAcquireAcquired,
            44,
        ),
        (
            LockCallbackCompletionRouteUnknownFixturePathV1::NativeAcquireBusy,
            44,
        ),
        (
            LockCallbackCompletionRouteUnknownFixturePathV1::SharedLocalAcquire,
            8,
        ),
        (
            LockCallbackCompletionRouteUnknownFixturePathV1::SharedLocalRelease,
            8,
        ),
    ] {
        assert_eq!(leaves.keys().filter(|key| key.path == path).count(), count);
    }
}

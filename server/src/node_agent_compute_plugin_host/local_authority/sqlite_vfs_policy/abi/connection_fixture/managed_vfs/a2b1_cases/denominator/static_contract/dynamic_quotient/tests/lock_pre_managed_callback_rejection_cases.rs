//! Independent frozen fixtures for the q9 pre-managed callback rejection tranche.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use super::*;

pub(super) const LOCK_PRE_MANAGED_CALLBACK_REJECTION_MEMBER_COUNT: usize = 528;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum LockPreManagedCallbackRejectionFamilyV1 {
    AdmissionRouteUnknownDirect,
    AdmissionCounterOverflowDirect,
    UnsupportedFileRoleCompleted,
    UnsupportedFileRoleRouteUnknown,
    ShmDetachedCompleted,
    ShmDetachedRouteUnknown,
}

impl LockPreManagedCallbackRejectionFamilyV1 {
    const ALL: [Self; 6] = [
        Self::AdmissionRouteUnknownDirect,
        Self::AdmissionCounterOverflowDirect,
        Self::UnsupportedFileRoleCompleted,
        Self::UnsupportedFileRoleRouteUnknown,
        Self::ShmDetachedCompleted,
        Self::ShmDetachedRouteUnknown,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct LockPreManagedCallbackRejectionKeyV1 {
    pub(super) family: LockPreManagedCallbackRejectionFamilyV1,
    pub(super) action: LockActionV1,
    pub(super) first: u8,
    pub(super) count: u8,
    pub(super) mask: u8,
}

#[derive(Clone)]
pub(super) struct FrozenLockPreManagedCallbackRejectionLeafV1 {
    pub(super) record: LeafRecordV1,
    pub(super) descriptor: TerminalDescriptorV1,
    pub(super) member: StaticMemberSealV1,
}

pub(super) fn frozen_lock_pre_managed_callback_rejection_leaves_v1() -> &'static BTreeMap<
    LockPreManagedCallbackRejectionKeyV1,
    FrozenLockPreManagedCallbackRejectionLeafV1,
> {
    static LEAVES: OnceLock<
        BTreeMap<LockPreManagedCallbackRejectionKeyV1, FrozenLockPreManagedCallbackRejectionLeafV1>,
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
                let Some(key) = pre_managed_callback_rejection_key_v1(record, descriptor) else {
                    return Ok(());
                };
                let previous = leaves.insert(
                    key,
                    FrozenLockPreManagedCallbackRejectionLeafV1 {
                        record: record.clone(),
                        descriptor: *descriptor,
                        member: StaticMemberSealV1 {
                            case_key_sha256: seal.case_key_sha256,
                            full_record_sha256: seal.full_record_sha256,
                        },
                    },
                );
                if previous.is_some() {
                    return Err(format!("duplicate frozen q9 member {key:?}"));
                }
                Ok(())
            },
        )
        .expect("the frozen Lock authority must validate before q9 source tests");
        assert_eq!(
            leaves.len(),
            LOCK_PRE_MANAGED_CALLBACK_REJECTION_MEMBER_COUNT
        );
        leaves
    })
}

pub(super) fn lock_pre_managed_callback_rejection_descriptor_v1(
    key: LockPreManagedCallbackRejectionKeyV1,
    capability: RunnerCapabilityV1,
) -> TerminalDescriptorV1 {
    let mut descriptor = frozen_lock_pre_managed_callback_rejection_leaves_v1()[&key].descriptor;
    let TerminalDescriptorV1::Lock(value) = &mut descriptor else {
        unreachable!("a q9 fixture must have a Lock descriptor")
    };
    value.recipe.capability = capability;
    descriptor
}

fn pre_managed_callback_rejection_key_v1(
    record: &LeafRecordV1,
    descriptor: &TerminalDescriptorV1,
) -> Option<LockPreManagedCallbackRejectionKeyV1> {
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
    let family = family_v1(value.source_site, value.stimulus, value.axes.completion)?;
    if value.prestate != PrestateV1::Lock(LockPrestateV1::NotReached)
        || value.operation != LockOperationV1::CallbackAdmission
        || value.phase != PhaseV1::CallbackAdmission
        || value.timing != TimingV1::BeforeCall
        || value.occurrence != OccurrenceV1::Natural
        || value.recipe.fixture != FixtureV1::ManagedWalMainSingleConnection
        || value.recipe.callback != CallbackV1::XShmLock
        || value.recipe.fault_seam != FaultSeamV1::RegistryAdmission
        || value.recipe.observer != ObserverV1::LockCallbackAndSnapshot
        || value.recipe.cleanup != CleanupV1::ParentOwnedRoot
        || value.recipe.capability
            != RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete)
        || mask != expected_mask
        || value.axes != expected_axes_v1(family, action, first, count, mask)
    {
        return None;
    }
    let LeafOutcomeV1::Terminal(expected) = &record.outcome else {
        return None;
    };
    (expected == &expected_v1(family)).then_some(LockPreManagedCallbackRejectionKeyV1 {
        family,
        action,
        first,
        count,
        mask,
    })
}

fn family_v1(
    source: SourceSiteV1,
    stimulus: StimulusV1,
    completion: ReachabilityV1<LockCompletionV1>,
) -> Option<LockPreManagedCallbackRejectionFamilyV1> {
    use LockPreManagedCallbackRejectionFamilyV1 as F;
    match (source, stimulus, completion) {
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

fn expected_v1(family: LockPreManagedCallbackRejectionFamilyV1) -> ExpectedV1 {
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
    ExpectedV1 {
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
        phase: "CallbackAdmission".to_owned(),
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
fn frozen_q9_family_is_six_by_eighty_eight_with_unique_seals_and_keys() {
    let leaves = frozen_lock_pre_managed_callback_rejection_leaves_v1();
    assert_eq!(
        leaves.len(),
        LOCK_PRE_MANAGED_CALLBACK_REJECTION_MEMBER_COUNT
    );
    assert_eq!(
        leaves
            .values()
            .map(|leaf| leaf.member)
            .collect::<BTreeSet<_>>()
            .len(),
        528
    );
    let normalized_keys = leaves
        .values()
        .map(|leaf| {
            project_validated_dynamic_terminal_v1(&leaf.record, &leaf.descriptor)
                .unwrap()
                .semantic_key
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(normalized_keys.len(), 528);
    for family in LockPreManagedCallbackRejectionFamilyV1::ALL {
        assert_eq!(leaves.keys().filter(|key| key.family == family).count(), 88);
    }
    for (action, per_family) in [
        (LockActionV1::LockShared, 8),
        (LockActionV1::LockExclusive, 36),
        (LockActionV1::UnlockShared, 8),
        (LockActionV1::UnlockExclusive, 36),
    ] {
        assert_eq!(
            leaves.keys().filter(|key| key.action == action).count(),
            per_family * 6
        );
    }
}

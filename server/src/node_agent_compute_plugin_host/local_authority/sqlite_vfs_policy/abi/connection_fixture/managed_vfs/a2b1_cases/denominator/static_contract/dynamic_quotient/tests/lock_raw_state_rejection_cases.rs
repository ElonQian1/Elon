//! Independent typed fixtures for the eleven q11 Lock raw-state rejection terminals.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use super::super::super::terminal_descriptor::RawStateV1;
use super::*;

pub(super) const LOCK_RAW_STATE_REJECTION_MEMBER_COUNT: usize = 11;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum FrozenLockRawStateRejectionCaseV1 {
    NullFileDirect,
    UninstalledDirect,
    MethodsNullStatePresentDirect,
    ForeignMethodsStateNullDirect,
    ForeignMethodsStatePresentDirect,
    ExactMethodsStateNullDirect,
    OtherTypePayloadMissingDropCompleted,
    OtherTypePayloadPresentDropCompleted,
    OtherTypePayloadPresentDropUnwindCaught,
    ExpectedTypePayloadMissingDropCompleted,
    HandleBoundFileMissingDirect,
}

impl FrozenLockRawStateRejectionCaseV1 {
    pub(super) const ALL_V1: [Self; LOCK_RAW_STATE_REJECTION_MEMBER_COUNT] = [
        Self::NullFileDirect,
        Self::UninstalledDirect,
        Self::MethodsNullStatePresentDirect,
        Self::ForeignMethodsStateNullDirect,
        Self::ForeignMethodsStatePresentDirect,
        Self::ExactMethodsStateNullDirect,
        Self::OtherTypePayloadMissingDropCompleted,
        Self::OtherTypePayloadPresentDropCompleted,
        Self::OtherTypePayloadPresentDropUnwindCaught,
        Self::ExpectedTypePayloadMissingDropCompleted,
        Self::HandleBoundFileMissingDirect,
    ];

    pub(super) const fn from_typed_v1(
        raw_state: RawStateV1,
        completion: LockCompletionV1,
    ) -> Option<Self> {
        match (raw_state, completion) {
            (RawStateV1::NullFile, LockCompletionV1::Direct) => Some(Self::NullFileDirect),
            (RawStateV1::Uninstalled, LockCompletionV1::Direct) => Some(Self::UninstalledDirect),
            (RawStateV1::MethodsNullStatePresent, LockCompletionV1::Direct) => {
                Some(Self::MethodsNullStatePresentDirect)
            }
            (RawStateV1::ForeignMethodsStateNull, LockCompletionV1::Direct) => {
                Some(Self::ForeignMethodsStateNullDirect)
            }
            (RawStateV1::ForeignMethodsStatePresent, LockCompletionV1::Direct) => {
                Some(Self::ForeignMethodsStatePresentDirect)
            }
            (RawStateV1::ExactMethodsStateNull, LockCompletionV1::Direct) => {
                Some(Self::ExactMethodsStateNullDirect)
            }
            (RawStateV1::OtherTypePayloadMissing, LockCompletionV1::RawDropCompleted) => {
                Some(Self::OtherTypePayloadMissingDropCompleted)
            }
            (RawStateV1::OtherTypePayloadPresent, LockCompletionV1::RawDropCompleted) => {
                Some(Self::OtherTypePayloadPresentDropCompleted)
            }
            (RawStateV1::OtherTypePayloadPresent, LockCompletionV1::RawDropUnwindCaught) => {
                Some(Self::OtherTypePayloadPresentDropUnwindCaught)
            }
            (RawStateV1::ExpectedTypePayloadMissing, LockCompletionV1::RawDropCompleted) => {
                Some(Self::ExpectedTypePayloadMissingDropCompleted)
            }
            (RawStateV1::HandleBoundFileMissing, LockCompletionV1::Direct) => {
                Some(Self::HandleBoundFileMissingDirect)
            }
            _ => None,
        }
    }

    pub(super) const fn raw_state_v1(self) -> RawStateV1 {
        match self {
            Self::NullFileDirect => RawStateV1::NullFile,
            Self::UninstalledDirect => RawStateV1::Uninstalled,
            Self::MethodsNullStatePresentDirect => RawStateV1::MethodsNullStatePresent,
            Self::ForeignMethodsStateNullDirect => RawStateV1::ForeignMethodsStateNull,
            Self::ForeignMethodsStatePresentDirect => RawStateV1::ForeignMethodsStatePresent,
            Self::ExactMethodsStateNullDirect => RawStateV1::ExactMethodsStateNull,
            Self::OtherTypePayloadMissingDropCompleted => RawStateV1::OtherTypePayloadMissing,
            Self::OtherTypePayloadPresentDropCompleted
            | Self::OtherTypePayloadPresentDropUnwindCaught => RawStateV1::OtherTypePayloadPresent,
            Self::ExpectedTypePayloadMissingDropCompleted => RawStateV1::ExpectedTypePayloadMissing,
            Self::HandleBoundFileMissingDirect => RawStateV1::HandleBoundFileMissing,
        }
    }

    pub(super) const fn completion_v1(self) -> LockCompletionV1 {
        match self {
            Self::NullFileDirect
            | Self::UninstalledDirect
            | Self::MethodsNullStatePresentDirect
            | Self::ForeignMethodsStateNullDirect
            | Self::ForeignMethodsStatePresentDirect
            | Self::ExactMethodsStateNullDirect
            | Self::HandleBoundFileMissingDirect => LockCompletionV1::Direct,
            Self::OtherTypePayloadMissingDropCompleted
            | Self::OtherTypePayloadPresentDropCompleted
            | Self::ExpectedTypePayloadMissingDropCompleted => LockCompletionV1::RawDropCompleted,
            Self::OtherTypePayloadPresentDropUnwindCaught => LockCompletionV1::RawDropUnwindCaught,
        }
    }
}

#[derive(Clone)]
pub(super) struct FrozenLockRawStateRejectionLeafV1 {
    pub(super) record: LeafRecordV1,
    pub(super) descriptor: TerminalDescriptorV1,
    pub(super) member: StaticMemberSealV1,
}

pub(super) fn frozen_lock_raw_state_rejection_leaves_v1(
) -> &'static BTreeMap<FrozenLockRawStateRejectionCaseV1, FrozenLockRawStateRejectionLeafV1> {
    static LEAVES: OnceLock<
        BTreeMap<FrozenLockRawStateRejectionCaseV1, FrozenLockRawStateRejectionLeafV1>,
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
                let Some(case) = raw_state_rejection_v1(record, descriptor) else {
                    return Ok(());
                };
                let previous = leaves.insert(
                    case,
                    FrozenLockRawStateRejectionLeafV1 {
                        record: record.clone(),
                        descriptor: *descriptor,
                        member: StaticMemberSealV1 {
                            case_key_sha256: seal.case_key_sha256,
                            full_record_sha256: seal.full_record_sha256,
                        },
                    },
                );
                if previous.is_some() {
                    return Err(format!("duplicate frozen q11 raw-state member {case:?}"));
                }
                Ok(())
            },
        )
        .expect("the frozen Lock authority must validate before q11 source tests");
        assert_eq!(leaves.len(), LOCK_RAW_STATE_REJECTION_MEMBER_COUNT);
        leaves
    })
}

pub(super) fn lock_raw_state_rejection_descriptor_v1(
    case: FrozenLockRawStateRejectionCaseV1,
    capability: RunnerCapabilityV1,
) -> TerminalDescriptorV1 {
    let mut descriptor = frozen_lock_raw_state_rejection_leaves_v1()[&case].descriptor;
    let TerminalDescriptorV1::Lock(value) = &mut descriptor else {
        unreachable!("a q11 fixture must have a Lock descriptor")
    };
    value.recipe.capability = capability;
    descriptor
}

pub(super) fn lock_raw_state_rejection_expected_groups_v1(
) -> BTreeSet<(DynamicClassKeyV1, StaticMemberSealV1)> {
    frozen_lock_raw_state_rejection_leaves_v1()
        .values()
        .map(|leaf| {
            (
                prepare_dynamic_terminal_v1(&leaf.record, &leaf.descriptor)
                    .expect("a frozen q11 descriptor must prepare")
                    .key,
                leaf.member,
            )
        })
        .collect()
}

fn raw_state_rejection_v1(
    record: &LeafRecordV1,
    descriptor: &TerminalDescriptorV1,
) -> Option<FrozenLockRawStateRejectionCaseV1> {
    let TerminalDescriptorV1::Lock(value) = descriptor else {
        return None;
    };
    let StimulusV1::LockRaw(raw_state) = value.stimulus else {
        return None;
    };
    let ReachabilityV1::Reached(completion) = value.axes.completion else {
        return None;
    };
    let case = FrozenLockRawStateRejectionCaseV1::from_typed_v1(raw_state, completion)?;
    let handle_bound = case == FrozenLockRawStateRejectionCaseV1::HandleBoundFileMissingDirect;
    if value.source_site
        != if handle_bound {
            SourceSiteV1::AdapterDispatch
        } else {
            SourceSiteV1::RawStateAbandon
        }
        || value.prestate != PrestateV1::Lock(LockPrestateV1::NotReached)
        || value.operation
            != if handle_bound {
                LockOperationV1::AdapterDispatch
            } else {
                LockOperationV1::RawAbandon
            }
        || value.phase
            != if handle_bound {
                PhaseV1::Adapter
            } else {
                PhaseV1::RawAdmission
            }
        || value.timing
            != if handle_bound {
                TimingV1::BeforeCall
            } else {
                TimingV1::Cleanup
            }
        || value.occurrence != OccurrenceV1::Natural
        || value.recipe.fixture != FixtureV1::AbiRawOnly
        || value.recipe.callback != CallbackV1::XShmLock
        || value.recipe.fault_seam != FaultSeamV1::RawState
        || value.recipe.observer
            != if handle_bound {
                ObserverV1::LockCallbackAndSnapshot
            } else {
                ObserverV1::CustodyAndCleanup
            }
        || value.recipe.cleanup != CleanupV1::ParentOwnedRoot
        || value.recipe.capability
            != RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete)
        || value.axes != expected_axes_v1(case.completion_v1())
    {
        return None;
    }
    let LeafOutcomeV1::Terminal(expected) = &record.outcome else {
        return None;
    };
    (expected == &expected_v1(case)).then_some(case)
}

const fn expected_axes_v1(completion: LockCompletionV1) -> LockAxesV1 {
    LockAxesV1 {
        completion: ReachabilityV1::Reached(completion),
        ..LockAxesV1::NOT_REACHED
    }
}

fn expected_v1(case: FrozenLockRawStateRejectionCaseV1) -> ExpectedV1 {
    let handle_bound = case == FrozenLockRawStateRejectionCaseV1::HandleBoundFileMissingDirect;
    let mut expected = ExpectedV1 {
        sqlite: SqliteResultV1::LockUnavailable,
        disposition: TerminalDispositionV1::Returned,
        phase: (if handle_bound {
            "Adapter"
        } else {
            "RawAdmission"
        })
        .to_owned(),
        failure: FailureClassV1::ProtocolViolation,
        mutation: MutationStateV1::None,
        lock_outcome_uncertain: false,
        lock_effect: LockEffectV1::NotReached,
        dms_lock: DmsLockCustodyV1::NotReached,
        raw_slots: CustodyStateV1::NotReached,
        route: CustodyStateV1::NotReached,
        callback: CustodyStateV1::NotReached,
        file: CustodyStateV1::NotReached,
        mapping: CustodyStateV1::NotReached,
        view: CustodyStateV1::NotReached,
        payload: CustodyStateV1::NotReached,
        counts: ObservableCountsV1::default(),
    };
    match case {
        FrozenLockRawStateRejectionCaseV1::NullFileDirect => {}
        FrozenLockRawStateRejectionCaseV1::UninstalledDirect => {
            expected.raw_slots = CustodyStateV1::Cleared;
        }
        FrozenLockRawStateRejectionCaseV1::MethodsNullStatePresentDirect
        | FrozenLockRawStateRejectionCaseV1::ForeignMethodsStateNullDirect
        | FrozenLockRawStateRejectionCaseV1::ForeignMethodsStatePresentDirect
        | FrozenLockRawStateRejectionCaseV1::ExactMethodsStateNullDirect => {
            expected.raw_slots = CustodyStateV1::Retained;
        }
        FrozenLockRawStateRejectionCaseV1::OtherTypePayloadMissingDropCompleted
        | FrozenLockRawStateRejectionCaseV1::ExpectedTypePayloadMissingDropCompleted => {
            expected.disposition = TerminalDispositionV1::Abandoned;
            expected.raw_slots = CustodyStateV1::Cleared;
            expected.payload = CustodyStateV1::Cleared;
        }
        FrozenLockRawStateRejectionCaseV1::OtherTypePayloadPresentDropCompleted => {
            expected.disposition = TerminalDispositionV1::Abandoned;
            expected.raw_slots = CustodyStateV1::Cleared;
            expected.payload = CustodyStateV1::Released;
        }
        FrozenLockRawStateRejectionCaseV1::OtherTypePayloadPresentDropUnwindCaught => {
            expected.disposition = TerminalDispositionV1::Quarantined;
            expected.raw_slots = CustodyStateV1::Cleared;
            expected.payload = CustodyStateV1::Quarantined;
        }
        FrozenLockRawStateRejectionCaseV1::HandleBoundFileMissingDirect => {
            expected.raw_slots = CustodyStateV1::Unchanged;
            expected.file = CustodyStateV1::Cleared;
            expected.payload = CustodyStateV1::Retained;
        }
    }
    expected
}

#[test]
fn frozen_q11_family_is_exact_unique_and_excludes_sentinel_raw_states() {
    let leaves = frozen_lock_raw_state_rejection_leaves_v1();
    assert_eq!(
        leaves.keys().copied().collect::<BTreeSet<_>>(),
        FrozenLockRawStateRejectionCaseV1::ALL_V1
            .into_iter()
            .collect::<BTreeSet<_>>()
    );
    assert_eq!(
        leaves
            .values()
            .map(|leaf| leaf.member)
            .collect::<BTreeSet<_>>()
            .len(),
        LOCK_RAW_STATE_REJECTION_MEMBER_COUNT
    );
    for sentinel in [RawStateV1::DropCompleted, RawStateV1::DropUnwindCaught] {
        for completion in [
            LockCompletionV1::Direct,
            LockCompletionV1::RawDropCompleted,
            LockCompletionV1::RawDropUnwindCaught,
        ] {
            assert!(
                FrozenLockRawStateRejectionCaseV1::from_typed_v1(sentinel, completion).is_none()
            );
        }
    }
}

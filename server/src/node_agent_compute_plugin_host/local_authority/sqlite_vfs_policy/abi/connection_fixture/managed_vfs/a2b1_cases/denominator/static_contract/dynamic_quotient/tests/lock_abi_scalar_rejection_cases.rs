//! Independent typed fixtures for the seven q10 Lock ABI scalar rejections.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use super::*;

pub(super) const LOCK_ABI_SCALAR_REJECTION_MEMBER_COUNT: usize = 7;

pub(super) const LOCK_ABI_SCALAR_REJECTION_PROFILES_V1: [
    LockAbiScalarV1;
    LOCK_ABI_SCALAR_REJECTION_MEMBER_COUNT
] = [
    scalar(ValidityV1::Invalid, ValidityV1::Invalid, ValidityV1::Invalid),
    scalar(ValidityV1::Invalid, ValidityV1::Invalid, ValidityV1::Valid),
    scalar(ValidityV1::Invalid, ValidityV1::Valid, ValidityV1::Invalid),
    scalar(ValidityV1::Invalid, ValidityV1::Valid, ValidityV1::Valid),
    scalar(ValidityV1::Valid, ValidityV1::Invalid, ValidityV1::Invalid),
    scalar(ValidityV1::Valid, ValidityV1::Invalid, ValidityV1::Valid),
    scalar(ValidityV1::Valid, ValidityV1::Valid, ValidityV1::Invalid),
];

#[derive(Clone)]
pub(super) struct FrozenLockAbiScalarRejectionLeafV1 {
    pub(super) record: LeafRecordV1,
    pub(super) descriptor: TerminalDescriptorV1,
    pub(super) member: StaticMemberSealV1,
}

pub(super) fn frozen_lock_abi_scalar_rejection_leaves_v1(
) -> &'static BTreeMap<LockAbiScalarV1, FrozenLockAbiScalarRejectionLeafV1> {
    static LEAVES: OnceLock<
        BTreeMap<LockAbiScalarV1, FrozenLockAbiScalarRejectionLeafV1>,
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
                let Some(scalar) = abi_scalar_rejection_v1(record, descriptor) else {
                    return Ok(());
                };
                let previous = leaves.insert(
                    scalar,
                    FrozenLockAbiScalarRejectionLeafV1 {
                        record: record.clone(),
                        descriptor: *descriptor,
                        member: StaticMemberSealV1 {
                            case_key_sha256: seal.case_key_sha256,
                            full_record_sha256: seal.full_record_sha256,
                        },
                    },
                );
                if previous.is_some() {
                    return Err(format!("duplicate frozen q10 ABI scalar member {scalar:?}"));
                }
                Ok(())
            },
        )
        .expect("the frozen Lock authority must validate before q10 source tests");
        assert_eq!(leaves.len(), LOCK_ABI_SCALAR_REJECTION_MEMBER_COUNT);
        leaves
    })
}

pub(super) fn lock_abi_scalar_rejection_descriptor_v1(
    scalar: LockAbiScalarV1,
    capability: RunnerCapabilityV1,
) -> TerminalDescriptorV1 {
    let mut descriptor = frozen_lock_abi_scalar_rejection_leaves_v1()[&scalar].descriptor;
    let TerminalDescriptorV1::Lock(value) = &mut descriptor else {
        unreachable!("a q10 fixture must have a Lock descriptor")
    };
    value.recipe.capability = capability;
    descriptor
}

pub(super) fn lock_abi_scalar_rejection_expected_groups_v1(
) -> BTreeSet<(DynamicClassKeyV1, StaticMemberSealV1)> {
    frozen_lock_abi_scalar_rejection_leaves_v1()
        .values()
        .map(|leaf| {
            (
                prepare_dynamic_terminal_v1(&leaf.record, &leaf.descriptor)
                    .expect("a frozen q10 descriptor must prepare")
                    .key,
                leaf.member,
            )
        })
        .collect()
}

fn abi_scalar_rejection_v1(
    record: &LeafRecordV1,
    descriptor: &TerminalDescriptorV1,
) -> Option<LockAbiScalarV1> {
    let TerminalDescriptorV1::Lock(value) = descriptor else {
        return None;
    };
    let StimulusV1::LockAbi(scalar) = value.stimulus else {
        return None;
    };
    if !LOCK_ABI_SCALAR_REJECTION_PROFILES_V1.contains(&scalar)
        || value.source_site != SourceSiteV1::LockAbiBoundary
        || value.prestate != PrestateV1::Lock(LockPrestateV1::NotReached)
        || value.operation != LockOperationV1::AbiValidation
        || value.phase != PhaseV1::AbiValidation
        || value.timing != TimingV1::BeforeCall
        || value.occurrence != OccurrenceV1::Natural
        || value.recipe.fixture != FixtureV1::AbiRawOnly
        || value.recipe.callback != CallbackV1::XShmLock
        || value.recipe.fault_seam != FaultSeamV1::AbiBoundary
        || value.recipe.observer != ObserverV1::LockCallbackAndSnapshot
        || value.recipe.cleanup != CleanupV1::ParentOwnedRoot
        || value.recipe.capability
            != RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete)
        || value.axes != expected_axes_v1()
    {
        return None;
    }
    let LeafOutcomeV1::Terminal(expected) = &record.outcome else {
        return None;
    };
    (expected == &expected_v1()).then_some(scalar)
}

const fn scalar(
    offset: ValidityV1,
    count: ValidityV1,
    flags: ValidityV1,
) -> LockAbiScalarV1 {
    LockAbiScalarV1 {
        offset,
        count,
        flags,
    }
}

const fn expected_axes_v1() -> LockAxesV1 {
    LockAxesV1 {
        completion: ReachabilityV1::Reached(LockCompletionV1::Direct),
        ..LockAxesV1::NOT_REACHED
    }
}

fn expected_v1() -> ExpectedV1 {
    ExpectedV1 {
        sqlite: SqliteResultV1::LockUnavailable,
        disposition: TerminalDispositionV1::Returned,
        phase: "AbiValidation".to_owned(),
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
    }
}

#[test]
fn frozen_q10_family_is_exact_unique_and_excludes_the_all_valid_profile() {
    let leaves = frozen_lock_abi_scalar_rejection_leaves_v1();
    assert_eq!(
        leaves.keys().copied().collect::<BTreeSet<_>>(),
        LOCK_ABI_SCALAR_REJECTION_PROFILES_V1
            .into_iter()
            .collect::<BTreeSet<_>>()
    );
    assert_eq!(
        leaves
            .values()
            .map(|leaf| leaf.member)
            .collect::<BTreeSet<_>>()
            .len(),
        LOCK_ABI_SCALAR_REJECTION_MEMBER_COUNT
    );
    assert!(!leaves.contains_key(&scalar(
        ValidityV1::Valid,
        ValidityV1::Valid,
        ValidityV1::Valid,
    )));
}

//! Exact admission and isolated-receipt tests for 88 completed local protocol rejections.

use super::lock_local_protocol_rejection_cases::{
    frozen_lock_local_protocol_rejection_leaves_v1,
    lock_local_protocol_rejection_descriptor_v1, lock_local_protocol_rejection_leaf_v1,
    LockLocalProtocolRejectionKeyV1, LOCK_LOCAL_PROTOCOL_REJECTION_MEMBER_COUNT,
};
#[cfg(windows)]
use super::lock_local_protocol_rejection_cases::LockLocalProtocolRejectionPathV1;
use super::super::runner_admission::{
    compile_for_test, local_protocol_rejection_catalog_row_count_for_test,
    validate_lock_program_for_test,
};
#[cfg(windows)]
use super::super::runner_admission::{
    run_lock_isolated_for_test, LockRunnerIsolatedOutcomeV1, RunnerAdmissionDecisionV1,
};
#[cfg(windows)]
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2_dynamic_evidence::{
    lock_local_protocol_rejection_selector_for_test,
    selected_lock_local_protocol_rejection_selector_for_test,
};
use super::*;

#[cfg(windows)]
const EXACT_TEST: &str = "node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2b1_cases::denominator::static_contract::dynamic_quotient::tests::runner_admission_lock_local_protocol_rejection_supported::isolated_local_protocol_rejection_family_receipts_are_exact";

fn supported_key_and_member(
    case: LockLocalProtocolRejectionKeyV1,
) -> (DynamicClassKeyV1, StaticMemberSealV1) {
    let leaf = lock_local_protocol_rejection_leaf_v1(case);
    let descriptor = lock_local_protocol_rejection_descriptor_v1(
        case,
        RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete),
    );
    let validated = project_validated_dynamic_terminal_v1(&leaf.record, &descriptor).unwrap();
    assert_eq!(validated.descriptor_binding.member, leaf.member);
    let mut semantic = validated.semantic_key;
    semantic.recipe.capability = RunnerCapabilityV1::Supported;
    (semantic, leaf.member)
}

fn assert_rejected(key: DynamicClassKeyV1, member: StaticMemberSealV1, mutation: &str) {
    assert!(
        validate_lock_program_for_test(&key, member, compile_for_test(&key)).is_err(),
        "Lock q8 admission accepted {mutation}"
    );
}

#[test]
fn all_88_exact_descriptors_and_members_are_admitted() {
    let leaves = frozen_lock_local_protocol_rejection_leaves_v1();
    assert_eq!(leaves.len(), LOCK_LOCAL_PROTOCOL_REJECTION_MEMBER_COUNT);
    assert_eq!(
        local_protocol_rejection_catalog_row_count_for_test(),
        LOCK_LOCAL_PROTOCOL_REJECTION_MEMBER_COUNT
    );
    for (&case, leaf) in leaves {
        let (key, member) = supported_key_and_member(case);
        assert_eq!(member, leaf.member);
        validate_lock_program_for_test(&key, member, compile_for_test(&key)).unwrap_or_else(
            |error| panic!("exact Lock q8 member {case:?} was rejected: {error:?}"),
        );
    }
}

#[test]
fn every_q8_program_rejects_a_sibling_member_seal() {
    let leaves = frozen_lock_local_protocol_rejection_leaves_v1();
    let members = leaves.values().map(|leaf| leaf.member).collect::<Vec<_>>();
    for (index, &case) in leaves.keys().enumerate() {
        let (key, member) = supported_key_and_member(case);
        let sibling = members[(index + 1) % members.len()];
        assert_ne!(member, sibling);
        assert_rejected(key, sibling, "a sibling frozen member seal");
    }
}

#[test]
fn q8_programs_reject_completion_prestate_expected_recipe_and_range_drift() {
    let (&case, _) = frozen_lock_local_protocol_rejection_leaves_v1()
        .iter()
        .next()
        .expect("q8 fixture");
    let (key, member) = supported_key_and_member(case);

    let mut completion = key;
    let DynamicAxesV1::Lock(axes) = &mut completion.axes else {
        unreachable!()
    };
    axes.completion = ReachabilityV1::Reached(LockCompletionV1::RouteUnknown);
    assert_rejected(completion, member, "a RouteUnknown completion");

    let mut range_mismatch = key;
    range_mismatch.prestate = PrestateV1::Lock(LockPrestateV1::ExclusiveRangeMismatch);
    assert_rejected(range_mismatch, member, "an exclusive-range-mismatch prestate");

    let mut operation = key;
    operation.operation = DynamicOperationV1::Lock(LockOperationV1::LocalRelease);
    assert_rejected(operation, member, "a swapped local operation");

    let mut failure = key;
    failure.expected.failure = FailureClassV1::RegistryRejected;
    assert_rejected(failure, member, "a registry-rejected expected vector");

    let mut fixture = key;
    fixture.recipe.fixture = FixtureV1::ManagedWalMainTwoConnections;
    assert_rejected(fixture, member, "a two-connection fixture");

    let mut cleanup = key;
    cleanup.recipe.cleanup = CleanupV1::RetainUnsafeCustodyThenParentCleanup;
    assert_rejected(cleanup, member, "unsafe-retention cleanup");

    let mut held = key;
    let DynamicAxesV1::Lock(axes) = &mut held.axes else {
        unreachable!()
    };
    axes.held_shared_mask = ReachabilityV1::Reached(0);
    axes.held_exclusive_mask = ReachabilityV1::Reached(0);
    assert_rejected(held, member, "a cleared own-overlap prestate");

    let mut range = key;
    let DynamicAxesV1::Lock(axes) = &mut range.axes else {
        unreachable!()
    };
    axes.count = ReachabilityV1::Reached(2);
    axes.mask = ReachabilityV1::Reached(3);
    assert_rejected(range, member, "a shared multi-slot range");
}

#[cfg(windows)]
#[test]
fn isolated_local_protocol_rejection_family_receipts_are_exact() -> anyhow::Result<()> {
    let selected_child = selected_lock_local_protocol_rejection_selector_for_test();
    for (&case, leaf) in frozen_lock_local_protocol_rejection_leaves_v1() {
        if let Some(selected) = selected_child.as_deref() {
            let candidate = lock_local_protocol_rejection_selector_for_test(
                path_tag(case.path),
                action_tag(case.action),
                case.first,
                case.count,
            )
            .map_err(anyhow::Error::msg)?;
            if candidate != selected {
                continue;
            }
        }
        let descriptor =
            lock_local_protocol_rejection_descriptor_v1(case, RunnerCapabilityV1::Supported);
        let (key, member) = supported_key_and_member(case);
        let plan = compile_for_test(&key);
        let execution = match run_lock_isolated_for_test(EXACT_TEST, &key, member, plan)? {
            LockRunnerIsolatedOutcomeV1::ChildReported => return Ok(()),
            LockRunnerIsolatedOutcomeV1::ParentReceipt(receipt) => receipt,
        };
        let validated = project_validated_dynamic_terminal_with_lock_execution_v1(
            &leaf.record,
            &descriptor,
            execution,
        )
        .map_err(|error| anyhow::anyhow!("supported Lock q8 member {case:?} failed: {error:?}"))?;
        assert!(validated.projection.is_ok());
        assert!(matches!(
            validated.runner_admission.decision(),
            RunnerAdmissionDecisionV1::Supported { .. }
        ));
    }
    match selected_child {
        Some(selected) => Err(anyhow::anyhow!(
            "parent-selected Lock q8 member was not found: {selected}"
        )),
        None => Ok(()),
    }
}

#[cfg(windows)]
const fn path_tag(path: LockLocalProtocolRejectionPathV1) -> u64 {
    match path {
        LockLocalProtocolRejectionPathV1::OwnOverlap => 1,
        LockLocalProtocolRejectionPathV1::NotHeld => 2,
    }
}

#[cfg(windows)]
const fn action_tag(action: LockActionV1) -> u64 {
    match action {
        LockActionV1::LockShared => 1,
        LockActionV1::LockExclusive => 2,
        LockActionV1::UnlockShared => 3,
        LockActionV1::UnlockExclusive => 4,
    }
}

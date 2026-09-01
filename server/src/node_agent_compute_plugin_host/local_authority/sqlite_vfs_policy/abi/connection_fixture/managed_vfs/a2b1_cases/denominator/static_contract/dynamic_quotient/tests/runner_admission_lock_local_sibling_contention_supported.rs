//! Exact admission and isolated receipt tests for 44 local sibling-contention programs.

use super::lock_local_sibling_contention_cases::{
    frozen_lock_local_sibling_contention_leaves_v1, lock_local_sibling_contention_descriptor_v1,
    lock_local_sibling_contention_leaf_v1, LockLocalSiblingContentionKeyV1,
    LOCK_LOCAL_SIBLING_CONTENTION_MEMBER_COUNT,
};
use super::super::runner_admission::{
    compile_for_test, local_sibling_contention_catalog_row_count_for_test,
    validate_lock_program_for_test,
};
#[cfg(windows)]
use super::super::runner_admission::{
    run_lock_isolated_for_test, LockRunnerIsolatedOutcomeV1, RunnerAdmissionDecisionV1,
};
#[cfg(windows)]
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2_dynamic_evidence::{
    lock_local_sibling_contention_selector_for_test,
    selected_lock_local_sibling_contention_selector_for_test,
};
use super::*;

#[cfg(windows)]
const EXACT_TEST: &str = "node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2b1_cases::denominator::static_contract::dynamic_quotient::tests::runner_admission_lock_local_sibling_contention_supported::isolated_local_sibling_contention_family_receipts_are_exact";

fn supported_key_and_member(
    case: LockLocalSiblingContentionKeyV1,
) -> (DynamicClassKeyV1, StaticMemberSealV1) {
    let leaf = lock_local_sibling_contention_leaf_v1(case);
    let descriptor = lock_local_sibling_contention_descriptor_v1(
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
        "Lock local sibling-contention admission accepted {mutation}"
    );
}

#[test]
fn all_44_exact_descriptors_and_members_are_admitted() {
    let leaves = frozen_lock_local_sibling_contention_leaves_v1();
    assert_eq!(leaves.len(), LOCK_LOCAL_SIBLING_CONTENTION_MEMBER_COUNT);
    assert_eq!(
        local_sibling_contention_catalog_row_count_for_test(),
        LOCK_LOCAL_SIBLING_CONTENTION_MEMBER_COUNT
    );
    for (&case, leaf) in leaves {
        let (key, member) = supported_key_and_member(case);
        assert_eq!(member, leaf.member);
        validate_lock_program_for_test(&key, member, compile_for_test(&key)).unwrap_or_else(
            |error| {
                panic!(
                    "exact Lock local sibling-contention member {case:?} was rejected: {error:?}"
                )
            },
        );
    }
}

#[test]
fn every_local_sibling_contention_program_rejects_a_sibling_member_seal() {
    let leaves = frozen_lock_local_sibling_contention_leaves_v1();
    let members = leaves.values().map(|leaf| leaf.member).collect::<Vec<_>>();
    for (index, &case) in leaves.keys().enumerate() {
        let (key, member) = supported_key_and_member(case);
        let sibling = members[(index + 1) % members.len()];
        assert_ne!(member, sibling);
        assert_rejected(key, sibling, "a sibling frozen member seal");
    }
}

#[test]
fn local_sibling_contention_programs_reject_semantic_expected_recipe_and_range_drift() {
    let (&case, _) = frozen_lock_local_sibling_contention_leaves_v1()
        .iter()
        .next()
        .expect("local sibling-contention fixture");
    let (key, member) = supported_key_and_member(case);

    let mut completion = key;
    let DynamicAxesV1::Lock(axes) = &mut completion.axes else {
        unreachable!()
    };
    axes.completion = ReachabilityV1::Reached(LockCompletionV1::RouteUnknown);
    assert_rejected(completion, member, "route-unknown completion");

    let mut initialization = key;
    let DynamicAxesV1::Lock(axes) = &mut initialization.axes else {
        unreachable!()
    };
    axes.initialization = ReachabilityV1::Reached(InitializationProfileV1::NodeLive);
    assert_rejected(initialization, member, "a reached initialization profile");

    let mut sibling = key;
    let DynamicAxesV1::Lock(axes) = &mut sibling.axes else {
        unreachable!()
    };
    match case.action {
        LockActionV1::LockShared => axes.sibling_exclusive_mask = ReachabilityV1::Reached(0),
        LockActionV1::LockExclusive => axes.sibling_shared_mask = ReachabilityV1::Reached(0),
        LockActionV1::UnlockShared | LockActionV1::UnlockExclusive => unreachable!(),
    }
    assert_rejected(sibling, member, "cleared sibling contention");

    let mut failure = key;
    failure.expected.failure = FailureClassV1::RegistryRejected;
    assert_rejected(failure, member, "a registry-rejected expected vector");

    let mut cleanup = key;
    cleanup.recipe.cleanup = CleanupV1::RetainUnsafeCustodyThenParentCleanup;
    assert_rejected(cleanup, member, "unsafe-retention cleanup");

    let mut action = key;
    let DynamicAxesV1::Lock(axes) = &mut action.axes else {
        unreachable!()
    };
    axes.action = ReachabilityV1::Reached(LockActionV1::UnlockShared);
    assert_rejected(action, member, "an unlock action");

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
fn isolated_local_sibling_contention_family_receipts_are_exact() -> anyhow::Result<()> {
    let selected_child = selected_lock_local_sibling_contention_selector_for_test();
    for (&case, leaf) in frozen_lock_local_sibling_contention_leaves_v1() {
        if let Some(selected) = selected_child.as_deref() {
            let candidate = lock_local_sibling_contention_selector_for_test(
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
            lock_local_sibling_contention_descriptor_v1(case, RunnerCapabilityV1::Supported);
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
        .map_err(|error| {
            anyhow::anyhow!(
                "supported Lock local sibling-contention member {case:?} failed: {error:?}"
            )
        })?;
        assert!(validated.projection.is_ok());
        assert!(matches!(
            validated.runner_admission.decision(),
            RunnerAdmissionDecisionV1::Supported { .. }
        ));
    }
    match selected_child {
        Some(selected) => Err(anyhow::anyhow!(
            "parent-selected Lock local sibling-contention member was not found: {selected}"
        )),
        None => Ok(()),
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

//! Exact admission tests for both 1,320-member stored-poison Lock completions.

use super::lock_stored_poison_cases::{self,
    frozen_lock_stored_poison_leaves_v1, lock_stored_poison_descriptor_v1,
    LockStoredPoisonCompletionV1, LOCK_STORED_POISON_MEMBER_COUNT,
};
use super::super::runner_admission::{
    compile_for_test, stored_poison_catalog_row_count_for_test, validate_lock_program_for_test,
};
#[cfg(windows)]
use super::super::runner_admission::{
    run_lock_isolated_for_test, LockRunnerIsolatedOutcomeV1, RunnerAdmissionDecisionV1,
};
#[cfg(windows)]
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2_dynamic_evidence::{
    lock_stored_poison_selector_for_test, selected_lock_stored_poison_selector_for_test,
};
use super::*;

#[cfg(windows)]
const EXACT_TEST: &str = "node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2b1_cases::denominator::static_contract::dynamic_quotient::tests::runner_admission_lock_stored_poison_supported::isolated_stored_poison_family_receipts_are_exact";

fn supported_key_and_member(
    key: lock_stored_poison_cases::LockStoredPoisonKeyV1,
) -> (DynamicClassKeyV1, StaticMemberSealV1) {
    let leaf = lock_stored_poison_cases::lock_stored_poison_leaf_v1(key);
    let descriptor = lock_stored_poison_descriptor_v1(
        key,
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
        "Lock stored-poison admission accepted {mutation}"
    );
}

#[test]
fn all_2640_exact_descriptors_and_members_are_admitted() {
    let leaves = frozen_lock_stored_poison_leaves_v1();
    assert_eq!(leaves.len(), LOCK_STORED_POISON_MEMBER_COUNT);
    assert_eq!(
        stored_poison_catalog_row_count_for_test(),
        LOCK_STORED_POISON_MEMBER_COUNT
    );
    for (&case, leaf) in leaves {
        let (key, member) = supported_key_and_member(case);
        assert_eq!(member, leaf.member);
        validate_lock_program_for_test(&key, member, compile_for_test(&key)).unwrap_or_else(
            |error| panic!("exact Lock stored-poison member {case:?} was rejected: {error:?}"),
        );
    }
}

#[test]
fn every_stored_poison_program_rejects_a_sibling_member_seal() {
    let leaves = frozen_lock_stored_poison_leaves_v1();
    let members = leaves.values().map(|leaf| leaf.member).collect::<Vec<_>>();
    for (index, &case) in leaves.keys().enumerate() {
        let (key, member) = supported_key_and_member(case);
        let sibling = members[(index + 1) % members.len()];
        assert_ne!(member, sibling);
        assert_rejected(key, sibling, "a sibling frozen member seal");
    }
}

#[test]
fn stored_poison_programs_reject_semantic_recipe_expected_and_range_drift() {
    let leaves = frozen_lock_stored_poison_leaves_v1();
    for source_completion in [
        LockStoredPoisonCompletionV1::RetentionSucceeded,
        LockStoredPoisonCompletionV1::RetentionRouteUnknown,
    ] {
        let (&case, _) = leaves
            .iter()
            .find(|(case, _)| case.completion == source_completion)
            .expect("stored-poison completion fixture");
        let (key, member) = supported_key_and_member(case);
        let mut completion = key;
        let DynamicAxesV1::Lock(axes) = &mut completion.axes else {
            unreachable!()
        };
        axes.completion = ReachabilityV1::Reached(match case.completion {
            LockStoredPoisonCompletionV1::RetentionSucceeded => {
                LockCompletionV1::UnsafeRetentionRouteUnknownThenRouteUnknown
            }
            LockStoredPoisonCompletionV1::RetentionRouteUnknown => {
                LockCompletionV1::UnsafeRetentionSucceededThenRouteUnknown
            }
        });
        assert_rejected(
            completion,
            member,
            "the opposite completion with the original seal",
        );
    }

    let (&case, _) = leaves.iter().next().expect("stored-poison fixture");
    let (key, member) = supported_key_and_member(case);

    let mut cleanup = key;
    cleanup.recipe.cleanup = CleanupV1::ParentOwnedRoot;
    assert_rejected(cleanup, member, "parent-owned cleanup");

    let mut mutation = key;
    mutation.expected.mutation = MutationStateV1::Uncertain;
    assert_rejected(mutation, member, "expected mutation drift");

    let mut route = key;
    route.expected.route = CustodyStateV1::Unchanged;
    assert_rejected(route, member, "route custody drift");

    let mut count = key;
    let DynamicAxesV1::Lock(axes) = &mut count.axes else {
        unreachable!()
    };
    axes.count = ReachabilityV1::Reached(2);
    axes.mask = ReachabilityV1::Reached(3);
    assert_rejected(count, member, "shared multi-slot drift");

    let mut mask = key;
    let DynamicAxesV1::Lock(axes) = &mut mask.axes else {
        unreachable!()
    };
    axes.mask = ReachabilityV1::Reached(2);
    assert_rejected(mask, member, "range mask drift");
}

#[cfg(windows)]
#[test]
fn isolated_stored_poison_family_receipts_are_exact() -> anyhow::Result<()> {
    let selected_child = selected_lock_stored_poison_selector_for_test();
    for (&case, leaf) in frozen_lock_stored_poison_leaves_v1() {
        if let Some(selected) = selected_child.as_deref() {
            let candidate = lock_stored_poison_selector_for_test(
                stored_poison_action_tag(case.action),
                stored_poison_profile_tag(case.profile),
                case.first,
                case.count,
                stored_poison_completion_tag(case.completion),
            )
            .map_err(anyhow::Error::msg)?;
            if candidate != selected {
                continue;
            }
        }
        let descriptor = lock_stored_poison_descriptor_v1(case, RunnerCapabilityV1::Supported);
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
            anyhow::anyhow!("supported Lock stored-poison member {case:?} failed: {error:?}")
        })?;
        assert!(validated.projection.is_ok());
        assert!(matches!(
            validated.runner_admission.decision(),
            RunnerAdmissionDecisionV1::Supported { .. }
        ));
    }
    match selected_child {
        Some(selected) => Err(anyhow::anyhow!(
            "parent-selected Lock stored-poison member was not found: {selected}"
        )),
        None => Ok(()),
    }
}

#[cfg(windows)]
const fn stored_poison_action_tag(value: LockStoredPoisonActionV1) -> u64 {
    match value {
        LockStoredPoisonActionV1::LockShared => 1,
        LockStoredPoisonActionV1::LockExclusive => 2,
        LockStoredPoisonActionV1::UnlockShared => 3,
        LockStoredPoisonActionV1::UnlockExclusive => 4,
    }
}

#[cfg(windows)]
const fn stored_poison_profile_tag(value: LockStoredPoisonProfileV1) -> u64 {
    match value {
        LockStoredPoisonProfileV1::GateNoMutation => 1,
        LockStoredPoisonProfileV1::FileCloseNoMutation => 2,
        LockStoredPoisonProfileV1::ExactSiblingDeleteNoMutation => 3,
        LockStoredPoisonProfileV1::ExactSiblingOpenUncertain => 4,
        LockStoredPoisonProfileV1::DmsTruncateUncertain => 5,
        LockStoredPoisonProfileV1::FileCloseUncertain => 6,
        LockStoredPoisonProfileV1::ExactSiblingDeleteUncertain => 7,
        LockStoredPoisonProfileV1::FileGrowUncertain => 8,
        LockStoredPoisonProfileV1::MappingCloseUncertain => 9,
        LockStoredPoisonProfileV1::ViewUnmapUncertain => 10,
        LockStoredPoisonProfileV1::LockReleaseUncertain => 11,
        LockStoredPoisonProfileV1::ConnectionDetachUncertain => 12,
        LockStoredPoisonProfileV1::DeleteAuthorizationUncertain => 13,
        LockStoredPoisonProfileV1::DmsExclusiveReleaseUncertain => 14,
        LockStoredPoisonProfileV1::DmsSharedReleaseUncertain => 15,
    }
}

#[cfg(windows)]
const fn stored_poison_completion_tag(value: LockStoredPoisonCompletionV1) -> u64 {
    match value {
        LockStoredPoisonCompletionV1::RetentionSucceeded => 3,
        LockStoredPoisonCompletionV1::RetentionRouteUnknown => 4,
    }
}

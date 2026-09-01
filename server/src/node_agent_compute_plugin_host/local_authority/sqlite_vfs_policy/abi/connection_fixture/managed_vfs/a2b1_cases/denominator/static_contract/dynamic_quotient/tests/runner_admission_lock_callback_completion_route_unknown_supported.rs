//! Exact admission and isolated receipt tests for 192 callback route-unknown programs.

use super::lock_callback_completion_route_unknown_cases::{
    frozen_lock_callback_completion_route_unknown_leaves_v1,
    lock_callback_completion_route_unknown_descriptor_v1,
    lock_callback_completion_route_unknown_leaf_v1,
    LockCallbackCompletionRouteUnknownFixturePathV1,
    LockCallbackCompletionRouteUnknownKeyV1,
    LOCK_CALLBACK_COMPLETION_ROUTE_UNKNOWN_MEMBER_COUNT,
};
use super::super::runner_admission::{
    callback_completion_route_unknown_catalog_row_count_for_test, compile_for_test,
    validate_lock_program_for_test,
};
#[cfg(windows)]
use super::super::runner_admission::{
    run_lock_isolated_for_test, LockRunnerIsolatedOutcomeV1, RunnerAdmissionDecisionV1,
};
#[cfg(windows)]
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2_dynamic_evidence::{
    lock_callback_route_unknown_selector_for_test,
    selected_lock_callback_route_unknown_selector_for_test, LockRunnerActionV1,
    LockRunnerCallbackRouteUnknownPathV1,
};
use super::*;

#[cfg(windows)]
const EXACT_TEST: &str = "node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2b1_cases::denominator::static_contract::dynamic_quotient::tests::runner_admission_lock_callback_completion_route_unknown_supported::isolated_callback_completion_route_unknown_family_receipts_are_exact";

fn supported_key_and_member(
    case: LockCallbackCompletionRouteUnknownKeyV1,
) -> (DynamicClassKeyV1, StaticMemberSealV1) {
    let leaf = lock_callback_completion_route_unknown_leaf_v1(case);
    let descriptor = lock_callback_completion_route_unknown_descriptor_v1(
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
        "Lock callback route-unknown admission accepted {mutation}"
    );
}

#[test]
fn all_192_exact_descriptors_and_members_are_admitted() {
    let leaves = frozen_lock_callback_completion_route_unknown_leaves_v1();
    assert_eq!(
        leaves.len(),
        LOCK_CALLBACK_COMPLETION_ROUTE_UNKNOWN_MEMBER_COUNT
    );
    assert_eq!(
        callback_completion_route_unknown_catalog_row_count_for_test(),
        LOCK_CALLBACK_COMPLETION_ROUTE_UNKNOWN_MEMBER_COUNT
    );
    for (&case, leaf) in leaves {
        let (key, member) = supported_key_and_member(case);
        assert_eq!(member, leaf.member);
        validate_lock_program_for_test(&key, member, compile_for_test(&key)).unwrap_or_else(
            |error| {
                panic!("exact Lock callback route-unknown member {case:?} was rejected: {error:?}")
            },
        );
    }
}

#[test]
fn every_callback_route_unknown_program_rejects_a_sibling_member_seal() {
    let leaves = frozen_lock_callback_completion_route_unknown_leaves_v1();
    let members = leaves.values().map(|leaf| leaf.member).collect::<Vec<_>>();
    for (index, &case) in leaves.keys().enumerate() {
        let (key, member) = supported_key_and_member(case);
        let sibling = members[(index + 1) % members.len()];
        assert_ne!(member, sibling);
        assert_rejected(key, sibling, "a sibling frozen member seal");
    }
}

#[test]
fn callback_route_unknown_programs_reject_completion_expected_recipe_and_range_drift() {
    let (&case, _) = frozen_lock_callback_completion_route_unknown_leaves_v1()
        .iter()
        .next()
        .expect("callback route-unknown fixture");
    let (key, member) = supported_key_and_member(case);

    let mut completion = key;
    let DynamicAxesV1::Lock(axes) = &mut completion.axes else {
        unreachable!()
    };
    axes.completion = ReachabilityV1::Reached(LockCompletionV1::Completed);
    assert_rejected(completion, member, "completed callback completion");

    let mut callback = key;
    callback.expected.callback = CustodyStateV1::Released;
    assert_rejected(callback, member, "released callback custody");

    let mut route = key;
    route.expected.route = CustodyStateV1::Unchanged;
    assert_rejected(route, member, "unchanged route custody");

    let mut disposition = key;
    disposition.expected.disposition = TerminalDispositionV1::Returned;
    assert_rejected(disposition, member, "returned disposition");

    let mut failure = key;
    failure.expected.failure = FailureClassV1::BusyNoMutation;
    assert_rejected(failure, member, "pre-completion failure class");

    let mut cleanup = key;
    cleanup.recipe.cleanup = CleanupV1::RetainUnsafeCustodyThenParentCleanup;
    assert_rejected(cleanup, member, "unsafe-retention cleanup");

    let mut action = key;
    let DynamicAxesV1::Lock(axes) = &mut action.axes else {
        unreachable!()
    };
    axes.action = ReachabilityV1::Reached(LockActionV1::UnlockExclusive);
    assert_rejected(action, member, "a path-incompatible action");

    let shared_case = frozen_lock_callback_completion_route_unknown_leaves_v1()
        .keys()
        .copied()
        .find(|case| {
            case.path == LockCallbackCompletionRouteUnknownFixturePathV1::NativeAcquireAcquired
                && case.action == LockActionV1::LockShared
        })
        .expect("shared native-acquire fixture");
    let (mut range, range_member) = supported_key_and_member(shared_case);
    let DynamicAxesV1::Lock(axes) = &mut range.axes else {
        unreachable!()
    };
    axes.count = ReachabilityV1::Reached(2);
    axes.mask = ReachabilityV1::Reached(3);
    assert_rejected(range, range_member, "a shared multi-slot range");
}

#[test]
fn callback_route_unknown_programs_reject_action_prestate_swaps() {
    let leaves = frozen_lock_callback_completion_route_unknown_leaves_v1();
    let sibling_case = leaves
        .keys()
        .copied()
        .find(|case| {
            case.path == LockCallbackCompletionRouteUnknownFixturePathV1::LocalSiblingContention
                && case.action == LockActionV1::LockShared
        })
        .expect("shared local sibling-contention fixture");
    let (mut sibling, sibling_member) = supported_key_and_member(sibling_case);
    sibling.prestate = PrestateV1::Lock(LockPrestateV1::SiblingAnyContention);
    assert_rejected(
        sibling,
        sibling_member,
        "LockShared with exclusive-action sibling prestate",
    );

    let release_case = leaves
        .keys()
        .copied()
        .find(|case| {
            case.path == LockCallbackCompletionRouteUnknownFixturePathV1::NativeRelease
                && case.action == LockActionV1::UnlockShared
        })
        .expect("shared native-release fixture");
    let (mut release, release_member) = supported_key_and_member(release_case);
    release.prestate = PrestateV1::Lock(LockPrestateV1::OwnExclusiveHeld);
    assert_rejected(
        release,
        release_member,
        "UnlockShared with exclusive-held prestate",
    );
}

#[cfg(windows)]
#[test]
fn isolated_callback_completion_route_unknown_family_receipts_are_exact() -> anyhow::Result<()> {
    let selected_child = selected_lock_callback_route_unknown_selector_for_test();
    for (&case, leaf) in frozen_lock_callback_completion_route_unknown_leaves_v1() {
        if let Some(selected) = selected_child.as_deref() {
            let candidate = lock_callback_route_unknown_selector_for_test(
                runner_path(case.path),
                runner_action(case.action),
                case.first,
                case.count,
                case.mask,
            )
            .map_err(anyhow::Error::msg)?;
            if candidate != selected {
                continue;
            }
        }
        let descriptor = lock_callback_completion_route_unknown_descriptor_v1(
            case,
            RunnerCapabilityV1::Supported,
        );
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
                "supported Lock callback route-unknown member {case:?} failed: {error:?}"
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
            "parent-selected Lock callback route-unknown member was not found: {selected}"
        )),
        None => Ok(()),
    }
}

#[cfg(windows)]
const fn runner_path(
    path: LockCallbackCompletionRouteUnknownFixturePathV1,
) -> LockRunnerCallbackRouteUnknownPathV1 {
    match path {
        LockCallbackCompletionRouteUnknownFixturePathV1::LocalSiblingContention => {
            LockRunnerCallbackRouteUnknownPathV1::LocalSiblingContention
        }
        LockCallbackCompletionRouteUnknownFixturePathV1::NativeRelease => {
            LockRunnerCallbackRouteUnknownPathV1::NativeRelease
        }
        LockCallbackCompletionRouteUnknownFixturePathV1::NativeAcquireAcquired => {
            LockRunnerCallbackRouteUnknownPathV1::NativeAcquireAcquired
        }
        LockCallbackCompletionRouteUnknownFixturePathV1::NativeAcquireBusy => {
            LockRunnerCallbackRouteUnknownPathV1::NativeAcquireBusy
        }
        LockCallbackCompletionRouteUnknownFixturePathV1::SharedLocalAcquire => {
            LockRunnerCallbackRouteUnknownPathV1::SharedLocalAcquire
        }
        LockCallbackCompletionRouteUnknownFixturePathV1::SharedLocalRelease => {
            LockRunnerCallbackRouteUnknownPathV1::SharedLocalRelease
        }
    }
}

#[cfg(windows)]
const fn runner_action(action: LockActionV1) -> LockRunnerActionV1 {
    match action {
        LockActionV1::LockShared => LockRunnerActionV1::LockShared,
        LockActionV1::LockExclusive => LockRunnerActionV1::LockExclusive,
        LockActionV1::UnlockShared => LockRunnerActionV1::UnlockShared,
        LockActionV1::UnlockExclusive => LockRunnerActionV1::UnlockExclusive,
    }
}

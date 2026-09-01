//! Windows isolated controlled-fault admission for every exact q13 member.

use super::super::runner_admission::{
    compile_for_test, run_lock_isolated_for_test, LockRunnerIsolatedOutcomeV1,
    RunnerAdmissionDecisionV1,
};
use super::lock_native_acquire_existing_first_exclusive_release_error_cases::{
    frozen_lock_existing_first_exclusive_release_error_leaves_v1,
    lock_existing_first_exclusive_release_error_descriptor_v1,
    FrozenExistingFirstExclusiveReleaseCompletionV1,
    FrozenLockExistingFirstExclusiveReleaseErrorCaseV1,
    LOCK_EXISTING_FIRST_EXCLUSIVE_RELEASE_ERROR_MEMBER_COUNT,
};
use super::*;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2_dynamic_evidence::{
    lock_native_acquire_existing_first_exclusive_release_error_selector_for_test,
    selected_lock_native_acquire_existing_first_exclusive_release_error_selector_for_test,
    LockRunnerActionV1, LockRunnerExistingFirstExclusiveReleaseCompletionV1,
};

const EXACT_TEST: &str = "node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2b1_cases::denominator::static_contract::dynamic_quotient::tests::runner_admission_lock_native_acquire_existing_first_exclusive_release_error_supported::isolated_q13_receipts_are_exact_controlled_fault_actual";

fn supported_key_and_member(
    case: FrozenLockExistingFirstExclusiveReleaseErrorCaseV1,
) -> (DynamicClassKeyV1, StaticMemberSealV1) {
    let leaf = &frozen_lock_existing_first_exclusive_release_error_leaves_v1()[&case];
    let validated = project_validated_dynamic_terminal_v1(&leaf.record, &leaf.descriptor).unwrap();
    assert_eq!(validated.descriptor_binding.member, leaf.member);
    let mut key = validated.semantic_key;
    key.recipe.capability = RunnerCapabilityV1::Supported;
    (key, leaf.member)
}

#[test]
fn isolated_q13_receipts_are_exact_controlled_fault_actual() -> anyhow::Result<()> {
    let selected_child =
        selected_lock_native_acquire_existing_first_exclusive_release_error_selector_for_test();
    let mut parent_receipts = 0;
    for (&case, leaf) in frozen_lock_existing_first_exclusive_release_error_leaves_v1() {
        let candidate =
            lock_native_acquire_existing_first_exclusive_release_error_selector_for_test(
                runner_action_v1(case.action),
                case.first,
                case.count,
                case.mask,
                runner_completion_v1(case.completion),
            )
            .map_err(anyhow::Error::msg)?;
        if selected_child
            .as_deref()
            .is_some_and(|selected| selected != candidate.as_str())
        {
            continue;
        }
        let descriptor = lock_existing_first_exclusive_release_error_descriptor_v1(
            case,
            RunnerCapabilityV1::Supported,
        );
        let (key, member) = supported_key_and_member(case);
        let execution =
            match run_lock_isolated_for_test(EXACT_TEST, &key, member, compile_for_test(&key))? {
                LockRunnerIsolatedOutcomeV1::ChildReported => return Ok(()),
                LockRunnerIsolatedOutcomeV1::ParentReceipt(receipt) => receipt,
            };
        let validated = project_validated_dynamic_terminal_with_lock_execution_v1(
            &leaf.record,
            &descriptor,
            execution,
        )
        .map_err(|error| anyhow::anyhow!("supported q13 member {case:?} failed: {error:?}"))?;
        assert!(validated.projection.is_ok());
        assert!(matches!(
            validated.runner_admission.decision(),
            RunnerAdmissionDecisionV1::Supported { .. }
        ));
        parent_receipts += 1;
    }
    match selected_child {
        Some(selected) => Err(anyhow::anyhow!(
            "parent-selected q13 member was not found: {selected}"
        )),
        None => {
            assert_eq!(
                parent_receipts,
                LOCK_EXISTING_FIRST_EXCLUSIVE_RELEASE_ERROR_MEMBER_COUNT
            );
            Ok(())
        }
    }
}
const fn runner_action_v1(action: LockActionV1) -> LockRunnerActionV1 {
    match action {
        LockActionV1::LockShared => LockRunnerActionV1::LockShared,
        LockActionV1::LockExclusive => LockRunnerActionV1::LockExclusive,
        LockActionV1::UnlockShared => LockRunnerActionV1::UnlockShared,
        LockActionV1::UnlockExclusive => LockRunnerActionV1::UnlockExclusive,
    }
}

const fn runner_completion_v1(
    completion: FrozenExistingFirstExclusiveReleaseCompletionV1,
) -> LockRunnerExistingFirstExclusiveReleaseCompletionV1 {
    match completion {
        FrozenExistingFirstExclusiveReleaseCompletionV1::RetentionSucceeded => {
            LockRunnerExistingFirstExclusiveReleaseCompletionV1::RetentionSucceeded
        }
        FrozenExistingFirstExclusiveReleaseCompletionV1::RetentionRouteUnknown => {
            LockRunnerExistingFirstExclusiveReleaseCompletionV1::RetentionRouteUnknown
        }
    }
}

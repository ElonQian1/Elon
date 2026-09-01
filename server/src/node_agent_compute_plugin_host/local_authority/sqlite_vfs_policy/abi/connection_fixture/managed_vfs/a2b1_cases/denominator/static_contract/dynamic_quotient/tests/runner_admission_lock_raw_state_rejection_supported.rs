//! Windows isolated execution admission for all eleven q11 Lock raw-state rejection members.

use super::super::runner_admission::{
    compile_for_test, run_lock_isolated_for_test, LockRunnerIsolatedOutcomeV1,
    RunnerAdmissionDecisionV1,
};
use super::lock_raw_state_rejection_cases::{
    frozen_lock_raw_state_rejection_leaves_v1, lock_raw_state_rejection_descriptor_v1,
    FrozenLockRawStateRejectionCaseV1, LOCK_RAW_STATE_REJECTION_MEMBER_COUNT,
};
use super::*;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2_dynamic_evidence::{
    lock_raw_state_rejection_selector_for_test,
    selected_lock_raw_state_rejection_selector_for_test,
};

const EXACT_TEST: &str = "node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2b1_cases::denominator::static_contract::dynamic_quotient::tests::runner_admission_lock_raw_state_rejection_supported::isolated_raw_state_rejection_family_receipts_are_exact";

fn supported_key_and_member(
    case: FrozenLockRawStateRejectionCaseV1,
) -> (DynamicClassKeyV1, StaticMemberSealV1) {
    let leaf = &frozen_lock_raw_state_rejection_leaves_v1()[&case];
    let descriptor = lock_raw_state_rejection_descriptor_v1(
        case,
        RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete),
    );
    let validated = project_validated_dynamic_terminal_v1(&leaf.record, &descriptor).unwrap();
    assert_eq!(validated.descriptor_binding.member, leaf.member);
    let mut key = validated.semantic_key;
    key.recipe.capability = RunnerCapabilityV1::Supported;
    (key, leaf.member)
}

#[test]
fn isolated_raw_state_rejection_family_receipts_are_exact() -> anyhow::Result<()> {
    for (raw, completion) in [(7, 1), (1, 6), (9, 7), (10, 7)] {
        assert!(lock_raw_state_rejection_selector_for_test(raw, completion).is_err());
    }
    for unknown in [0, 11, u64::MAX] {
        assert!(lock_raw_state_rejection_selector_for_test(unknown, 1).is_err());
        assert!(lock_raw_state_rejection_selector_for_test(1, unknown).is_err());
    }

    let selected_child = selected_lock_raw_state_rejection_selector_for_test();
    let mut parent_receipts = 0;
    for (&case, leaf) in frozen_lock_raw_state_rejection_leaves_v1() {
        let candidate = lock_raw_state_rejection_selector_for_test(
            raw_state_tag_v1(case),
            completion_tag_v1(case),
        )
        .map_err(anyhow::Error::msg)?;
        if selected_child
            .as_deref()
            .is_some_and(|selected| selected != candidate.as_str())
        {
            continue;
        }
        let descriptor =
            lock_raw_state_rejection_descriptor_v1(case, RunnerCapabilityV1::Supported);
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
        .map_err(|error| anyhow::anyhow!("supported q11 member {case:?} failed: {error:?}"))?;
        assert!(validated.projection.is_ok());
        assert!(matches!(
            validated.runner_admission.decision(),
            RunnerAdmissionDecisionV1::Supported { .. }
        ));
        parent_receipts += 1;
    }
    match selected_child {
        Some(selected) => Err(anyhow::anyhow!(
            "parent-selected q11 raw-state member was not found: {selected}"
        )),
        None => {
            assert_eq!(parent_receipts, LOCK_RAW_STATE_REJECTION_MEMBER_COUNT);
            Ok(())
        }
    }
}

const fn raw_state_tag_v1(case: FrozenLockRawStateRejectionCaseV1) -> u64 {
    match case {
        FrozenLockRawStateRejectionCaseV1::NullFileDirect => 1,
        FrozenLockRawStateRejectionCaseV1::UninstalledDirect => 2,
        FrozenLockRawStateRejectionCaseV1::MethodsNullStatePresentDirect => 3,
        FrozenLockRawStateRejectionCaseV1::ForeignMethodsStateNullDirect => 4,
        FrozenLockRawStateRejectionCaseV1::ForeignMethodsStatePresentDirect => 5,
        FrozenLockRawStateRejectionCaseV1::ExactMethodsStateNullDirect => 6,
        FrozenLockRawStateRejectionCaseV1::OtherTypePayloadMissingDropCompleted => 7,
        FrozenLockRawStateRejectionCaseV1::OtherTypePayloadPresentDropCompleted
        | FrozenLockRawStateRejectionCaseV1::OtherTypePayloadPresentDropUnwindCaught => 8,
        FrozenLockRawStateRejectionCaseV1::ExpectedTypePayloadMissingDropCompleted => 9,
        FrozenLockRawStateRejectionCaseV1::HandleBoundFileMissingDirect => 10,
    }
}

const fn completion_tag_v1(case: FrozenLockRawStateRejectionCaseV1) -> u64 {
    match case {
        FrozenLockRawStateRejectionCaseV1::OtherTypePayloadMissingDropCompleted
        | FrozenLockRawStateRejectionCaseV1::OtherTypePayloadPresentDropCompleted
        | FrozenLockRawStateRejectionCaseV1::ExpectedTypePayloadMissingDropCompleted => 6,
        FrozenLockRawStateRejectionCaseV1::OtherTypePayloadPresentDropUnwindCaught => 7,
        _ => 1,
    }
}

//! Windows isolated execution admission for all seven q10 ABI-scalar rejection members.

use super::super::runner_admission::{
    compile_for_test, run_lock_isolated_for_test, LockRunnerIsolatedOutcomeV1,
    RunnerAdmissionDecisionV1,
};
use super::lock_abi_scalar_rejection_cases::{
    frozen_lock_abi_scalar_rejection_leaves_v1, lock_abi_scalar_rejection_descriptor_v1,
    LOCK_ABI_SCALAR_REJECTION_MEMBER_COUNT,
};
use super::*;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2_dynamic_evidence::{
    lock_abi_scalar_rejection_selector_for_test,
    selected_lock_abi_scalar_rejection_selector_for_test,
};

const EXACT_TEST: &str = "node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2b1_cases::denominator::static_contract::dynamic_quotient::tests::runner_admission_lock_abi_scalar_rejection_supported::isolated_abi_scalar_rejection_family_receipts_are_exact";

fn supported_key_and_member(scalar: LockAbiScalarV1) -> (DynamicClassKeyV1, StaticMemberSealV1) {
    let leaf = &frozen_lock_abi_scalar_rejection_leaves_v1()[&scalar];
    let descriptor = lock_abi_scalar_rejection_descriptor_v1(
        scalar,
        RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete),
    );
    let validated = project_validated_dynamic_terminal_v1(&leaf.record, &descriptor).unwrap();
    assert_eq!(validated.descriptor_binding.member, leaf.member);
    let mut key = validated.semantic_key;
    key.recipe.capability = RunnerCapabilityV1::Supported;
    (key, leaf.member)
}

#[test]
fn isolated_abi_scalar_rejection_family_receipts_are_exact() -> anyhow::Result<()> {
    assert!(lock_abi_scalar_rejection_selector_for_test(2, 2, 2).is_err());
    for unknown in [0, 3, u64::MAX] {
        assert!(lock_abi_scalar_rejection_selector_for_test(unknown, 1, 1).is_err());
        assert!(lock_abi_scalar_rejection_selector_for_test(1, unknown, 1).is_err());
        assert!(lock_abi_scalar_rejection_selector_for_test(1, 1, unknown).is_err());
    }

    let selected_child = selected_lock_abi_scalar_rejection_selector_for_test();
    let mut parent_receipts = 0;
    for (&scalar, leaf) in frozen_lock_abi_scalar_rejection_leaves_v1() {
        let candidate = lock_abi_scalar_rejection_selector_for_test(
            validity_tag(scalar.offset),
            validity_tag(scalar.count),
            validity_tag(scalar.flags),
        )
        .map_err(anyhow::Error::msg)?;
        if selected_child
            .as_deref()
            .is_some_and(|selected| selected != candidate.as_str())
        {
            continue;
        }
        let descriptor =
            lock_abi_scalar_rejection_descriptor_v1(scalar, RunnerCapabilityV1::Supported);
        let (key, member) = supported_key_and_member(scalar);
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
        .map_err(|error| anyhow::anyhow!("supported q10 member {scalar:?} failed: {error:?}"))?;
        assert!(validated.projection.is_ok());
        assert!(matches!(
            validated.runner_admission.decision(),
            RunnerAdmissionDecisionV1::Supported { .. }
        ));
        parent_receipts += 1;
    }
    match selected_child {
        Some(selected) => Err(anyhow::anyhow!(
            "parent-selected q10 ABI scalar member was not found: {selected}"
        )),
        None => {
            assert_eq!(parent_receipts, LOCK_ABI_SCALAR_REJECTION_MEMBER_COUNT);
            Ok(())
        }
    }
}

const fn validity_tag(value: ValidityV1) -> u64 {
    match value {
        ValidityV1::Invalid => 1,
        ValidityV1::Valid => 2,
    }
}

//! Independent Windows child tests for every exact positive Lock lifecycle program.

use super::super::runner_admission::{
    compile_for_test, run_lock_isolated_for_test, tamper_lock_implementation_digest_for_test,
    LockRunnerIsolatedOutcomeV1, RunnerAdmissionDecisionV1,
};
use super::program_inventory::{
    lock_lifecycle_cases, lock_lifecycle_descriptor, lock_lifecycle_record,
    LockLifecycleProgramCaseV1,
};
use super::*;

const EXACT_TEST_PREFIX: &str = "node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2b1_cases::denominator::static_contract::dynamic_quotient::tests::runner_admission_lock_lifecycle_supported::";

fn supported_key_and_member(
    case: LockLifecycleProgramCaseV1,
) -> (DynamicClassKeyV1, StaticMemberSealV1) {
    let record = lock_lifecycle_record(case);
    let descriptor = lock_lifecycle_descriptor(
        case,
        RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete),
    );
    let validated = project_validated_dynamic_terminal_v1(&record, &descriptor).unwrap();
    let mut key = validated.semantic_key;
    key.recipe.capability = RunnerCapabilityV1::Supported;
    (key, validated.descriptor_binding.member)
}

fn exercise_supported_projection(
    case: LockLifecycleProgramCaseV1,
    exact_test: &str,
) -> anyhow::Result<()> {
    let record = lock_lifecycle_record(case);
    let descriptor = lock_lifecycle_descriptor(case, RunnerCapabilityV1::Supported);
    let (key, member) = supported_key_and_member(case);
    let plan = compile_for_test(&key);
    let execution = match run_lock_isolated_for_test(exact_test, &key, member, plan)? {
        LockRunnerIsolatedOutcomeV1::ChildReported => return Ok(()),
        LockRunnerIsolatedOutcomeV1::ParentReceipt(receipt) => receipt,
    };
    let validated =
        project_validated_dynamic_terminal_with_lock_execution_v1(&record, &descriptor, execution)
            .map_err(|error| anyhow::anyhow!("supported Lock lifecycle failed: {error:?}"))?;
    assert!(validated.projection.is_ok());
    assert!(matches!(
        validated.runner_admission.decision(),
        RunnerAdmissionDecisionV1::Supported { .. }
    ));
    Ok(())
}

macro_rules! exact_lock_lifecycle_test {
    ($name:ident, $index:expr) => {
        #[test]
        fn $name() -> anyhow::Result<()> {
            let cases = lock_lifecycle_cases();
            assert_eq!(cases.len(), 104);
            exercise_supported_projection(
                cases[$index],
                &format!("{EXACT_TEST_PREFIX}{}", stringify!($name)),
            )
        }
    };
}

exact_lock_lifecycle_test!(native_acquire_shared_first0_count1, 0);
exact_lock_lifecycle_test!(native_acquire_shared_first1_count1, 1);
exact_lock_lifecycle_test!(native_acquire_shared_first2_count1, 2);
exact_lock_lifecycle_test!(native_acquire_shared_first3_count1, 3);
exact_lock_lifecycle_test!(native_acquire_shared_first4_count1, 4);
exact_lock_lifecycle_test!(native_acquire_shared_first5_count1, 5);
exact_lock_lifecycle_test!(native_acquire_shared_first6_count1, 6);
exact_lock_lifecycle_test!(native_acquire_shared_first7_count1, 7);
exact_lock_lifecycle_test!(native_acquire_exclusive_first0_count1, 8);
exact_lock_lifecycle_test!(native_acquire_exclusive_first0_count2, 9);
exact_lock_lifecycle_test!(native_acquire_exclusive_first0_count3, 10);
exact_lock_lifecycle_test!(native_acquire_exclusive_first0_count4, 11);
exact_lock_lifecycle_test!(native_acquire_exclusive_first0_count5, 12);
exact_lock_lifecycle_test!(native_acquire_exclusive_first0_count6, 13);
exact_lock_lifecycle_test!(native_acquire_exclusive_first0_count7, 14);
exact_lock_lifecycle_test!(native_acquire_exclusive_first0_count8, 15);
exact_lock_lifecycle_test!(native_acquire_exclusive_first1_count1, 16);
exact_lock_lifecycle_test!(native_acquire_exclusive_first1_count2, 17);
exact_lock_lifecycle_test!(native_acquire_exclusive_first1_count3, 18);
exact_lock_lifecycle_test!(native_acquire_exclusive_first1_count4, 19);
exact_lock_lifecycle_test!(native_acquire_exclusive_first1_count5, 20);
exact_lock_lifecycle_test!(native_acquire_exclusive_first1_count6, 21);
exact_lock_lifecycle_test!(native_acquire_exclusive_first1_count7, 22);
exact_lock_lifecycle_test!(native_acquire_exclusive_first2_count1, 23);
exact_lock_lifecycle_test!(native_acquire_exclusive_first2_count2, 24);
exact_lock_lifecycle_test!(native_acquire_exclusive_first2_count3, 25);
exact_lock_lifecycle_test!(native_acquire_exclusive_first2_count4, 26);
exact_lock_lifecycle_test!(native_acquire_exclusive_first2_count5, 27);
exact_lock_lifecycle_test!(native_acquire_exclusive_first2_count6, 28);
exact_lock_lifecycle_test!(native_acquire_exclusive_first3_count1, 29);
exact_lock_lifecycle_test!(native_acquire_exclusive_first3_count2, 30);
exact_lock_lifecycle_test!(native_acquire_exclusive_first3_count3, 31);
exact_lock_lifecycle_test!(native_acquire_exclusive_first3_count4, 32);
exact_lock_lifecycle_test!(native_acquire_exclusive_first3_count5, 33);
exact_lock_lifecycle_test!(native_acquire_exclusive_first4_count1, 34);
exact_lock_lifecycle_test!(native_acquire_exclusive_first4_count2, 35);
exact_lock_lifecycle_test!(native_acquire_exclusive_first4_count3, 36);
exact_lock_lifecycle_test!(native_acquire_exclusive_first4_count4, 37);
exact_lock_lifecycle_test!(native_acquire_exclusive_first5_count1, 38);
exact_lock_lifecycle_test!(native_acquire_exclusive_first5_count2, 39);
exact_lock_lifecycle_test!(native_acquire_exclusive_first5_count3, 40);
exact_lock_lifecycle_test!(native_acquire_exclusive_first6_count1, 41);
exact_lock_lifecycle_test!(native_acquire_exclusive_first6_count2, 42);
exact_lock_lifecycle_test!(native_acquire_exclusive_first7_count1, 43);

exact_lock_lifecycle_test!(native_release_shared_first0_count1, 44);
exact_lock_lifecycle_test!(native_release_shared_first1_count1, 45);
exact_lock_lifecycle_test!(native_release_shared_first2_count1, 46);
exact_lock_lifecycle_test!(native_release_shared_first3_count1, 47);
exact_lock_lifecycle_test!(native_release_shared_first4_count1, 48);
exact_lock_lifecycle_test!(native_release_shared_first5_count1, 49);
exact_lock_lifecycle_test!(native_release_shared_first6_count1, 50);
exact_lock_lifecycle_test!(native_release_shared_first7_count1, 51);
exact_lock_lifecycle_test!(native_release_exclusive_first0_count1, 52);
exact_lock_lifecycle_test!(native_release_exclusive_first0_count2, 53);
exact_lock_lifecycle_test!(native_release_exclusive_first0_count3, 54);
exact_lock_lifecycle_test!(native_release_exclusive_first0_count4, 55);
exact_lock_lifecycle_test!(native_release_exclusive_first0_count5, 56);
exact_lock_lifecycle_test!(native_release_exclusive_first0_count6, 57);
exact_lock_lifecycle_test!(native_release_exclusive_first0_count7, 58);
exact_lock_lifecycle_test!(native_release_exclusive_first0_count8, 59);
exact_lock_lifecycle_test!(native_release_exclusive_first1_count1, 60);
exact_lock_lifecycle_test!(native_release_exclusive_first1_count2, 61);
exact_lock_lifecycle_test!(native_release_exclusive_first1_count3, 62);
exact_lock_lifecycle_test!(native_release_exclusive_first1_count4, 63);
exact_lock_lifecycle_test!(native_release_exclusive_first1_count5, 64);
exact_lock_lifecycle_test!(native_release_exclusive_first1_count6, 65);
exact_lock_lifecycle_test!(native_release_exclusive_first1_count7, 66);
exact_lock_lifecycle_test!(native_release_exclusive_first2_count1, 67);
exact_lock_lifecycle_test!(native_release_exclusive_first2_count2, 68);
exact_lock_lifecycle_test!(native_release_exclusive_first2_count3, 69);
exact_lock_lifecycle_test!(native_release_exclusive_first2_count4, 70);
exact_lock_lifecycle_test!(native_release_exclusive_first2_count5, 71);
exact_lock_lifecycle_test!(native_release_exclusive_first2_count6, 72);
exact_lock_lifecycle_test!(native_release_exclusive_first3_count1, 73);
exact_lock_lifecycle_test!(native_release_exclusive_first3_count2, 74);
exact_lock_lifecycle_test!(native_release_exclusive_first3_count3, 75);
exact_lock_lifecycle_test!(native_release_exclusive_first3_count4, 76);
exact_lock_lifecycle_test!(native_release_exclusive_first3_count5, 77);
exact_lock_lifecycle_test!(native_release_exclusive_first4_count1, 78);
exact_lock_lifecycle_test!(native_release_exclusive_first4_count2, 79);
exact_lock_lifecycle_test!(native_release_exclusive_first4_count3, 80);
exact_lock_lifecycle_test!(native_release_exclusive_first4_count4, 81);
exact_lock_lifecycle_test!(native_release_exclusive_first5_count1, 82);
exact_lock_lifecycle_test!(native_release_exclusive_first5_count2, 83);
exact_lock_lifecycle_test!(native_release_exclusive_first5_count3, 84);
exact_lock_lifecycle_test!(native_release_exclusive_first6_count1, 85);
exact_lock_lifecycle_test!(native_release_exclusive_first6_count2, 86);
exact_lock_lifecycle_test!(native_release_exclusive_first7_count1, 87);

exact_lock_lifecycle_test!(shared_local_acquire_first0_count1, 88);
exact_lock_lifecycle_test!(shared_local_release_first0_count1, 89);
exact_lock_lifecycle_test!(shared_local_acquire_first1_count1, 90);
exact_lock_lifecycle_test!(shared_local_release_first1_count1, 91);
exact_lock_lifecycle_test!(shared_local_acquire_first2_count1, 92);
exact_lock_lifecycle_test!(shared_local_release_first2_count1, 93);
exact_lock_lifecycle_test!(shared_local_acquire_first3_count1, 94);
exact_lock_lifecycle_test!(shared_local_release_first3_count1, 95);
exact_lock_lifecycle_test!(shared_local_acquire_first4_count1, 96);
exact_lock_lifecycle_test!(shared_local_release_first4_count1, 97);
exact_lock_lifecycle_test!(shared_local_acquire_first5_count1, 98);
exact_lock_lifecycle_test!(shared_local_release_first5_count1, 99);
exact_lock_lifecycle_test!(shared_local_acquire_first6_count1, 100);
exact_lock_lifecycle_test!(shared_local_release_first6_count1, 101);
exact_lock_lifecycle_test!(shared_local_acquire_first7_count1, 102);
exact_lock_lifecycle_test!(shared_local_release_first7_count1, 103);

fn execution_for(
    case: LockLifecycleProgramCaseV1,
    exact_test: &str,
) -> anyhow::Result<Option<super::super::runner_admission::LockRunnerExecutionReceiptV1>> {
    let (key, member) = supported_key_and_member(case);
    let plan = compile_for_test(&key);
    Ok(
        match run_lock_isolated_for_test(exact_test, &key, member, plan)? {
            LockRunnerIsolatedOutcomeV1::ChildReported => None,
            LockRunnerIsolatedOutcomeV1::ParentReceipt(receipt) => Some(receipt),
        },
    )
}

#[test]
fn lifecycle_receipt_rejects_member_replay() -> anyhow::Result<()> {
    let cases = lock_lifecycle_cases();
    let source = cases[0];
    let exact = format!("{EXACT_TEST_PREFIX}lifecycle_receipt_rejects_member_replay");
    let Some(execution) = execution_for(source, &exact)? else {
        return Ok(());
    };
    let mut record = lock_lifecycle_record(source);
    record.key.identity.leaf_id.push_str("-other-member");
    assert!(project_validated_dynamic_terminal_with_lock_execution_v1(
        &record,
        &lock_lifecycle_descriptor(source, RunnerCapabilityV1::Supported),
        execution,
    )
    .is_err());
    Ok(())
}

#[test]
fn lifecycle_receipt_rejects_cross_range_replay() -> anyhow::Result<()> {
    reject_cross_case_replay(8, 9, "lifecycle_receipt_rejects_cross_range_replay")
}

#[test]
fn lifecycle_receipt_rejects_cross_action_replay() -> anyhow::Result<()> {
    reject_cross_case_replay(0, 8, "lifecycle_receipt_rejects_cross_action_replay")
}

#[test]
fn lifecycle_receipt_rejects_cross_operation_replay() -> anyhow::Result<()> {
    reject_cross_case_replay(0, 88, "lifecycle_receipt_rejects_cross_operation_replay")
}

#[test]
fn lifecycle_receipt_rejects_selected_sibling_path_replay() -> anyhow::Result<()> {
    reject_cross_case_replay(
        88,
        89,
        "lifecycle_receipt_rejects_selected_sibling_path_replay",
    )
}

fn reject_cross_case_replay(
    source_index: usize,
    target_index: usize,
    test_name: &'static str,
) -> anyhow::Result<()> {
    let cases = lock_lifecycle_cases();
    let source = cases[source_index];
    let target = cases[target_index];
    let exact = format!("{EXACT_TEST_PREFIX}{test_name}");
    let Some(execution) = execution_for(source, &exact)? else {
        return Ok(());
    };
    assert!(project_validated_dynamic_terminal_with_lock_execution_v1(
        &lock_lifecycle_record(target),
        &lock_lifecycle_descriptor(target, RunnerCapabilityV1::Supported),
        execution,
    )
    .is_err());
    Ok(())
}

#[test]
fn lifecycle_receipt_rejects_implementation_digest_tamper() -> anyhow::Result<()> {
    let case = lock_lifecycle_cases()[0];
    let exact =
        format!("{EXACT_TEST_PREFIX}lifecycle_receipt_rejects_implementation_digest_tamper");
    let Some(mut execution) = execution_for(case, &exact)? else {
        return Ok(());
    };
    tamper_lock_implementation_digest_for_test(&mut execution, Digest32([0x71; 32]));
    assert!(project_validated_dynamic_terminal_with_lock_execution_v1(
        &lock_lifecycle_record(case),
        &lock_lifecycle_descriptor(case, RunnerCapabilityV1::Supported),
        execution,
    )
    .is_err());
    Ok(())
}

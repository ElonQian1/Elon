//! Independent Windows child tests for six exact positive Map single-region lifecycle programs.

use super::super::runner_admission::{
    compile_for_test, run_isolated_for_test, tamper_implementation_digest_for_test,
    validate_map_program_for_test, MapRunnerExecutionReceiptV1, MapRunnerIsolatedOutcomeV1,
    RunnerAdmissionDecisionV1,
};
use super::map_program_cases::{
    map_lifecycle_descriptor_v1, map_lifecycle_leaf_v1, MapLifecycleProgramCaseV1,
    MAP_LIFECYCLE_CASES,
};
use super::*;

const EXACT_TEST_PREFIX: &str = "node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2b1_cases::denominator::static_contract::dynamic_quotient::tests::runner_admission_map_lifecycle_supported::";

fn supported_key_and_member(
    case: MapLifecycleProgramCaseV1,
) -> (DynamicClassKeyV1, StaticMemberSealV1) {
    let leaf = map_lifecycle_leaf_v1(case);
    let descriptor = map_lifecycle_descriptor_v1(
        case,
        RunnerCapabilityV1::Missing(CapabilityGapV1::QuotientRunnerNotIntegrated),
    );
    let validated = project_validated_dynamic_terminal_v1(&leaf.record, &descriptor).unwrap();
    assert_eq!(validated.descriptor_binding.member, leaf.member);
    let mut key = validated.semantic_key;
    key.recipe.capability = RunnerCapabilityV1::Supported;
    (key, leaf.member)
}

fn exercise_supported_projection(
    case: MapLifecycleProgramCaseV1,
    exact_test: &str,
) -> anyhow::Result<()> {
    let leaf = map_lifecycle_leaf_v1(case);
    let descriptor = map_lifecycle_descriptor_v1(case, RunnerCapabilityV1::Supported);
    let (key, member) = supported_key_and_member(case);
    let plan = compile_for_test(&key);
    let execution = match run_isolated_for_test(exact_test, &key, member, plan)? {
        MapRunnerIsolatedOutcomeV1::ChildReported => return Ok(()),
        MapRunnerIsolatedOutcomeV1::ParentReceipt(receipt) => receipt,
    };
    let validated = project_validated_dynamic_terminal_with_map_execution_v1(
        &leaf.record,
        &descriptor,
        execution,
    )
    .map_err(|error| anyhow::anyhow!("supported Map lifecycle failed: {error:?}"))?;
    assert!(validated.projection.is_ok());
    assert!(matches!(
        validated.runner_admission.decision(),
        RunnerAdmissionDecisionV1::Supported { .. }
    ));
    Ok(())
}

macro_rules! exact_map_lifecycle_test {
    ($name:ident, $case:ident) => {
        #[test]
        fn $name() -> anyhow::Result<()> {
            exercise_supported_projection(
                MapLifecycleProgramCaseV1::$case,
                &format!("{EXACT_TEST_PREFIX}{}", stringify!($name)),
            )
        }
    };
}

exact_map_lifecycle_test!(empty_observe_not_present_completed, EmptyObserveNotPresent);
exact_map_lifecycle_test!(empty_extend_mapped_completed, EmptyExtendMapped);
exact_map_lifecycle_test!(reuse_observe_mapped_completed, ReuseObserveMapped);
exact_map_lifecycle_test!(reuse_extend_mapped_completed, ReuseExtendMapped);
exact_map_lifecycle_test!(
    target_missing_observe_not_present_completed,
    MissingObserveNotPresent
);
exact_map_lifecycle_test!(target_missing_extend_mapped_completed, MissingExtendMapped);

#[test]
fn six_exact_lifecycle_programs_bind_their_frozen_members_before_spawn() {
    for case in MAP_LIFECYCLE_CASES {
        let leaf = map_lifecycle_leaf_v1(case);
        let (key, member) = supported_key_and_member(case);
        assert_eq!(member, leaf.member);
        validate_map_program_for_test(&key, member, compile_for_test(&key)).unwrap();
    }
}

#[test]
fn lifecycle_program_rejects_cross_member_replay_before_spawn() {
    let (key, _) = supported_key_and_member(MapLifecycleProgramCaseV1::EmptyObserveNotPresent);
    let replayed = map_lifecycle_leaf_v1(MapLifecycleProgramCaseV1::EmptyExtendMapped).member;
    assert!(validate_map_program_for_test(&key, replayed, compile_for_test(&key)).is_err());
}

#[test]
fn lifecycle_program_rejects_expected_drift_before_spawn() {
    let (mut key, member) =
        supported_key_and_member(MapLifecycleProgramCaseV1::EmptyObserveNotPresent);
    key.expected.payload = CustodyStateV1::Retained;
    assert!(validate_map_program_for_test(&key, member, compile_for_test(&key)).is_err());
}

#[test]
fn lifecycle_program_rejects_prestate_drift_before_spawn() {
    let (mut key, member) = supported_key_and_member(MapLifecycleProgramCaseV1::ReuseObserveMapped);
    key.prestate = PrestateV1::Map(MapPrestateV1::RegionsEmpty);
    assert!(validate_map_program_for_test(&key, member, compile_for_test(&key)).is_err());
}

#[test]
fn lifecycle_program_rejects_ordinal_drift_before_spawn() {
    let (mut key, member) =
        supported_key_and_member(MapLifecycleProgramCaseV1::MissingExtendMapped);
    let DynamicAxesV1::Map(axes) = &mut key.axes else {
        unreachable!()
    };
    axes.ordinal = ReachabilityV1::Reached(2);
    assert!(validate_map_program_for_test(&key, member, compile_for_test(&key)).is_err());
}

fn execution_for(
    case: MapLifecycleProgramCaseV1,
    exact_test: &str,
) -> anyhow::Result<Option<MapRunnerExecutionReceiptV1>> {
    let (key, member) = supported_key_and_member(case);
    let plan = compile_for_test(&key);
    Ok(
        match run_isolated_for_test(exact_test, &key, member, plan)? {
            MapRunnerIsolatedOutcomeV1::ChildReported => None,
            MapRunnerIsolatedOutcomeV1::ParentReceipt(receipt) => Some(receipt),
        },
    )
}

#[test]
fn lifecycle_receipt_rejects_cross_path_replay() -> anyhow::Result<()> {
    let source = MapLifecycleProgramCaseV1::EmptyExtendMapped;
    let target = MapLifecycleProgramCaseV1::MissingExtendMapped;
    let exact = format!("{EXACT_TEST_PREFIX}lifecycle_receipt_rejects_cross_path_replay");
    let Some(execution) = execution_for(source, &exact)? else {
        return Ok(());
    };
    let target_leaf = map_lifecycle_leaf_v1(target);
    assert!(project_validated_dynamic_terminal_with_map_execution_v1(
        &target_leaf.record,
        &map_lifecycle_descriptor_v1(target, RunnerCapabilityV1::Supported),
        execution,
    )
    .is_err());
    Ok(())
}

#[test]
fn lifecycle_receipt_rejects_implementation_digest_tamper() -> anyhow::Result<()> {
    let case = MapLifecycleProgramCaseV1::ReuseObserveMapped;
    let exact =
        format!("{EXACT_TEST_PREFIX}lifecycle_receipt_rejects_implementation_digest_tamper");
    let Some(mut execution) = execution_for(case, &exact)? else {
        return Ok(());
    };
    tamper_implementation_digest_for_test(&mut execution, Digest32([0x6d; 32]));
    let leaf = map_lifecycle_leaf_v1(case);
    assert!(project_validated_dynamic_terminal_with_map_execution_v1(
        &leaf.record,
        &map_lifecycle_descriptor_v1(case, RunnerCapabilityV1::Supported),
        execution,
    )
    .is_err());
    Ok(())
}

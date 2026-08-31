use super::super::runner_admission::{
    compile_for_test, run_isolated_for_test, tamper_implementation_digest_for_test,
    MapRunnerIsolatedOutcomeV1, RunnerAdmissionDecisionV1,
};
use super::map_program_cases::{request_budget_descriptor_v1, request_budget_leaf_v1};
use super::*;

const EXACT_TEST_PREFIX: &str = "node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2b1_cases::denominator::static_contract::dynamic_quotient::tests::runner_admission_supported::";

fn supported_key_and_member(
    stimulus: MapManagedStimulusV1,
    mode: MapModeV1,
) -> (DynamicClassKeyV1, StaticMemberSealV1) {
    let leaf = request_budget_leaf_v1(stimulus, mode);
    let descriptor = request_budget_descriptor_v1(
        stimulus,
        mode,
        RunnerCapabilityV1::Missing(CapabilityGapV1::QuotientRunnerNotIntegrated),
    );
    let validated = project_validated_dynamic_terminal_v1(&leaf.record, &descriptor).unwrap();
    assert_eq!(validated.descriptor_binding.member, leaf.member);
    let mut key = validated.semantic_key;
    key.recipe.capability = RunnerCapabilityV1::Supported;
    (key, leaf.member)
}

#[test]
fn exact_supported_map_class_still_rejects_without_private_execution_receipt() {
    let leaf = request_budget_leaf_v1(MapManagedStimulusV1::RegionCountBudget, MapModeV1::Extend);
    let descriptor = request_budget_descriptor_v1(
        MapManagedStimulusV1::RegionCountBudget,
        MapModeV1::Extend,
        RunnerCapabilityV1::Supported,
    );
    assert_eq!(
        project_dynamic_class_v1(&leaf.record, &descriptor),
        Err(ProjectionErrorV1::Invalid(
            ProjectionViolationV1::RunnerAdmissionUnsealedSupported,
        ))
    );
}

#[test]
fn supported_capability_is_limited_to_the_exact_programmed_map_class() {
    let leaf = request_budget_leaf_v1(MapManagedStimulusV1::RegionCountBudget, MapModeV1::Extend);
    let mut descriptor = request_budget_descriptor_v1(
        MapManagedStimulusV1::RegionCountBudget,
        MapModeV1::Extend,
        RunnerCapabilityV1::Supported,
    );
    let TerminalDescriptorV1::Map(value) = &mut descriptor else {
        unreachable!()
    };
    value.stimulus = StimulusV1::MapManaged(MapManagedStimulusV1::AllocationGranularity);
    assert_eq!(
        project_dynamic_class_v1(&leaf.record, &descriptor),
        Err(ProjectionErrorV1::Invalid(
            ProjectionViolationV1::MapProducerRecipeMismatch,
        ))
    );
}

#[test]
fn exact_program_rejects_expected_semantics_drift_before_child_spawn() {
    let (mut key, member) =
        supported_key_and_member(MapManagedStimulusV1::RegionCountBudget, MapModeV1::Extend);
    key.expected.callback = CustodyStateV1::NotReached;
    let plan = compile_for_test(&key);
    assert!(
        run_isolated_for_test("must-not-spawn-for-invalid-map-program", &key, member, plan,)
            .is_err()
    );
}

fn exercise_supported_projection(
    stimulus: MapManagedStimulusV1,
    mode: MapModeV1,
    exact_test: &str,
) -> anyhow::Result<()> {
    let leaf = request_budget_leaf_v1(stimulus, mode);
    let descriptor = request_budget_descriptor_v1(stimulus, mode, RunnerCapabilityV1::Supported);
    let (key, member) = supported_key_and_member(stimulus, mode);
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
    .map_err(|error| anyhow::anyhow!("supported Map projection failed: {error:?}"))?;
    assert!(validated.projection.is_ok());
    assert!(matches!(
        validated.runner_admission.decision(),
        RunnerAdmissionDecisionV1::Supported { .. }
    ));
    Ok(())
}

#[test]
fn isolated_region_size_budget_receipt_can_authorize_exact_supported_projection(
) -> anyhow::Result<()> {
    exercise_supported_projection(
        MapManagedStimulusV1::RegionSizeBudget,
        MapModeV1::Extend,
        &format!(
            "{EXACT_TEST_PREFIX}isolated_region_size_budget_receipt_can_authorize_exact_supported_projection"
        ),
    )
}

#[test]
fn isolated_installed_map_receipt_can_authorize_exact_supported_projection() -> anyhow::Result<()> {
    exercise_supported_projection(
        MapManagedStimulusV1::RegionCountBudget,
        MapModeV1::Extend,
        &format!(
            "{EXACT_TEST_PREFIX}isolated_installed_map_receipt_can_authorize_exact_supported_projection"
        ),
    )
}

#[test]
fn isolated_logical_size_budget_receipt_can_authorize_exact_supported_projection(
) -> anyhow::Result<()> {
    exercise_supported_projection(
        MapManagedStimulusV1::LogicalSizeBudget,
        MapModeV1::Extend,
        &format!(
            "{EXACT_TEST_PREFIX}isolated_logical_size_budget_receipt_can_authorize_exact_supported_projection"
        ),
    )
}

#[test]
fn isolated_observe_region_size_budget_receipt_can_authorize_exact_supported_projection(
) -> anyhow::Result<()> {
    exercise_supported_projection(
        MapManagedStimulusV1::RegionSizeBudget,
        MapModeV1::Observe,
        &format!(
            "{EXACT_TEST_PREFIX}isolated_observe_region_size_budget_receipt_can_authorize_exact_supported_projection"
        ),
    )
}

#[test]
fn isolated_observe_region_count_budget_receipt_can_authorize_exact_supported_projection(
) -> anyhow::Result<()> {
    exercise_supported_projection(
        MapManagedStimulusV1::RegionCountBudget,
        MapModeV1::Observe,
        &format!(
            "{EXACT_TEST_PREFIX}isolated_observe_region_count_budget_receipt_can_authorize_exact_supported_projection"
        ),
    )
}

#[test]
fn isolated_observe_logical_size_budget_receipt_can_authorize_exact_supported_projection(
) -> anyhow::Result<()> {
    exercise_supported_projection(
        MapManagedStimulusV1::LogicalSizeBudget,
        MapModeV1::Observe,
        &format!(
            "{EXACT_TEST_PREFIX}isolated_observe_logical_size_budget_receipt_can_authorize_exact_supported_projection"
        ),
    )
}

#[test]
fn isolated_map_receipt_rejects_implementation_digest_tamper() -> anyhow::Result<()> {
    let leaf = request_budget_leaf_v1(MapManagedStimulusV1::RegionCountBudget, MapModeV1::Extend);
    let descriptor = request_budget_descriptor_v1(
        MapManagedStimulusV1::RegionCountBudget,
        MapModeV1::Extend,
        RunnerCapabilityV1::Supported,
    );
    let (key, member) =
        supported_key_and_member(MapManagedStimulusV1::RegionCountBudget, MapModeV1::Extend);
    let plan = compile_for_test(&key);
    let exact_test =
        format!("{EXACT_TEST_PREFIX}isolated_map_receipt_rejects_implementation_digest_tamper");
    let mut execution = match run_isolated_for_test(&exact_test, &key, member, plan)? {
        MapRunnerIsolatedOutcomeV1::ChildReported => return Ok(()),
        MapRunnerIsolatedOutcomeV1::ParentReceipt(receipt) => receipt,
    };
    tamper_implementation_digest_for_test(&mut execution, Digest32([0x7c; 32]));
    assert_eq!(
        project_validated_dynamic_terminal_with_map_execution_v1(
            &leaf.record,
            &descriptor,
            execution,
        ),
        Err(ProjectionErrorV1::Invalid(
            ProjectionViolationV1::RunnerAdmissionMapExecutionReceiptMismatch,
        ))
    );
    Ok(())
}

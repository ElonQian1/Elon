use super::super::runner_admission::{
    compile_for_test, run_isolated_for_test, tamper_implementation_digest_for_test,
    MapRunnerIsolatedOutcomeV1, RunnerAdmissionDecisionV1,
};
use super::*;

const EXACT_TEST_PREFIX: &str = "node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2b1_cases::denominator::static_contract::dynamic_quotient::tests::runner_admission_supported::";

fn region_count_budget_record() -> LeafRecordV1 {
    let mut value = record(
        "map-region-count-budget-supported",
        "region-index-exceeds-authority-budget",
    );
    let LeafOutcomeV1::Terminal(expected) = &mut value.outcome else {
        unreachable!()
    };
    expected.phase = "RequestValidation".to_owned();
    expected.raw_slots = CustodyStateV1::Unchanged;
    expected.route = CustodyStateV1::Unchanged;
    expected.callback = CustodyStateV1::Released;
    expected.file = CustodyStateV1::Retained;
    expected.counts.callback_begin = 1;
    expected.counts.callback_complete = 1;
    value
}

fn region_count_budget_descriptor(capability: RunnerCapabilityV1) -> TerminalDescriptorV1 {
    TerminalDescriptorV1::map(
        SourceSiteV1::ManagedRequestValidation,
        StimulusV1::MapManaged(MapManagedStimulusV1::RegionCountBudget),
        PrestateV1::Map(MapPrestateV1::NotReached),
        MapOperationV1::ManagedRequest,
        PhaseV1::RequestValidation,
        TimingV1::BeforeCall,
        OccurrenceV1::Natural,
        ExecutionRecipeV1::new(
            FixtureV1::ManagedWalMainSingleConnection,
            CallbackV1::XShmMap,
            FaultSeamV1::ManagedRequest,
            ObserverV1::MapCallbackAndSnapshot,
            CleanupV1::ParentOwnedRoot,
            capability,
        ),
        MapAxesV1 {
            mode: ReachabilityV1::Reached(MapModeV1::Extend),
            completion: ReachabilityV1::Reached(MapCompletionV1::Completed),
            ..MapAxesV1::NOT_REACHED
        },
    )
}

fn supported_key_and_member() -> (DynamicClassKeyV1, StaticMemberSealV1) {
    let record = region_count_budget_record();
    let descriptor = region_count_budget_descriptor(RunnerCapabilityV1::Missing(
        CapabilityGapV1::QuotientRunnerNotIntegrated,
    ));
    let validated = project_validated_dynamic_terminal_v1(&record, &descriptor).unwrap();
    let mut key = validated.semantic_key;
    key.recipe.capability = RunnerCapabilityV1::Supported;
    (key, validated.descriptor_binding.member)
}

#[test]
fn exact_supported_map_class_still_rejects_without_private_execution_receipt() {
    let record = region_count_budget_record();
    let descriptor = region_count_budget_descriptor(RunnerCapabilityV1::Supported);
    assert_eq!(
        project_dynamic_class_v1(&record, &descriptor),
        Err(ProjectionErrorV1::Invalid(
            ProjectionViolationV1::RunnerAdmissionUnsealedSupported,
        ))
    );
}

#[test]
fn supported_capability_is_limited_to_the_exact_programmed_map_class() {
    let record = region_count_budget_record();
    let mut descriptor = region_count_budget_descriptor(RunnerCapabilityV1::Supported);
    let TerminalDescriptorV1::Map(value) = &mut descriptor else {
        unreachable!()
    };
    value.stimulus = StimulusV1::MapManaged(MapManagedStimulusV1::LogicalSizeBudget);
    assert_eq!(
        project_dynamic_class_v1(&record, &descriptor),
        Err(ProjectionErrorV1::Invalid(
            ProjectionViolationV1::MapProducerRecipeMismatch,
        ))
    );
}

#[test]
fn exact_program_rejects_expected_semantics_drift_before_child_spawn() {
    let (mut key, member) = supported_key_and_member();
    key.expected.callback = CustodyStateV1::NotReached;
    let plan = compile_for_test(&key);
    assert!(
        run_isolated_for_test("must-not-spawn-for-invalid-map-program", &key, member, plan,)
            .is_err()
    );
}

#[test]
fn isolated_installed_map_receipt_can_authorize_exact_supported_projection() -> anyhow::Result<()> {
    let record = region_count_budget_record();
    let descriptor = region_count_budget_descriptor(RunnerCapabilityV1::Supported);
    let (key, member) = supported_key_and_member();
    let plan = compile_for_test(&key);
    let exact_test = format!(
        "{EXACT_TEST_PREFIX}isolated_installed_map_receipt_can_authorize_exact_supported_projection"
    );
    let execution = match run_isolated_for_test(&exact_test, &key, member, plan)? {
        MapRunnerIsolatedOutcomeV1::ChildReported => return Ok(()),
        MapRunnerIsolatedOutcomeV1::ParentReceipt(receipt) => receipt,
    };
    let validated =
        project_validated_dynamic_terminal_with_map_execution_v1(&record, &descriptor, execution)
            .map_err(|error| anyhow::anyhow!("supported Map projection failed: {error:?}"))?;
    assert!(validated.projection.is_ok());
    assert!(matches!(
        validated.runner_admission.decision(),
        RunnerAdmissionDecisionV1::Supported { .. }
    ));
    Ok(())
}

#[test]
fn isolated_map_receipt_rejects_implementation_digest_tamper() -> anyhow::Result<()> {
    let record = region_count_budget_record();
    let descriptor = region_count_budget_descriptor(RunnerCapabilityV1::Supported);
    let (key, member) = supported_key_and_member();
    let plan = compile_for_test(&key);
    let exact_test =
        format!("{EXACT_TEST_PREFIX}isolated_map_receipt_rejects_implementation_digest_tamper");
    let mut execution = match run_isolated_for_test(&exact_test, &key, member, plan)? {
        MapRunnerIsolatedOutcomeV1::ChildReported => return Ok(()),
        MapRunnerIsolatedOutcomeV1::ParentReceipt(receipt) => receipt,
    };
    tamper_implementation_digest_for_test(&mut execution, Digest32([0x7c; 32]));
    assert_eq!(
        project_validated_dynamic_terminal_with_map_execution_v1(&record, &descriptor, execution,),
        Err(ProjectionErrorV1::Invalid(
            ProjectionViolationV1::RunnerAdmissionMapExecutionReceiptMismatch,
        ))
    );
    Ok(())
}

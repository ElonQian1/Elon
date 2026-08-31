use super::super::runner_admission::ExecutionProgramInventoryStatusV1;
use super::*;

pub(super) fn region_count_budget_record() -> LeafRecordV1 {
    let mut value = record(
        "map-region-count-budget-program-inventory",
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

pub(super) fn budget_descriptor(
    stimulus: MapManagedStimulusV1,
    mode: MapModeV1,
    capability: RunnerCapabilityV1,
) -> TerminalDescriptorV1 {
    TerminalDescriptorV1::map(
        SourceSiteV1::ManagedRequestValidation,
        StimulusV1::MapManaged(stimulus),
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
            mode: ReachabilityV1::Reached(mode),
            completion: ReachabilityV1::Reached(MapCompletionV1::Completed),
            ..MapAxesV1::NOT_REACHED
        },
    )
}

#[test]
fn exact_source_program_is_inventoried_without_granting_supported() {
    let record = region_count_budget_record();
    for mode in [MapModeV1::Observe, MapModeV1::Extend] {
        let descriptor = budget_descriptor(
            MapManagedStimulusV1::RegionCountBudget,
            mode,
            RunnerCapabilityV1::Missing(CapabilityGapV1::QuotientRunnerNotIntegrated),
        );
        let prepared = prepare_dynamic_terminal_v1(&record, &descriptor).unwrap();
        let receipt = super::super::runner_admission::inventory_v1(&prepared.key).unwrap();
        assert_eq!(
            receipt.normalized_descriptor_sha256(),
            prepared.descriptor_binding.descriptor_semantic_sha256
        );
        assert_ne!(
            receipt.program_id(),
            prepared.descriptor_binding.descriptor_semantic_sha256
        );
        assert_eq!(
            receipt.program_id(),
            super::super::runner_admission::execution_program_id_v1(
                receipt.normalized_key().root,
                receipt.normalized_descriptor_sha256(),
                receipt.plan_sha256(),
            )
        );
        assert!(matches!(
            receipt.status(),
            ExecutionProgramInventoryStatusV1::SourcePresentReceiptRequired { .. }
        ));
        assert_eq!(
            receipt.normalized_key().recipe.capability,
            RunnerCapabilityV1::Missing(CapabilityGapV1::QuotientRunnerNotIntegrated)
        );
    }
    assert_eq!(
        project_dynamic_class_v1(
            &record,
            &budget_descriptor(
                MapManagedStimulusV1::RegionCountBudget,
                MapModeV1::Extend,
                RunnerCapabilityV1::Supported,
            ),
        ),
        Err(ProjectionErrorV1::Invalid(
            ProjectionViolationV1::RunnerAdmissionUnsealedSupported,
        ))
    );
}

#[test]
fn unimplemented_budget_program_stays_planned_missing() {
    let record = region_count_budget_record();
    let descriptor = budget_descriptor(
        MapManagedStimulusV1::LogicalSizeBudget,
        MapModeV1::Extend,
        RunnerCapabilityV1::Missing(CapabilityGapV1::QuotientRunnerNotIntegrated),
    );
    let prepared = prepare_dynamic_terminal_v1(&record, &descriptor).unwrap();
    let receipt = super::super::runner_admission::inventory_v1(&prepared.key).unwrap();
    assert_eq!(
        receipt.status(),
        ExecutionProgramInventoryStatusV1::PlannedMissing(
            CapabilityGapV1::QuotientRunnerNotIntegrated,
        )
    );
}

#[test]
fn full_map_program_inventory_accounts_for_every_frozen_member_without_opening_quotient() {
    let bundle =
        build_map_execution_program_inventory_v1(&super::super::super::map::graph()).unwrap();
    let inventory = &bundle.inventory;
    assert_eq!(inventory.member_count, 43_476);
    assert_eq!(bundle.reverse_index.len(), 43_476);
    assert_eq!(inventory.source_present_member_count, 2);
    assert_eq!(inventory.source_present_group_count, 2);
    assert_eq!(inventory.planned_missing_member_count, 43_474);
    assert_eq!(
        inventory
            .source_present_member_count
            .checked_add(inventory.planned_missing_member_count),
        Some(43_476),
    );
    assert_eq!(
        inventory
            .source_present_group_count
            .checked_add(inventory.planned_missing_group_count),
        Some(inventory.program_group_count),
    );
    assert!(inventory.source_present_group_count > 0);
    assert!(inventory.planned_missing_group_count > 0);
    assert_ne!(inventory.inventory_sha256, Digest32::ZERO);
    assert!(bundle.groups.iter().all(|group| !matches!(
        group.normalized_key.recipe.capability,
        RunnerCapabilityV1::Supported,
    )));
    let source_groups = bundle
        .groups
        .iter()
        .filter(|group| {
            matches!(
                group.status,
                ExecutionProgramInventoryStatusV1::SourcePresentReceiptRequired { .. }
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(source_groups.len(), 2);
    assert!(source_groups.iter().all(|group| group.member_count == 1));
    let record = region_count_budget_record();
    let expected_source_keys = [MapModeV1::Observe, MapModeV1::Extend].map(|mode| {
        let descriptor = budget_descriptor(
            MapManagedStimulusV1::RegionCountBudget,
            mode,
            RunnerCapabilityV1::Missing(CapabilityGapV1::QuotientRunnerNotIntegrated),
        );
        prepare_dynamic_terminal_v1(&record, &descriptor)
            .unwrap()
            .key
    });
    assert!(source_groups
        .iter()
        .all(|group| expected_source_keys.contains(&group.normalized_key)));
    assert!(expected_source_keys.iter().all(|expected| {
        source_groups
            .iter()
            .filter(|group| group.normalized_key == *expected)
            .count()
            == 1
    }));
    assert!(bundle.groups.iter().all(|group| match group.status {
        ExecutionProgramInventoryStatusV1::SourcePresentReceiptRequired { .. } => {
            expected_source_keys.contains(&group.normalized_key)
        }
        ExecutionProgramInventoryStatusV1::PlannedMissing(gap) => {
            gap == CapabilityGapV1::QuotientRunnerNotIntegrated
        }
    }));
}

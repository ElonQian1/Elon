use super::super::runner_admission::ExecutionProgramInventoryStatusV1;
use super::*;

pub(super) fn request_budget_record() -> LeafRecordV1 {
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

const REQUEST_BUDGET_STIMULI: [MapManagedStimulusV1; 3] = [
    MapManagedStimulusV1::RegionSizeBudget,
    MapManagedStimulusV1::RegionCountBudget,
    MapManagedStimulusV1::LogicalSizeBudget,
];

const LOCK_REQUEST_VALIDATION_PROGRAMS: [(LockActionV1, LockManagedStimulusV1); 10] = [
    (
        LockActionV1::LockShared,
        LockManagedStimulusV1::RangeOverflow,
    ),
    (
        LockActionV1::LockShared,
        LockManagedStimulusV1::EndPastEight,
    ),
    (
        LockActionV1::LockShared,
        LockManagedStimulusV1::SharedMultiSlot,
    ),
    (
        LockActionV1::LockExclusive,
        LockManagedStimulusV1::RangeOverflow,
    ),
    (
        LockActionV1::LockExclusive,
        LockManagedStimulusV1::EndPastEight,
    ),
    (
        LockActionV1::UnlockShared,
        LockManagedStimulusV1::RangeOverflow,
    ),
    (
        LockActionV1::UnlockShared,
        LockManagedStimulusV1::EndPastEight,
    ),
    (
        LockActionV1::UnlockShared,
        LockManagedStimulusV1::SharedMultiSlot,
    ),
    (
        LockActionV1::UnlockExclusive,
        LockManagedStimulusV1::RangeOverflow,
    ),
    (
        LockActionV1::UnlockExclusive,
        LockManagedStimulusV1::EndPastEight,
    ),
];

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

pub(super) fn lock_request_validation_record() -> LeafRecordV1 {
    let mut value = record(
        "lock-request-validation-program-inventory",
        "managed-request-rejected",
    );
    value.key.identity.root = RootOperationV1::Lock;
    let LeafOutcomeV1::Terminal(expected) = &mut value.outcome else {
        unreachable!()
    };
    expected.sqlite = SqliteResultV1::LockUnavailable;
    expected.phase = "RequestValidation".to_owned();
    expected.lock_effect = LockEffectV1::Unchanged;
    expected.raw_slots = CustodyStateV1::Unchanged;
    expected.file = CustodyStateV1::Unchanged;
    value
}

pub(super) fn lock_request_validation_descriptor(
    action: LockActionV1,
    stimulus: LockManagedStimulusV1,
    capability: RunnerCapabilityV1,
) -> TerminalDescriptorV1 {
    TerminalDescriptorV1::lock(
        SourceSiteV1::ManagedRequestValidation,
        StimulusV1::LockManaged(stimulus),
        PrestateV1::Lock(LockPrestateV1::NotReached),
        LockOperationV1::ManagedRequest,
        PhaseV1::RequestValidation,
        TimingV1::BeforeCall,
        OccurrenceV1::Natural,
        ExecutionRecipeV1::new(
            FixtureV1::ManagedWalMainSingleConnection,
            CallbackV1::XShmLock,
            FaultSeamV1::ManagedRequest,
            ObserverV1::LockCallbackAndSnapshot,
            CleanupV1::ParentOwnedRoot,
            capability,
        ),
        LockAxesV1 {
            action: ReachabilityV1::Reached(action),
            completion: ReachabilityV1::Reached(LockCompletionV1::Direct),
            ..LockAxesV1::NOT_REACHED
        },
    )
}

#[test]
fn exact_source_program_is_inventoried_without_granting_supported() {
    let record = request_budget_record();
    for stimulus in REQUEST_BUDGET_STIMULI {
        for mode in [MapModeV1::Observe, MapModeV1::Extend] {
            let descriptor = budget_descriptor(
                stimulus,
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
    }
    for stimulus in REQUEST_BUDGET_STIMULI {
        assert_eq!(
            project_dynamic_class_v1(
                &record,
                &budget_descriptor(stimulus, MapModeV1::Extend, RunnerCapabilityV1::Supported,),
            ),
            Err(ProjectionErrorV1::Invalid(
                ProjectionViolationV1::RunnerAdmissionUnsealedSupported,
            ))
        );
    }
}

#[test]
fn allocation_granularity_program_stays_planned_missing() {
    let mut record = request_budget_record();
    let LeafOutcomeV1::Terminal(expected) = &mut record.outcome else {
        unreachable!()
    };
    expected.failure = FailureClassV1::IoBeforeMutation;
    let descriptor = TerminalDescriptorV1::map(
        SourceSiteV1::ManagedRequestValidation,
        StimulusV1::MapManaged(MapManagedStimulusV1::AllocationGranularity),
        PrestateV1::Map(MapPrestateV1::NotReached),
        MapOperationV1::ManagedRequest,
        PhaseV1::RequestValidation,
        TimingV1::AtCall,
        OccurrenceV1::Natural,
        ExecutionRecipeV1::new(
            FixtureV1::ManagedWalMainSingleConnection,
            CallbackV1::XShmMap,
            FaultSeamV1::NativeOperation,
            ObserverV1::MapCallbackAndSnapshot,
            CleanupV1::ParentOwnedRoot,
            RunnerCapabilityV1::Missing(CapabilityGapV1::QuotientRunnerNotIntegrated),
        ),
        MapAxesV1 {
            mode: ReachabilityV1::Reached(MapModeV1::Extend),
            completion: ReachabilityV1::Reached(MapCompletionV1::Completed),
            ..MapAxesV1::NOT_REACHED
        },
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
    assert_eq!(inventory.source_present_member_count, 6);
    assert_eq!(inventory.source_present_group_count, 6);
    assert_eq!(inventory.planned_missing_member_count, 43_470);
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
    assert_eq!(source_groups.len(), 6);
    assert!(source_groups.iter().all(|group| group.member_count == 1));
    let record = request_budget_record();
    let expected_source_keys = REQUEST_BUDGET_STIMULI
        .into_iter()
        .flat_map(|stimulus| {
            [MapModeV1::Observe, MapModeV1::Extend].map(|mode| {
                let descriptor = budget_descriptor(
                    stimulus,
                    mode,
                    RunnerCapabilityV1::Missing(CapabilityGapV1::QuotientRunnerNotIntegrated),
                );
                prepare_dynamic_terminal_v1(&record, &descriptor)
                    .unwrap()
                    .key
            })
        })
        .collect::<Vec<_>>();
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

#[test]
fn exact_lock_request_validation_programs_are_inventoried_without_granting_supported() {
    let record = lock_request_validation_record();
    for (action, stimulus) in LOCK_REQUEST_VALIDATION_PROGRAMS {
        let descriptor = lock_request_validation_descriptor(
            action,
            stimulus,
            RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete),
        );
        let prepared = prepare_dynamic_terminal_v1(&record, &descriptor).unwrap();
        let receipt = super::super::runner_admission::inventory_v1(&prepared.key).unwrap();
        assert!(matches!(
            receipt.status(),
            ExecutionProgramInventoryStatusV1::SourcePresentReceiptRequired { .. }
        ));
        assert_eq!(
            receipt.normalized_key().recipe.capability,
            RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete),
        );
        assert_eq!(
            project_dynamic_class_v1(
                &record,
                &lock_request_validation_descriptor(
                    action,
                    stimulus,
                    RunnerCapabilityV1::Supported,
                ),
            ),
            Err(ProjectionErrorV1::Invalid(
                ProjectionViolationV1::RunnerAdmissionUnsealedSupported,
            )),
        );
    }
}

#[test]
fn full_lock_program_inventory_accounts_for_every_frozen_member_without_opening_quotient() {
    let bundle =
        build_lock_execution_program_inventory_v1(&super::super::super::lock::graph()).unwrap();
    let inventory = &bundle.inventory;
    assert_eq!(inventory.member_count, 8_668);
    assert_eq!(bundle.reverse_index.len(), 8_668);
    assert_eq!(inventory.source_present_member_count, 10);
    assert_eq!(inventory.source_present_group_count, 10);
    assert_eq!(inventory.planned_missing_member_count, 8_658);
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
    assert_eq!(source_groups.len(), 10);
    assert!(source_groups.iter().all(|group| group.member_count == 1));
    let record = lock_request_validation_record();
    let expected_source_keys = LOCK_REQUEST_VALIDATION_PROGRAMS
        .into_iter()
        .map(|(action, stimulus)| {
            prepare_dynamic_terminal_v1(
                &record,
                &lock_request_validation_descriptor(
                    action,
                    stimulus,
                    RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete),
                ),
            )
            .unwrap()
            .key
        })
        .collect::<Vec<_>>();
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
            gap == CapabilityGapV1::LockObservationIncomplete
        }
    }));
}

#[test]
fn inventory_builders_and_review_wrappers_reject_cross_root_replay() {
    let map_graph = super::super::super::map::graph();
    let lock_graph = super::super::super::lock::graph();
    assert!(build_map_execution_program_inventory_v1(&lock_graph).is_err());
    assert!(build_lock_execution_program_inventory_v1(&map_graph).is_err());

    let map_inventory = build_map_execution_program_inventory_v1(&map_graph).unwrap();
    let lock_inventory = build_lock_execution_program_inventory_v1(&lock_graph).unwrap();
    let map_binding =
        super::super::candidate::validate_frozen_pass(&map_graph, RootOperationV1::Map, |_| Ok(()))
            .unwrap();
    let lock_binding =
        super::super::candidate::validate_frozen_pass(&lock_graph, RootOperationV1::Lock, |_| {
            Ok(())
        })
        .unwrap();

    assert!(matches!(
        review_map_execution_program_inventory_v1(lock_inventory.clone(), &map_binding),
        Err(ProgramCatalogAdmissionErrorV1::RootMismatch)
    ));
    assert!(matches!(
        review_lock_execution_program_inventory_v1(map_inventory.clone(), &lock_binding),
        Err(ProgramCatalogAdmissionErrorV1::RootMismatch)
    ));
    assert!(matches!(
        review_lock_execution_program_inventory_v1(map_inventory, &map_binding),
        Err(ProgramCatalogAdmissionErrorV1::RootMismatch)
    ));
    assert!(matches!(
        review_map_execution_program_inventory_v1(lock_inventory, &lock_binding),
        Err(ProgramCatalogAdmissionErrorV1::RootMismatch)
    ));
}

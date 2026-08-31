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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LockLifecyclePathCaseV1 {
    NativeAcquire,
    NativeRelease,
    SharedLocalAcquire,
    SharedLocalRelease,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LockLifecycleProgramCaseV1 {
    pub(super) path: LockLifecyclePathCaseV1,
    pub(super) action: LockActionV1,
    pub(super) first: u8,
    pub(super) count: u8,
    pub(super) mask: u8,
}

pub(super) fn lock_lifecycle_cases() -> Vec<LockLifecycleProgramCaseV1> {
    let mut cases = Vec::with_capacity(104);
    for first in 0..8 {
        cases.push(lock_lifecycle_case(
            LockLifecyclePathCaseV1::NativeAcquire,
            LockActionV1::LockShared,
            first,
            1,
        ));
    }
    for first in 0..8 {
        for count in 1..=8 - first {
            cases.push(lock_lifecycle_case(
                LockLifecyclePathCaseV1::NativeAcquire,
                LockActionV1::LockExclusive,
                first,
                count,
            ));
        }
    }
    for first in 0..8 {
        cases.push(lock_lifecycle_case(
            LockLifecyclePathCaseV1::NativeRelease,
            LockActionV1::UnlockShared,
            first,
            1,
        ));
    }
    for first in 0..8 {
        for count in 1..=8 - first {
            cases.push(lock_lifecycle_case(
                LockLifecyclePathCaseV1::NativeRelease,
                LockActionV1::UnlockExclusive,
                first,
                count,
            ));
        }
    }
    for first in 0..8 {
        cases.push(lock_lifecycle_case(
            LockLifecyclePathCaseV1::SharedLocalAcquire,
            LockActionV1::LockShared,
            first,
            1,
        ));
        cases.push(lock_lifecycle_case(
            LockLifecyclePathCaseV1::SharedLocalRelease,
            LockActionV1::UnlockShared,
            first,
            1,
        ));
    }
    assert_eq!(cases.len(), 104);
    cases
}

const fn lock_lifecycle_case(
    path: LockLifecyclePathCaseV1,
    action: LockActionV1,
    first: u8,
    count: u8,
) -> LockLifecycleProgramCaseV1 {
    let mask = ((((1_u16 << count) - 1) << first) & 0xff) as u8;
    LockLifecycleProgramCaseV1 {
        path,
        action,
        first,
        count,
        mask,
    }
}

pub(super) fn lock_lifecycle_record(case: LockLifecycleProgramCaseV1) -> LeafRecordV1 {
    let label = format!(
        "lock-lifecycle-{:?}-{:?}-first{}-count{}",
        case.path, case.action, case.first, case.count
    );
    let mut value = record(&label, "completed");
    value.key.identity.root = RootOperationV1::Lock;
    let LeafOutcomeV1::Terminal(expected) = &mut value.outcome else {
        unreachable!()
    };
    let mode = if matches!(
        case.action,
        LockActionV1::LockShared | LockActionV1::UnlockShared
    ) {
        LockModeV1::Shared
    } else {
        LockModeV1::Exclusive
    };
    let native = matches!(
        case.path,
        LockLifecyclePathCaseV1::NativeAcquire | LockLifecyclePathCaseV1::NativeRelease
    );
    expected.sqlite = SqliteResultV1::Ok;
    expected.disposition = TerminalDispositionV1::Returned;
    expected.phase = "Success".to_owned();
    expected.failure = FailureClassV1::None;
    expected.mutation = MutationStateV1::Known;
    expected.lock_outcome_uncertain = false;
    expected.lock_effect = if matches!(
        case.path,
        LockLifecyclePathCaseV1::NativeAcquire | LockLifecyclePathCaseV1::SharedLocalAcquire
    ) {
        LockEffectV1::Acquired {
            mode,
            mask: case.mask,
            native,
        }
    } else {
        LockEffectV1::Released {
            mode,
            mask: case.mask,
            native,
        }
    };
    expected.dms_lock = DmsLockCustodyV1::ExistingShared;
    expected.raw_slots = CustodyStateV1::Unchanged;
    expected.route = CustodyStateV1::Unchanged;
    expected.callback = CustodyStateV1::Released;
    expected.file = CustodyStateV1::Unchanged;
    expected.mapping = CustodyStateV1::NotReached;
    expected.view = CustodyStateV1::NotReached;
    expected.payload = CustodyStateV1::NotReached;
    expected.counts = ObservableCountsV1 {
        callback_begin: 1,
        callback_complete: 1,
        native_lock: u16::from(case.path == LockLifecyclePathCaseV1::NativeAcquire),
        native_unlock: u16::from(case.path == LockLifecyclePathCaseV1::NativeRelease),
        ..ObservableCountsV1::default()
    };
    value
}

pub(super) fn lock_lifecycle_descriptor(
    case: LockLifecycleProgramCaseV1,
    capability: RunnerCapabilityV1,
) -> TerminalDescriptorV1 {
    let (source, stimulus, prestate, operation, timing, fixture, seam, initialization) =
        match case.path {
            LockLifecyclePathCaseV1::NativeAcquire => (
                SourceSiteV1::LockNativeAcquire,
                LockManagedStimulusV1::NativeAcquire,
                LockPrestateV1::NoHeldLocks,
                LockOperationV1::NativeAcquire,
                TimingV1::AfterSuccess,
                FixtureV1::ManagedWalMainSingleConnection,
                FaultSeamV1::NativeOperation,
                ReachabilityV1::Reached(InitializationProfileV1::NodeLive),
            ),
            LockLifecyclePathCaseV1::NativeRelease => (
                SourceSiteV1::LockNativeRelease,
                LockManagedStimulusV1::NativeRelease,
                if case.action == LockActionV1::UnlockShared {
                    LockPrestateV1::OwnSharedHeld
                } else {
                    LockPrestateV1::OwnExclusiveHeld
                },
                LockOperationV1::NativeRelease,
                TimingV1::AfterSuccess,
                FixtureV1::ManagedWalMainSingleConnection,
                FaultSeamV1::NativeOperation,
                ReachabilityV1::NotReached,
            ),
            LockLifecyclePathCaseV1::SharedLocalAcquire => (
                SourceSiteV1::LockLocalState,
                LockManagedStimulusV1::LocalState,
                LockPrestateV1::SiblingSharedCoalesced,
                LockOperationV1::LocalAcquire,
                TimingV1::Natural,
                FixtureV1::ManagedWalMainTwoConnections,
                FaultSeamV1::Natural,
                ReachabilityV1::NotReached,
            ),
            LockLifecyclePathCaseV1::SharedLocalRelease => (
                SourceSiteV1::LockLocalState,
                LockManagedStimulusV1::LocalState,
                LockPrestateV1::SiblingSharedCoalesced,
                LockOperationV1::LocalRelease,
                TimingV1::Natural,
                FixtureV1::ManagedWalMainTwoConnections,
                FaultSeamV1::Natural,
                ReachabilityV1::NotReached,
            ),
        };
    let (held_shared, held_exclusive, sibling_shared) = match case.path {
        LockLifecyclePathCaseV1::NativeAcquire => (0, 0, 0),
        LockLifecyclePathCaseV1::NativeRelease => {
            if case.action == LockActionV1::UnlockShared {
                (case.mask, 0, 0)
            } else {
                (0, case.mask, 0)
            }
        }
        LockLifecyclePathCaseV1::SharedLocalAcquire => (0, 0, case.mask),
        LockLifecyclePathCaseV1::SharedLocalRelease => (case.mask, 0, case.mask),
    };
    TerminalDescriptorV1::lock(
        source,
        StimulusV1::LockManaged(stimulus),
        PrestateV1::Lock(prestate),
        operation,
        PhaseV1::Success,
        timing,
        OccurrenceV1::Natural,
        ExecutionRecipeV1::new(
            fixture,
            CallbackV1::XShmLock,
            seam,
            ObserverV1::LockCallbackAndSnapshot,
            CleanupV1::ParentOwnedRoot,
            capability,
        ),
        LockAxesV1 {
            action: ReachabilityV1::Reached(case.action),
            first: ReachabilityV1::Reached(case.first),
            count: ReachabilityV1::Reached(case.count),
            mask: ReachabilityV1::Reached(case.mask),
            initialization,
            held_shared_mask: ReachabilityV1::Reached(held_shared),
            held_exclusive_mask: ReachabilityV1::Reached(held_exclusive),
            sibling_shared_mask: ReachabilityV1::Reached(sibling_shared),
            sibling_exclusive_mask: ReachabilityV1::Reached(0),
            completion: ReachabilityV1::Reached(LockCompletionV1::Completed),
        },
    )
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
fn exact_lock_lifecycle_programs_are_inventoried_without_granting_supported() {
    let cases = lock_lifecycle_cases();
    assert_eq!(cases.len(), 104);
    for case in cases {
        let record = lock_lifecycle_record(case);
        let descriptor = lock_lifecycle_descriptor(
            case,
            RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete),
        );
        let prepared = prepare_dynamic_terminal_v1(&record, &descriptor).unwrap();
        let receipt = super::super::runner_admission::inventory_v1(&prepared.key).unwrap();
        assert!(matches!(
            receipt.status(),
            ExecutionProgramInventoryStatusV1::SourcePresentReceiptRequired { .. }
        ));
        assert_eq!(
            project_dynamic_class_v1(
                &record,
                &lock_lifecycle_descriptor(case, RunnerCapabilityV1::Supported),
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
    assert_eq!(inventory.source_present_member_count, 114);
    assert_eq!(inventory.source_present_group_count, 114);
    assert_eq!(inventory.planned_missing_member_count, 8_554);
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
    assert_eq!(source_groups.len(), 114);
    assert!(source_groups.iter().all(|group| group.member_count == 1));
    let record = lock_request_validation_record();
    let mut expected_source_keys = LOCK_REQUEST_VALIDATION_PROGRAMS
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
    expected_source_keys.extend(lock_lifecycle_cases().into_iter().map(|case| {
        let record = lock_lifecycle_record(case);
        prepare_dynamic_terminal_v1(
            &record,
            &lock_lifecycle_descriptor(
                case,
                RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete),
            ),
        )
        .unwrap()
        .key
    }));
    assert_eq!(expected_source_keys.len(), 114);
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

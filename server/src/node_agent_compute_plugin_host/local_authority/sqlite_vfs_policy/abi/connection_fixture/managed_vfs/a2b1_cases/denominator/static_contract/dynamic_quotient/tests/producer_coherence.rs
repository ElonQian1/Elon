use super::super::super::{
    source_leaf_authority::{LeafOutcomeV1, LeafRecordV1, RootOperationV1, SqliteResultV1},
    terminal_descriptor::{
        CallbackV1, CapabilityGapV1, CleanupV1, ExecutionRecipeV1, FaultSeamV1, FixtureV1,
        InitializationFaultSiteV1, InitializationPathV1, InitializationProfileV1,
        InitializationStimulusV1, LockActionV1, LockAxesV1, LockCompletionV1,
        LockManagedStimulusV1, LockOperationV1, LockPrestateV1, MapAxesV1, MapCompletionV1,
        MapFilePathV1, MapManagedStimulusV1, MapModeV1, MapOperationV1, MapPrestateV1,
        MapProfileV1, MapRegionPrestateV1, MapRegionSizeArmV1, ObserverV1, OccurrenceV1, PhaseV1,
        PrestateV1, ReachabilityV1, RunnerCapabilityV1, SourceSiteV1, StimulusV1,
        TerminalDescriptorV1, TimingV1,
    },
};
use super::super::{project_dynamic_class_v1, ProjectionErrorV1, ProjectionViolationV1};

mod followup;

fn record(root: RootOperationV1, phase: PhaseV1) -> LeafRecordV1 {
    let mut value = super::record("producer-coherence", "typed-only");
    value.key.identity.root = root;
    let LeafOutcomeV1::Terminal(expected) = &mut value.outcome else {
        unreachable!()
    };
    expected.phase = phase.static_name().to_owned();
    if root == RootOperationV1::Lock {
        expected.sqlite = SqliteResultV1::LockUnavailable;
    }
    value
}

fn assert_missing(record: &LeafRecordV1, descriptor: &TerminalDescriptorV1) {
    assert!(matches!(
        project_dynamic_class_v1(record, descriptor),
        Err(ProjectionErrorV1::RunnerCapabilityMissing(_))
    ));
}

fn assert_invalid(
    record: &LeafRecordV1,
    descriptor: &TerminalDescriptorV1,
    violation: ProjectionViolationV1,
) {
    assert_eq!(
        project_dynamic_class_v1(record, descriptor),
        Err(ProjectionErrorV1::Invalid(violation))
    );
}

fn map_profile(mode: MapModeV1, file_path: MapFilePathV1) -> MapProfileV1 {
    MapProfileV1 {
        mode,
        initialization: InitializationProfileV1::NodeLive,
        prestate: MapRegionPrestateV1::Empty,
        region_size_arm: MapRegionSizeArmV1::Same,
        file_path,
        prior_mutation: false,
        preexisting_mapping: false,
    }
}

fn map_recipe(completion: MapCompletionV1) -> ExecutionRecipeV1 {
    let (observer, cleanup) = match completion {
        MapCompletionV1::Direct
        | MapCompletionV1::Completed
        | MapCompletionV1::RawDropCompleted => (
            ObserverV1::MapCallbackAndSnapshot,
            CleanupV1::ParentOwnedRoot,
        ),
        _ => (
            ObserverV1::CustodyAndCleanup,
            CleanupV1::RetainUnsafeCustodyThenParentCleanup,
        ),
    };
    ExecutionRecipeV1::new(
        FixtureV1::ManagedWalMainSingleConnection,
        CallbackV1::XShmMap,
        FaultSeamV1::NativeOperation,
        observer,
        cleanup,
        RunnerCapabilityV1::Missing(CapabilityGapV1::QuotientRunnerNotIntegrated),
    )
}

fn map_file_descriptor(grow: bool) -> TerminalDescriptorV1 {
    let completion = if grow {
        MapCompletionV1::UnsafeRetentionSucceededThenRouteUnknown
    } else {
        MapCompletionV1::Completed
    };
    let mode = if grow {
        MapModeV1::Extend
    } else {
        MapModeV1::Observe
    };
    let file_path = if grow {
        MapFilePathV1::GrowAttempted
    } else {
        MapFilePathV1::SizeSufficient
    };
    TerminalDescriptorV1::map(
        if grow {
            SourceSiteV1::MapFileGrow
        } else {
            SourceSiteV1::MapFileSize
        },
        StimulusV1::MapManaged(if grow {
            MapManagedStimulusV1::FileGrow
        } else {
            MapManagedStimulusV1::FileSize
        }),
        PrestateV1::Map(MapPrestateV1::RegionsEmpty),
        if grow {
            MapOperationV1::FileGrow
        } else {
            MapOperationV1::FileSize
        },
        if grow {
            PhaseV1::FileGrow
        } else {
            PhaseV1::FileSize
        },
        TimingV1::AtCall,
        OccurrenceV1::Natural,
        map_recipe(completion),
        MapAxesV1 {
            mode: ReachabilityV1::Reached(mode),
            profile: ReachabilityV1::Reached(map_profile(mode, file_path)),
            completion: ReachabilityV1::Reached(completion),
            ..MapAxesV1::NOT_REACHED
        },
    )
}

fn map_loop_descriptor() -> TerminalDescriptorV1 {
    let completion = MapCompletionV1::UnsafeRetentionSucceededThenRouteUnknown;
    let profile = map_profile(MapModeV1::Extend, MapFilePathV1::SizeSufficient);
    TerminalDescriptorV1::map(
        SourceSiteV1::MapMappingCreate,
        StimulusV1::MapManaged(MapManagedStimulusV1::MappingCreate),
        PrestateV1::Map(MapPrestateV1::RegionsEmpty),
        MapOperationV1::MappingCreate,
        PhaseV1::MappingCreate,
        TimingV1::AtCall,
        OccurrenceV1::Exact(2),
        map_recipe(completion),
        MapAxesV1 {
            mode: ReachabilityV1::Reached(MapModeV1::Extend),
            profile: ReachabilityV1::Reached(profile),
            ordinal: ReachabilityV1::Reached(2),
            regions_to_create: ReachabilityV1::Reached(2),
            completion: ReachabilityV1::Reached(completion),
        },
    )
}

fn lock_recipe(fixture: FixtureV1, cleanup: CleanupV1, seam: FaultSeamV1) -> ExecutionRecipeV1 {
    ExecutionRecipeV1::new(
        fixture,
        CallbackV1::XShmLock,
        seam,
        ObserverV1::LockCallbackAndSnapshot,
        cleanup,
        RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete),
    )
}

fn lock_axes(
    action: LockActionV1,
    first: u8,
    count: u8,
    masks: [u8; 4],
    completion: LockCompletionV1,
) -> LockAxesV1 {
    let mask = (((1_u16 << (first + count)) - (1_u16 << first)) as u8);
    LockAxesV1 {
        action: ReachabilityV1::Reached(action),
        first: ReachabilityV1::Reached(first),
        count: ReachabilityV1::Reached(count),
        mask: ReachabilityV1::Reached(mask),
        held_shared_mask: ReachabilityV1::Reached(masks[0]),
        held_exclusive_mask: ReachabilityV1::Reached(masks[1]),
        sibling_shared_mask: ReachabilityV1::Reached(masks[2]),
        sibling_exclusive_mask: ReachabilityV1::Reached(masks[3]),
        completion: ReachabilityV1::Reached(completion),
        ..LockAxesV1::NOT_REACHED
    }
}

fn lock_local_descriptor() -> TerminalDescriptorV1 {
    TerminalDescriptorV1::lock(
        SourceSiteV1::LockLocalState,
        StimulusV1::LockManaged(LockManagedStimulusV1::LocalState),
        PrestateV1::Lock(LockPrestateV1::SiblingExclusiveContention),
        LockOperationV1::LocalAcquire,
        PhaseV1::LockAcquire,
        TimingV1::Natural,
        OccurrenceV1::Natural,
        lock_recipe(
            FixtureV1::ManagedWalMainTwoConnections,
            CleanupV1::ParentOwnedRoot,
            FaultSeamV1::Natural,
        ),
        lock_axes(
            LockActionV1::LockShared,
            0,
            1,
            [0, 0, 0, 1],
            LockCompletionV1::Completed,
        ),
    )
}

fn lock_native_acquire_descriptor() -> TerminalDescriptorV1 {
    let mut axes = lock_axes(
        LockActionV1::LockExclusive,
        1,
        2,
        [0, 0, 0, 0],
        LockCompletionV1::Completed,
    );
    axes.initialization = ReachabilityV1::Reached(InitializationProfileV1::NodeLive);
    TerminalDescriptorV1::lock(
        SourceSiteV1::LockNativeAcquire,
        StimulusV1::LockManaged(LockManagedStimulusV1::NativeAcquire),
        PrestateV1::Lock(LockPrestateV1::NoHeldLocks),
        LockOperationV1::NativeAcquire,
        PhaseV1::Success,
        TimingV1::AfterSuccess,
        OccurrenceV1::Natural,
        lock_recipe(
            FixtureV1::ManagedWalMainSingleConnection,
            CleanupV1::ParentOwnedRoot,
            FaultSeamV1::NativeOperation,
        ),
        axes,
    )
}

fn lock_native_release_descriptor() -> TerminalDescriptorV1 {
    let completion = LockCompletionV1::UnsafeRetentionSucceededThenRouteUnknown;
    TerminalDescriptorV1::lock(
        SourceSiteV1::LockNativeRelease,
        StimulusV1::LockManaged(LockManagedStimulusV1::NativeRelease),
        PrestateV1::Lock(LockPrestateV1::OwnExclusiveHeld),
        LockOperationV1::NativeRelease,
        PhaseV1::LockRelease,
        TimingV1::AtCall,
        OccurrenceV1::Natural,
        lock_recipe(
            FixtureV1::ManagedWalMainSingleConnection,
            CleanupV1::RetainUnsafeCustodyThenParentCleanup,
            FaultSeamV1::NativeOperation,
        ),
        lock_axes(
            LockActionV1::UnlockExclusive,
            0,
            2,
            [0, 3, 0, 0],
            completion,
        ),
    )
}

#[test]
fn map_file_size_and_grow_are_not_interchangeable_before_capability_gate() {
    let mut size = map_file_descriptor(false);
    let size_record = record(RootOperationV1::Map, PhaseV1::FileSize);
    assert_missing(&size_record, &size);
    let TerminalDescriptorV1::Map(value) = &mut size else {
        unreachable!()
    };
    value.stimulus = StimulusV1::MapManaged(MapManagedStimulusV1::FileGrow);
    assert_invalid(
        &size_record,
        &size,
        ProjectionViolationV1::MapProducerTupleMismatch,
    );

    let mut grow = map_file_descriptor(true);
    let grow_record = record(RootOperationV1::Map, PhaseV1::FileGrow);
    assert_missing(&grow_record, &grow);
    let TerminalDescriptorV1::Map(value) = &mut grow else {
        unreachable!()
    };
    value.source_site = SourceSiteV1::MapFileSize;
    assert_invalid(
        &grow_record,
        &grow,
        ProjectionViolationV1::MapProducerTupleMismatch,
    );
}

#[test]
fn map_loop_occurrence_must_equal_typed_ordinal() {
    let mut descriptor = map_loop_descriptor();
    let record = record(RootOperationV1::Map, PhaseV1::MappingCreate);
    assert_missing(&record, &descriptor);
    let TerminalDescriptorV1::Map(value) = &mut descriptor else {
        unreachable!()
    };
    value.occurrence = OccurrenceV1::Exact(1);
    assert_invalid(
        &record,
        &descriptor,
        ProjectionViolationV1::MapProducerAxesMismatch,
    );
}

#[test]
fn map_unsafe_completion_recipe_rejects_each_single_axis_forgery() {
    let descriptor = map_file_descriptor(true);
    let record = record(RootOperationV1::Map, PhaseV1::FileGrow);
    for axis in 0..3 {
        let mut forged = descriptor;
        let TerminalDescriptorV1::Map(value) = &mut forged else {
            unreachable!()
        };
        match axis {
            0 => value.axes.completion = ReachabilityV1::Reached(MapCompletionV1::Completed),
            1 => value.recipe.observer = ObserverV1::MapCallbackAndSnapshot,
            _ => value.recipe.cleanup = CleanupV1::ParentOwnedRoot,
        }
        assert_invalid(
            &record,
            &forged,
            ProjectionViolationV1::MapProducerRecipeMismatch,
        );
    }
}

#[test]
fn lock_local_tuple_fixture_and_masks_are_closed() {
    let descriptor = lock_local_descriptor();
    let record = record(RootOperationV1::Lock, PhaseV1::LockAcquire);
    assert_missing(&record, &descriptor);

    let mut wrong_tuple = descriptor;
    let TerminalDescriptorV1::Lock(value) = &mut wrong_tuple else {
        unreachable!()
    };
    value.operation = LockOperationV1::NativeAcquire;
    assert_invalid(
        &record,
        &wrong_tuple,
        ProjectionViolationV1::LockProducerTupleMismatch,
    );

    let mut wrong_fixture = descriptor;
    let TerminalDescriptorV1::Lock(value) = &mut wrong_fixture else {
        unreachable!()
    };
    value.recipe.fixture = FixtureV1::ManagedWalMainSingleConnection;
    assert_invalid(
        &record,
        &wrong_fixture,
        ProjectionViolationV1::LockProducerRecipeMismatch,
    );

    let mut wrong_mask = descriptor;
    let TerminalDescriptorV1::Lock(value) = &mut wrong_mask else {
        unreachable!()
    };
    value.axes.sibling_exclusive_mask = ReachabilityV1::Reached(0);
    assert_invalid(
        &record,
        &wrong_mask,
        ProjectionViolationV1::LockProducerAxesMismatch,
    );
}

#[test]
fn lock_native_acquire_requires_its_initialization_axis() {
    let mut descriptor = lock_native_acquire_descriptor();
    let record = record(RootOperationV1::Lock, PhaseV1::Success);
    assert_missing(&record, &descriptor);
    let TerminalDescriptorV1::Lock(value) = &mut descriptor else {
        unreachable!()
    };
    value.axes.initialization = ReachabilityV1::NotReached;
    assert_invalid(
        &record,
        &descriptor,
        ProjectionViolationV1::LockProducerAxesMismatch,
    );
}

#[test]
fn lock_unsafe_completion_keeps_lock_observer_and_retain_cleanup() {
    let descriptor = lock_native_release_descriptor();
    let record = record(RootOperationV1::Lock, PhaseV1::LockRelease);
    assert_missing(&record, &descriptor);
    for axis in 0..3 {
        let mut forged = descriptor;
        let TerminalDescriptorV1::Lock(value) = &mut forged else {
            unreachable!()
        };
        match axis {
            0 => value.axes.completion = ReachabilityV1::Reached(LockCompletionV1::Completed),
            1 => value.recipe.observer = ObserverV1::CustodyAndCleanup,
            _ => value.recipe.cleanup = CleanupV1::ParentOwnedRoot,
        }
        assert_invalid(
            &record,
            &forged,
            ProjectionViolationV1::LockProducerRecipeMismatch,
        );
    }
}

#[test]
fn lock_initialization_fault_site_cannot_cross_source_family() {
    let completion = LockCompletionV1::UnsafeRetentionSucceededThenRouteUnknown;
    let descriptor = TerminalDescriptorV1::lock(
        SourceSiteV1::InitializationOpen,
        StimulusV1::Initialization(InitializationStimulusV1 {
            fault_site: InitializationFaultSiteV1::ParentHandle,
            path: InitializationPathV1::Opening,
            cleanup_rewrite: false,
        }),
        PrestateV1::Lock(LockPrestateV1::NoHeldLocks),
        LockOperationV1::Initialization,
        PhaseV1::ExactSiblingOpen,
        TimingV1::AtCall,
        OccurrenceV1::Natural,
        lock_recipe(
            FixtureV1::ManagedWalMainSingleConnection,
            CleanupV1::RetainUnsafeCustodyThenParentCleanup,
            FaultSeamV1::Initialization,
        ),
        lock_axes(LockActionV1::LockShared, 0, 1, [0, 0, 0, 0], completion),
    );
    let record = record(RootOperationV1::Lock, PhaseV1::ExactSiblingOpen);
    assert_missing(&record, &descriptor);
    let mut forged = descriptor;
    let TerminalDescriptorV1::Lock(value) = &mut forged else {
        unreachable!()
    };
    value.source_site = SourceSiteV1::InitializationDms;
    assert_invalid(
        &record,
        &forged,
        ProjectionViolationV1::LockProducerTupleMismatch,
    );
}

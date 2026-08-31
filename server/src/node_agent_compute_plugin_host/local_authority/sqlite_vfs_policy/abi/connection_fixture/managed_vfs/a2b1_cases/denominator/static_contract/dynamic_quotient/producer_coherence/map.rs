use super::super::super::terminal_descriptor::{
    CallbackV1, CleanupV1, FaultSeamV1, FixtureV1, InitializationFaultSiteV1, InitializationPathV1,
    InitializationProfileV1, MapAxesV1, MapCompletionV1, MapFilePathV1, MapManagedStimulusV1,
    MapOperationV1, MapPrestateV1, MapRegionPrestateV1, MapTerminalDescriptorV1, ObserverV1,
    OccurrenceV1, PhaseV1, PresenceV1, PrestateV1, RawStateV1, ReachabilityV1, RunnerCapabilityV1,
    SourceSiteV1, StimulusV1, TimingV1, ValidityV1,
};
use super::super::projector::{ProjectionErrorV1, ProjectionViolationV1};
use super::{invalid, valid_initialization_tuple, valid_stored_poison_phase};

#[derive(Clone, Copy)]
enum AxesShape {
    Early,
    Mode,
    Profile,
    Loop,
    ProfileOrLoop,
}

pub(super) fn validate(value: MapTerminalDescriptorV1) -> Result<(), ProjectionErrorV1> {
    validate_recipe(value)?;
    super::map_axes::validate(value)?;
    if !valid_tuple(value) {
        return Err(invalid(ProjectionViolationV1::MapProducerTupleMismatch));
    }
    if !valid_completion(value) {
        return Err(invalid(ProjectionViolationV1::MapProducerAxesMismatch));
    }
    Ok(())
}

fn validate_recipe(value: MapTerminalDescriptorV1) -> Result<(), ProjectionErrorV1> {
    let ReachabilityV1::Reached(completion) = value.axes.completion else {
        return Err(invalid(ProjectionViolationV1::MapProducerAxesMismatch));
    };
    let expected_fixture = if matches!(value.axes.mode, ReachabilityV1::Reached(_)) {
        FixtureV1::ManagedWalMainSingleConnection
    } else {
        FixtureV1::AbiRawOnly
    };
    let (observer, cleanup) = match completion {
        MapCompletionV1::Direct
        | MapCompletionV1::Completed
        | MapCompletionV1::RawDropCompleted => (
            ObserverV1::MapCallbackAndSnapshot,
            CleanupV1::ParentOwnedRoot,
        ),
        MapCompletionV1::RouteUnknown
        | MapCompletionV1::UnsafeRetentionSucceededThenRouteUnknown
        | MapCompletionV1::UnsafeRetentionRouteUnknownThenRouteUnknown
        | MapCompletionV1::RawDropUnwindCaught => (
            ObserverV1::CustodyAndCleanup,
            CleanupV1::RetainUnsafeCustodyThenParentCleanup,
        ),
    };
    if value.recipe.fixture == expected_fixture
        && value.recipe.callback == CallbackV1::XShmMap
        && value.recipe.observer == observer
        && value.recipe.cleanup == cleanup
        && valid_map_capability(value)
    {
        Ok(())
    } else {
        Err(invalid(ProjectionViolationV1::MapProducerRecipeMismatch))
    }
}

fn valid_map_capability(value: MapTerminalDescriptorV1) -> bool {
    super::valid_map_capability(value.recipe)
        || (value.recipe.capability == RunnerCapabilityV1::Supported
            && value.stimulus == StimulusV1::MapManaged(MapManagedStimulusV1::RegionCountBudget)
            && value.axes.completion == ReachabilityV1::Reached(MapCompletionV1::Completed))
}

fn valid_tuple(value: MapTerminalDescriptorV1) -> bool {
    let PrestateV1::Map(prestate) = value.prestate else {
        return false;
    };
    let seam = value.recipe.fault_seam;
    match value.stimulus {
        StimulusV1::MapAbi(scalar) => {
            value.source_site == SourceSiteV1::MapAbiBoundary
                && prestate == MapPrestateV1::NotReached
                && value.operation == MapOperationV1::AbiValidation
                && value.phase == PhaseV1::AbiValidation
                && value.timing == TimingV1::BeforeCall
                && seam == FaultSeamV1::AbiBoundary
                && shape(value.axes, AxesShape::Early)
                && (scalar.output == PresenceV1::Absent
                    || scalar.region == ValidityV1::Invalid
                    || scalar.region_size == ValidityV1::Invalid
                    || scalar.extend == ValidityV1::Invalid)
        }
        StimulusV1::MapRaw(raw) => valid_raw_tuple(value, prestate, seam, raw),
        StimulusV1::Initialization(stimulus) => {
            prestate == MapPrestateV1::NodeAbsent
                && value.operation == MapOperationV1::Initialization
                && seam == FaultSeamV1::Initialization
                && shape(value.axes, AxesShape::Mode)
                && valid_initialization_tuple(
                    value.source_site,
                    stimulus,
                    value.phase,
                    value.timing,
                )
        }
        StimulusV1::MapManaged(stimulus) => valid_managed_tuple(value, prestate, seam, stimulus),
        StimulusV1::LockAbi(_) | StimulusV1::LockRaw(_) | StimulusV1::LockManaged(_) => false,
    }
}

fn valid_raw_tuple(
    value: MapTerminalDescriptorV1,
    prestate: MapPrestateV1,
    seam: FaultSeamV1,
    raw: RawStateV1,
) -> bool {
    let common = prestate == MapPrestateV1::NotReached
        && value.occurrence == OccurrenceV1::Natural
        && seam == FaultSeamV1::RawState
        && shape(value.axes, AxesShape::Early);
    common
        && ((raw == RawStateV1::HandleBoundFileMissing
            && value.source_site == SourceSiteV1::AdapterDispatch
            && value.operation == MapOperationV1::AdapterDispatch
            && value.phase == PhaseV1::Adapter
            && value.timing == TimingV1::AtCall)
            || (raw != RawStateV1::HandleBoundFileMissing
                && !matches!(
                    raw,
                    RawStateV1::DropCompleted | RawStateV1::DropUnwindCaught
                )
                && value.source_site == SourceSiteV1::RawStateAbandon
                && value.operation == MapOperationV1::RawAbandon
                && value.phase == PhaseV1::RawAdmission
                && value.timing == TimingV1::Cleanup))
}

fn valid_managed_tuple(
    value: MapTerminalDescriptorV1,
    prestate: MapPrestateV1,
    seam: FaultSeamV1,
    stimulus: MapManagedStimulusV1,
) -> bool {
    use MapManagedStimulusV1 as S;
    match stimulus {
        S::CallbackRouteUnknownPriorQuarantine
        | S::CallbackCounterOverflow
        | S::CallbackUnsupportedFileRole
        | S::CallbackShmDetached => managed_exact(
            value,
            prestate,
            seam,
            SourceSiteV1::RegistryCallbackAdmission,
            MapPrestateV1::NotReached,
            MapOperationV1::CallbackAdmission,
            PhaseV1::CallbackAdmission,
            TimingV1::AtCall,
            FaultSeamV1::RegistryAdmission,
            AxesShape::Mode,
        ),
        S::RegionSizeBudget | S::RegionCountBudget | S::LogicalSizeBudget => managed_exact(
            value,
            prestate,
            seam,
            SourceSiteV1::ManagedRequestValidation,
            MapPrestateV1::NotReached,
            MapOperationV1::ManagedRequest,
            PhaseV1::RequestValidation,
            TimingV1::BeforeCall,
            FaultSeamV1::ManagedRequest,
            AxesShape::Mode,
        ),
        S::AllocationGranularity => managed_exact(
            value,
            prestate,
            seam,
            SourceSiteV1::ManagedRequestValidation,
            MapPrestateV1::NotReached,
            MapOperationV1::ManagedRequest,
            PhaseV1::RequestValidation,
            TimingV1::AtCall,
            FaultSeamV1::NativeOperation,
            AxesShape::Mode,
        ),
        S::StoredPoison => {
            value.source_site == SourceSiteV1::CoordinatorState
                && matches!(prestate, MapPrestateV1::StoredPoison(_))
                && value.operation == MapOperationV1::ManagedRequest
                && valid_stored_poison_phase(value.phase)
                && value.timing == TimingV1::BeforeCall
                && seam == FaultSeamV1::ManagedRequest
                && shape(value.axes, AxesShape::Mode)
        }
        S::RegionSize => {
            value.source_site == SourceSiteV1::ManagedRequestValidation
                && value.operation == MapOperationV1::ManagedRequest
                && value.phase == PhaseV1::RequestValidation
                && value.timing == TimingV1::BeforeCall
                && seam == FaultSeamV1::ManagedRequest
                && profile_prestate(value.axes, prestate)
                && shape(value.axes, AxesShape::Profile)
        }
        S::FileSize => {
            value.source_site == SourceSiteV1::MapFileSize
                && value.operation == MapOperationV1::FileSize
                && profile_prestate(value.axes, prestate)
                && shape(value.axes, AxesShape::Profile)
                && ((value.phase == PhaseV1::RequestValidation
                    && value.timing == TimingV1::AfterSuccess
                    && seam == FaultSeamV1::ManagedRequest)
                    || (value.phase == PhaseV1::FileSize
                        && value.timing == TimingV1::AtCall
                        && seam == FaultSeamV1::NativeOperation))
        }
        S::FileGrow => {
            value.source_site == SourceSiteV1::MapFileGrow
                && value.operation == MapOperationV1::FileGrow
                && value.phase == PhaseV1::FileGrow
                && value.timing == TimingV1::AtCall
                && seam == FaultSeamV1::NativeOperation
                && profile_prestate(value.axes, prestate)
                && shape(value.axes, AxesShape::Profile)
        }
        S::MappingCreate | S::ViewMap | S::MappingClose => {
            valid_loop_tuple(value, prestate, seam, stimulus)
        }
        S::Success => valid_success_tuple(value, prestate, seam),
        S::Initialization | S::RegionLoop => false,
    }
}

#[allow(clippy::too_many_arguments)]
fn managed_exact(
    value: MapTerminalDescriptorV1,
    prestate: MapPrestateV1,
    seam: FaultSeamV1,
    source: SourceSiteV1,
    expected_prestate: MapPrestateV1,
    operation: MapOperationV1,
    phase: PhaseV1,
    timing: TimingV1,
    expected_seam: FaultSeamV1,
    axes: AxesShape,
) -> bool {
    value.source_site == source
        && prestate == expected_prestate
        && value.operation == operation
        && value.phase == phase
        && value.timing == timing
        && seam == expected_seam
        && shape(value.axes, axes)
}

fn valid_loop_tuple(
    value: MapTerminalDescriptorV1,
    prestate: MapPrestateV1,
    seam: FaultSeamV1,
    stimulus: MapManagedStimulusV1,
) -> bool {
    let (source, operation, phase, timing, expected_seam) = match stimulus {
        MapManagedStimulusV1::MappingCreate => (
            SourceSiteV1::MapMappingCreate,
            MapOperationV1::MappingCreate,
            PhaseV1::MappingCreate,
            TimingV1::AtCall,
            FaultSeamV1::NativeOperation,
        ),
        MapManagedStimulusV1::ViewMap => (
            SourceSiteV1::MapViewMap,
            MapOperationV1::ViewMap,
            PhaseV1::ViewMap,
            TimingV1::AtCall,
            FaultSeamV1::NativeOperation,
        ),
        MapManagedStimulusV1::MappingClose => (
            SourceSiteV1::MapMappingClose,
            MapOperationV1::MappingClose,
            PhaseV1::MappingClose,
            TimingV1::Cleanup,
            FaultSeamV1::Cleanup,
        ),
        _ => return false,
    };
    value.source_site == source
        && value.operation == operation
        && value.phase == phase
        && value.timing == timing
        && seam == expected_seam
        && profile_prestate(value.axes, prestate)
        && shape(value.axes, AxesShape::Loop)
}

fn valid_success_tuple(
    value: MapTerminalDescriptorV1,
    prestate: MapPrestateV1,
    seam: FaultSeamV1,
) -> bool {
    if !profile_prestate(value.axes, prestate) {
        return false;
    }
    if value.source_site == SourceSiteV1::CallbackCompletion {
        return value.operation == MapOperationV1::CallbackCompletion
            && value.phase == PhaseV1::CallbackCompletion
            && value.timing == TimingV1::AfterSuccess
            && seam == FaultSeamV1::CallbackCompletion
            && shape(value.axes, AxesShape::ProfileOrLoop);
    }
    let profile = match value.axes.profile {
        ReachabilityV1::Reached(value) => value,
        _ => return false,
    };
    value.operation == MapOperationV1::SuccessProjection
        && value.phase == PhaseV1::Success
        && value.timing == TimingV1::Natural
        && seam == FaultSeamV1::Natural
        && ((value.source_site == SourceSiteV1::MapFileSize
            && profile.file_path == MapFilePathV1::ObserveNotPresent
            && shape(value.axes, AxesShape::Profile))
            || (value.source_site == SourceSiteV1::CoordinatorState
                && profile.prestate == MapRegionPrestateV1::Reuse
                && shape(value.axes, AxesShape::Profile))
            || (value.source_site == SourceSiteV1::AbiProjection
                && shape(value.axes, AxesShape::Loop)))
}

fn profile_prestate(axes: MapAxesV1, prestate: MapPrestateV1) -> bool {
    match axes.profile {
        ReachabilityV1::Reached(profile) => match profile.prestate {
            MapRegionPrestateV1::Empty => prestate == MapPrestateV1::RegionsEmpty,
            MapRegionPrestateV1::NonemptyTargetMissing => prestate == MapPrestateV1::TargetMissing,
            MapRegionPrestateV1::Reuse => prestate == MapPrestateV1::TargetMapped,
            MapRegionPrestateV1::ObserveNotPresent => false,
        },
        ReachabilityV1::NotReached => false,
    }
}

fn shape(axes: MapAxesV1, expected: AxesShape) -> bool {
    let mode = matches!(axes.mode, ReachabilityV1::Reached(_));
    let profile = matches!(axes.profile, ReachabilityV1::Reached(_));
    let ordinal = matches!(axes.ordinal, ReachabilityV1::Reached(_));
    let regions = matches!(axes.regions_to_create, ReachabilityV1::Reached(_));
    match expected {
        AxesShape::Early => !mode && !profile && !ordinal && !regions,
        AxesShape::Mode => mode && !profile && !ordinal && !regions,
        AxesShape::Profile => mode && profile && !ordinal && !regions,
        AxesShape::Loop => mode && profile && ordinal && regions,
        AxesShape::ProfileOrLoop => mode && profile && ordinal == regions,
    }
}

fn valid_completion(value: MapTerminalDescriptorV1) -> bool {
    let ReachabilityV1::Reached(completion) = value.axes.completion else {
        return false;
    };
    let safe = matches!(
        completion,
        MapCompletionV1::Completed | MapCompletionV1::RouteUnknown
    );
    let unsafe_path = matches!(
        completion,
        MapCompletionV1::UnsafeRetentionSucceededThenRouteUnknown
            | MapCompletionV1::UnsafeRetentionRouteUnknownThenRouteUnknown
    );
    match value.stimulus {
        StimulusV1::MapAbi(_) => completion == MapCompletionV1::Direct,
        StimulusV1::MapRaw(raw) => match raw {
            RawStateV1::NullFile
            | RawStateV1::Uninstalled
            | RawStateV1::MethodsNullStatePresent
            | RawStateV1::ForeignMethodsStateNull
            | RawStateV1::ForeignMethodsStatePresent
            | RawStateV1::ExactMethodsStateNull
            | RawStateV1::HandleBoundFileMissing => completion == MapCompletionV1::Direct,
            RawStateV1::OtherTypePayloadMissing | RawStateV1::ExpectedTypePayloadMissing => {
                completion == MapCompletionV1::RawDropCompleted
            }
            RawStateV1::OtherTypePayloadPresent => matches!(
                completion,
                MapCompletionV1::RawDropCompleted | MapCompletionV1::RawDropUnwindCaught
            ),
            RawStateV1::DropCompleted | RawStateV1::DropUnwindCaught => false,
        },
        StimulusV1::Initialization(stimulus) => {
            let may_be_safe = !stimulus.cleanup_rewrite
                && matches!(
                    (stimulus.fault_site, stimulus.path),
                    (
                        InitializationFaultSiteV1::DmsExclusiveAcquire,
                        InitializationPathV1::Existing
                    ) | (
                        InitializationFaultSiteV1::DmsSharedAcquire,
                        InitializationPathV1::ExistingJoiner
                    )
                );
            unsafe_path || (may_be_safe && safe)
        }
        StimulusV1::MapManaged(stimulus) => match stimulus {
            MapManagedStimulusV1::CallbackRouteUnknownPriorQuarantine
            | MapManagedStimulusV1::CallbackCounterOverflow => {
                completion == MapCompletionV1::Direct
            }
            MapManagedStimulusV1::CallbackUnsupportedFileRole
            | MapManagedStimulusV1::CallbackShmDetached
            | MapManagedStimulusV1::RegionSizeBudget
            | MapManagedStimulusV1::RegionCountBudget
            | MapManagedStimulusV1::LogicalSizeBudget
            | MapManagedStimulusV1::AllocationGranularity => safe,
            MapManagedStimulusV1::StoredPoison
            | MapManagedStimulusV1::FileGrow
            | MapManagedStimulusV1::MappingClose => unsafe_path,
            MapManagedStimulusV1::RegionSize => safe || unsafe_path,
            MapManagedStimulusV1::FileSize => file_size_completion(value, safe, unsafe_path),
            MapManagedStimulusV1::MappingCreate | MapManagedStimulusV1::ViewMap => {
                let (ReachabilityV1::Reached(profile), ReachabilityV1::Reached(ordinal)) =
                    (value.axes.profile, value.axes.ordinal)
                else {
                    return false;
                };
                if ordinal == 1 && !profile.prior_mutation {
                    safe
                } else {
                    unsafe_path
                }
            }
            MapManagedStimulusV1::Success => {
                if value.operation == MapOperationV1::CallbackCompletion {
                    completion == MapCompletionV1::RouteUnknown
                } else {
                    completion == MapCompletionV1::Completed
                }
            }
            MapManagedStimulusV1::Initialization | MapManagedStimulusV1::RegionLoop => false,
        },
        StimulusV1::LockAbi(_) | StimulusV1::LockRaw(_) | StimulusV1::LockManaged(_) => false,
    }
}

fn file_size_completion(value: MapTerminalDescriptorV1, safe: bool, unsafe_path: bool) -> bool {
    if value.phase == PhaseV1::RequestValidation {
        return safe;
    }
    let ReachabilityV1::Reached(profile) = value.axes.profile else {
        return false;
    };
    if matches!(
        profile.initialization,
        InitializationProfileV1::NodeLive | InitializationProfileV1::ExistingJoinerShared
    ) {
        safe
    } else {
        unsafe_path
    }
}

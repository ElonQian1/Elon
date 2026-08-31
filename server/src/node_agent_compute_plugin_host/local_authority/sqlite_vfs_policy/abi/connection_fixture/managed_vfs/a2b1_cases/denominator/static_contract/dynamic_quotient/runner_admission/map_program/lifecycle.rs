//! Source-bound specification for six exact positive Map single-region lifecycle programs.
//!
//! The matcher consumes only typed descriptor fields, the exact observable vector and frozen
//! member seals. Leaf ids, test selectors and fixture names are not admission inputs.

mod source_scope;

use super::super::super::super::{
    source_leaf_authority::{
        CustodyStateV1, Digest32, DmsLockCustodyV1, FailureClassV1, LockEffectV1, MutationStateV1,
        ObservableCountsV1, RootOperationV1, SqliteResultV1, TerminalDispositionV1,
    },
    terminal_descriptor::{
        CallbackV1, CleanupV1, FaultSeamV1, FixtureV1, InitializationProfileV1, MapCompletionV1,
        MapFilePathV1, MapManagedStimulusV1, MapModeV1, MapOperationV1, MapPrestateV1,
        MapProfileV1, MapRegionPrestateV1, MapRegionSizeArmV1, ObserverV1, OccurrenceV1, PhaseV1,
        PrestateV1, ReachabilityV1, SourceSiteV1, StimulusV1, TimingV1,
    },
};
use super::super::super::{
    DynamicAxesV1, DynamicClassKeyV1, DynamicExpectedV1, DynamicOperationV1, StaticMemberSealV1,
    DYNAMIC_PROJECTOR_SCHEMA_V1,
};
use super::super::CompiledRunnerPlanV1;
use super::{MapProgramCaseV1, MapProgramSpecV1, MapRunnerExecutionViolationV1, ProgramModeV1};
use source_scope::digest_implementation_v1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MapLifecyclePathSpecV1 {
    EmptyObserveNotPresent,
    EmptyExtendMapped,
    ReuseObserveMapped,
    ReuseExtendMapped,
    MissingObserveNotPresent,
    MissingExtendMapped,
}

impl MapLifecyclePathSpecV1 {
    const fn mode(self) -> ProgramModeV1 {
        match self {
            Self::EmptyObserveNotPresent
            | Self::ReuseObserveMapped
            | Self::MissingObserveNotPresent => ProgramModeV1::Observe,
            Self::EmptyExtendMapped | Self::ReuseExtendMapped | Self::MissingExtendMapped => {
                ProgramModeV1::Extend
            }
        }
    }

    pub(super) const fn implementation_tag(self) -> u8 {
        match self {
            Self::EmptyObserveNotPresent => 1,
            Self::EmptyExtendMapped => 2,
            Self::ReuseObserveMapped => 3,
            Self::ReuseExtendMapped => 4,
            Self::MissingObserveNotPresent => 5,
            Self::MissingExtendMapped => 6,
        }
    }
}

pub(super) fn program_spec_v1(
    key: &DynamicClassKeyV1,
    plan: CompiledRunnerPlanV1,
) -> Result<MapProgramSpecV1, MapRunnerExecutionViolationV1> {
    if plan != super::super::compile_v1(key) {
        return Err(MapRunnerExecutionViolationV1::PlanBindingMismatch);
    }
    let Some(path) = classify_path_v1(key) else {
        return Err(MapRunnerExecutionViolationV1::UnsupportedProgram);
    };
    if key.expected != expected_v1(path) {
        return Err(MapRunnerExecutionViolationV1::UnsupportedProgram);
    }
    Ok(MapProgramSpecV1 {
        mode: path.mode(),
        case: MapProgramCaseV1::Lifecycle(path),
        member: member_v1(path),
        normalized_descriptor_sha256: plan.normalized_descriptor_sha256,
        plan_sha256: plan.plan_sha256,
        implementation_sha256: digest_implementation_v1(path),
    })
}

fn classify_path_v1(key: &DynamicClassKeyV1) -> Option<MapLifecyclePathSpecV1> {
    if key.schema_version != DYNAMIC_PROJECTOR_SCHEMA_V1
        || key.root != RootOperationV1::Map
        || key.stimulus != StimulusV1::MapManaged(MapManagedStimulusV1::Success)
        || key.operation != DynamicOperationV1::Map(MapOperationV1::SuccessProjection)
        || key.phase != PhaseV1::Success
        || key.timing != TimingV1::Natural
        || key.recipe.fixture != FixtureV1::ManagedWalMainSingleConnection
        || key.recipe.callback != CallbackV1::XShmMap
        || key.recipe.fault_seam != FaultSeamV1::Natural
        || key.recipe.observer != ObserverV1::MapCallbackAndSnapshot
        || key.recipe.cleanup != CleanupV1::ParentOwnedRoot
    {
        return None;
    }
    let DynamicAxesV1::Map(axes) = key.axes else {
        return None;
    };
    let ReachabilityV1::Reached(profile) = axes.profile else {
        return None;
    };
    if axes.mode != ReachabilityV1::Reached(profile.mode)
        || axes.completion != ReachabilityV1::Reached(MapCompletionV1::Completed)
    {
        return None;
    }
    use MapLifecyclePathSpecV1 as P;
    match (
        key.source_site,
        key.prestate,
        key.occurrence,
        profile,
        axes.ordinal,
        axes.regions_to_create,
    ) {
        (
            SourceSiteV1::MapFileSize,
            PrestateV1::Map(MapPrestateV1::RegionsEmpty),
            OccurrenceV1::Natural,
            MapProfileV1 {
                mode: MapModeV1::Observe,
                initialization: InitializationProfileV1::CreatedFirstShared,
                prestate: MapRegionPrestateV1::Empty,
                region_size_arm: MapRegionSizeArmV1::UnsetAssigned,
                file_path: MapFilePathV1::ObserveNotPresent,
                prior_mutation: true,
                preexisting_mapping: false,
            },
            ReachabilityV1::NotReached,
            ReachabilityV1::NotReached,
        ) => Some(P::EmptyObserveNotPresent),
        (
            SourceSiteV1::AbiProjection,
            PrestateV1::Map(MapPrestateV1::RegionsEmpty),
            OccurrenceV1::Exact(1),
            MapProfileV1 {
                mode: MapModeV1::Extend,
                initialization: InitializationProfileV1::CreatedFirstShared,
                prestate: MapRegionPrestateV1::Empty,
                region_size_arm: MapRegionSizeArmV1::UnsetAssigned,
                file_path: MapFilePathV1::GrowSucceeded,
                prior_mutation: true,
                preexisting_mapping: false,
            },
            ReachabilityV1::Reached(1),
            ReachabilityV1::Reached(1),
        ) => Some(P::EmptyExtendMapped),
        (
            SourceSiteV1::CoordinatorState,
            PrestateV1::Map(MapPrestateV1::TargetMapped),
            OccurrenceV1::Natural,
            MapProfileV1 {
                mode: MapModeV1::Observe,
                initialization: InitializationProfileV1::NodeLive,
                prestate: MapRegionPrestateV1::Reuse,
                region_size_arm: MapRegionSizeArmV1::Same,
                file_path: MapFilePathV1::SizeSufficient,
                prior_mutation: true,
                preexisting_mapping: true,
            },
            ReachabilityV1::NotReached,
            ReachabilityV1::NotReached,
        ) => Some(P::ReuseObserveMapped),
        (
            SourceSiteV1::CoordinatorState,
            PrestateV1::Map(MapPrestateV1::TargetMapped),
            OccurrenceV1::Natural,
            MapProfileV1 {
                mode: MapModeV1::Extend,
                initialization: InitializationProfileV1::NodeLive,
                prestate: MapRegionPrestateV1::Reuse,
                region_size_arm: MapRegionSizeArmV1::Same,
                file_path: MapFilePathV1::SizeSufficient,
                prior_mutation: true,
                preexisting_mapping: true,
            },
            ReachabilityV1::NotReached,
            ReachabilityV1::NotReached,
        ) => Some(P::ReuseExtendMapped),
        (
            SourceSiteV1::MapFileSize,
            PrestateV1::Map(MapPrestateV1::TargetMissing),
            OccurrenceV1::Natural,
            MapProfileV1 {
                mode: MapModeV1::Observe,
                initialization: InitializationProfileV1::NodeLive,
                prestate: MapRegionPrestateV1::NonemptyTargetMissing,
                region_size_arm: MapRegionSizeArmV1::Same,
                file_path: MapFilePathV1::ObserveNotPresent,
                prior_mutation: true,
                preexisting_mapping: true,
            },
            ReachabilityV1::NotReached,
            ReachabilityV1::NotReached,
        ) => Some(P::MissingObserveNotPresent),
        (
            SourceSiteV1::AbiProjection,
            PrestateV1::Map(MapPrestateV1::TargetMissing),
            OccurrenceV1::Exact(1),
            MapProfileV1 {
                mode: MapModeV1::Extend,
                initialization: InitializationProfileV1::NodeLive,
                prestate: MapRegionPrestateV1::NonemptyTargetMissing,
                region_size_arm: MapRegionSizeArmV1::Same,
                file_path: MapFilePathV1::GrowSucceeded,
                prior_mutation: true,
                preexisting_mapping: true,
            },
            ReachabilityV1::Reached(1),
            ReachabilityV1::Reached(1),
        ) => Some(P::MissingExtendMapped),
        _ => None,
    }
}

fn expected_v1(path: MapLifecyclePathSpecV1) -> DynamicExpectedV1 {
    use MapLifecyclePathSpecV1 as P;
    let not_present = matches!(
        path,
        P::EmptyObserveNotPresent | P::MissingObserveNotPresent
    );
    let preexisting = matches!(
        path,
        P::ReuseObserveMapped
            | P::ReuseExtendMapped
            | P::MissingObserveNotPresent
            | P::MissingExtendMapped
    );
    let created_first = matches!(path, P::EmptyObserveNotPresent | P::EmptyExtendMapped);
    let grew = matches!(path, P::EmptyExtendMapped | P::MissingExtendMapped);
    DynamicExpectedV1 {
        sqlite: SqliteResultV1::Ok,
        disposition: TerminalDispositionV1::Returned,
        phase: PhaseV1::Success,
        failure: if not_present {
            FailureClassV1::NotPresent
        } else {
            FailureClassV1::None
        },
        mutation: if created_first || grew {
            MutationStateV1::Known
        } else {
            MutationStateV1::None
        },
        lock_outcome_uncertain: false,
        lock_effect: LockEffectV1::NotReached,
        dms_lock: if created_first {
            DmsLockCustodyV1::AcquiredShared
        } else {
            DmsLockCustodyV1::ExistingShared
        },
        raw_slots: CustodyStateV1::Unchanged,
        route: CustodyStateV1::Unchanged,
        callback: CustodyStateV1::Released,
        file: CustodyStateV1::Retained,
        mapping: if not_present && !preexisting {
            CustodyStateV1::Unchanged
        } else {
            CustodyStateV1::Retained
        },
        view: if not_present && !preexisting {
            CustodyStateV1::Unchanged
        } else {
            CustodyStateV1::Retained
        },
        payload: if not_present {
            CustodyStateV1::Released
        } else {
            CustodyStateV1::Retained
        },
        counts: ObservableCountsV1 {
            callback_begin: 1,
            callback_complete: 1,
            native_lock: if created_first { 2 } else { 0 },
            native_unlock: if created_first { 1 } else { 0 },
            file_grow: u16::from(grew),
            mapping_create: u16::from(grew),
            view_map: u16::from(grew),
        },
    }
}

const fn member_v1(path: MapLifecyclePathSpecV1) -> StaticMemberSealV1 {
    let (case_key_sha256, full_record_sha256) = match path {
        MapLifecyclePathSpecV1::EmptyObserveNotPresent => (
            [
                0xa4, 0x4f, 0x8f, 0x31, 0xf8, 0xf4, 0x09, 0x28, 0x41, 0xf5, 0x7c, 0x4e, 0x35, 0x86,
                0xbe, 0x10, 0xc9, 0xcc, 0x05, 0xff, 0x25, 0xb3, 0x39, 0x20, 0x8f, 0x66, 0x61, 0x65,
                0x59, 0x8d, 0x7b, 0x4f,
            ],
            [
                0x42, 0x46, 0x91, 0x05, 0x6f, 0xfb, 0xe9, 0xdf, 0xeb, 0xe6, 0x2b, 0xb0, 0x74, 0xcd,
                0xd7, 0xef, 0xae, 0x59, 0xe8, 0x35, 0xa4, 0xc7, 0xa6, 0x6d, 0x1f, 0x98, 0x34, 0xab,
                0x0c, 0x5f, 0x2f, 0x70,
            ],
        ),
        MapLifecyclePathSpecV1::EmptyExtendMapped => (
            [
                0xde, 0xfd, 0xa9, 0x9f, 0xb6, 0x45, 0x96, 0x65, 0x94, 0xb8, 0x53, 0x3a, 0x3f, 0x5a,
                0xda, 0xc3, 0x4b, 0x8e, 0xb8, 0x39, 0xe6, 0xe4, 0x82, 0x07, 0x63, 0x62, 0x5f, 0x32,
                0xeb, 0x66, 0x2b, 0xe1,
            ],
            [
                0xa5, 0x5b, 0x7b, 0x0d, 0xc9, 0x6e, 0x49, 0x76, 0xc4, 0xd5, 0x72, 0xdd, 0xd4, 0xdf,
                0x9d, 0xc7, 0x0b, 0x63, 0x6e, 0x81, 0x39, 0xeb, 0xa0, 0x07, 0x96, 0x3d, 0xf6, 0x8e,
                0x9a, 0xe4, 0x0c, 0x92,
            ],
        ),
        MapLifecyclePathSpecV1::ReuseObserveMapped => (
            [
                0x7a, 0xe2, 0x94, 0x82, 0x67, 0x17, 0x4f, 0x5e, 0xa6, 0x1d, 0x98, 0x9a, 0x0d, 0xf2,
                0x91, 0x35, 0x74, 0x91, 0xc8, 0xa3, 0x3d, 0x54, 0x92, 0xd3, 0xf1, 0x31, 0x87, 0x3f,
                0xe4, 0xef, 0xd0, 0x84,
            ],
            [
                0x29, 0x9c, 0xf2, 0x26, 0x40, 0xbb, 0x8e, 0xc1, 0xa2, 0x23, 0xd0, 0x2d, 0x43, 0xc6,
                0x61, 0xc7, 0x52, 0x0e, 0xa6, 0xaf, 0xce, 0x80, 0x7c, 0x62, 0x49, 0x73, 0xf8, 0x59,
                0xf1, 0xf9, 0xe7, 0xf5,
            ],
        ),
        MapLifecyclePathSpecV1::ReuseExtendMapped => (
            [
                0x50, 0x59, 0x65, 0x8a, 0xc6, 0x8f, 0x5b, 0xc7, 0x0f, 0x46, 0x2f, 0x47, 0x85, 0x8a,
                0xa5, 0x69, 0x83, 0x2b, 0x0b, 0x9a, 0x16, 0x05, 0x4e, 0xb4, 0x89, 0x94, 0xb5, 0x14,
                0x41, 0xba, 0x5e, 0x6b,
            ],
            [
                0xba, 0xde, 0xc3, 0x97, 0xdc, 0xf3, 0x4c, 0x9a, 0xca, 0xab, 0xa9, 0x73, 0x07, 0xf9,
                0xaf, 0x78, 0x06, 0x14, 0xda, 0x52, 0xe1, 0x42, 0x4a, 0x44, 0x10, 0x4b, 0x01, 0xa5,
                0x9d, 0xd6, 0x22, 0xb2,
            ],
        ),
        MapLifecyclePathSpecV1::MissingObserveNotPresent => (
            [
                0xfe, 0xd1, 0x69, 0xf7, 0x18, 0x7f, 0x44, 0x9e, 0xd5, 0xf1, 0x21, 0x4f, 0x67, 0x74,
                0x39, 0x0a, 0x3e, 0x34, 0x7f, 0x50, 0xfb, 0x23, 0xd8, 0x38, 0x69, 0xb2, 0xf1, 0x46,
                0x59, 0xf9, 0xd1, 0x3c,
            ],
            [
                0x16, 0xaa, 0x37, 0x6b, 0x8d, 0x86, 0x60, 0x79, 0xa1, 0x97, 0x76, 0x50, 0xb6, 0x49,
                0xfe, 0x9f, 0x37, 0x59, 0xbb, 0xf8, 0x80, 0xc9, 0x58, 0xf7, 0x1f, 0x03, 0xe2, 0xf7,
                0xd0, 0xe6, 0xef, 0xc5,
            ],
        ),
        MapLifecyclePathSpecV1::MissingExtendMapped => (
            [
                0x9d, 0x90, 0x29, 0xa3, 0xb7, 0x6b, 0xa3, 0x8a, 0x64, 0xef, 0x8d, 0x10, 0x32, 0x5c,
                0x66, 0xec, 0x15, 0x55, 0xdd, 0xcf, 0x56, 0x00, 0x33, 0x89, 0xee, 0x2e, 0x9b, 0xd6,
                0x49, 0x96, 0x43, 0x98,
            ],
            [
                0x12, 0x4e, 0x6e, 0xd4, 0x2b, 0x07, 0x14, 0xc8, 0xbf, 0xa2, 0x2b, 0x7b, 0x21, 0x22,
                0x32, 0x9a, 0x21, 0x65, 0x64, 0x21, 0xe1, 0x13, 0x54, 0x11, 0x2b, 0xca, 0xbf, 0x87,
                0x8b, 0x48, 0xc5, 0x3a,
            ],
        ),
    };
    StaticMemberSealV1 {
        case_key_sha256: Digest32(case_key_sha256),
        full_record_sha256: Digest32(full_record_sha256),
    }
}

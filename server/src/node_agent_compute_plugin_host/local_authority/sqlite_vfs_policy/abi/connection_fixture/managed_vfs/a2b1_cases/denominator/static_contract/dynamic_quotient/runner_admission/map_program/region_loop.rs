//! Source-bound programs for two exact Map extend region-loop success families.
//!
//! Admission consumes only the complete typed descriptor, exact expected vector and the frozen
//! member pair selected from the checked-in catalog. Leaf ids and display text are never inputs.

mod catalog;
mod source_scope;

use super::super::super::super::{
    source_leaf_authority::{
        CustodyStateV1, DmsLockCustodyV1, FailureClassV1, LockEffectV1, MutationStateV1,
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
    DynamicAxesV1, DynamicClassKeyV1, DynamicExpectedV1, DynamicOperationV1,
    DYNAMIC_PROJECTOR_SCHEMA_V1,
};
use super::super::CompiledRunnerPlanV1;
use super::{MapProgramCaseV1, MapProgramSpecV1, MapRunnerExecutionViolationV1, ProgramModeV1};
use catalog::exact_member_v1;
use source_scope::digest_implementation_v1;

const EMPTY_EXTEND_COUNT: u16 = 256;
const MISSING_EXTEND_COUNT: u16 = 255;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MapRegionLoopFamilyV1 {
    EmptyExtend,
    MissingExtend,
}

impl MapRegionLoopFamilyV1 {
    const fn tag(self) -> &'static str {
        match self {
            Self::EmptyExtend => "empty_extend",
            Self::MissingExtend => "missing_extend",
        }
    }

    pub(super) const fn max_regions(self) -> u16 {
        match self {
            Self::EmptyExtend => EMPTY_EXTEND_COUNT,
            Self::MissingExtend => MISSING_EXTEND_COUNT,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MapRegionLoopProgramV1 {
    family: MapRegionLoopFamilyV1,
    regions_to_create: u16,
}

impl MapRegionLoopProgramV1 {
    pub(super) const fn family(self) -> MapRegionLoopFamilyV1 {
        self.family
    }

    pub(super) const fn regions_to_create(self) -> u16 {
        self.regions_to_create
    }

    pub(super) const fn target_region(self) -> u16 {
        match self.family {
            MapRegionLoopFamilyV1::EmptyExtend => self.regions_to_create - 1,
            MapRegionLoopFamilyV1::MissingExtend => self.regions_to_create,
        }
    }

    pub(super) const fn implementation_tag(self) -> u8 {
        match self.family {
            MapRegionLoopFamilyV1::EmptyExtend => 1,
            MapRegionLoopFamilyV1::MissingExtend => 2,
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
    let Some(program) = classify_program_v1(key) else {
        return Err(MapRunnerExecutionViolationV1::UnsupportedProgram);
    };
    if key.expected != expected_v1(program) {
        return Err(MapRunnerExecutionViolationV1::UnsupportedProgram);
    }
    Ok(MapProgramSpecV1 {
        mode: ProgramModeV1::Extend,
        case: MapProgramCaseV1::RegionLoop(program),
        member: exact_member_v1(program.family, program.regions_to_create)?,
        normalized_descriptor_sha256: plan.normalized_descriptor_sha256,
        plan_sha256: plan.plan_sha256,
        implementation_sha256: digest_implementation_v1(program),
    })
}

fn classify_program_v1(key: &DynamicClassKeyV1) -> Option<MapRegionLoopProgramV1> {
    if key.schema_version != DYNAMIC_PROJECTOR_SCHEMA_V1
        || key.root != RootOperationV1::Map
        || key.source_site != SourceSiteV1::AbiProjection
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
    let (
        OccurrenceV1::Exact(occurrence),
        ReachabilityV1::Reached(profile),
        ReachabilityV1::Reached(ordinal),
        ReachabilityV1::Reached(regions_to_create),
    ) = (
        key.occurrence,
        axes.profile,
        axes.ordinal,
        axes.regions_to_create,
    )
    else {
        return None;
    };
    if axes.mode != ReachabilityV1::Reached(MapModeV1::Extend)
        || axes.completion != ReachabilityV1::Reached(MapCompletionV1::Completed)
        || occurrence != ordinal
        || ordinal != regions_to_create
    {
        return None;
    }
    let family = match (key.prestate, profile) {
        (
            PrestateV1::Map(MapPrestateV1::RegionsEmpty),
            MapProfileV1 {
                mode: MapModeV1::Extend,
                initialization: InitializationProfileV1::CreatedFirstShared,
                prestate: MapRegionPrestateV1::Empty,
                region_size_arm: MapRegionSizeArmV1::UnsetAssigned,
                file_path: MapFilePathV1::GrowSucceeded,
                prior_mutation: true,
                preexisting_mapping: false,
            },
        ) => MapRegionLoopFamilyV1::EmptyExtend,
        (
            PrestateV1::Map(MapPrestateV1::TargetMissing),
            MapProfileV1 {
                mode: MapModeV1::Extend,
                initialization: InitializationProfileV1::NodeLive,
                prestate: MapRegionPrestateV1::NonemptyTargetMissing,
                region_size_arm: MapRegionSizeArmV1::Same,
                file_path: MapFilePathV1::GrowSucceeded,
                prior_mutation: true,
                preexisting_mapping: true,
            },
        ) => MapRegionLoopFamilyV1::MissingExtend,
        _ => return None,
    };
    (regions_to_create != 0 && regions_to_create <= family.max_regions()).then_some(
        MapRegionLoopProgramV1 {
            family,
            regions_to_create,
        },
    )
}

fn expected_v1(program: MapRegionLoopProgramV1) -> DynamicExpectedV1 {
    let created_first = program.family == MapRegionLoopFamilyV1::EmptyExtend;
    DynamicExpectedV1 {
        sqlite: SqliteResultV1::Ok,
        disposition: TerminalDispositionV1::Returned,
        phase: PhaseV1::Success,
        failure: FailureClassV1::None,
        mutation: MutationStateV1::Known,
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
        mapping: CustodyStateV1::Retained,
        view: CustodyStateV1::Retained,
        payload: CustodyStateV1::Retained,
        counts: ObservableCountsV1 {
            callback_begin: 1,
            callback_complete: 1,
            native_lock: if created_first { 2 } else { 0 },
            native_unlock: if created_first { 1 } else { 0 },
            file_grow: 1,
            mapping_create: program.regions_to_create,
            view_map: program.regions_to_create,
        },
    }
}

#[cfg(test)]
pub(super) fn catalog_row_count_for_test() -> usize {
    catalog::catalog_row_count_for_test()
}

//! Frozen Map program fixtures shared by inventory and sealed-execution tests.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};

use super::super::super::terminal_descriptor::{
    MapFilePathV1, MapProfileV1, MapRegionPrestateV1, MapRegionSizeArmV1,
};

use super::*;

const MAP_PROGRAM_LEAF_IDS: [&str; 12] = [
    "map.observe.managed.region-size-rejected.projection.completion-succeeded.terminal",
    "map.observe.managed.region-count-rejected.projection.completion-succeeded.terminal",
    "map.observe.managed.logical-size-rejected.projection.completion-succeeded.terminal",
    "map.extend.managed.region-size-rejected.projection.completion-succeeded.terminal",
    "map.extend.managed.region-count-rejected.projection.completion-succeeded.terminal",
    "map.extend.managed.logical-size-rejected.projection.completion-succeeded.terminal",
    "map.observe.managed.initialization.success.created-first-shared.post-init.regions-empty.region-size-unset.observe-not-present.projection.terminal.success",
    "map.extend.managed.initialization.success.created-first-shared.post-init.regions-empty.region-size-unset.extend-grow.succeeded.region-loop.ordinal-001.target.projection.terminal.success",
    "map.observe.managed.initialization.success.node-live.post-init.target-reusable.region-size-same.size-sufficient.reuse.projection.terminal.success",
    "map.extend.managed.initialization.success.node-live.post-init.target-reusable.region-size-same.size-sufficient.reuse.projection.terminal.success",
    "map.observe.managed.initialization.success.node-live.post-init.regions-nonempty-target-missing.region-size-same.observe-not-present.projection.terminal.success",
    "map.extend.managed.initialization.success.node-live.post-init.regions-nonempty-target-missing.region-size-same.extend-grow.succeeded.region-loop.ordinal-001.target.projection.terminal.success",
];

#[derive(Clone)]
pub(super) struct FrozenMapProgramLeafV1 {
    pub(super) record: LeafRecordV1,
    pub(super) descriptor: TerminalDescriptorV1,
    pub(super) member: StaticMemberSealV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MapLifecycleProgramCaseV1 {
    EmptyObserveNotPresent,
    EmptyExtendMapped,
    ReuseObserveMapped,
    ReuseExtendMapped,
    MissingObserveNotPresent,
    MissingExtendMapped,
}

pub(super) const MAP_LIFECYCLE_CASES: [MapLifecycleProgramCaseV1; 6] = [
    MapLifecycleProgramCaseV1::EmptyObserveNotPresent,
    MapLifecycleProgramCaseV1::EmptyExtendMapped,
    MapLifecycleProgramCaseV1::ReuseObserveMapped,
    MapLifecycleProgramCaseV1::ReuseExtendMapped,
    MapLifecycleProgramCaseV1::MissingObserveNotPresent,
    MapLifecycleProgramCaseV1::MissingExtendMapped,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MapRegionLoopFamilyV1 {
    CreatedFirstEmptyExtend,
    NodeLiveTargetMissingExtend,
}

pub(super) const MAP_REGION_LOOP_FAMILIES: [MapRegionLoopFamilyV1; 2] = [
    MapRegionLoopFamilyV1::CreatedFirstEmptyExtend,
    MapRegionLoopFamilyV1::NodeLiveTargetMissingExtend,
];

pub(super) const MAP_REGION_LOOP_MEMBER_COUNT: usize = 511;

impl MapRegionLoopFamilyV1 {
    pub(super) const fn max_ordinal(self) -> u16 {
        match self {
            Self::CreatedFirstEmptyExtend => 256,
            Self::NodeLiveTargetMissingExtend => 255,
        }
    }

    const fn ordinal_one_lifecycle_case(self) -> MapLifecycleProgramCaseV1 {
        match self {
            Self::CreatedFirstEmptyExtend => MapLifecycleProgramCaseV1::EmptyExtendMapped,
            Self::NodeLiveTargetMissingExtend => MapLifecycleProgramCaseV1::MissingExtendMapped,
        }
    }
}

impl MapLifecycleProgramCaseV1 {
    pub(super) const fn leaf_id(self) -> &'static str {
        match self {
            Self::EmptyObserveNotPresent => MAP_PROGRAM_LEAF_IDS[6],
            Self::EmptyExtendMapped => MAP_PROGRAM_LEAF_IDS[7],
            Self::ReuseObserveMapped => MAP_PROGRAM_LEAF_IDS[8],
            Self::ReuseExtendMapped => MAP_PROGRAM_LEAF_IDS[9],
            Self::MissingObserveNotPresent => MAP_PROGRAM_LEAF_IDS[10],
            Self::MissingExtendMapped => MAP_PROGRAM_LEAF_IDS[11],
        }
    }
}

pub(super) fn map_lifecycle_leaf_v1(case: MapLifecycleProgramCaseV1) -> FrozenMapProgramLeafV1 {
    frozen_map_program_leaf_v1(case.leaf_id())
}

pub(super) fn map_lifecycle_descriptor_v1(
    case: MapLifecycleProgramCaseV1,
    capability: RunnerCapabilityV1,
) -> TerminalDescriptorV1 {
    let mut descriptor = map_lifecycle_leaf_v1(case).descriptor;
    let TerminalDescriptorV1::Map(value) = &mut descriptor else {
        unreachable!("a Map frozen program must have a Map descriptor")
    };
    value.recipe.capability = capability;
    descriptor
}

pub(super) fn map_region_loop_leaf_v1(
    family: MapRegionLoopFamilyV1,
    ordinal: u16,
) -> FrozenMapProgramLeafV1 {
    frozen_map_region_loop_leaves_v1()
        .get(&(family, ordinal))
        .unwrap_or_else(|| panic!("missing frozen Map region-loop member {family:?}/{ordinal}"))
        .clone()
}

pub(super) fn map_region_loop_descriptor_v1(
    family: MapRegionLoopFamilyV1,
    ordinal: u16,
    capability: RunnerCapabilityV1,
) -> TerminalDescriptorV1 {
    let mut descriptor = map_region_loop_leaf_v1(family, ordinal).descriptor;
    let TerminalDescriptorV1::Map(value) = &mut descriptor else {
        unreachable!("a Map frozen program must have a Map descriptor")
    };
    value.recipe.capability = capability;
    descriptor
}

pub(super) fn frozen_map_region_loop_leaves_v1(
) -> &'static BTreeMap<(MapRegionLoopFamilyV1, u16), FrozenMapProgramLeafV1> {
    static LEAVES: OnceLock<BTreeMap<(MapRegionLoopFamilyV1, u16), FrozenMapProgramLeafV1>> =
        OnceLock::new();
    LEAVES.get_or_init(|| {
        let graph = super::super::super::map::graph();
        let mut leaves = BTreeMap::new();
        super::super::super::source_leaf_authority::validate_map_graph_with_records(
            &graph,
            |leaf| {
                let StreamedLeafV1::Terminal {
                    record,
                    descriptor,
                    seal,
                } = leaf
                else {
                    return Ok(());
                };
                let Some(key) = map_region_loop_key_v1(descriptor) else {
                    return Ok(());
                };
                let previous = leaves.insert(
                    key,
                    FrozenMapProgramLeafV1 {
                        record: record.clone(),
                        descriptor: *descriptor,
                        member: StaticMemberSealV1 {
                            case_key_sha256: seal.case_key_sha256,
                            full_record_sha256: seal.full_record_sha256,
                        },
                    },
                );
                if previous.is_some() {
                    return Err(format!(
                        "duplicate frozen Map region-loop member {:?}/{}",
                        key.0, key.1
                    ));
                }
                Ok(())
            },
        )
        .expect("the frozen Map authority must validate before program tests");
        assert_eq!(leaves.len(), MAP_REGION_LOOP_MEMBER_COUNT);
        leaves
    })
}

fn map_region_loop_key_v1(
    descriptor: &TerminalDescriptorV1,
) -> Option<(MapRegionLoopFamilyV1, u16)> {
    let TerminalDescriptorV1::Map(value) = descriptor else {
        return None;
    };
    if value.source_site != SourceSiteV1::AbiProjection
        || value.stimulus != StimulusV1::MapManaged(MapManagedStimulusV1::Success)
        || value.operation != MapOperationV1::SuccessProjection
        || value.phase != PhaseV1::Success
        || value.timing != TimingV1::Natural
        || value.recipe.fixture != FixtureV1::ManagedWalMainSingleConnection
        || value.recipe.callback != CallbackV1::XShmMap
        || value.recipe.fault_seam != FaultSeamV1::Natural
        || value.recipe.observer != ObserverV1::MapCallbackAndSnapshot
        || value.recipe.cleanup != CleanupV1::ParentOwnedRoot
        || value.recipe.capability
            != RunnerCapabilityV1::Missing(CapabilityGapV1::QuotientRunnerNotIntegrated)
        || value.axes.mode != ReachabilityV1::Reached(MapModeV1::Extend)
        || value.axes.completion != ReachabilityV1::Reached(MapCompletionV1::Completed)
    {
        return None;
    }
    let (
        OccurrenceV1::Exact(occurrence),
        ReachabilityV1::Reached(profile),
        ReachabilityV1::Reached(ordinal),
        ReachabilityV1::Reached(regions_to_create),
    ) = (
        value.occurrence,
        value.axes.profile,
        value.axes.ordinal,
        value.axes.regions_to_create,
    )
    else {
        return None;
    };
    if occurrence != ordinal || ordinal != regions_to_create || ordinal == 0 {
        return None;
    }
    let family = match (value.prestate, profile) {
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
        ) => MapRegionLoopFamilyV1::CreatedFirstEmptyExtend,
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
        ) => MapRegionLoopFamilyV1::NodeLiveTargetMissingExtend,
        _ => return None,
    };
    (ordinal <= family.max_ordinal()).then_some((family, ordinal))
}

pub(super) fn request_budget_leaf_v1(
    stimulus: MapManagedStimulusV1,
    mode: MapModeV1,
) -> FrozenMapProgramLeafV1 {
    let index = match (mode, stimulus) {
        (MapModeV1::Observe, MapManagedStimulusV1::RegionSizeBudget) => 0,
        (MapModeV1::Observe, MapManagedStimulusV1::RegionCountBudget) => 1,
        (MapModeV1::Observe, MapManagedStimulusV1::LogicalSizeBudget) => 2,
        (MapModeV1::Extend, MapManagedStimulusV1::RegionSizeBudget) => 3,
        (MapModeV1::Extend, MapManagedStimulusV1::RegionCountBudget) => 4,
        (MapModeV1::Extend, MapManagedStimulusV1::LogicalSizeBudget) => 5,
        _ => panic!("requested a non-programmed Map request-budget leaf"),
    };
    frozen_map_program_leaf_v1(MAP_PROGRAM_LEAF_IDS[index])
}

pub(super) fn request_budget_descriptor_v1(
    stimulus: MapManagedStimulusV1,
    mode: MapModeV1,
    capability: RunnerCapabilityV1,
) -> TerminalDescriptorV1 {
    let mut descriptor = request_budget_leaf_v1(stimulus, mode).descriptor;
    let TerminalDescriptorV1::Map(value) = &mut descriptor else {
        unreachable!("a Map frozen program must have a Map descriptor")
    };
    value.recipe.capability = capability;
    descriptor
}

fn frozen_map_program_leaf_v1(leaf_id: &'static str) -> FrozenMapProgramLeafV1 {
    frozen_map_program_leaves_v1()
        .get(leaf_id)
        .unwrap_or_else(|| panic!("missing frozen Map program leaf {leaf_id}"))
        .clone()
}

fn frozen_map_program_leaves_v1() -> &'static BTreeMap<&'static str, FrozenMapProgramLeafV1> {
    static LEAVES: OnceLock<BTreeMap<&'static str, FrozenMapProgramLeafV1>> = OnceLock::new();
    LEAVES.get_or_init(|| {
        let graph = super::super::super::map::graph();
        let mut leaves = BTreeMap::new();
        super::super::super::source_leaf_authority::validate_map_graph_with_records(
            &graph,
            |leaf| {
                let StreamedLeafV1::Terminal {
                    record,
                    descriptor,
                    seal,
                } = leaf
                else {
                    return Ok(());
                };
                let Some(leaf_id) = MAP_PROGRAM_LEAF_IDS
                    .iter()
                    .copied()
                    .find(|candidate| *candidate == record.key.identity.leaf_id.as_str())
                else {
                    return Ok(());
                };
                let previous = leaves.insert(
                    leaf_id,
                    FrozenMapProgramLeafV1 {
                        record: record.clone(),
                        descriptor: *descriptor,
                        member: StaticMemberSealV1 {
                            case_key_sha256: seal.case_key_sha256,
                            full_record_sha256: seal.full_record_sha256,
                        },
                    },
                );
                if previous.is_some() {
                    return Err(format!("duplicate frozen Map program leaf {leaf_id}"));
                }
                Ok(())
            },
        )
        .expect("the frozen Map authority must validate before program tests");
        assert_eq!(leaves.len(), MAP_PROGRAM_LEAF_IDS.len());
        leaves
    })
}

#[test]
fn frozen_map_region_loop_fixtures_are_contiguous_unique_and_preserve_ordinal_one() {
    let leaves = frozen_map_region_loop_leaves_v1();
    assert_eq!(leaves.len(), MAP_REGION_LOOP_MEMBER_COUNT);
    assert_eq!(leaves.keys().copied().collect::<BTreeSet<_>>().len(), 511);
    assert_eq!(
        leaves
            .values()
            .map(|leaf| leaf.member)
            .collect::<BTreeSet<_>>()
            .len(),
        511
    );
    for family in MAP_REGION_LOOP_FAMILIES {
        assert_eq!(
            (1..=family.max_ordinal())
                .filter(|ordinal| leaves.contains_key(&(family, *ordinal)))
                .count(),
            usize::from(family.max_ordinal())
        );
        assert_eq!(
            map_region_loop_leaf_v1(family, 1).member,
            map_lifecycle_leaf_v1(family.ordinal_one_lifecycle_case()).member
        );
    }
}

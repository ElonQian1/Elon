//! Frozen Map program fixtures shared by inventory and sealed-execution tests.

use std::{collections::BTreeMap, sync::OnceLock};

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

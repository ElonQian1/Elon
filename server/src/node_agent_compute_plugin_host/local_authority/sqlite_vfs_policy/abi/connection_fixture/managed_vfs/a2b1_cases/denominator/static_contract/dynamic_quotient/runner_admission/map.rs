use super::super::super::{
    source_leaf_authority::RootOperationV1, terminal_descriptor::CapabilityGapV1,
};
use super::{RunnerPlanBlueprintV1, RunnerPlanStageV1};

const MAP_RUNNER_STAGES_V1: &[RunnerPlanStageV1] = &[
    RunnerPlanStageV1::ValidatedDescriptor,
    RunnerPlanStageV1::ProducerCoherence,
    RunnerPlanStageV1::CallbackBeginCompleteLedger,
    RunnerPlanStageV1::MapFileGrowthObservation,
    RunnerPlanStageV1::MapMappingCreationObservation,
    RunnerPlanStageV1::MapViewMappingObservation,
    RunnerPlanStageV1::MapPayloadCustodyObservation,
    RunnerPlanStageV1::FaultCallCleanupAggregate,
    RunnerPlanStageV1::ParentOwnedCleanupReceipt,
    RunnerPlanStageV1::WindowsChildIsolation,
    RunnerPlanStageV1::FrozenManifestClassExecution,
];

pub(super) const fn blueprint_v1() -> RunnerPlanBlueprintV1 {
    RunnerPlanBlueprintV1 {
        root: RootOperationV1::Map,
        expected_gap: CapabilityGapV1::QuotientRunnerNotIntegrated,
        stages: MAP_RUNNER_STAGES_V1,
    }
}

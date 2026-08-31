use super::super::super::{
    source_leaf_authority::RootOperationV1, terminal_descriptor::CapabilityGapV1,
};
use super::{RunnerPlanBlueprintV1, RunnerPlanStageV1};

const LOCK_RUNNER_STAGES_V1: &[RunnerPlanStageV1] = &[
    RunnerPlanStageV1::ValidatedDescriptor,
    RunnerPlanStageV1::ProducerCoherence,
    RunnerPlanStageV1::CallbackBeginCompleteLedger,
    RunnerPlanStageV1::LockSelectedConnectionPrePost,
    RunnerPlanStageV1::LockSiblingConnectionPrePost,
    RunnerPlanStageV1::LockRawAbiReceipt,
    RunnerPlanStageV1::LockNativeOperationReceipt,
    RunnerPlanStageV1::FaultCallCleanupAggregate,
    RunnerPlanStageV1::ParentOwnedCleanupReceipt,
    RunnerPlanStageV1::WindowsChildIsolation,
    RunnerPlanStageV1::FrozenManifestClassExecution,
];

pub(super) const fn blueprint_v1() -> RunnerPlanBlueprintV1 {
    RunnerPlanBlueprintV1 {
        root: RootOperationV1::Lock,
        expected_gap: CapabilityGapV1::LockObservationIncomplete,
        stages: LOCK_RUNNER_STAGES_V1,
    }
}

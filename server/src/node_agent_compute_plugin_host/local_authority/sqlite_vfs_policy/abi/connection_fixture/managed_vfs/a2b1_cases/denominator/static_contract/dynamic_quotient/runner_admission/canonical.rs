use sha2::{Digest as _, Sha256};

use super::super::super::source_leaf_authority::{Digest32, RootOperationV1};
use super::super::canonical_tags::gap_tag;
use super::super::DYNAMIC_PROJECTOR_SCHEMA_V1;
use super::{
    RunnerAdmissionDecisionV1, RunnerAdmissionReceiptV1, RunnerPlanBlueprintV1, RunnerPlanStageV1,
};

const RUNNER_PLAN_DOMAIN_V1: &str = "ELON-A2-MAP-LOCK-DYNAMIC-RUNNER-PLAN-V1";
const RUNNER_ADMISSION_BINDING_DOMAIN_V1: &str =
    "ELON-A2-MAP-LOCK-DYNAMIC-RUNNER-ADMISSION-BINDING-V1";

pub(super) fn digest_runner_plan_v1(
    blueprint: RunnerPlanBlueprintV1,
    normalized_descriptor_sha256: Digest32,
) -> Digest32 {
    let mut out = StableHasher::new(RUNNER_PLAN_DOMAIN_V1);
    out.u16("projector_schema_version", DYNAMIC_PROJECTOR_SCHEMA_V1);
    out.text("root", blueprint.root.canonical_name());
    out.digest("normalized_descriptor_sha256", normalized_descriptor_sha256);
    out.u16("expected_gap", gap_tag(blueprint.expected_gap));
    out.u64("stage_count", blueprint.stages.len() as u64);
    for stage in blueprint.stages {
        out.u16("stage", stage_tag(*stage));
    }
    out.finish()
}

pub(super) fn digest_runner_admission_binding_v1(
    root: RootOperationV1,
    receipts: impl IntoIterator<Item = RunnerAdmissionReceiptV1>,
) -> Digest32 {
    let mut receipts = receipts.into_iter().collect::<Vec<_>>();
    receipts.sort_unstable_by_key(|receipt| receipt.member);
    let mut out = StableHasher::new(RUNNER_ADMISSION_BINDING_DOMAIN_V1);
    out.text("root", root.canonical_name());
    out.u64("receipt_count", receipts.len() as u64);
    for receipt in receipts {
        out.digest("case_key_sha256", receipt.member.case_key_sha256);
        out.digest("full_record_sha256", receipt.member.full_record_sha256);
        out.digest(
            "normalized_descriptor_sha256",
            receipt.normalized_descriptor_sha256,
        );
        out.digest("plan_sha256", receipt.plan_sha256);
        match receipt.decision {
            RunnerAdmissionDecisionV1::Missing(gap) => {
                out.u16("decision", 1);
                out.u16("exact_missing_gap", gap_tag(gap));
            }
            RunnerAdmissionDecisionV1::Supported {
                implementation_sha256,
                execution_sha256,
            } => {
                out.u16("decision", 2);
                out.digest("implementation_sha256", implementation_sha256);
                out.digest("execution_sha256", execution_sha256);
            }
        }
    }
    out.finish()
}

fn stage_tag(value: RunnerPlanStageV1) -> u16 {
    match value {
        RunnerPlanStageV1::ValidatedDescriptor => 1,
        RunnerPlanStageV1::ProducerCoherence => 2,
        RunnerPlanStageV1::CallbackBeginCompleteLedger => 3,
        RunnerPlanStageV1::MapFileGrowthObservation => 10,
        RunnerPlanStageV1::MapMappingCreationObservation => 11,
        RunnerPlanStageV1::MapViewMappingObservation => 12,
        RunnerPlanStageV1::MapPayloadCustodyObservation => 13,
        RunnerPlanStageV1::LockSelectedConnectionPrePost => 20,
        RunnerPlanStageV1::LockSiblingConnectionPrePost => 21,
        RunnerPlanStageV1::LockRawAbiReceipt => 22,
        RunnerPlanStageV1::LockNativeOperationReceipt => 23,
        RunnerPlanStageV1::FaultCallCleanupAggregate => 30,
        RunnerPlanStageV1::ParentOwnedCleanupReceipt => 31,
        RunnerPlanStageV1::WindowsChildIsolation => 40,
        RunnerPlanStageV1::FrozenManifestClassExecution => 41,
    }
}

struct StableHasher(Sha256);

impl StableHasher {
    fn new(domain: &str) -> Self {
        let mut out = Sha256::new();
        out.update(domain.as_bytes());
        out.update([0]);
        Self(out)
    }

    fn text(&mut self, label: &str, value: &str) {
        self.bytes(label, value.as_bytes());
    }

    fn u16(&mut self, label: &str, value: u16) {
        self.bytes(label, &value.to_be_bytes());
    }

    fn u64(&mut self, label: &str, value: u64) {
        self.bytes(label, &value.to_be_bytes());
    }

    fn digest(&mut self, label: &str, value: Digest32) {
        self.bytes(label, &value.0);
    }

    fn bytes(&mut self, label: &str, value: &[u8]) {
        self.0.update(label.as_bytes());
        self.0.update([0]);
        self.0.update((value.len() as u64).to_be_bytes());
        self.0.update(value);
    }

    fn finish(self) -> Digest32 {
        Digest32(self.0.finalize().into())
    }
}

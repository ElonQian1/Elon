//! Sealed, source-derived admission plans for dynamic Map/Lock runners.
//!
//! A producer-owned `RunnerCapabilityV1::Supported` value is only a declaration. It is never an
//! execution permit. This module compiles the exact root plan from an already validated,
//! producer-coherent semantic key and emits only the currently honest planned-missing receipt.

mod canonical;
mod lock;
mod map;

use super::super::{
    source_leaf_authority::{Digest32, RootOperationV1},
    terminal_descriptor::{CapabilityGapV1, RunnerCapabilityV1},
};
use super::{digest_normalized_descriptor_semantics_v1, DynamicClassKeyV1, StaticMemberSealV1};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RunnerAdmissionReceiptV1 {
    member: StaticMemberSealV1,
    normalized_descriptor_sha256: Digest32,
    plan_sha256: Digest32,
    exact_missing_gap: CapabilityGapV1,
}

impl RunnerAdmissionReceiptV1 {
    pub(super) const fn member(self) -> StaticMemberSealV1 {
        self.member
    }

    pub(super) const fn normalized_descriptor_sha256(self) -> Digest32 {
        self.normalized_descriptor_sha256
    }

    #[cfg(test)]
    pub(super) const fn plan_sha256(self) -> Digest32 {
        self.plan_sha256
    }

    pub(super) const fn exact_missing_gap(self) -> CapabilityGapV1 {
        self.exact_missing_gap
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RunnerAdmissionViolationV1 {
    UnsealedSupportedClaim,
    DeclaredGapMismatch {
        expected: CapabilityGapV1,
        actual: CapabilityGapV1,
    },
    PlanBindingMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CompiledRunnerPlanV1 {
    root: RootOperationV1,
    normalized_descriptor_sha256: Digest32,
    plan_sha256: Digest32,
    expected_gap: CapabilityGapV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RunnerPlanBlueprintV1 {
    root: RootOperationV1,
    expected_gap: CapabilityGapV1,
    stages: &'static [RunnerPlanStageV1],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunnerPlanStageV1 {
    ValidatedDescriptor,
    ProducerCoherence,
    CallbackBeginCompleteLedger,
    MapFileGrowthObservation,
    MapMappingCreationObservation,
    MapViewMappingObservation,
    MapPayloadCustodyObservation,
    LockSelectedConnectionPrePost,
    LockSiblingConnectionPrePost,
    LockRawAbiReceipt,
    LockNativeOperationReceipt,
    FaultCallCleanupAggregate,
    ParentOwnedCleanupReceipt,
    WindowsChildIsolation,
    FrozenManifestClassExecution,
}

pub(super) fn resolve_v1(
    key: &DynamicClassKeyV1,
    member: StaticMemberSealV1,
) -> Result<RunnerAdmissionReceiptV1, RunnerAdmissionViolationV1> {
    let plan = compile_v1(key);
    resolve_with_plan_v1(key, member, plan)
}

pub(super) fn digest_binding_v1(
    root: RootOperationV1,
    receipts: impl IntoIterator<Item = RunnerAdmissionReceiptV1>,
) -> Digest32 {
    canonical::digest_runner_admission_binding_v1(root, receipts)
}

fn compile_v1(key: &DynamicClassKeyV1) -> CompiledRunnerPlanV1 {
    let normalized_descriptor_sha256 = digest_normalized_descriptor_semantics_v1(key);
    let blueprint = match key.root {
        RootOperationV1::Map => map::blueprint_v1(),
        RootOperationV1::Lock => lock::blueprint_v1(),
    };
    CompiledRunnerPlanV1 {
        root: blueprint.root,
        normalized_descriptor_sha256,
        plan_sha256: canonical::digest_runner_plan_v1(blueprint, normalized_descriptor_sha256),
        expected_gap: blueprint.expected_gap,
    }
}

fn resolve_with_plan_v1(
    key: &DynamicClassKeyV1,
    member: StaticMemberSealV1,
    plan: CompiledRunnerPlanV1,
) -> Result<RunnerAdmissionReceiptV1, RunnerAdmissionViolationV1> {
    if plan != compile_v1(key) {
        return Err(RunnerAdmissionViolationV1::PlanBindingMismatch);
    }
    let actual_gap = match key.recipe.capability {
        RunnerCapabilityV1::Supported => {
            return Err(RunnerAdmissionViolationV1::UnsealedSupportedClaim)
        }
        RunnerCapabilityV1::Missing(gap) => gap,
    };
    if actual_gap != plan.expected_gap {
        return Err(RunnerAdmissionViolationV1::DeclaredGapMismatch {
            expected: plan.expected_gap,
            actual: actual_gap,
        });
    }
    Ok(RunnerAdmissionReceiptV1 {
        member,
        normalized_descriptor_sha256: plan.normalized_descriptor_sha256,
        plan_sha256: plan.plan_sha256,
        exact_missing_gap: plan.expected_gap,
    })
}

#[cfg(test)]
pub(super) fn compile_for_test(key: &DynamicClassKeyV1) -> CompiledRunnerPlanV1 {
    compile_v1(key)
}

#[cfg(test)]
pub(super) fn resolve_with_plan_for_test(
    key: &DynamicClassKeyV1,
    member: StaticMemberSealV1,
    plan: CompiledRunnerPlanV1,
) -> Result<RunnerAdmissionReceiptV1, RunnerAdmissionViolationV1> {
    resolve_with_plan_v1(key, member, plan)
}

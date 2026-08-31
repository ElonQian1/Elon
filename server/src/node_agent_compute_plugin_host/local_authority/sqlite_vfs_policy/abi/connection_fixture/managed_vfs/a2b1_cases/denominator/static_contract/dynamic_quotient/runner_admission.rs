//! Sealed, source-derived admission plans for dynamic Map/Lock runners.
//!
//! A producer-owned `RunnerCapabilityV1::Supported` value is only a declaration. It is never an
//! execution permit. This module compiles the exact root plan from an already validated,
//! producer-coherent semantic key. Missing declarations receive a planned-missing receipt; the
//! exact source-supported Map programs can receive `Supported` only after consuming their
//! private, process-isolated execution receipts.

mod canonical;
mod lock;
mod lock_program;
mod map;
mod map_program;

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
    decision: RunnerAdmissionDecisionV1,
}

#[cfg(all(test, windows))]
pub(super) use lock_program::tamper_lock_implementation_digest_for_test;
pub(super) use lock_program::LockRunnerExecutionReceiptV1;
#[cfg(all(test, windows))]
pub(super) use lock_program::{
    run_lock_isolated_for_test, LockRunnerExecutionErrorV1, LockRunnerIsolatedOutcomeV1,
};
#[cfg(all(test, windows))]
pub(super) use map_program::tamper_implementation_digest_for_test;
pub(super) use map_program::MapRunnerExecutionReceiptV1;
#[cfg(test)]
pub(super) use map_program::{
    region_loop_catalog_row_count_for_test,
    validate_program_for_test as validate_map_program_for_test,
};
#[cfg(all(test, windows))]
pub(super) use map_program::{
    run_isolated_for_test, MapRunnerExecutionErrorV1, MapRunnerIsolatedOutcomeV1,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RunnerAdmissionDecisionV1 {
    Supported {
        implementation_sha256: Digest32,
        execution_sha256: Digest32,
    },
    Missing(CapabilityGapV1),
}

/// Pre-manifest source inventory only. This status never grants catalog admission and cannot
/// substitute for a process-isolated execution receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ExecutionProgramInventoryStatusV1 {
    PlannedMissing(CapabilityGapV1),
    SourcePresentReceiptRequired { implementation_sha256: Digest32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ExecutionProgramInventoryReceiptV1 {
    normalized_key: DynamicClassKeyV1,
    normalized_descriptor_sha256: Digest32,
    program_id: Digest32,
    plan_sha256: Digest32,
    status: ExecutionProgramInventoryStatusV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExecutionProgramInventoryViolationV1 {
    LockProgramLookupFailed,
    MapProgramLookupFailed,
}

impl ExecutionProgramInventoryReceiptV1 {
    pub(super) const fn normalized_key(self) -> DynamicClassKeyV1 {
        self.normalized_key
    }

    pub(super) const fn program_id(self) -> Digest32 {
        self.program_id
    }

    pub(super) const fn normalized_descriptor_sha256(self) -> Digest32 {
        self.normalized_descriptor_sha256
    }

    pub(super) const fn plan_sha256(self) -> Digest32 {
        self.plan_sha256
    }

    pub(super) const fn status(self) -> ExecutionProgramInventoryStatusV1 {
        self.status
    }
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

    pub(super) const fn decision(self) -> RunnerAdmissionDecisionV1 {
        self.decision
    }

    pub(super) const fn exact_missing_gap(self) -> Option<CapabilityGapV1> {
        match self.decision {
            RunnerAdmissionDecisionV1::Missing(gap) => Some(gap),
            RunnerAdmissionDecisionV1::Supported { .. } => None,
        }
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
    LockExecutionReceiptMismatch,
    MapExecutionReceiptMismatch,
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

pub(super) fn resolve_with_map_execution_v1(
    key: &DynamicClassKeyV1,
    member: StaticMemberSealV1,
    execution: MapRunnerExecutionReceiptV1,
) -> Result<RunnerAdmissionReceiptV1, RunnerAdmissionViolationV1> {
    let plan = compile_v1(key);
    resolve_supported_map_with_plan_v1(key, member, plan, execution)
}

pub(super) fn resolve_with_lock_execution_v1(
    key: &DynamicClassKeyV1,
    member: StaticMemberSealV1,
    execution: LockRunnerExecutionReceiptV1,
) -> Result<RunnerAdmissionReceiptV1, RunnerAdmissionViolationV1> {
    let plan = compile_v1(key);
    resolve_supported_lock_with_plan_v1(key, member, plan, execution)
}

/// Preserve the planned-missing execution decision while a separately reviewed source-program
/// receipt authorizes only pre-manifest semantic cataloging. This function never returns
/// `Supported`; callers must already hold and consume the opaque program-catalog receipt.
pub(super) fn resolve_planned_for_program_catalog_v1(
    key: &DynamicClassKeyV1,
    member: StaticMemberSealV1,
) -> Result<RunnerAdmissionReceiptV1, RunnerAdmissionViolationV1> {
    let plan = compile_v1(key);
    let mut normalized = *key;
    normalized.recipe.capability = RunnerCapabilityV1::Missing(plan.expected_gap);
    resolve_with_plan_v1(&normalized, member, plan)
}

pub(super) fn inventory_v1(
    key: &DynamicClassKeyV1,
) -> Result<ExecutionProgramInventoryReceiptV1, ExecutionProgramInventoryViolationV1> {
    let plan = compile_v1(key);
    let mut normalized_key = *key;
    normalized_key.recipe.capability = RunnerCapabilityV1::Missing(plan.expected_gap);
    let status = match key.root {
        RootOperationV1::Map => {
            match map_program::implementation_for_inventory_v1(&normalized_key, plan)
                .map_err(|_| ExecutionProgramInventoryViolationV1::MapProgramLookupFailed)?
            {
                None => ExecutionProgramInventoryStatusV1::PlannedMissing(plan.expected_gap),
                Some(implementation_sha256) => {
                    ExecutionProgramInventoryStatusV1::SourcePresentReceiptRequired {
                        implementation_sha256,
                    }
                }
            }
        }
        RootOperationV1::Lock => {
            match lock_program::implementation_for_inventory_v1(&normalized_key, plan)
                .map_err(|_| ExecutionProgramInventoryViolationV1::LockProgramLookupFailed)?
            {
                None => ExecutionProgramInventoryStatusV1::PlannedMissing(plan.expected_gap),
                Some(implementation_sha256) => {
                    ExecutionProgramInventoryStatusV1::SourcePresentReceiptRequired {
                        implementation_sha256,
                    }
                }
            }
        }
    };
    Ok(ExecutionProgramInventoryReceiptV1 {
        normalized_key,
        normalized_descriptor_sha256: plan.normalized_descriptor_sha256,
        program_id: execution_program_id_v1(
            plan.root,
            plan.normalized_descriptor_sha256,
            plan.plan_sha256,
        ),
        plan_sha256: plan.plan_sha256,
        status,
    })
}

pub(super) fn execution_program_id_v1(
    root: RootOperationV1,
    normalized_descriptor_sha256: Digest32,
    plan_sha256: Digest32,
) -> Digest32 {
    canonical::digest_execution_program_id_v1(root, normalized_descriptor_sha256, plan_sha256)
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
        decision: RunnerAdmissionDecisionV1::Missing(plan.expected_gap),
    })
}

fn resolve_supported_map_with_plan_v1(
    key: &DynamicClassKeyV1,
    member: StaticMemberSealV1,
    plan: CompiledRunnerPlanV1,
    execution: MapRunnerExecutionReceiptV1,
) -> Result<RunnerAdmissionReceiptV1, RunnerAdmissionViolationV1> {
    if plan != compile_v1(key) {
        return Err(RunnerAdmissionViolationV1::PlanBindingMismatch);
    }
    if key.recipe.capability != RunnerCapabilityV1::Supported {
        return Err(RunnerAdmissionViolationV1::UnsealedSupportedClaim);
    }
    let validated = map_program::validate_execution_receipt_v1(key, member, plan, execution)
        .map_err(|_| RunnerAdmissionViolationV1::MapExecutionReceiptMismatch)?;
    Ok(RunnerAdmissionReceiptV1 {
        member,
        normalized_descriptor_sha256: plan.normalized_descriptor_sha256,
        plan_sha256: plan.plan_sha256,
        decision: RunnerAdmissionDecisionV1::Supported {
            implementation_sha256: validated.implementation_sha256(),
            execution_sha256: validated.execution_sha256(),
        },
    })
}

fn resolve_supported_lock_with_plan_v1(
    key: &DynamicClassKeyV1,
    member: StaticMemberSealV1,
    plan: CompiledRunnerPlanV1,
    execution: LockRunnerExecutionReceiptV1,
) -> Result<RunnerAdmissionReceiptV1, RunnerAdmissionViolationV1> {
    if plan != compile_v1(key) {
        return Err(RunnerAdmissionViolationV1::PlanBindingMismatch);
    }
    if key.recipe.capability != RunnerCapabilityV1::Supported {
        return Err(RunnerAdmissionViolationV1::UnsealedSupportedClaim);
    }
    let validated = lock_program::validate_execution_receipt_v1(key, member, plan, execution)
        .map_err(|_| RunnerAdmissionViolationV1::LockExecutionReceiptMismatch)?;
    Ok(RunnerAdmissionReceiptV1 {
        member,
        normalized_descriptor_sha256: plan.normalized_descriptor_sha256,
        plan_sha256: plan.plan_sha256,
        decision: RunnerAdmissionDecisionV1::Supported {
            implementation_sha256: validated.implementation_sha256(),
            execution_sha256: validated.execution_sha256(),
        },
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

#[cfg(test)]
pub(super) fn resolve_with_map_execution_for_test(
    key: &DynamicClassKeyV1,
    member: StaticMemberSealV1,
    plan: CompiledRunnerPlanV1,
    execution: MapRunnerExecutionReceiptV1,
) -> Result<RunnerAdmissionReceiptV1, RunnerAdmissionViolationV1> {
    resolve_supported_map_with_plan_v1(key, member, plan, execution)
}

#[cfg(test)]
pub(super) fn resolve_with_lock_execution_for_test(
    key: &DynamicClassKeyV1,
    member: StaticMemberSealV1,
    plan: CompiledRunnerPlanV1,
    execution: LockRunnerExecutionReceiptV1,
) -> Result<RunnerAdmissionReceiptV1, RunnerAdmissionViolationV1> {
    resolve_supported_lock_with_plan_v1(key, member, plan, execution)
}

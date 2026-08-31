//! Sealed admission bridge for the executable Map dynamic-quotient programs.

mod lifecycle;
mod region_loop;
mod request_budget;

#[cfg(windows)]
use std::fmt;

use sha2::{Digest, Sha256};

use super::super::super::{
    source_leaf_authority::Digest32, terminal_descriptor::RunnerCapabilityV1,
};
use super::super::{DynamicClassKeyV1, StaticMemberSealV1};
use super::CompiledRunnerPlanV1;
use lifecycle::{program_spec_v1 as lifecycle_program_spec_v1, MapLifecyclePathSpecV1};
use region_loop::{program_spec_v1 as region_loop_program_spec_v1, MapRegionLoopProgramV1};
use request_budget::{program_spec_v1 as request_budget_program_spec_v1, MapRequestBudgetGuardV1};

#[cfg(windows)]
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2_dynamic_evidence::{
    run_map_lifecycle_program_isolated, run_map_program_isolated,
    run_map_region_loop_program_isolated, MapRunnerEvidenceReceiptV1,
    MapRunnerIsolatedEvidenceV1, MapRunnerLifecycleBindingV1, MapRunnerLifecyclePathV1,
    MapRunnerModeV1, MapRunnerProgramBindingV1, MapRunnerRegionLoopBindingV1,
    MapRunnerRegionLoopFamilyV1, MapRunnerRequestBudgetV1,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProgramModeV1 {
    Observe,
    Extend,
}

#[derive(Clone, Copy)]
enum MapProgramCaseV1 {
    RequestBudget(MapRequestBudgetGuardV1),
    Lifecycle(MapLifecyclePathSpecV1),
    RegionLoop(MapRegionLoopProgramV1),
}

#[derive(Clone, Copy)]
struct MapProgramSpecV1 {
    mode: ProgramModeV1,
    case: MapProgramCaseV1,
    member: StaticMemberSealV1,
    normalized_descriptor_sha256: Digest32,
    plan_sha256: Digest32,
    implementation_sha256: Digest32,
}

/// A real execution receipt. Private fields and the absent public constructor prevent digest-only
/// callers from converting a capability declaration or compiled plan into execution authority.
pub(in super::super) struct MapRunnerExecutionReceiptV1 {
    normalized_descriptor_sha256: Digest32,
    member: StaticMemberSealV1,
    plan_sha256: Digest32,
    implementation_sha256: Digest32,
    root_commitment_sha256: Digest32,
    child_fingerprint_sha256: Digest32,
    registration_commitment_sha256: Digest32,
    payload_commitment_sha256: Digest32,
    environment_sha256: Digest32,
    cleanup_sha256: Digest32,
    native_receipt_sha256: Digest32,
    child_exit_code: i32,
    execution_sha256: Digest32,
}

pub(super) struct ValidatedMapRunnerExecutionV1 {
    implementation_sha256: Digest32,
    execution_sha256: Digest32,
}

impl ValidatedMapRunnerExecutionV1 {
    pub(super) const fn implementation_sha256(&self) -> Digest32 {
        self.implementation_sha256
    }

    pub(super) const fn execution_sha256(&self) -> Digest32 {
        self.execution_sha256
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MapRunnerExecutionViolationV1 {
    UnsupportedProgram,
    PlanBindingMismatch,
    MemberSealMismatch,
    MemberCatalogInvalid,
    ReceiptBindingMismatch,
    ExecutionSealMismatch,
}

#[cfg(windows)]
pub(in super::super) enum MapRunnerIsolatedOutcomeV1 {
    ParentReceipt(MapRunnerExecutionReceiptV1),
    ChildReported,
}

#[cfg(windows)]
#[derive(Debug)]
pub(in super::super) struct MapRunnerExecutionErrorV1(String);

#[cfg(windows)]
impl fmt::Display for MapRunnerExecutionErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(windows)]
impl std::error::Error for MapRunnerExecutionErrorV1 {}

pub(super) fn validate_execution_receipt_v1(
    key: &DynamicClassKeyV1,
    member: StaticMemberSealV1,
    plan: CompiledRunnerPlanV1,
    receipt: MapRunnerExecutionReceiptV1,
) -> Result<ValidatedMapRunnerExecutionV1, MapRunnerExecutionViolationV1> {
    let program = program_v1(key, member, plan)?;
    if receipt.normalized_descriptor_sha256 != program.normalized_descriptor_sha256
        || receipt.member != program.member
        || receipt.plan_sha256 != program.plan_sha256
        || receipt.implementation_sha256 != program.implementation_sha256
        || receipt.root_commitment_sha256 == Digest32::ZERO
        || receipt.child_fingerprint_sha256 == Digest32::ZERO
        || receipt.registration_commitment_sha256 == Digest32::ZERO
        || receipt.payload_commitment_sha256 == Digest32::ZERO
        || receipt.environment_sha256 == Digest32::ZERO
        || receipt.cleanup_sha256 == Digest32::ZERO
        || receipt.native_receipt_sha256 == Digest32::ZERO
        || receipt.child_exit_code != 0
    {
        return Err(MapRunnerExecutionViolationV1::ReceiptBindingMismatch);
    }
    let execution_sha256 = digest_execution_receipt_v1(&receipt);
    if receipt.execution_sha256 != execution_sha256 {
        return Err(MapRunnerExecutionViolationV1::ExecutionSealMismatch);
    }
    Ok(ValidatedMapRunnerExecutionV1 {
        implementation_sha256: receipt.implementation_sha256,
        execution_sha256,
    })
}

#[cfg(windows)]
pub(in super::super) fn run_isolated_for_test(
    exact_test: &str,
    key: &DynamicClassKeyV1,
    member: StaticMemberSealV1,
    plan: CompiledRunnerPlanV1,
) -> Result<MapRunnerIsolatedOutcomeV1, MapRunnerExecutionErrorV1> {
    let program = program_v1(key, member, plan)
        .map_err(|violation| MapRunnerExecutionErrorV1(format!("{violation:?}")))?;
    let evidence = match program.case {
        MapProgramCaseV1::RequestBudget(guard) => run_map_program_isolated(
            exact_test,
            MapRunnerProgramBindingV1 {
                mode: runner_mode_v1(program.mode),
                request_budget: match guard {
                    MapRequestBudgetGuardV1::RegionSize => MapRunnerRequestBudgetV1::RegionSize,
                    MapRequestBudgetGuardV1::RegionCount => MapRunnerRequestBudgetV1::RegionCount,
                    MapRequestBudgetGuardV1::LogicalSize => MapRunnerRequestBudgetV1::LogicalSize,
                },
                normalized_descriptor_sha256: program.normalized_descriptor_sha256.0,
                case_key_sha256: member.case_key_sha256.0,
                full_record_sha256: member.full_record_sha256.0,
                plan_sha256: program.plan_sha256.0,
                implementation_sha256: program.implementation_sha256.0,
            },
        ),
        MapProgramCaseV1::Lifecycle(path) => run_map_lifecycle_program_isolated(
            exact_test,
            MapRunnerLifecycleBindingV1 {
                path: match path {
                    MapLifecyclePathSpecV1::EmptyObserveNotPresent => {
                        MapRunnerLifecyclePathV1::EmptyObserveNotPresent
                    }
                    MapLifecyclePathSpecV1::EmptyExtendMapped => {
                        MapRunnerLifecyclePathV1::EmptyExtendMapped
                    }
                    MapLifecyclePathSpecV1::ReuseObserveMapped => {
                        MapRunnerLifecyclePathV1::ReuseObserveMapped
                    }
                    MapLifecyclePathSpecV1::ReuseExtendMapped => {
                        MapRunnerLifecyclePathV1::ReuseExtendMapped
                    }
                    MapLifecyclePathSpecV1::MissingObserveNotPresent => {
                        MapRunnerLifecyclePathV1::MissingObserveNotPresent
                    }
                    MapLifecyclePathSpecV1::MissingExtendMapped => {
                        MapRunnerLifecyclePathV1::MissingExtendMapped
                    }
                },
                normalized_descriptor_sha256: program.normalized_descriptor_sha256.0,
                case_key_sha256: member.case_key_sha256.0,
                full_record_sha256: member.full_record_sha256.0,
                plan_sha256: program.plan_sha256.0,
                implementation_sha256: program.implementation_sha256.0,
            },
        ),
        MapProgramCaseV1::RegionLoop(region_loop) => run_map_region_loop_program_isolated(
            exact_test,
            MapRunnerRegionLoopBindingV1 {
                family: match region_loop.family() {
                    region_loop::MapRegionLoopFamilyV1::EmptyExtend => {
                        MapRunnerRegionLoopFamilyV1::CreatedFirstEmptyExtendMapped
                    }
                    region_loop::MapRegionLoopFamilyV1::MissingExtend => {
                        MapRunnerRegionLoopFamilyV1::NodeLiveMissingExtendMapped
                    }
                },
                target_region: u32::from(region_loop.target_region()),
                regions_to_create: region_loop.regions_to_create(),
                normalized_descriptor_sha256: program.normalized_descriptor_sha256.0,
                case_key_sha256: member.case_key_sha256.0,
                full_record_sha256: member.full_record_sha256.0,
                plan_sha256: program.plan_sha256.0,
                implementation_sha256: program.implementation_sha256.0,
            },
        ),
    };
    match evidence.map_err(|error| MapRunnerExecutionErrorV1(error.to_string()))? {
        MapRunnerIsolatedEvidenceV1::ChildReported => Ok(MapRunnerIsolatedOutcomeV1::ChildReported),
        MapRunnerIsolatedEvidenceV1::ParentReceipt(evidence) => Ok(
            MapRunnerIsolatedOutcomeV1::ParentReceipt(seal_execution_receipt(program, evidence)),
        ),
    }
}

#[cfg(windows)]
const fn runner_mode_v1(mode: ProgramModeV1) -> MapRunnerModeV1 {
    match mode {
        ProgramModeV1::Observe => MapRunnerModeV1::Observe,
        ProgramModeV1::Extend => MapRunnerModeV1::Extend,
    }
}

#[cfg(all(test, windows))]
pub(in super::super) fn tamper_implementation_digest_for_test(
    receipt: &mut MapRunnerExecutionReceiptV1,
    digest: Digest32,
) {
    receipt.implementation_sha256 = digest;
}

pub(super) fn implementation_for_inventory_v1(
    key: &DynamicClassKeyV1,
    plan: CompiledRunnerPlanV1,
) -> Result<Option<Digest32>, MapRunnerExecutionViolationV1> {
    match source_program_spec_v1(key, plan) {
        Ok(program) => Ok(Some(program.implementation_sha256)),
        Err(MapRunnerExecutionViolationV1::UnsupportedProgram) => Ok(None),
        Err(error) => Err(error),
    }
}

fn program_v1(
    key: &DynamicClassKeyV1,
    member: StaticMemberSealV1,
    plan: CompiledRunnerPlanV1,
) -> Result<MapProgramSpecV1, MapRunnerExecutionViolationV1> {
    let program = source_program_spec_v1(key, plan)?;
    if key.recipe.capability != RunnerCapabilityV1::Supported {
        return Err(MapRunnerExecutionViolationV1::UnsupportedProgram);
    }
    if member != program.member {
        return Err(MapRunnerExecutionViolationV1::MemberSealMismatch);
    }
    Ok(program)
}

fn source_program_spec_v1(
    key: &DynamicClassKeyV1,
    plan: CompiledRunnerPlanV1,
) -> Result<MapProgramSpecV1, MapRunnerExecutionViolationV1> {
    match request_budget_program_spec_v1(key, plan) {
        Ok(program) => Ok(program),
        Err(MapRunnerExecutionViolationV1::UnsupportedProgram) => {
            match region_loop_program_spec_v1(key, plan) {
                Ok(program) => Ok(program),
                Err(MapRunnerExecutionViolationV1::UnsupportedProgram) => {
                    lifecycle_program_spec_v1(key, plan)
                }
                Err(error) => Err(error),
            }
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
pub(super) fn region_loop_catalog_row_count_for_test() -> usize {
    region_loop::catalog_row_count_for_test()
}

#[cfg(test)]
pub(super) fn validate_program_for_test(
    key: &DynamicClassKeyV1,
    member: StaticMemberSealV1,
    plan: CompiledRunnerPlanV1,
) -> Result<(), MapRunnerExecutionViolationV1> {
    program_v1(key, member, plan).map(|_| ())
}

#[cfg(windows)]
fn seal_execution_receipt(
    program: MapProgramSpecV1,
    evidence: MapRunnerEvidenceReceiptV1,
) -> MapRunnerExecutionReceiptV1 {
    let (root, child, registration, payload, environment, cleanup, native, exit_code) =
        evidence.into_bindings();
    let mut receipt = MapRunnerExecutionReceiptV1 {
        normalized_descriptor_sha256: program.normalized_descriptor_sha256,
        member: program.member,
        plan_sha256: program.plan_sha256,
        implementation_sha256: program.implementation_sha256,
        root_commitment_sha256: Digest32(root),
        child_fingerprint_sha256: Digest32(child),
        registration_commitment_sha256: Digest32(registration),
        payload_commitment_sha256: Digest32(payload),
        environment_sha256: Digest32(environment),
        cleanup_sha256: Digest32(cleanup),
        native_receipt_sha256: Digest32(native),
        child_exit_code: exit_code,
        execution_sha256: Digest32::ZERO,
    };
    receipt.execution_sha256 = digest_execution_receipt_v1(&receipt);
    receipt
}

fn digest_execution_receipt_v1(receipt: &MapRunnerExecutionReceiptV1) -> Digest32 {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-map-quotient-real-execution-v1\0");
    for digest in [
        receipt.normalized_descriptor_sha256,
        receipt.member.case_key_sha256,
        receipt.member.full_record_sha256,
        receipt.plan_sha256,
        receipt.implementation_sha256,
        receipt.root_commitment_sha256,
        receipt.child_fingerprint_sha256,
        receipt.registration_commitment_sha256,
        receipt.payload_commitment_sha256,
        receipt.environment_sha256,
        receipt.cleanup_sha256,
        receipt.native_receipt_sha256,
    ] {
        hasher.update(digest.0);
    }
    hasher.update(receipt.child_exit_code.to_le_bytes());
    Digest32(hasher.finalize().into())
}

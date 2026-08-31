//! Identity-erasing, fail-closed projection from static terminals to dynamic execution classes.

mod candidate;
mod canonical;
mod canonical_tags;
mod catalog;
mod descriptor_binding;
mod lock_native_acquire_busy_source_scope;
mod lock_stored_poison_source_scope;
mod manifest;
mod manifest_canonical;
mod map_runtime_source_scope;
mod membership_commitment;
mod model;
mod producer_coherence;
mod program_inventory;
mod program_inventory_canonical;
mod projector;
mod runner_admission;

pub(crate) use candidate::{
    build_lock_dynamic_candidate_v1, build_map_dynamic_candidate_v1, DynamicCandidateErrorV1,
};
use canonical::{
    digest_dynamic_class_key_v1, digest_dynamic_expected_v1,
    digest_normalized_descriptor_semantics_v1, digest_static_member_seal_v1,
};
pub(crate) use catalog::CatalogErrorV1;
use catalog::{DynamicCatalogBuilderV1, DynamicCatalogV1, DynamicClassV1, ProjectionFailureV1};
pub(crate) use manifest::DynamicManifestBundleV1;
use manifest::{
    build_dynamic_manifest_v1, DynamicClassSealV1, DynamicManifestContextV1,
    DynamicQuotientManifestV1, ManifestBuildErrorV1, ReverseIndexEntryV1,
};
use model::{
    DynamicAxesV1, DynamicClassKeyV1, DynamicExpectedV1, DynamicOperationV1, DynamicProjectionV1,
    StaticMemberSealV1, DYNAMIC_PROJECTOR_SCHEMA_V1,
};
pub(crate) use program_inventory::ProgramCatalogAdmissionErrorV1;
use program_inventory::{
    build_lock_execution_program_inventory_v1, build_map_execution_program_inventory_v1,
};
use projector::{
    prepare_dynamic_terminal_v1, project_dynamic_class_v1, project_validated_dynamic_terminal_v1,
    project_validated_dynamic_terminal_with_lock_execution_v1,
    project_validated_dynamic_terminal_with_map_execution_v1,
    project_validated_dynamic_terminal_with_program_catalog_v1, ProjectionErrorV1,
    ProjectionViolationV1,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExecutionProgramInventorySummaryV1 {
    pub(super) member_count: u64,
    pub(super) program_group_count: u64,
    pub(super) source_present_member_count: u64,
    pub(super) source_present_group_count: u64,
    pub(super) planned_missing_member_count: u64,
    pub(super) planned_missing_group_count: u64,
    pub(super) inventory_sha256: String,
}

pub(super) fn inspect_map_execution_program_inventory_v1(
    graph: &super::model::ContractGraph,
) -> Result<ExecutionProgramInventorySummaryV1, String> {
    let bundle =
        build_map_execution_program_inventory_v1(graph).map_err(|error| format!("{error:?}"))?;
    summarize_execution_program_inventory_v1(bundle)
}

pub(super) fn inspect_lock_execution_program_inventory_v1(
    graph: &super::model::ContractGraph,
) -> Result<ExecutionProgramInventorySummaryV1, String> {
    let bundle =
        build_lock_execution_program_inventory_v1(graph).map_err(|error| format!("{error:?}"))?;
    summarize_execution_program_inventory_v1(bundle)
}

fn summarize_execution_program_inventory_v1(
    bundle: program_inventory::ExecutionProgramInventoryBundleV1,
) -> Result<ExecutionProgramInventorySummaryV1, String> {
    let inventory = bundle.inventory;
    Ok(ExecutionProgramInventorySummaryV1 {
        member_count: inventory.member_count,
        program_group_count: inventory.program_group_count,
        source_present_member_count: inventory.source_present_member_count,
        source_present_group_count: inventory.source_present_group_count,
        planned_missing_member_count: inventory.planned_missing_member_count,
        planned_missing_group_count: inventory.planned_missing_group_count,
        inventory_sha256: inventory.inventory_sha256.to_lower_hex(),
    })
}

#[cfg(test)]
mod tests;

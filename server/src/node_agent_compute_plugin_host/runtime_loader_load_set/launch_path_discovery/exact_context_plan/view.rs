//! Narrow borrowed view used only by a future authenticated grant-ready resolver.

use super::{
    PreliminaryResolutionRequestsPlannedWork, WindowsPreliminaryContentLeaseRequestRef,
    WindowsPreliminaryImportEdgeKind, WindowsPreliminaryLaunchPathComponentRequest,
    WindowsPreliminaryModuleEdgeLocator, WindowsPreliminaryModuleResolutionRequest,
    WindowsPreliminarySearchDirectoryBinding, WindowsPreliminarySearchDirectoryTarget,
};

use super::super::prelease_pe_material::AuthenticatedWindowsPreLeasePeMaterial;

use crate::node_agent_compute_plugin_host::runtime_loader_load_set::model::WindowsLoaderWorkingDirectoryLocation;

pub(in super::super::super) struct PreliminaryWindowsRunnerSelectedContextView<'plan> {
    pub(in super::super::super) working_directory_location: WindowsLoaderWorkingDirectoryLocation,
    pub(in super::super::super) working_directory_relative_path: &'plan str,
    pub(in super::super::super) working_directory_identity_digest: &'plan str,
    pub(in super::super::super) working_directory_component_set_digest: &'plan str,
    pub(in super::super::super) working_directory_observation_receipt_digest: &'plan str,
    pub(in super::super::super) application_identity_digest: &'plan str,
    pub(in super::super::super) application_component_set_digest: &'plan str,
    pub(in super::super::super) application_observation_receipt_digest: &'plan str,
    pub(in super::super::super) application_directory_identity_digest: &'plan str,
    pub(in super::super::super) application_directory_location:
        super::WindowsPreliminaryRetainedDirectoryLocation,
    pub(in super::super::super) application_directory_component_set_digest: &'plan str,
    pub(in super::super::super) application_directory_observation_receipt_digest: &'plan str,
    pub(in super::super::super) context_intent_digest: &'plan str,
    pub(in super::super::super) selection_binding_digest: &'plan str,
}

pub(in super::super::super) struct PreliminaryWindowsRunnerResolutionRequestPlanView<'plan> {
    pe_material: &'plan AuthenticatedWindowsPreLeasePeMaterial,
    pub(in super::super::super) admission_source_digest: &'plan str,
    pub(in super::super::super) admission_receipt_digest: &'plan str,
    pub(in super::super::super) extraction_plan_digest: &'plan str,
    pub(in super::super::super) extraction_evidence_digest: &'plan str,
    pub(in super::super::super) launch_path_candidate_set_digest: &'plan str,
    pub(in super::super::super) selected_context:
        PreliminaryWindowsRunnerSelectedContextView<'plan>,
    pub(in super::super::super) prelease_pe_material_digest: &'plan str,
    pub(in super::super::super) parser_policy_digest: &'plan str,
    pub(in super::super::super) authenticated_preloaded_module_set_digest: &'plan str,
    pub(in super::super::super) preliminary_request_plan_digest: &'plan str,
    pub(in super::super::super) resolution_route_order: &'plan [String],
    pub(in super::super::super) search_directories:
        &'plan [WindowsPreliminarySearchDirectoryBinding],
    pub(in super::super::super) module_resolution_requests:
        &'plan [WindowsPreliminaryModuleResolutionRequest],
    pub(in super::super::super) launch_path_component_requests:
        &'plan [WindowsPreliminaryLaunchPathComponentRequest],
    pub(in super::super::super) content_lease_requests:
        &'plan [WindowsPreliminaryContentLeaseRequestRef],
}

impl PreliminaryResolutionRequestsPlannedWork<'_> {
    pub(in super::super::super) fn borrow_resolution_request_plan(
        &self,
    ) -> PreliminaryWindowsRunnerResolutionRequestPlanView<'_> {
        PreliminaryWindowsRunnerResolutionRequestPlanView {
            pe_material: &self.pe_material,
            admission_source_digest: &self.plan.admission_source_digest,
            admission_receipt_digest: &self.plan.admission_receipt_digest,
            extraction_plan_digest: &self.plan.extraction_plan_digest,
            extraction_evidence_digest: &self.plan.extraction_evidence_digest,
            launch_path_candidate_set_digest: &self.plan.launch_path_candidate_set_digest,
            selected_context: PreliminaryWindowsRunnerSelectedContextView {
                working_directory_location: self.plan.selected_context.working_directory_location,
                working_directory_relative_path: &self
                    .plan
                    .selected_context
                    .working_directory_relative_path,
                working_directory_identity_digest: &self
                    .plan
                    .selected_context
                    .working_directory_identity_digest,
                working_directory_component_set_digest: &self
                    .plan
                    .selected_context
                    .working_directory_component_set_digest,
                working_directory_observation_receipt_digest: &self
                    .plan
                    .selected_context
                    .working_directory_observation_receipt_digest,
                application_identity_digest: &self
                    .plan
                    .selected_context
                    .application_identity_digest,
                application_component_set_digest: &self
                    .plan
                    .selected_context
                    .application_component_set_digest,
                application_observation_receipt_digest: &self
                    .plan
                    .selected_context
                    .application_observation_receipt_digest,
                application_directory_identity_digest: &self
                    .plan
                    .selected_context
                    .application_directory_identity_digest,
                application_directory_location: self
                    .plan
                    .selected_context
                    .application_directory_location,
                application_directory_component_set_digest: &self
                    .plan
                    .selected_context
                    .application_directory_component_set_digest,
                application_directory_observation_receipt_digest: &self
                    .plan
                    .selected_context
                    .application_directory_observation_receipt_digest,
                context_intent_digest: &self.plan.selected_context.context_intent_digest,
                selection_binding_digest: &self.plan.selected_context.selection_binding_digest,
            },
            prelease_pe_material_digest: &self.plan.prelease_pe_material_digest,
            parser_policy_digest: &self.plan.parser_policy_digest,
            authenticated_preloaded_module_set_digest: &self
                .plan
                .authenticated_preloaded_module_set_digest,
            preliminary_request_plan_digest: &self.plan.preliminary_request_plan_digest,
            resolution_route_order: &self.plan.resolution_route_order,
            search_directories: &self.plan.search_directories,
            module_resolution_requests: &self.plan.module_resolution_requests,
            launch_path_component_requests: &self.plan.launch_path_component_requests,
            content_lease_requests: &self.plan.content_lease_requests,
        }
    }
}

impl PreliminaryWindowsRunnerResolutionRequestPlanView<'_> {
    pub(in super::super::super) fn package_image_count(&self) -> usize {
        self.pe_material.package_images().len()
    }

    pub(in super::super::super) fn package_image_binding(
        &self,
        parsed_image_ordinal: usize,
    ) -> Option<(usize, usize, &str, &str, &str, u64)> {
        self.pe_material
            .package_images()
            .get(parsed_image_ordinal)
            .map(|image| {
                (
                    image.parsed_image_ordinal(),
                    image.package_file_ordinal(),
                    image.normalized_module_name(),
                    image.file_identity_digest(),
                    image.sealed_file_digest(),
                    image.size_bytes(),
                )
            })
    }
}

impl WindowsPreliminarySearchDirectoryBinding {
    pub(in super::super::super) fn request_binding(
        &self,
    ) -> (usize, &str, &WindowsPreliminarySearchDirectoryTarget, &str) {
        (
            self.search_step_ordinal,
            &self.role,
            &self.target,
            &self.binding_digest,
        )
    }
}

impl WindowsPreliminaryModuleResolutionRequest {
    pub(in super::super::super) fn request_binding(
        &self,
    ) -> (
        usize,
        usize,
        &WindowsPreliminaryModuleEdgeLocator,
        usize,
        usize,
        WindowsPreliminaryImportEdgeKind,
        &str,
        Option<&str>,
        Option<u16>,
        &[usize],
        &str,
    ) {
        (
            self.request_ordinal,
            self.global_import_edge_ordinal,
            &self.edge_locator,
            self.importer_graph_edge_ordinal,
            self.importer_image_ordinal,
            self.import_kind,
            &self.normalized_name,
            self.imported_symbol_name.as_deref(),
            self.imported_symbol_ordinal,
            &self.ordered_search_step_ordinals,
            self.grant_ready_resolution_status,
        )
    }
}

impl WindowsPreliminaryLaunchPathComponentRequest {
    pub(in super::super::super) fn request_binding(
        &self,
    ) -> (usize, &str, usize, &str, &str, &str) {
        (
            self.request_ordinal,
            self.path_kind,
            self.component_ordinal,
            &self.parent_identity_digest,
            &self.normalized_component,
            &self.expected_object_identity_digest,
        )
    }
}

impl WindowsPreliminaryContentLeaseRequestRef {
    pub(in super::super::super) fn package_file_ordinal(&self) -> usize {
        match self {
            Self::PackageFile {
                package_file_ordinal,
            } => *package_file_ordinal,
        }
    }
}

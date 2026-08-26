//! Domain-separated digests for exact selection and preliminary resolution requests.

use sha2::{Digest, Sha256};

use crate::node_agent_compute_plugin_host::runtime_loader_load_set::model::WindowsLoaderWorkingDirectoryLocation;

use super::{
    AuthenticatedWindowsRunnerWorkingDirectorySelector,
    PreliminaryWindowsRunnerResolutionRequestPlan, WindowsPreliminaryContentLeaseRequestRef,
    WindowsPreliminaryImportEdgeKind, WindowsPreliminaryModuleEdgeLocator,
    WindowsPreliminarySearchDirectoryRole,
};

impl PreliminaryWindowsRunnerResolutionRequestPlan {
    pub(super) fn recompute_digest(&self) -> String {
        let mut digest =
            PlanDigest::new(b"ELON_WINDOWS_RUNNER_PRELIMINARY_RESOLUTION_REQUEST_PLAN_V1");
        for value in [
            &self.admission_source_digest,
            &self.admission_receipt_digest,
            &self.extraction_plan_digest,
            &self.extraction_evidence_digest,
            &self.launch_path_candidate_set_digest,
            &self.selected_context.selection_binding_digest,
            &self.prelease_pe_material_digest,
            &self.parser_policy_digest,
            &self.authenticated_preloaded_module_set_digest,
        ] {
            digest.text(value);
        }
        for route in &self.resolution_route_order {
            digest.text(route);
        }
        for directory in &self.search_directories {
            digest.text(&directory.binding_digest);
        }
        for request in &self.module_resolution_requests {
            digest.usize(request.request_ordinal);
            digest.usize(request.global_import_edge_ordinal);
            match &request.edge_locator {
                WindowsPreliminaryModuleEdgeLocator::Import {
                    source_import_edge_ordinal,
                    descriptor_ordinal,
                    thunk_ordinal,
                    edge_evidence_digest,
                } => {
                    digest.text("import");
                    digest.usize(*source_import_edge_ordinal);
                    digest.usize(*descriptor_ordinal);
                    digest.usize(*thunk_ordinal);
                    digest.text(edge_evidence_digest);
                }
                WindowsPreliminaryModuleEdgeLocator::Forwarder {
                    source_import_edge_ordinal,
                    forwarder_hop_ordinal,
                    source_export_name,
                    source_export_ordinal,
                    hop_evidence_digest,
                } => {
                    digest.text("forwarder");
                    digest.usize(*source_import_edge_ordinal);
                    digest.usize(*forwarder_hop_ordinal);
                    digest.optional_text(source_export_name.as_deref());
                    digest.optional_u16(*source_export_ordinal);
                    digest.text(hop_evidence_digest);
                }
            }
            digest.usize(request.importer_graph_edge_ordinal);
            digest.usize(request.importer_image_ordinal);
            digest.text(match request.import_kind {
                WindowsPreliminaryImportEdgeKind::Normal => "normal",
                WindowsPreliminaryImportEdgeKind::Delay => "delay",
                WindowsPreliminaryImportEdgeKind::Forwarder => "forwarder",
            });
            digest.text(&request.normalized_name);
            digest.optional_text(request.imported_symbol_name.as_deref());
            digest.optional_u16(request.imported_symbol_ordinal);
            for search_step_ordinal in &request.ordered_search_step_ordinals {
                digest.usize(*search_step_ordinal);
            }
            digest.text(request.grant_ready_resolution_status);
        }
        for request in &self.launch_path_component_requests {
            digest.usize(request.request_ordinal);
            digest.text(request.path_kind);
            digest.usize(request.component_ordinal);
            digest.text(&request.parent_identity_digest);
            digest.text(&request.normalized_component);
            digest.text(&request.expected_object_identity_digest);
        }
        for request in &self.content_lease_requests {
            match request {
                WindowsPreliminaryContentLeaseRequestRef::PackageFile {
                    package_file_ordinal,
                } => {
                    digest.text("package_file");
                    digest.usize(*package_file_ordinal);
                }
            }
        }
        digest.finish()
    }
}

impl WindowsPreliminarySearchDirectoryRole {
    pub(super) fn unique_key(&self) -> String {
        match self {
            Self::ApplicationDirectory => "application_directory".to_owned(),
            Self::CurrentDirectory => "current_directory".to_owned(),
            Self::PackageRoot => "package_root".to_owned(),
            Self::PlanDirectory { directory_ordinal } => {
                format!("plan_directory:{directory_ordinal}")
            }
            Self::SystemDirectory => "system_directory".to_owned(),
            Self::WindowsDirectory => "windows_directory".to_owned(),
            Self::SideBySideAssemblyDirectory => "side_by_side_assembly_directory".to_owned(),
        }
    }

    pub(super) fn policy_phase(&self) -> u8 {
        match self {
            Self::ApplicationDirectory => 0,
            Self::PackageRoot | Self::PlanDirectory { .. } => 1,
            Self::SideBySideAssemblyDirectory => 2,
            Self::SystemDirectory => 3,
            Self::WindowsDirectory => 4,
            Self::CurrentDirectory => 5,
        }
    }
}

pub(super) fn working_directory_location_name(
    value: &WindowsLoaderWorkingDirectoryLocation,
) -> &'static str {
    match value {
        WindowsLoaderWorkingDirectoryLocation::PackageRoot => "package_root",
        WindowsLoaderWorkingDirectoryLocation::PlanDirectory { .. } => "plan_directory",
    }
}

pub(super) struct PlanDigest(Sha256);

impl PlanDigest {
    pub(super) fn new(domain: &[u8]) -> Self {
        let mut value = Sha256::new();
        value.update(domain);
        Self(value)
    }

    pub(super) fn text(&mut self, value: &str) {
        self.0.update((value.len() as u64).to_le_bytes());
        self.0.update(value.as_bytes());
    }

    pub(super) fn optional_text(&mut self, value: Option<&str>) {
        match value {
            Some(value) => {
                self.0.update([1]);
                self.text(value);
            }
            None => self.0.update([0]),
        }
    }

    pub(super) fn optional_u16(&mut self, value: Option<u16>) {
        match value {
            Some(value) => {
                self.0.update([1]);
                self.0.update(value.to_le_bytes());
            }
            None => self.0.update([0]),
        }
    }

    pub(super) fn optional_usize(&mut self, value: Option<usize>) {
        match value {
            Some(value) => {
                self.0.update([1]);
                self.usize(value);
            }
            None => self.0.update([0]),
        }
    }

    pub(super) fn usize(&mut self, value: usize) {
        self.0.update((value as u64).to_le_bytes());
    }

    pub(super) fn u64(&mut self, value: u64) {
        self.0.update(value.to_le_bytes());
    }

    pub(super) fn boolean(&mut self, value: bool) {
        self.0.update([u8::from(value)]);
    }

    pub(super) fn selector(
        &mut self,
        selector: &AuthenticatedWindowsRunnerWorkingDirectorySelector,
    ) {
        match selector {
            AuthenticatedWindowsRunnerWorkingDirectorySelector::PackageRoot => {
                self.text("package_root");
            }
            AuthenticatedWindowsRunnerWorkingDirectorySelector::PlanDirectory {
                directory_ordinal,
                relative_path,
            } => {
                self.text("plan_directory");
                self.usize(*directory_ordinal);
                self.text(relative_path);
            }
        }
    }

    pub(super) fn finish(self) -> String {
        hex::encode(self.0.finalize())
    }
}

//! Domain-separated canonical digests for the private grant-ready plan.

use anyhow::{bail, Result};
use sha2::{Digest, Sha256};

use super::*;

impl GrantReadyWindowsRunnerResolutionPlan {
    pub(super) fn validate_digests(
        &self,
        preliminary: &PreliminaryWindowsRunnerResolutionRequestPlanView<'_>,
    ) -> Result<()> {
        let terminal_set = self.recompute_terminal_set_digest();
        let disposition_set = self.recompute_disposition_set_digest();
        let external_set = self.recompute_external_directory_set_digest();
        let system_request_set = self.recompute_system_request_set_digest();
        let plan = self.recompute_plan_digest(
            preliminary.preliminary_request_plan_digest,
            &terminal_set,
            &disposition_set,
            &external_set,
            &system_request_set,
        );
        if terminal_set != self.exact_terminal_resolution_set_digest
            || disposition_set != self.exact_searched_name_disposition_set_digest
            || external_set != self.external_directory_authority_set_digest
            || system_request_set != self.resolved_system_image_request_set_digest
            || plan != self.grant_ready_resolution_plan_digest
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_GRANT_READY_DIGEST_CHANGED");
        }
        Ok(())
    }

    fn recompute_terminal_set_digest(&self) -> String {
        let mut digest = GrantDigest::new(b"ELON_WINDOWS_GRANT_READY_TERMINAL_SET_V1");
        for entry in &self.preloaded_terminal_authorities {
            digest.text("preloaded");
            digest.usize(entry.authority_record_ordinal);
            digest.usize(entry.preloaded_module_ordinal);
            digest.text(&entry.module_cache_key);
            digest.text(&entry.component_identity_digest);
            digest.text(&entry.immutable_section_identity_digest);
            digest.text(&entry.authenticated_evidence_digest);
        }
        for entry in &self.known_dll_terminal_authorities {
            digest.text("known_dll");
            digest.usize(entry.authority_record_ordinal);
            digest.text(&entry.module_cache_key);
            digest.text(&entry.section_identity_digest);
            digest.text(&entry.component_identity_digest);
            digest.text(&entry.immutable_section_identity_digest);
            digest.text(&entry.section_image_mapping_receipt_digest);
            digest.text(&entry.section_namespace_generation_digest);
        }
        for entry in &self.api_set_resolutions {
            digest.text("api_set");
            digest.usize(entry.api_set_resolution_ordinal);
            digest.text(&entry.normalized_contract_name);
            digest.text(&entry.normalized_host_module_cache_key);
            digest.non_recursive_terminal(&entry.host_terminal);
            digest.text(&entry.os_build_identity_digest);
            digest.text(&entry.schema_identity_digest);
            digest.text(&entry.contract_host_binding_set_digest);
            digest.text(&entry.resolution_binding_digest);
        }
        for entry in &self.module_resolutions {
            digest.usize(entry.request_ordinal);
            digest.usize(entry.global_import_edge_ordinal);
            digest.edge_locator(&entry.edge_locator);
            digest.usize(entry.importer_graph_edge_ordinal);
            digest.usize(entry.importer_parsed_image_ordinal);
            digest.import_kind(entry.import_kind);
            digest.text(&entry.normalized_requested_name);
            digest.optional_text(entry.imported_symbol_name.as_deref());
            digest.optional_u16(entry.imported_symbol_ordinal);
            digest.terminal(&entry.terminal);
            digest.text(&entry.resolution_binding_digest);
        }
        digest.finish()
    }

    fn recompute_disposition_set_digest(&self) -> String {
        let mut digest = GrantDigest::new(b"ELON_WINDOWS_GRANT_READY_DISPOSITION_SET_V1");
        for entry in &self.searched_name_dispositions {
            digest.usize(entry.searched_name_ordinal);
            digest.usize(entry.module_request_ordinal);
            digest.usize(entry.step_position);
            digest.usize(entry.search_step_ordinal);
            digest.text(&entry.normalized_searched_name);
            match &entry.disposition {
                WindowsGrantReadySearchedNameDisposition::MustRemainAbsent => digest.text("absent"),
                WindowsGrantReadySearchedNameDisposition::ShadowedByEarlierName {
                    earlier_searched_name_ordinal,
                } => {
                    digest.text("shadowed");
                    digest.usize(*earlier_searched_name_ordinal);
                }
                WindowsGrantReadySearchedNameDisposition::Terminal { terminal } => {
                    digest.text("terminal");
                    digest.terminal(terminal);
                }
            }
            digest.text(&entry.grant_request_digest);
            digest.text(&entry.disposition_binding_digest);
        }
        digest.finish()
    }

    fn recompute_external_directory_set_digest(&self) -> String {
        let mut digest = GrantDigest::new(b"ELON_WINDOWS_GRANT_READY_DIRECTORY_SET_V1");
        for step in &self.search_directories {
            digest.usize(step.search_step_ordinal);
            digest.text(&step.role);
            match &step.target {
                WindowsGrantReadySearchDirectoryTarget::RetainedPreliminaryCandidate {
                    location,
                    preliminary_search_step_ordinal,
                    preliminary_binding_digest,
                } => {
                    digest.text("retained");
                    match location {
                        WindowsPreliminaryRetainedDirectoryLocation::PackageRoot => {
                            digest.text("package_root")
                        }
                        WindowsPreliminaryRetainedDirectoryLocation::PlanDirectory {
                            directory_ordinal,
                        } => {
                            digest.text("plan_directory");
                            digest.usize(*directory_ordinal);
                        }
                    }
                    digest.usize(*preliminary_search_step_ordinal);
                    digest.text(preliminary_binding_digest);
                }
                WindowsGrantReadySearchDirectoryTarget::ExternalDirectory {
                    external_owner_ordinal,
                } => {
                    digest.text("external");
                    digest.usize(*external_owner_ordinal);
                }
            }
            digest.text(&step.authority_binding_digest);
        }
        digest.finish()
    }

    fn recompute_system_request_set_digest(&self) -> String {
        let mut digest = GrantDigest::new(b"ELON_WINDOWS_GRANT_READY_SYSTEM_REQUEST_SET_V1");
        for request in &self.resolved_filesystem_system_image_requests {
            digest.usize(request.resolution_request_ordinal);
            digest.usize(request.canonical_dedupe_ordinal);
            digest.usize(request.candidate_owner_ordinal);
            digest.usize(request.primary_use_ordinal);
            digest.text(&request.normalized_name);
            digest.text(&request.search_directory_authority_binding_digest);
            digest.text(&request.resolved_component_identity_digest);
            digest.text(&request.expected_file_identity_digest);
            digest.text(&request.concrete_servicing_generation_digest);
            digest.text(&request.code_integrity_evidence_digest);
            digest.text(&request.servicing_resolution_receipt_digest);
            digest.text(&request.namespace_alias_currentness_receipt_digest);
            digest.text(&request.candidate_binding_digest);
            for usage in &request.uses {
                digest.usize(usage.module_request_ordinal);
                digest.usize(usage.searched_name_ordinal);
                digest.usize(usage.search_step_ordinal);
                digest.text(&usage.normalized_searched_name);
                digest.text(&usage.search_directory_authority_binding_digest);
                digest.text(match usage.route {
                    WindowsGrantReadySystemImageUseRoute::OrdinaryFilesystem => "filesystem",
                    WindowsGrantReadySystemImageUseRoute::SideBySide => "side_by_side",
                });
            }
            digest.text(&request.lease_request_digest);
        }
        digest.finish()
    }

    fn recompute_plan_digest(
        &self,
        preliminary_request_plan_digest: &str,
        terminal_set: &str,
        disposition_set: &str,
        external_set: &str,
        system_request_set: &str,
    ) -> String {
        let mut digest = GrantDigest::new(b"ELON_WINDOWS_GRANT_READY_RESOLUTION_PLAN_V1");
        for value in [
            preliminary_request_plan_digest,
            terminal_set,
            disposition_set,
            external_set,
            system_request_set,
        ] {
            digest.text(value);
        }
        digest.finish()
    }
}

pub(super) fn recompute_system_image_lease_request_digest(
    request: &WindowsGrantReadyResolvedFilesystemSystemImageRequest,
) -> String {
    let mut digest = GrantDigest::new(b"ELON_WINDOWS_SYSTEM_IMAGE_CONTENT_LEASE_REQUEST_V1");
    digest.usize(request.resolution_request_ordinal);
    digest.usize(request.canonical_dedupe_ordinal);
    digest.usize(request.candidate_owner_ordinal);
    digest.text(&request.normalized_name);
    digest.text(&request.search_directory_authority_binding_digest);
    digest.text(&request.resolved_component_identity_digest);
    digest.text(&request.expected_file_identity_digest);
    digest.text(&request.concrete_servicing_generation_digest);
    digest.text(&request.code_integrity_evidence_digest);
    digest.text(&request.servicing_resolution_receipt_digest);
    digest.text(&request.namespace_alias_currentness_receipt_digest);
    digest.text(&request.candidate_binding_digest);
    for usage in &request.uses {
        digest.usize(usage.module_request_ordinal);
        digest.usize(usage.searched_name_ordinal);
        digest.usize(usage.search_step_ordinal);
        digest.text(&usage.normalized_searched_name);
        digest.text(&usage.search_directory_authority_binding_digest);
        digest.text(match usage.route {
            WindowsGrantReadySystemImageUseRoute::OrdinaryFilesystem => "filesystem",
            WindowsGrantReadySystemImageUseRoute::SideBySide => "side_by_side",
        });
    }
    digest.finish()
}

pub(super) fn recompute_external_directory_authority_digest(
    owner: &WindowsGrantReadyExternalSearchDirectoryCustody,
) -> String {
    let mut digest = GrantDigest::new(b"ELON_WINDOWS_EXTERNAL_SEARCH_DIRECTORY_AUTHORITY_V1");
    digest.usize(owner.external_owner_ordinal);
    digest.usize(owner.search_step_ordinal);
    let binding = owner.directory.path_currentness_binding();
    for value in [
        binding.0,
        binding.1,
        binding.2,
        binding.3,
        binding.4,
        binding.5,
        binding.6,
        &owner.handle_chain_authority_digest,
        &owner.namespace_alias_currentness_receipt_digest,
    ] {
        digest.text(value);
    }
    digest.finish()
}

pub(super) fn recompute_searched_name_disposition_binding_digest(
    searched: &WindowsGrantReadySearchedNameDispositionRecord,
    directory_authority_binding_digest: &str,
) -> String {
    let mut digest = GrantDigest::new(b"ELON_WINDOWS_SEARCHED_NAME_DISPOSITION_BINDING_V1");
    digest.usize(searched.searched_name_ordinal);
    digest.usize(searched.module_request_ordinal);
    digest.usize(searched.step_position);
    digest.usize(searched.search_step_ordinal);
    digest.text(directory_authority_binding_digest);
    digest.text(&searched.normalized_searched_name);
    match &searched.disposition {
        WindowsGrantReadySearchedNameDisposition::MustRemainAbsent => digest.text("absent"),
        WindowsGrantReadySearchedNameDisposition::ShadowedByEarlierName {
            earlier_searched_name_ordinal,
        } => {
            digest.text("shadowed");
            digest.usize(*earlier_searched_name_ordinal);
        }
        WindowsGrantReadySearchedNameDisposition::Terminal { terminal } => {
            digest.text("terminal");
            digest.terminal(terminal);
        }
    }
    digest.finish()
}

pub(super) fn recompute_name_grant_request_digest(
    searched: &WindowsGrantReadySearchedNameDispositionRecord,
    directory_authority_binding_digest: &str,
) -> String {
    let mut digest = GrantDigest::new(b"ELON_WINDOWS_SEARCHED_NAME_GRANT_REQUEST_V1");
    digest.text(directory_authority_binding_digest);
    digest.text(&searched.normalized_searched_name);
    digest.text(&searched.disposition_binding_digest);
    digest.finish()
}

pub(super) fn recompute_api_set_resolution_binding_digest(
    api: &WindowsGrantReadyApiSetResolution,
) -> String {
    let mut digest = GrantDigest::new(b"ELON_WINDOWS_API_SET_RESOLUTION_BINDING_V1");
    digest.usize(api.api_set_resolution_ordinal);
    digest.text(&api.normalized_contract_name);
    digest.text(&api.normalized_host_module_cache_key);
    digest.non_recursive_terminal(&api.host_terminal);
    digest.text(&api.os_build_identity_digest);
    digest.text(&api.schema_identity_digest);
    digest.text(&api.contract_host_binding_set_digest);
    digest.finish()
}

pub(super) fn recompute_module_resolution_binding_digest(
    module: &WindowsGrantReadyModuleResolution,
) -> String {
    let mut digest = GrantDigest::new(b"ELON_WINDOWS_MODULE_RESOLUTION_BINDING_V1");
    digest.usize(module.request_ordinal);
    digest.usize(module.global_import_edge_ordinal);
    digest.edge_locator(&module.edge_locator);
    digest.usize(module.importer_graph_edge_ordinal);
    digest.usize(module.importer_parsed_image_ordinal);
    digest.import_kind(module.import_kind);
    digest.text(&module.normalized_requested_name);
    digest.optional_text(module.imported_symbol_name.as_deref());
    digest.optional_u16(module.imported_symbol_ordinal);
    digest.terminal(&module.terminal);
    digest.finish()
}

struct GrantDigest(Sha256);

impl GrantDigest {
    fn new(domain: &[u8]) -> Self {
        let mut digest = Sha256::new();
        digest.update(domain);
        Self(digest)
    }

    fn text(&mut self, value: &str) {
        self.0.update((value.len() as u64).to_le_bytes());
        self.0.update(value.as_bytes());
    }

    fn usize(&mut self, value: usize) {
        self.0.update((value as u64).to_le_bytes());
    }

    fn optional_text(&mut self, value: Option<&str>) {
        match value {
            Some(value) => {
                self.0.update([1]);
                self.text(value);
            }
            None => self.0.update([0]),
        }
    }

    fn optional_u16(&mut self, value: Option<u16>) {
        match value {
            Some(value) => {
                self.0.update([1]);
                self.0.update(value.to_le_bytes());
            }
            None => self.0.update([0]),
        }
    }

    fn optional_usize(&mut self, value: Option<usize>) {
        match value {
            Some(value) => {
                self.0.update([1]);
                self.usize(value);
            }
            None => self.0.update([0]),
        }
    }

    fn import_kind(&mut self, value: WindowsGrantReadyImportEdgeKind) {
        self.text(match value {
            WindowsGrantReadyImportEdgeKind::Normal => "normal",
            WindowsGrantReadyImportEdgeKind::Delay => "delay",
            WindowsGrantReadyImportEdgeKind::Forwarder => "forwarder",
        });
    }

    fn edge_locator(&mut self, value: &WindowsPreliminaryModuleEdgeLocator) {
        match value {
            WindowsPreliminaryModuleEdgeLocator::Import {
                source_import_edge_ordinal,
                descriptor_ordinal,
                thunk_ordinal,
                edge_evidence_digest,
            } => {
                self.text("import");
                self.usize(*source_import_edge_ordinal);
                self.usize(*descriptor_ordinal);
                self.usize(*thunk_ordinal);
                self.text(edge_evidence_digest);
            }
            WindowsPreliminaryModuleEdgeLocator::Forwarder {
                source_import_edge_ordinal,
                forwarder_hop_ordinal,
                source_export_name,
                source_export_ordinal,
                hop_evidence_digest,
            } => {
                self.text("forwarder");
                self.usize(*source_import_edge_ordinal);
                self.usize(*forwarder_hop_ordinal);
                self.optional_text(source_export_name.as_deref());
                self.optional_u16(*source_export_ordinal);
                self.text(hop_evidence_digest);
            }
        }
    }

    fn non_recursive_terminal(&mut self, value: &WindowsGrantReadyNonRecursiveModuleTerminalRef) {
        match value {
            WindowsGrantReadyNonRecursiveModuleTerminalRef::AuthenticatedPreloaded {
                preloaded_authority_record_ordinal,
            } => {
                self.text("preloaded");
                self.usize(*preloaded_authority_record_ordinal);
            }
            WindowsGrantReadyNonRecursiveModuleTerminalRef::PackageFile {
                package_file_ordinal,
                parsed_image_ordinal,
            } => {
                self.text("package");
                self.usize(*package_file_ordinal);
                self.usize(*parsed_image_ordinal);
            }
            WindowsGrantReadyNonRecursiveModuleTerminalRef::KnownDllSection {
                known_dll_authority_record_ordinal,
            } => {
                self.text("known_dll");
                self.usize(*known_dll_authority_record_ordinal);
            }
            WindowsGrantReadyNonRecursiveModuleTerminalRef::ResolvedFilesystemSystemImage {
                resolution_request_ordinal,
            } => {
                self.text("filesystem");
                self.usize(*resolution_request_ordinal);
            }
            WindowsGrantReadyNonRecursiveModuleTerminalRef::SideBySideSystemImage {
                resolution_request_ordinal,
            } => {
                self.text("side_by_side");
                self.usize(*resolution_request_ordinal);
            }
        }
    }

    fn terminal(&mut self, value: &WindowsGrantReadyModuleTerminalRef) {
        match value {
            WindowsGrantReadyModuleTerminalRef::NonRecursive(value) => {
                self.text("non_recursive");
                self.non_recursive_terminal(value);
            }
            WindowsGrantReadyModuleTerminalRef::ApiSetResolution {
                api_set_resolution_ordinal,
            } => {
                self.text("api_set");
                self.usize(*api_set_resolution_ordinal);
            }
        }
    }

    fn finish(self) -> String {
        hex::encode(self.0.finalize())
    }
}

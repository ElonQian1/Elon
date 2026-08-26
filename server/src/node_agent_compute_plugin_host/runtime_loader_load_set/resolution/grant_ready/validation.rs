//! Structural validation for the private grant-ready plan and its movable owner set.

use std::collections::HashSet;

use anyhow::{bail, Result};

use crate::node_agent_compute_plugin_host::{
    manifest_validation::is_sha256,
    runtime_loader_load_set::launch_path_discovery::WindowsPreliminaryImportEdgeKind,
};

use super::super::{
    SealedWindowsLoaderResolutionAuthority, WindowsLoaderApiSetHostResolution,
    WindowsLoaderImportBindingRef, WindowsLoaderImportEdgeKind,
    WindowsLoaderSearchedNameDisposition, WindowsLoaderSystemModuleBinding,
    WindowsLoaderSystemResolutionOrigin,
};
use super::*;

impl GrantReadyWindowsRunnerResolutionPlan {
    pub(super) fn validate_against(
        &self,
        preliminary: &PreliminaryWindowsRunnerResolutionRequestPlanView<'_>,
    ) -> Result<()> {
        self.validate_digests(preliminary)?;
        if self.search_directories.len() != preliminary.search_directories.len()
            || self.module_resolutions.len() != preliminary.module_resolution_requests.len()
            || preliminary.package_image_count() == 0
            || [
                &self.grant_ready_resolution_plan_digest,
                &self.exact_terminal_resolution_set_digest,
                &self.exact_searched_name_disposition_set_digest,
                &self.external_directory_authority_set_digest,
                &self.resolved_system_image_request_set_digest,
            ]
            .into_iter()
            .any(|digest| !is_sha256(digest))
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_GRANT_READY_PLAN_SHAPE_CHANGED");
        }

        for (ordinal, entry) in self.preloaded_terminal_authorities.iter().enumerate() {
            if entry.authority_record_ordinal != ordinal
                || entry.module_cache_key.is_empty()
                || [
                    &entry.component_identity_digest,
                    &entry.immutable_section_identity_digest,
                    &entry.authenticated_evidence_digest,
                ]
                .into_iter()
                .any(|digest| !is_sha256(digest))
            {
                bail!("COMPUTE_PLUGIN_WINDOWS_GRANT_READY_PRELOADED_AUTHORITY_CHANGED");
            }
        }
        for (ordinal, entry) in self.known_dll_terminal_authorities.iter().enumerate() {
            if entry.authority_record_ordinal != ordinal
                || entry.module_cache_key.is_empty()
                || [
                    &entry.section_identity_digest,
                    &entry.component_identity_digest,
                    &entry.immutable_section_identity_digest,
                    &entry.section_image_mapping_receipt_digest,
                    &entry.section_namespace_generation_digest,
                ]
                .into_iter()
                .any(|digest| !is_sha256(digest))
            {
                bail!("COMPUTE_PLUGIN_WINDOWS_GRANT_READY_KNOWN_DLL_AUTHORITY_CHANGED");
            }
        }
        for (ordinal, entry) in self.api_set_resolutions.iter().enumerate() {
            if entry.api_set_resolution_ordinal != ordinal
                || entry.normalized_contract_name.is_empty()
                || entry.normalized_host_module_cache_key.is_empty()
                || self.api_set_resolutions[..ordinal]
                    .iter()
                    .any(|prior| prior.normalized_contract_name == entry.normalized_contract_name)
                || matches!(
                    entry.host_terminal,
                    WindowsGrantReadyNonRecursiveModuleTerminalRef::PackageFile { .. }
                )
                || !self.non_recursive_terminal_valid(&entry.host_terminal, preliminary)
                || !self.non_recursive_terminal_key_matches(
                    &entry.host_terminal,
                    &entry.normalized_host_module_cache_key,
                    preliminary,
                )
                || [
                    &entry.os_build_identity_digest,
                    &entry.schema_identity_digest,
                    &entry.contract_host_binding_set_digest,
                ]
                .into_iter()
                .any(|digest| !is_sha256(digest))
                || !is_sha256(&entry.resolution_binding_digest)
                || entry.resolution_binding_digest
                    != super::digest::recompute_api_set_resolution_binding_digest(entry)
            {
                bail!("COMPUTE_PLUGIN_WINDOWS_GRANT_READY_API_SET_AUTHORITY_CHANGED");
            }
        }

        for (ordinal, (planned, requested)) in self
            .search_directories
            .iter()
            .zip(preliminary.search_directories)
            .enumerate()
        {
            let (request_ordinal, request_role, request_target, request_binding) =
                requested.request_binding();
            let target_matches = match (&planned.target, request_target) {
                (
                    WindowsGrantReadySearchDirectoryTarget::RetainedPreliminaryCandidate {
                        location,
                        preliminary_search_step_ordinal,
                        preliminary_binding_digest,
                    },
                    super::super::super::launch_path_discovery::WindowsPreliminarySearchDirectoryTarget::RetainedCandidate {
                        location: requested_location,
                        ..
                    },
                ) => {
                    location == requested_location
                        && *preliminary_search_step_ordinal == request_ordinal
                        && preliminary_binding_digest == request_binding
                        && planned.authority_binding_digest == request_binding
                }
                (
                    WindowsGrantReadySearchDirectoryTarget::ExternalDirectory { .. },
                    super::super::super::launch_path_discovery::WindowsPreliminarySearchDirectoryTarget::ExternalTypedOwnerRequired { .. },
                ) => true,
                _ => false,
            };
            if planned.search_step_ordinal != ordinal
                || request_ordinal != ordinal
                || planned.role != request_role
                || !is_sha256(&planned.authority_binding_digest)
                || !target_matches
            {
                bail!("COMPUTE_PLUGIN_WINDOWS_GRANT_READY_DIRECTORY_CHANGED");
            }
        }

        for (ordinal, (resolution, request)) in self
            .module_resolutions
            .iter()
            .zip(preliminary.module_resolution_requests)
            .enumerate()
        {
            let (
                request_ordinal,
                global_edge_ordinal,
                edge_locator,
                importer_graph_edge_ordinal,
                importer_image_ordinal,
                import_kind,
                requested_name,
                imported_symbol_name,
                imported_symbol_ordinal,
                ordered_steps,
                _,
            ) = request.request_binding();
            let terminal_disposition_count = resolution
                .searched_name_ordinals
                .iter()
                .filter(|searched| {
                    self.searched_name_dispositions
                        .get(**searched)
                        .is_some_and(|record| {
                            matches!(
                                &record.disposition,
                                WindowsGrantReadySearchedNameDisposition::Terminal { terminal }
                                    if terminal == &resolution.terminal
                            )
                        })
                })
                .count();
            let mut seen_searched_names = HashSet::new();
            let searched_sequence_invalid = resolution.searched_name_ordinals.len()
                > ordered_steps.len()
                || resolution.searched_name_ordinals.iter().enumerate().any(
                    |(position, searched_ordinal)| {
                        !seen_searched_names.insert(*searched_ordinal)
                            || self
                                .searched_name_dispositions
                                .get(*searched_ordinal)
                                .is_none_or(|record| {
                                    record.module_request_ordinal != ordinal
                                        || record.step_position != position
                                        || ordered_steps.get(position)
                                            != Some(&record.search_step_ordinal)
                                })
                    },
                );
            let terminal_requires_search =
                self.terminal_requires_filesystem_search(&resolution.terminal);
            if resolution.request_ordinal != ordinal
                || request_ordinal != ordinal
                || global_edge_ordinal != ordinal
                || resolution.global_import_edge_ordinal != global_edge_ordinal
                || &resolution.edge_locator != edge_locator
                || resolution.importer_graph_edge_ordinal != importer_graph_edge_ordinal
                || resolution.importer_parsed_image_ordinal != importer_image_ordinal
                || !same_import_kind(resolution.import_kind, import_kind)
                || resolution.normalized_requested_name != requested_name
                || resolution.imported_symbol_name.as_deref() != imported_symbol_name
                || resolution.imported_symbol_ordinal != imported_symbol_ordinal
                || !is_sha256(&resolution.resolution_binding_digest)
                || resolution.resolution_binding_digest
                    != super::digest::recompute_module_resolution_binding_digest(resolution)
                || !self.terminal_valid(&resolution.terminal, preliminary)
                || !self.module_terminal_key_matches(resolution, preliminary)
                || searched_sequence_invalid
                || (terminal_requires_search && resolution.searched_name_ordinals.is_empty())
                || (!terminal_requires_search && !resolution.searched_name_ordinals.is_empty())
                || resolution.searched_name_ordinals.iter().any(|searched| {
                    self.searched_name_dispositions
                        .get(*searched)
                        .is_none_or(|record| record.module_request_ordinal != ordinal)
                })
                || (!resolution.searched_name_ordinals.is_empty()
                    && (terminal_disposition_count != 1
                        || self
                            .searched_name_dispositions
                            .get(
                                *resolution
                                    .searched_name_ordinals
                                    .last()
                                    .unwrap_or(&usize::MAX),
                            )
                            .is_none_or(|record| {
                                !matches!(
                                    &record.disposition,
                                    WindowsGrantReadySearchedNameDisposition::Terminal { terminal }
                                        if terminal == &resolution.terminal
                                )
                            })))
            {
                bail!("COMPUTE_PLUGIN_WINDOWS_GRANT_READY_MODULE_CHANGED");
            }
        }

        let mut globally_referenced_searched_names = HashSet::new();
        if self.module_resolutions.iter().any(|module| {
            module
                .searched_name_ordinals
                .iter()
                .any(|ordinal| !globally_referenced_searched_names.insert(*ordinal))
        }) || globally_referenced_searched_names.len() != self.searched_name_dispositions.len()
            || (0..self.searched_name_dispositions.len())
                .any(|ordinal| !globally_referenced_searched_names.contains(&ordinal))
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_GRANT_READY_SEARCH_COVERAGE_CHANGED");
        }
        if self.api_set_resolutions.iter().any(|api| {
            !self.module_resolutions.iter().any(|module| {
                matches!(
                    module.terminal,
                    WindowsGrantReadyModuleTerminalRef::ApiSetResolution {
                        api_set_resolution_ordinal
                    } if api_set_resolution_ordinal == api.api_set_resolution_ordinal
                )
            })
        }) {
            bail!("COMPUTE_PLUGIN_WINDOWS_GRANT_READY_API_SET_COVERAGE_CHANGED");
        }

        for (ordinal, searched) in self.searched_name_dispositions.iter().enumerate() {
            let Some(module) = self.module_resolutions.get(searched.module_request_ordinal) else {
                bail!("COMPUTE_PLUGIN_WINDOWS_GRANT_READY_SEARCH_MODULE_MISSING");
            };
            let requested =
                &preliminary.module_resolution_requests[searched.module_request_ordinal];
            let ordered_steps = requested.request_binding().9;
            let Some(directory) = self.search_directories.get(searched.search_step_ordinal) else {
                bail!("COMPUTE_PLUGIN_WINDOWS_GRANT_READY_SEARCH_DIRECTORY_MISSING");
            };
            let disposition_valid = match &searched.disposition {
                WindowsGrantReadySearchedNameDisposition::MustRemainAbsent => true,
                WindowsGrantReadySearchedNameDisposition::ShadowedByEarlierName {
                    earlier_searched_name_ordinal: _,
                } => false,
                WindowsGrantReadySearchedNameDisposition::Terminal { terminal } => {
                    terminal == &module.terminal && self.terminal_valid(terminal, preliminary)
                }
            };
            if searched.searched_name_ordinal != ordinal
                || module.searched_name_ordinals.get(searched.step_position) != Some(&ordinal)
                || ordered_steps.get(searched.step_position) != Some(&searched.search_step_ordinal)
                || searched.normalized_searched_name.is_empty()
                || !is_sha256(&searched.grant_request_digest)
                || !is_sha256(&searched.disposition_binding_digest)
                || searched.disposition_binding_digest
                    != super::digest::recompute_searched_name_disposition_binding_digest(
                        searched,
                        &directory.authority_binding_digest,
                    )
                || searched.grant_request_digest
                    != super::digest::recompute_name_grant_request_digest(
                        searched,
                        &directory.authority_binding_digest,
                    )
                || !disposition_valid
            {
                bail!("COMPUTE_PLUGIN_WINDOWS_GRANT_READY_SEARCH_DISPOSITION_CHANGED");
            }
        }

        let mut resolved_file_identities = HashSet::new();
        let mut resolved_system_use_modules = HashSet::new();
        for (ordinal, request) in self
            .resolved_filesystem_system_image_requests
            .iter()
            .enumerate()
        {
            if request.resolution_request_ordinal != ordinal
                || request.canonical_dedupe_ordinal != ordinal
                || request.candidate_owner_ordinal != ordinal
                || request.primary_use_ordinal >= request.uses.len()
                || request.normalized_name.is_empty()
                || !resolved_file_identities.insert(&request.expected_file_identity_digest)
                || request.uses.is_empty()
                || [
                    &request.search_directory_authority_binding_digest,
                    &request.resolved_component_identity_digest,
                    &request.expected_file_identity_digest,
                    &request.concrete_servicing_generation_digest,
                    &request.code_integrity_evidence_digest,
                    &request.servicing_resolution_receipt_digest,
                    &request.namespace_alias_currentness_receipt_digest,
                    &request.candidate_binding_digest,
                    &request.lease_request_digest,
                ]
                .into_iter()
                .any(|digest| !is_sha256(digest))
                || request
                    .uses
                    .iter()
                    .any(|usage| self.system_image_use_invalid(ordinal, usage))
                || request
                    .uses
                    .iter()
                    .any(|usage| !resolved_system_use_modules.insert(usage.module_request_ordinal))
                || request
                    .uses
                    .get(request.primary_use_ordinal)
                    .is_none_or(|usage| {
                        usage.normalized_searched_name != request.normalized_name
                            || usage.search_directory_authority_binding_digest
                                != request.search_directory_authority_binding_digest
                    })
                || request.lease_request_digest
                    != super::digest::recompute_system_image_lease_request_digest(request)
            {
                bail!("COMPUTE_PLUGIN_WINDOWS_GRANT_READY_SYSTEM_REQUEST_CHANGED");
            }
        }
        for module in &self.module_resolutions {
            let Some(system_request_ordinal) =
                self.terminal_system_request_ordinal(&module.terminal)
            else {
                continue;
            };
            let exact_use_count = self
                .resolved_filesystem_system_image_requests
                .iter()
                .flat_map(|request| request.uses.iter())
                .filter(|usage| usage.module_request_ordinal == module.request_ordinal)
                .count();
            if exact_use_count != 1
                || self
                    .resolved_filesystem_system_image_requests
                    .get(system_request_ordinal)
                    .is_none_or(|request| {
                        !request
                            .uses
                            .iter()
                            .any(|usage| usage.module_request_ordinal == module.request_ordinal)
                    })
            {
                bail!("COMPUTE_PLUGIN_WINDOWS_GRANT_READY_SYSTEM_USE_COVERAGE_CHANGED");
            }
        }
        Ok(())
    }

    fn module_terminal_key_matches(
        &self,
        module: &WindowsGrantReadyModuleResolution,
        preliminary: &PreliminaryWindowsRunnerResolutionRequestPlanView<'_>,
    ) -> bool {
        match &module.terminal {
            WindowsGrantReadyModuleTerminalRef::NonRecursive(terminal) => self
                .non_recursive_terminal_key_matches(
                    terminal,
                    &module.normalized_requested_name,
                    preliminary,
                ),
            WindowsGrantReadyModuleTerminalRef::ApiSetResolution {
                api_set_resolution_ordinal,
            } => self
                .api_set_resolutions
                .get(*api_set_resolution_ordinal)
                .is_some_and(|api| {
                    api.normalized_contract_name == module.normalized_requested_name
                        && self.non_recursive_terminal_key_matches(
                            &api.host_terminal,
                            &api.normalized_host_module_cache_key,
                            preliminary,
                        )
                }),
        }
    }

    fn non_recursive_terminal_key_matches(
        &self,
        terminal: &WindowsGrantReadyNonRecursiveModuleTerminalRef,
        expected_key: &str,
        preliminary: &PreliminaryWindowsRunnerResolutionRequestPlanView<'_>,
    ) -> bool {
        match terminal {
            WindowsGrantReadyNonRecursiveModuleTerminalRef::AuthenticatedPreloaded {
                preloaded_authority_record_ordinal,
            } => self
                .preloaded_terminal_authorities
                .get(*preloaded_authority_record_ordinal)
                .is_some_and(|entry| entry.module_cache_key == expected_key),
            WindowsGrantReadyNonRecursiveModuleTerminalRef::PackageFile {
                parsed_image_ordinal,
                ..
            } => preliminary
                .package_image_binding(*parsed_image_ordinal)
                .is_some_and(|binding| binding.2 == expected_key),
            WindowsGrantReadyNonRecursiveModuleTerminalRef::KnownDllSection {
                known_dll_authority_record_ordinal,
            } => self
                .known_dll_terminal_authorities
                .get(*known_dll_authority_record_ordinal)
                .is_some_and(|entry| entry.module_cache_key == expected_key),
            WindowsGrantReadyNonRecursiveModuleTerminalRef::ResolvedFilesystemSystemImage {
                resolution_request_ordinal,
            }
            | WindowsGrantReadyNonRecursiveModuleTerminalRef::SideBySideSystemImage {
                resolution_request_ordinal,
            } => self
                .resolved_filesystem_system_image_requests
                .get(*resolution_request_ordinal)
                .is_some_and(|request| request.normalized_name == expected_key),
        }
    }

    fn terminal_requires_filesystem_search(
        &self,
        terminal: &WindowsGrantReadyModuleTerminalRef,
    ) -> bool {
        match terminal {
            WindowsGrantReadyModuleTerminalRef::NonRecursive(terminal) => matches!(
                terminal,
                WindowsGrantReadyNonRecursiveModuleTerminalRef::PackageFile { .. }
                    | WindowsGrantReadyNonRecursiveModuleTerminalRef::ResolvedFilesystemSystemImage { .. }
                    | WindowsGrantReadyNonRecursiveModuleTerminalRef::SideBySideSystemImage { .. }
            ),
            WindowsGrantReadyModuleTerminalRef::ApiSetResolution {
                api_set_resolution_ordinal,
            } => self
                .api_set_resolutions
                .get(*api_set_resolution_ordinal)
                .is_some_and(|api| {
                    matches!(
                        api.host_terminal,
                        WindowsGrantReadyNonRecursiveModuleTerminalRef::PackageFile { .. }
                            | WindowsGrantReadyNonRecursiveModuleTerminalRef::ResolvedFilesystemSystemImage { .. }
                            | WindowsGrantReadyNonRecursiveModuleTerminalRef::SideBySideSystemImage { .. }
                    )
                }),
        }
    }

    fn terminal_system_request_ordinal(
        &self,
        terminal: &WindowsGrantReadyModuleTerminalRef,
    ) -> Option<usize> {
        let terminal = match terminal {
            WindowsGrantReadyModuleTerminalRef::NonRecursive(terminal) => terminal,
            WindowsGrantReadyModuleTerminalRef::ApiSetResolution {
                api_set_resolution_ordinal,
            } => {
                &self
                    .api_set_resolutions
                    .get(*api_set_resolution_ordinal)?
                    .host_terminal
            }
        };
        match terminal {
            WindowsGrantReadyNonRecursiveModuleTerminalRef::ResolvedFilesystemSystemImage {
                resolution_request_ordinal,
            }
            | WindowsGrantReadyNonRecursiveModuleTerminalRef::SideBySideSystemImage {
                resolution_request_ordinal,
            } => Some(*resolution_request_ordinal),
            _ => None,
        }
    }

    fn terminal_valid(
        &self,
        terminal: &WindowsGrantReadyModuleTerminalRef,
        preliminary: &PreliminaryWindowsRunnerResolutionRequestPlanView<'_>,
    ) -> bool {
        match terminal {
            WindowsGrantReadyModuleTerminalRef::ApiSetResolution {
                api_set_resolution_ordinal,
            } => self
                .api_set_resolutions
                .get(*api_set_resolution_ordinal)
                .is_some_and(|entry| {
                    entry.api_set_resolution_ordinal == *api_set_resolution_ordinal
                        && self.non_recursive_terminal_valid(&entry.host_terminal, preliminary)
                }),
            WindowsGrantReadyModuleTerminalRef::NonRecursive(terminal) => {
                self.non_recursive_terminal_valid(terminal, preliminary)
            }
        }
    }

    fn non_recursive_terminal_valid(
        &self,
        terminal: &WindowsGrantReadyNonRecursiveModuleTerminalRef,
        preliminary: &PreliminaryWindowsRunnerResolutionRequestPlanView<'_>,
    ) -> bool {
        match terminal {
            WindowsGrantReadyNonRecursiveModuleTerminalRef::AuthenticatedPreloaded {
                preloaded_authority_record_ordinal,
            } => self
                .preloaded_terminal_authorities
                .get(*preloaded_authority_record_ordinal)
                .is_some_and(|entry| {
                    entry.authority_record_ordinal == *preloaded_authority_record_ordinal
                }),
            WindowsGrantReadyNonRecursiveModuleTerminalRef::PackageFile {
                package_file_ordinal,
                parsed_image_ordinal,
            } => preliminary
                .package_image_binding(*parsed_image_ordinal)
                .is_some_and(|binding| binding.1 == *package_file_ordinal),
            WindowsGrantReadyNonRecursiveModuleTerminalRef::KnownDllSection {
                known_dll_authority_record_ordinal,
            } => self
                .known_dll_terminal_authorities
                .get(*known_dll_authority_record_ordinal)
                .is_some_and(|entry| {
                    entry.authority_record_ordinal == *known_dll_authority_record_ordinal
                }),
            WindowsGrantReadyNonRecursiveModuleTerminalRef::ResolvedFilesystemSystemImage {
                resolution_request_ordinal,
            }
            | WindowsGrantReadyNonRecursiveModuleTerminalRef::SideBySideSystemImage {
                resolution_request_ordinal,
            } => self
                .resolved_filesystem_system_image_requests
                .get(*resolution_request_ordinal)
                .is_some(),
        }
    }

    fn system_image_use_invalid(
        &self,
        request_ordinal: usize,
        usage: &WindowsGrantReadyResolvedSystemImageUse,
    ) -> bool {
        let Some(module) = self.module_resolutions.get(usage.module_request_ordinal) else {
            return true;
        };
        let Some(searched) = self
            .searched_name_dispositions
            .get(usage.searched_name_ordinal)
        else {
            return true;
        };
        let search_directory_binding = self
            .search_directories
            .get(usage.search_step_ordinal)
            .map(|directory| directory.authority_binding_digest.as_str());
        let expected_terminal = match usage.route {
            WindowsGrantReadySystemImageUseRoute::OrdinaryFilesystem => {
                WindowsGrantReadyNonRecursiveModuleTerminalRef::ResolvedFilesystemSystemImage {
                    resolution_request_ordinal: request_ordinal,
                }
            }
            WindowsGrantReadySystemImageUseRoute::SideBySide => {
                WindowsGrantReadyNonRecursiveModuleTerminalRef::SideBySideSystemImage {
                    resolution_request_ordinal: request_ordinal,
                }
            }
        };
        searched.module_request_ordinal != usage.module_request_ordinal
            || searched.search_step_ordinal != usage.search_step_ordinal
            || searched.normalized_searched_name != usage.normalized_searched_name
            || search_directory_binding
                != Some(usage.search_directory_authority_binding_digest.as_str())
            || !is_sha256(&usage.search_directory_authority_binding_digest)
            || !matches!(
                &searched.disposition,
                WindowsGrantReadySearchedNameDisposition::Terminal { terminal }
                    if self.terminal_reaches_system_request(terminal, expected_terminal)
            )
            || !self.terminal_reaches_system_request(&module.terminal, expected_terminal)
    }

    fn terminal_reaches_system_request(
        &self,
        terminal: &WindowsGrantReadyModuleTerminalRef,
        expected: WindowsGrantReadyNonRecursiveModuleTerminalRef,
    ) -> bool {
        match terminal {
            WindowsGrantReadyModuleTerminalRef::NonRecursive(actual) => actual == &expected,
            WindowsGrantReadyModuleTerminalRef::ApiSetResolution {
                api_set_resolution_ordinal,
            } => self
                .api_set_resolutions
                .get(*api_set_resolution_ordinal)
                .is_some_and(|resolution| resolution.host_terminal == expected),
        }
    }
}

impl GrantReadyWindowsRunnerMovableOwnerSet {
    pub(super) fn validate_against(
        &self,
        plan: &GrantReadyWindowsRunnerResolutionPlan,
    ) -> Result<()> {
        let expected_external_count = plan
            .search_directories
            .iter()
            .filter(|step| {
                matches!(
                    step.target,
                    WindowsGrantReadySearchDirectoryTarget::ExternalDirectory { .. }
                )
            })
            .count();
        if self.external_search_directories.len() != expected_external_count {
            bail!("COMPUTE_PLUGIN_WINDOWS_GRANT_READY_MOVABLE_OWNER_COUNT_CHANGED");
        }
        for (ordinal, owner) in self.external_search_directories.iter().enumerate() {
            let Some(step) = plan.search_directories.get(owner.search_step_ordinal) else {
                bail!("COMPUTE_PLUGIN_WINDOWS_GRANT_READY_EXTERNAL_STEP_MISSING");
            };
            let directory_binding = owner.directory.path_currentness_binding();
            if owner.external_owner_ordinal != ordinal
                || !matches!(
                    step.target,
                    WindowsGrantReadySearchDirectoryTarget::ExternalDirectory {
                        external_owner_ordinal
                    } if external_owner_ordinal == ordinal
                )
                || !is_sha256(&owner.handle_chain_authority_digest)
                || !is_sha256(&owner.namespace_alias_currentness_receipt_digest)
                || owner.handle_chain_authority_digest != directory_binding.4
                || owner.namespace_alias_currentness_receipt_digest != directory_binding.6
                || step.authority_binding_digest
                    != super::digest::recompute_external_directory_authority_digest(owner)
            {
                bail!("COMPUTE_PLUGIN_WINDOWS_GRANT_READY_EXTERNAL_OWNER_CHANGED");
            }
        }
        Ok(())
    }
}

impl GrantAcquiredWindowsRunnerResolutionLeaseCustody<'_> {
    /// Validate the unique candidate handles only after name-grant success moved them into this
    /// post-grant owner. This is a borrowed proof seam for a future positive advancer; it neither
    /// constructs candidates nor exposes an extractor that could move them back to GrantReady.
    pub(super) fn validate_pending_system_image_candidates_after_grants(&self) -> Result<()> {
        self.movable_owners.validate_against(&self.plan)?;
        if self.pending_system_image_candidates.len()
            != self.plan.resolved_filesystem_system_image_requests.len()
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_POST_GRANT_SYSTEM_CANDIDATE_COUNT_CHANGED");
        }
        for (ordinal, owner) in self.pending_system_image_candidates.iter().enumerate() {
            let request = &self.plan.resolved_filesystem_system_image_requests[ordinal];
            let parent_identity = request
                .uses
                .get(request.primary_use_ordinal)
                .and_then(|usage| {
                    self.movable_owners
                        .external_search_directories
                        .iter()
                        .find(|directory| {
                            directory.search_step_ordinal == usage.search_step_ordinal
                        })
                })
                .map(|directory| directory.directory.path_currentness_binding().1);
            if owner.candidate_owner_ordinal != ordinal
                || owner.resolution_request_ordinal != request.resolution_request_ordinal
                || parent_identity.is_none()
                || !owner.candidate.matches_resolution_request(
                    parent_identity.unwrap_or_default(),
                    &request.normalized_name,
                    &request.resolved_component_identity_digest,
                    &request.expected_file_identity_digest,
                    &request.concrete_servicing_generation_digest,
                    &request.code_integrity_evidence_digest,
                    &request.servicing_resolution_receipt_digest,
                    &request.namespace_alias_currentness_receipt_digest,
                )
                || owner.candidate.binding().9 != request.candidate_binding_digest
            {
                bail!("COMPUTE_PLUGIN_WINDOWS_POST_GRANT_SYSTEM_CANDIDATE_CHANGED");
            }
        }
        Ok(())
    }
}

fn same_import_kind(
    grant_ready: WindowsGrantReadyImportEdgeKind,
    preliminary: WindowsPreliminaryImportEdgeKind,
) -> bool {
    matches!(
        (grant_ready, preliminary),
        (
            WindowsGrantReadyImportEdgeKind::Normal,
            WindowsPreliminaryImportEdgeKind::Normal
        ) | (
            WindowsGrantReadyImportEdgeKind::Delay,
            WindowsPreliminaryImportEdgeKind::Delay
        ) | (
            WindowsGrantReadyImportEdgeKind::Forwarder,
            WindowsPreliminaryImportEdgeKind::Forwarder
        )
    )
}

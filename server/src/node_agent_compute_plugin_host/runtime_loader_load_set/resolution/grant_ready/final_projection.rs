//! Final cross-stage projection from the private GrantReady plan into sealed resolution custody.

use std::collections::HashSet;

use anyhow::{bail, Result};

use crate::node_agent_compute_plugin_host::runtime_loader_load_set::launch_path_discovery::WindowsPreliminaryImportEdgeKind;

use super::super::{
    SealedWindowsLoaderResolutionAuthority, WindowsLoaderApiSetHostResolution,
    WindowsLoaderImportBindingRef, WindowsLoaderImportEdgeKind,
    WindowsLoaderSearchedNameDisposition, WindowsLoaderSystemModuleBinding,
    WindowsLoaderSystemResolutionOrigin,
};
use super::*;

impl GrantReadyWindowsRunnerResolutionPlan {
    /// Crosswalk the immutable GrantReady request/uses graph to the final unique system-image
    /// custody table and every edge reference. This is deliberately structural: matching aggregate
    /// digests cannot substitute for the same request ordinal, response, FileId owner, and uses.
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn validate_final_system_image_projection(
        &self,
        resolution: &SealedWindowsLoaderResolutionAuthority,
    ) -> Result<()> {
        self.validate_final_search_projection(resolution)?;
        self.validate_final_module_projection(resolution)?;
        if self.resolved_filesystem_system_image_requests.len()
            != resolution.resolved_filesystem_system_images.len()
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_FINAL_SYSTEM_IMAGE_OWNER_COUNT_CHANGED");
        }
        for (request_ordinal, request) in self
            .resolved_filesystem_system_image_requests
            .iter()
            .enumerate()
        {
            let Some(custody) = resolution
                .resolved_filesystem_system_images
                .get(request_ordinal)
            else {
                bail!("COMPUTE_PLUGIN_WINDOWS_FINAL_SYSTEM_IMAGE_OWNER_MISSING");
            };
            let Some(primary_use) = request.uses.get(request.primary_use_ordinal) else {
                bail!("COMPUTE_PLUGIN_WINDOWS_FINAL_SYSTEM_IMAGE_PRIMARY_USE_MISSING");
            };
            let Some(directory) = resolution
                .search_directories
                .get(primary_use.search_step_ordinal)
            else {
                bail!("COMPUTE_PLUGIN_WINDOWS_FINAL_SYSTEM_IMAGE_DIRECTORY_MISSING");
            };
            let Some(component_image) = resolution
                .system_module_images
                .component_images
                .iter()
                .find(|image| {
                    image.component_identity_digest == request.resolved_component_identity_digest
                        && image.image_file_identity_digest == request.expected_file_identity_digest
                        && image.code_integrity_evidence_digest
                            == request.code_integrity_evidence_digest
                        && image.servicing_generation_digest
                            == request.concrete_servicing_generation_digest
                })
            else {
                bail!("COMPUTE_PLUGIN_WINDOWS_FINAL_SYSTEM_IMAGE_COMPONENT_CHANGED");
            };
            if request.resolution_request_ordinal != request_ordinal
                || custody.resolution_request_ordinal != request_ordinal
                || directory.search_directory_ordinal != primary_use.search_step_ordinal
                || directory.policy_source_digest
                    != primary_use.search_directory_authority_binding_digest
                || !custody.outcome.matches_resolution_request(
                    request_ordinal,
                    &request.candidate_binding_digest,
                    &request.lease_request_digest,
                    &directory.directory_identity_digest,
                    &request.normalized_name,
                    &request.expected_file_identity_digest,
                    &component_image.immutable_section_identity_digest,
                    &request.concrete_servicing_generation_digest,
                )
            {
                bail!("COMPUTE_PLUGIN_WINDOWS_FINAL_SYSTEM_IMAGE_PROVENANCE_CHANGED");
            }

            let mut used_module_requests = HashSet::new();
            for usage in &request.uses {
                if !used_module_requests.insert(usage.module_request_ordinal)
                    || !self.final_system_image_use_matches(
                        request_ordinal,
                        request,
                        usage,
                        resolution,
                    )
                {
                    bail!("COMPUTE_PLUGIN_WINDOWS_FINAL_SYSTEM_IMAGE_USE_CHANGED");
                }
            }
            let final_reference_count = resolution
                .system_module_bindings
                .iter()
                .filter(|binding| {
                    binding
                        .filesystem_image_ref
                        .as_ref()
                        .is_some_and(|image_ref| {
                            image_ref.resolution_request_ordinal == request_ordinal
                        })
                })
                .count();
            if final_reference_count != request.uses.len() {
                bail!("COMPUTE_PLUGIN_WINDOWS_FINAL_SYSTEM_IMAGE_USE_COVERAGE_CHANGED");
            }
        }
        Ok(())
    }

    fn validate_final_module_projection(
        &self,
        resolution: &SealedWindowsLoaderResolutionAuthority,
    ) -> Result<()> {
        if self.module_resolutions.len()
            != resolution.package_module_bindings.len() + resolution.system_module_bindings.len()
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_FINAL_MODULE_COUNT_CHANGED");
        }
        for module in &self.module_resolutions {
            let package_matches = resolution
                .package_module_bindings
                .iter()
                .filter(|binding| self.final_package_module_matches(module, binding, resolution))
                .count();
            let system_matches = resolution
                .system_module_bindings
                .iter()
                .filter(|binding| self.final_system_module_matches(module, binding, resolution))
                .count();
            if package_matches + system_matches != 1 {
                bail!("COMPUTE_PLUGIN_WINDOWS_FINAL_MODULE_CROSSWALK_CHANGED");
            }
        }
        Ok(())
    }

    fn final_package_module_matches(
        &self,
        module: &WindowsGrantReadyModuleResolution,
        binding: &super::super::WindowsLoaderPackageModuleBinding,
        resolution: &SealedWindowsLoaderResolutionAuthority,
    ) -> bool {
        let WindowsGrantReadyModuleTerminalRef::NonRecursive(
            WindowsGrantReadyNonRecursiveModuleTerminalRef::PackageFile {
                package_file_ordinal,
                parsed_image_ordinal,
            },
        ) = module.terminal
        else {
            return false;
        };
        let Some((postlease_parsed_image_ordinal, postlease_image)) =
            postlease_parsed_image_for_prelease(resolution, parsed_image_ordinal)
        else {
            return false;
        };
        final_module_common_matches(
            module,
            binding.module_request_ordinal,
            binding.global_import_edge_ordinal,
            &binding.edge_locator,
            binding.importer_parsed_image_ordinal,
            binding.importer_graph_edge_ordinal,
            &binding.importer,
            binding.edge_kind,
            &binding.normalized_import_name,
            binding.imported_symbol_name.as_deref(),
            binding.imported_symbol_ordinal,
            resolution,
        ) && binding.resolved_package_file_ordinal == package_file_ordinal
            && postlease_image.parsed_image_ordinal == postlease_parsed_image_ordinal
            && postlease_image.node
                == (super::super::WindowsLoaderModuleNode::PackageFile {
                    package_file_ordinal,
                })
    }

    fn final_system_module_matches(
        &self,
        module: &WindowsGrantReadyModuleResolution,
        binding: &WindowsLoaderSystemModuleBinding,
        resolution: &SealedWindowsLoaderResolutionAuthority,
    ) -> bool {
        final_module_common_matches(
            module,
            binding.module_request_ordinal,
            binding.global_import_edge_ordinal,
            &binding.edge_locator,
            binding.importer_parsed_image_ordinal,
            binding.importer_graph_edge_ordinal,
            &binding.importer,
            binding.edge_kind,
            &binding.normalized_import_name,
            binding.imported_symbol_name.as_deref(),
            binding.imported_symbol_ordinal,
            resolution,
        ) && self.final_system_terminal_matches(&module.terminal, binding, resolution)
    }

    fn final_system_terminal_matches(
        &self,
        terminal: &WindowsGrantReadyModuleTerminalRef,
        binding: &WindowsLoaderSystemModuleBinding,
        resolution: &SealedWindowsLoaderResolutionAuthority,
    ) -> bool {
        match terminal {
            WindowsGrantReadyModuleTerminalRef::NonRecursive(terminal) => {
                self.final_non_recursive_system_terminal_matches(terminal, binding, resolution)
            }
            WindowsGrantReadyModuleTerminalRef::ApiSetResolution {
                api_set_resolution_ordinal,
            } => self
                .api_set_resolutions
                .get(*api_set_resolution_ordinal)
                .is_some_and(|api| {
                    matches!(
                        &binding.resolution_origin,
                        WindowsLoaderSystemResolutionOrigin::ApiSet {
                            normalized_contract_name,
                            host_component_identity_digest,
                            host_resolution,
                            } if normalized_contract_name == &api.normalized_contract_name
                            && api_set_host_terminal_matches(
                                self,
                                &api.host_terminal,
                                host_resolution,
                                binding,
                                resolution,
                            )
                            && api.os_build_identity_digest
                                == resolution.api_set_authority.os_build_identity_digest
                            && api.schema_identity_digest
                                == resolution.api_set_authority.schema_identity_digest
                            && api.contract_host_binding_set_digest
                                == resolution.api_set_authority.contract_host_binding_set_digest
                            && resolution.api_set_authority.contract_host_bindings.iter().any(
                                |final_contract| {
                                    final_contract.normalized_contract_name
                                        == api.normalized_contract_name
                                        && final_contract.host_module_cache_key
                                            == api.normalized_host_module_cache_key
                                        && final_contract.host_component_identity_digest
                                            == host_component_identity_digest.as_str()
                                        && binding.resolved_component_identity_digest
                                            == host_component_identity_digest.as_str()
                                },
                            )
                    )
                }),
        }
    }

    fn final_non_recursive_system_terminal_matches(
        &self,
        terminal: &WindowsGrantReadyNonRecursiveModuleTerminalRef,
        binding: &WindowsLoaderSystemModuleBinding,
        resolution: &SealedWindowsLoaderResolutionAuthority,
    ) -> bool {
        match terminal {
            WindowsGrantReadyNonRecursiveModuleTerminalRef::AuthenticatedPreloaded {
                preloaded_authority_record_ordinal,
            } => self
                .preloaded_terminal_authorities
                .get(*preloaded_authority_record_ordinal)
                .is_some_and(|entry| {
                    matches!(
                        &binding.resolution_origin,
                        WindowsLoaderSystemResolutionOrigin::Preloaded {
                            preloaded_module_ordinal
                        } if *preloaded_module_ordinal == entry.preloaded_module_ordinal
                    ) && resolution
                        .preloaded_module_authority
                        .modules
                        .get(entry.preloaded_module_ordinal)
                        .is_some_and(|final_entry| {
                            final_entry.resolved_module_cache_key == entry.module_cache_key
                                && final_entry.component_identity_digest
                                    == entry.component_identity_digest
                                && final_entry.immutable_section_identity_digest
                                    == entry.immutable_section_identity_digest
                                && final_entry.preload_evidence_digest
                                    == entry.authenticated_evidence_digest
                                && binding.resolved_component_identity_digest
                                    == entry.component_identity_digest
                                && binding.resolved_image_section_identity_digest
                                    == entry.immutable_section_identity_digest
                        })
                }),
            WindowsGrantReadyNonRecursiveModuleTerminalRef::KnownDllSection {
                known_dll_authority_record_ordinal,
            } => self
                .known_dll_terminal_authorities
                .get(*known_dll_authority_record_ordinal)
                .is_some_and(|entry| {
                    matches!(
                        &binding.resolution_origin,
                        WindowsLoaderSystemResolutionOrigin::KnownDll {
                            section_identity_digest
                        } if section_identity_digest == &entry.section_identity_digest
                    ) && resolution
                        .known_dll_authority
                        .sections
                        .iter()
                        .any(|final_entry| {
                            final_entry.resolved_module_cache_key == entry.module_cache_key
                                && final_entry.section_identity_digest
                                    == entry.section_identity_digest
                                && final_entry.component_identity_digest
                                    == entry.component_identity_digest
                                && final_entry.immutable_image_section_identity_digest
                                    == entry.immutable_section_identity_digest
                                && final_entry.section_image_mapping_receipt_digest
                                    == entry.section_image_mapping_receipt_digest
                                && resolution
                                    .known_dll_authority
                                    .section_namespace_generation_digest
                                    == entry.section_namespace_generation_digest
                                && binding.resolved_component_identity_digest
                                    == entry.component_identity_digest
                                && binding.resolved_image_section_identity_digest
                                    == entry.immutable_section_identity_digest
                        })
                }),
            WindowsGrantReadyNonRecursiveModuleTerminalRef::ResolvedFilesystemSystemImage {
                resolution_request_ordinal,
            } => {
                matches!(
                    &binding.resolution_origin,
                    WindowsLoaderSystemResolutionOrigin::FilesystemSearch { .. }
                ) && binding
                    .filesystem_image_ref
                    .as_ref()
                    .is_some_and(|image_ref| {
                        image_ref.resolution_request_ordinal == *resolution_request_ordinal
                    })
            }
            WindowsGrantReadyNonRecursiveModuleTerminalRef::SideBySideSystemImage {
                resolution_request_ordinal,
            } => {
                matches!(
                    &binding.resolution_origin,
                    WindowsLoaderSystemResolutionOrigin::SideBySide { .. }
                ) && binding
                    .filesystem_image_ref
                    .as_ref()
                    .is_some_and(|image_ref| {
                        image_ref.resolution_request_ordinal == *resolution_request_ordinal
                    })
            }
            WindowsGrantReadyNonRecursiveModuleTerminalRef::PackageFile { .. } => false,
        }
    }

    fn final_system_image_use_matches(
        &self,
        request_ordinal: usize,
        request: &WindowsGrantReadyResolvedFilesystemSystemImageRequest,
        usage: &WindowsGrantReadyResolvedSystemImageUse,
        resolution: &SealedWindowsLoaderResolutionAuthority,
    ) -> bool {
        let Some(module) = self.module_resolutions.get(usage.module_request_ordinal) else {
            return false;
        };
        let Some((binding_ordinal, binding)) = resolution
            .system_module_bindings
            .iter()
            .enumerate()
            .find(|(_, binding)| binding.module_request_ordinal == usage.module_request_ordinal)
        else {
            return false;
        };
        let Some((postlease_importer_ordinal, parsed_importer)) =
            postlease_parsed_image_for_prelease(resolution, module.importer_parsed_image_ordinal)
        else {
            return false;
        };
        let searched = resolution.searched_names.get(usage.searched_name_ordinal);
        let directory_matches = resolution
            .search_directories
            .get(usage.search_step_ordinal)
            .is_some_and(|directory| {
                directory.search_directory_ordinal == usage.search_step_ordinal
                    && directory.policy_source_digest
                        == usage.search_directory_authority_binding_digest
            });
        let searched_matches = searched.is_some_and(|searched| {
            matches!(
                searched.import_binding,
                WindowsLoaderImportBindingRef::System {
                    binding_ordinal: ordinal
                } if ordinal == binding_ordinal
            ) && searched.search_directory_ordinal == usage.search_step_ordinal
                && searched.normalized_name == usage.normalized_searched_name
                && matches!(
                    searched.disposition,
                    WindowsLoaderSearchedNameDisposition::ExpectedSystem { .. }
                )
        });
        binding
            .filesystem_image_ref
            .as_ref()
            .is_some_and(|image_ref| image_ref.resolution_request_ordinal == request_ordinal)
            && binding.module_request_ordinal == module.request_ordinal
            && binding.global_import_edge_ordinal == module.global_import_edge_ordinal
            && binding.edge_locator == module.edge_locator
            && binding.importer_parsed_image_ordinal == postlease_importer_ordinal
            && binding.importer_graph_edge_ordinal == module.importer_graph_edge_ordinal
            && binding.importer == parsed_importer.node
            && final_import_kind_matches(binding.edge_kind, module.import_kind)
            && binding.normalized_import_name == module.normalized_requested_name
            && binding.imported_symbol_name == module.imported_symbol_name
            && binding.imported_symbol_ordinal == module.imported_symbol_ordinal
            && binding.resolved_component_identity_digest
                == request.resolved_component_identity_digest
            && final_system_route_matches(binding, usage.route)
            && directory_matches
            && searched_matches
    }
}

fn final_import_kind_matches(
    final_kind: WindowsLoaderImportEdgeKind,
    planned_kind: WindowsGrantReadyImportEdgeKind,
) -> bool {
    matches!(
        (final_kind, planned_kind),
        (
            WindowsLoaderImportEdgeKind::NormalImport,
            WindowsGrantReadyImportEdgeKind::Normal
        ) | (
            WindowsLoaderImportEdgeKind::DelayImport,
            WindowsGrantReadyImportEdgeKind::Delay
        ) | (
            WindowsLoaderImportEdgeKind::Forwarder,
            WindowsGrantReadyImportEdgeKind::Forwarder
        )
    )
}

fn final_module_common_matches(
    module: &WindowsGrantReadyModuleResolution,
    module_request_ordinal: usize,
    global_import_edge_ordinal: usize,
    edge_locator: &WindowsPreliminaryModuleEdgeLocator,
    importer_parsed_image_ordinal: usize,
    importer_graph_edge_ordinal: usize,
    importer: &super::super::WindowsLoaderModuleNode,
    edge_kind: WindowsLoaderImportEdgeKind,
    normalized_import_name: &str,
    imported_symbol_name: Option<&str>,
    imported_symbol_ordinal: Option<u16>,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> bool {
    postlease_parsed_image_for_prelease(resolution, module.importer_parsed_image_ordinal)
        .is_some_and(|(postlease_ordinal, parsed)| {
            postlease_ordinal == importer_parsed_image_ordinal && &parsed.node == importer
        })
        && module_request_ordinal == module.request_ordinal
        && global_import_edge_ordinal == module.global_import_edge_ordinal
        && edge_locator == &module.edge_locator
        && importer_graph_edge_ordinal == module.importer_graph_edge_ordinal
        && final_import_kind_matches(edge_kind, module.import_kind)
        && normalized_import_name == module.normalized_requested_name
        && imported_symbol_name == module.imported_symbol_name.as_deref()
        && imported_symbol_ordinal == module.imported_symbol_ordinal
}

fn postlease_parsed_image_for_prelease(
    resolution: &SealedWindowsLoaderResolutionAuthority,
    prelease_parsed_image_ordinal: usize,
) -> Option<(usize, &super::super::WindowsPeParsedImageBinding)> {
    let cross = resolution
        .pe_import_graph
        .pre_post_cross_binding
        .parsed_image_cross_bindings
        .iter()
        .find(|cross| cross.prelease_parsed_image_ordinal == prelease_parsed_image_ordinal)?;
    let parsed = resolution
        .pe_import_graph
        .parsed_images
        .get(cross.postlease_parsed_image_ordinal)?;
    (parsed.parsed_image_ordinal == cross.postlease_parsed_image_ordinal)
        .then_some((cross.postlease_parsed_image_ordinal, parsed))
}

fn api_set_host_terminal_matches(
    plan: &GrantReadyWindowsRunnerResolutionPlan,
    terminal: &WindowsGrantReadyNonRecursiveModuleTerminalRef,
    final_host: &WindowsLoaderApiSetHostResolution,
    binding: &WindowsLoaderSystemModuleBinding,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> bool {
    match terminal {
        WindowsGrantReadyNonRecursiveModuleTerminalRef::AuthenticatedPreloaded {
            preloaded_authority_record_ordinal,
        } => plan
            .preloaded_terminal_authorities
            .get(*preloaded_authority_record_ordinal)
            .is_some_and(|entry| {
                matches!(
                    final_host,
                    WindowsLoaderApiSetHostResolution::Preloaded {
                        preloaded_module_ordinal
                    } if *preloaded_module_ordinal == entry.preloaded_module_ordinal
                ) && resolution
                    .preloaded_module_authority
                    .modules
                    .get(entry.preloaded_module_ordinal)
                    .is_some_and(|final_entry| {
                        final_entry.resolved_module_cache_key == entry.module_cache_key
                            && final_entry.component_identity_digest
                                == entry.component_identity_digest
                            && final_entry.immutable_section_identity_digest
                                == entry.immutable_section_identity_digest
                            && final_entry.preload_evidence_digest
                                == entry.authenticated_evidence_digest
                            && binding.resolved_component_identity_digest
                                == entry.component_identity_digest
                            && binding.resolved_image_section_identity_digest
                                == entry.immutable_section_identity_digest
                    })
            }),
        WindowsGrantReadyNonRecursiveModuleTerminalRef::KnownDllSection {
            known_dll_authority_record_ordinal,
        } => plan
            .known_dll_terminal_authorities
            .get(*known_dll_authority_record_ordinal)
            .is_some_and(|entry| {
                matches!(
                    final_host,
                    WindowsLoaderApiSetHostResolution::KnownDll {
                        section_identity_digest
                    } if section_identity_digest == &entry.section_identity_digest
                ) && resolution
                    .known_dll_authority
                    .sections
                    .iter()
                    .any(|final_entry| {
                        final_entry.resolved_module_cache_key == entry.module_cache_key
                            && final_entry.section_identity_digest == entry.section_identity_digest
                            && final_entry.component_identity_digest
                                == entry.component_identity_digest
                            && final_entry.immutable_image_section_identity_digest
                                == entry.immutable_section_identity_digest
                            && final_entry.section_image_mapping_receipt_digest
                                == entry.section_image_mapping_receipt_digest
                            && resolution
                                .known_dll_authority
                                .section_namespace_generation_digest
                                == entry.section_namespace_generation_digest
                            && binding.resolved_component_identity_digest
                                == entry.component_identity_digest
                            && binding.resolved_image_section_identity_digest
                                == entry.immutable_section_identity_digest
                    })
            }),
        WindowsGrantReadyNonRecursiveModuleTerminalRef::ResolvedFilesystemSystemImage {
            resolution_request_ordinal,
        } => {
            matches!(
                final_host,
                WindowsLoaderApiSetHostResolution::FilesystemSearch { .. }
            ) && binding
                .filesystem_image_ref
                .as_ref()
                .is_some_and(|image_ref| {
                    image_ref.resolution_request_ordinal == *resolution_request_ordinal
                })
        }
        WindowsGrantReadyNonRecursiveModuleTerminalRef::SideBySideSystemImage {
            resolution_request_ordinal,
        } => {
            matches!(
                final_host,
                WindowsLoaderApiSetHostResolution::SideBySide { .. }
            ) && binding
                .filesystem_image_ref
                .as_ref()
                .is_some_and(|image_ref| {
                    image_ref.resolution_request_ordinal == *resolution_request_ordinal
                })
        }
        WindowsGrantReadyNonRecursiveModuleTerminalRef::PackageFile { .. } => false,
    }
}

fn final_system_route_matches(
    binding: &WindowsLoaderSystemModuleBinding,
    route: WindowsGrantReadySystemImageUseRoute,
) -> bool {
    match (&binding.resolution_origin, route) {
        (
            WindowsLoaderSystemResolutionOrigin::FilesystemSearch { .. },
            WindowsGrantReadySystemImageUseRoute::OrdinaryFilesystem,
        )
        | (
            WindowsLoaderSystemResolutionOrigin::SideBySide { .. },
            WindowsGrantReadySystemImageUseRoute::SideBySide,
        ) => true,
        (
            WindowsLoaderSystemResolutionOrigin::ApiSet {
                host_resolution: WindowsLoaderApiSetHostResolution::FilesystemSearch { .. },
                ..
            },
            WindowsGrantReadySystemImageUseRoute::OrdinaryFilesystem,
        )
        | (
            WindowsLoaderSystemResolutionOrigin::ApiSet {
                host_resolution: WindowsLoaderApiSetHostResolution::SideBySide { .. },
                ..
            },
            WindowsGrantReadySystemImageUseRoute::SideBySide,
        ) => true,
        _ => false,
    }
}

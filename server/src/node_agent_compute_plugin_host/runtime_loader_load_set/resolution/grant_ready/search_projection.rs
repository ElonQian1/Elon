//! Exact GrantReady-to-final search-directory, search-sequence and searched-name projection.

use anyhow::{bail, Result};

use super::super::{
    SealedWindowsLoaderResolutionAuthority, WindowsLoaderImportBindingRef,
    WindowsLoaderSearchedNameDisposition,
};
use super::*;

impl GrantReadyWindowsRunnerResolutionPlan {
    pub(super) fn validate_final_search_projection(
        &self,
        resolution: &SealedWindowsLoaderResolutionAuthority,
    ) -> Result<()> {
        if self.search_directories.len() != resolution.search_directories.len()
            || self.searched_name_dispositions.len() != resolution.searched_names.len()
            || self.module_resolutions.len() != resolution.pe_import_graph.search_sequences.len()
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_FINAL_SEARCH_PROJECTION_COUNT_CHANGED");
        }
        for (ordinal, (planned, final_directory)) in self
            .search_directories
            .iter()
            .zip(&resolution.search_directories)
            .enumerate()
        {
            if planned.search_step_ordinal != ordinal
                || final_directory.search_directory_ordinal != ordinal
                || final_directory.policy_source_digest != planned.authority_binding_digest
            {
                bail!("COMPUTE_PLUGIN_WINDOWS_FINAL_SEARCH_DIRECTORY_CHANGED");
            }
        }
        for (ordinal, (module, sequence)) in self
            .module_resolutions
            .iter()
            .zip(&resolution.pe_import_graph.search_sequences)
            .enumerate()
        {
            let Some(edge_cross) = resolution
                .pe_import_graph
                .pre_post_cross_binding
                .import_edge_cross_bindings
                .get(ordinal)
            else {
                bail!("COMPUTE_PLUGIN_WINDOWS_FINAL_SEARCH_EDGE_CROSS_BINDING_MISSING");
            };
            if module.request_ordinal != ordinal
                || edge_cross.preliminary_request_ordinal != ordinal
                || sequence.sequence_ordinal != ordinal
                || sequence.import_binding != edge_cross.postlease_import_binding
                || sequence.searched_name_ordinals != module.searched_name_ordinals
            {
                bail!("COMPUTE_PLUGIN_WINDOWS_FINAL_SEARCH_SEQUENCE_CHANGED");
            }
        }
        for (ordinal, (planned, final_name)) in self
            .searched_name_dispositions
            .iter()
            .zip(&resolution.searched_names)
            .enumerate()
        {
            let Some(module) = self.module_resolutions.get(planned.module_request_ordinal) else {
                bail!("COMPUTE_PLUGIN_WINDOWS_FINAL_SEARCH_MODULE_MISSING");
            };
            let Some(edge_cross) = resolution
                .pe_import_graph
                .pre_post_cross_binding
                .import_edge_cross_bindings
                .get(planned.module_request_ordinal)
            else {
                bail!("COMPUTE_PLUGIN_WINDOWS_FINAL_SEARCH_EDGE_CROSS_BINDING_MISSING");
            };
            let Some(directory) = self.search_directories.get(planned.search_step_ordinal) else {
                bail!("COMPUTE_PLUGIN_WINDOWS_FINAL_SEARCH_DIRECTORY_MISSING");
            };
            if planned.searched_name_ordinal != ordinal
                || final_name.searched_name_ordinal != ordinal
                || final_name.import_binding != edge_cross.postlease_import_binding
                || final_name.search_step_ordinal != planned.step_position
                || final_name.search_directory_ordinal != planned.search_step_ordinal
                || final_name.normalized_name != planned.normalized_searched_name
                || final_name.search_directory_authority_binding_digest
                    != directory.authority_binding_digest
                || final_name.grant_request_digest != planned.grant_request_digest
                || final_name.disposition_binding_digest != planned.disposition_binding_digest
                || !self.final_searched_disposition_matches(
                    &planned.disposition,
                    module,
                    &edge_cross.postlease_import_binding,
                    &final_name.disposition,
                    resolution,
                )
            {
                bail!("COMPUTE_PLUGIN_WINDOWS_FINAL_SEARCHED_NAME_CHANGED");
            }
        }
        Ok(())
    }

    fn final_searched_disposition_matches(
        &self,
        planned: &WindowsGrantReadySearchedNameDisposition,
        module: &WindowsGrantReadyModuleResolution,
        import_binding: &WindowsLoaderImportBindingRef,
        final_disposition: &WindowsLoaderSearchedNameDisposition,
        resolution: &SealedWindowsLoaderResolutionAuthority,
    ) -> bool {
        match planned {
            WindowsGrantReadySearchedNameDisposition::MustRemainAbsent => {
                matches!(
                    final_disposition,
                    WindowsLoaderSearchedNameDisposition::MustRemainAbsent
                )
            }
            WindowsGrantReadySearchedNameDisposition::ShadowedByEarlierName {
                earlier_searched_name_ordinal: _,
            } => false,
            WindowsGrantReadySearchedNameDisposition::Terminal { terminal } => {
                if terminal != &module.terminal {
                    return false;
                }
                match (terminal, final_disposition) {
                    (
                        WindowsGrantReadyModuleTerminalRef::NonRecursive(
                            WindowsGrantReadyNonRecursiveModuleTerminalRef::PackageFile {
                                package_file_ordinal,
                                parsed_image_ordinal,
                            },
                        ),
                        WindowsLoaderSearchedNameDisposition::ExpectedPackage {
                            package_file_ordinal: final_file_ordinal,
                            image_file_identity_digest,
                        },
                    ) => resolution
                        .pe_import_graph
                        .pre_post_cross_binding
                        .parsed_image_cross_bindings
                        .iter()
                        .find(|cross| {
                            cross.prelease_parsed_image_ordinal == *parsed_image_ordinal
                                && cross.package_file_ordinal == *package_file_ordinal
                        })
                        .is_some_and(|cross| {
                            *final_file_ordinal == *package_file_ordinal
                                && image_file_identity_digest == &cross.file_identity_digest
                        }),
                    (
                        _,
                        WindowsLoaderSearchedNameDisposition::ExpectedSystem {
                            resolved_component_identity_digest,
                            image_file_identity_digest,
                            immutable_section_identity_digest,
                            servicing_generation_digest,
                        },
                    ) => {
                        let WindowsLoaderImportBindingRef::System { binding_ordinal } =
                            import_binding
                        else {
                            return false;
                        };
                        let Some(binding) = resolution.system_module_bindings.get(*binding_ordinal)
                        else {
                            return false;
                        };
                        let Some(image_ref) = &binding.filesystem_image_ref else {
                            return false;
                        };
                        resolution
                            .resolved_filesystem_system_images
                            .get(image_ref.resolution_request_ordinal)
                            .is_some_and(|custody| {
                                let lease = custody.outcome.image().content_lease_binding();
                                resolved_component_identity_digest
                                    == &binding.resolved_component_identity_digest
                                    && image_file_identity_digest == lease.0
                                    && immutable_section_identity_digest == lease.1
                                    && servicing_generation_digest == lease.2
                                    && immutable_section_identity_digest
                                        == &binding.resolved_image_section_identity_digest
                            })
                    }
                    _ => false,
                }
            }
        }
    }
}

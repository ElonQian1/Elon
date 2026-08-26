//! Exact source-owner and previous-wave target projection for recursive parsed images.

use anyhow::{anyhow, bail, Result};

use super::super::{
    SealedWindowsLoaderResolutionAuthority, WindowsLoaderApiSetHostResolution,
    WindowsLoaderModuleNode, WindowsLoaderSystemModuleBinding, WindowsLoaderSystemResolutionOrigin,
};
use super::{WindowsPeParsedImageSource, WindowsRecursiveImageOwnerRef};

pub(super) fn expected_frontier_for_range(
    first_request: usize,
    request_count: usize,
    next_wave_ordinal: usize,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<Vec<usize>> {
    let end_request = first_request
        .checked_add(request_count)
        .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_COUNT_OVERFLOW"))?;
    let mut targets = resolution
        .package_module_bindings
        .iter()
        .filter(|binding| {
            binding.module_request_ordinal >= first_request
                && binding.module_request_ordinal < end_request
        })
        .map(|binding| {
            (
                binding.module_request_ordinal,
                WindowsLoaderModuleNode::PackageFile {
                    package_file_ordinal: binding.resolved_package_file_ordinal,
                },
            )
        })
        .chain(
            resolution
                .system_module_bindings
                .iter()
                .filter(|binding| {
                    binding.module_request_ordinal >= first_request
                        && binding.module_request_ordinal < end_request
                })
                .map(|binding| {
                    (
                        binding.module_request_ordinal,
                        super::super::super::pe_graph_validation::system_binding_target_node(
                            binding,
                        ),
                    )
                }),
        )
        .collect::<Vec<_>>();
    targets.sort_by_key(|(module_request_ordinal, _)| *module_request_ordinal);

    let mut seen_targets = Vec::new();
    let mut frontier = Vec::new();
    for (producer_module_request_ordinal, target) in targets {
        if seen_targets.contains(&target) {
            continue;
        }
        seen_targets.push(target.clone());
        let parsed = resolution
            .pe_import_graph
            .parsed_images
            .iter()
            .find(|parsed| parsed.node == target)
            .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FRONTIER_TARGET_MISSING"))?;
        let WindowsPeParsedImageSource::RecursiveExpansion {
            parse_receipt_ordinal,
        } = parsed.source
        else {
            continue;
        };
        let receipt = resolution
            .pe_import_graph
            .recursive_resolution_closure
            .parse_receipts
            .get(parse_receipt_ordinal)
            .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FRONTIER_RECEIPT_MISSING"))?;
        if receipt.wave_ordinal > next_wave_ordinal {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FRONTIER_DELAYED");
        }
        if receipt.wave_ordinal == next_wave_ordinal {
            if receipt.producer_module_request_ordinal != producer_module_request_ordinal {
                bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FRONTIER_PRODUCER_CHANGED");
            }
            frontier.push(parse_receipt_ordinal);
        }
    }
    frontier.sort_unstable();
    frontier.dedup();
    Ok(frontier)
}

pub(super) fn source_owner_matches_producer_binding(
    owner: &WindowsRecursiveImageOwnerRef,
    node: &WindowsLoaderModuleNode,
    producer_module_request_ordinal: usize,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> bool {
    let package_binding = resolution
        .package_module_bindings
        .iter()
        .find(|binding| binding.module_request_ordinal == producer_module_request_ordinal);
    let system_binding = resolution
        .system_module_bindings
        .iter()
        .find(|binding| binding.module_request_ordinal == producer_module_request_ordinal);
    match owner {
        WindowsRecursiveImageOwnerRef::PackageContentLease {
            package_file_ordinal,
        } => {
            system_binding.is_none()
                && package_binding.is_some_and(|binding| {
                    binding.resolved_package_file_ordinal == *package_file_ordinal
                        && node
                            == &WindowsLoaderModuleNode::PackageFile {
                                package_file_ordinal: *package_file_ordinal,
                            }
                })
        }
        WindowsRecursiveImageOwnerRef::AuthenticatedPreloadedModule {
            preloaded_module_ordinal,
        } => {
            package_binding.is_none()
                && system_binding.is_some_and(|binding| {
                    system_binding_targets_node(binding, node)
                        && (matches!(
                            &binding.resolution_origin,
                            WindowsLoaderSystemResolutionOrigin::Preloaded {
                                preloaded_module_ordinal: final_ordinal,
                            } if final_ordinal == preloaded_module_ordinal
                        ) || matches!(
                            &binding.resolution_origin,
                            WindowsLoaderSystemResolutionOrigin::ApiSet {
                                host_resolution:
                                    WindowsLoaderApiSetHostResolution::Preloaded {
                                        preloaded_module_ordinal: final_ordinal,
                                    },
                                ..
                            } if final_ordinal == preloaded_module_ordinal
                        ))
                })
        }
        WindowsRecursiveImageOwnerRef::KnownDllSection {
            known_dll_authority_record_ordinal,
        } => {
            let Some(section) = resolution
                .known_dll_authority
                .sections
                .get(*known_dll_authority_record_ordinal)
            else {
                return false;
            };
            package_binding.is_none()
                && system_binding.is_some_and(|binding| {
                    system_binding_targets_node(binding, node)
                        && (matches!(
                            &binding.resolution_origin,
                            WindowsLoaderSystemResolutionOrigin::KnownDll {
                                section_identity_digest,
                            } if section_identity_digest == &section.section_identity_digest
                        ) || matches!(
                            &binding.resolution_origin,
                            WindowsLoaderSystemResolutionOrigin::ApiSet {
                                host_resolution:
                                    WindowsLoaderApiSetHostResolution::KnownDll {
                                        section_identity_digest,
                                    },
                                ..
                            } if section_identity_digest == &section.section_identity_digest
                        ))
                })
        }
        WindowsRecursiveImageOwnerRef::ResolvedFilesystemSystemImage {
            resolution_request_ordinal,
        } => {
            resolution
                .resolved_filesystem_system_images
                .get(*resolution_request_ordinal)
                .is_some()
                && package_binding.is_none()
                && system_binding.is_some_and(|binding| {
                    system_binding_targets_node(binding, node)
                        && binding
                            .filesystem_image_ref
                            .as_ref()
                            .is_some_and(|image_ref| {
                                image_ref.resolution_request_ordinal == *resolution_request_ordinal
                            })
                })
        }
    }
}

fn system_binding_targets_node(
    binding: &WindowsLoaderSystemModuleBinding,
    node: &WindowsLoaderModuleNode,
) -> bool {
    &super::super::super::pe_graph_validation::system_binding_target_node(binding) == node
}

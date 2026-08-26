use anyhow::{bail, Result};

use crate::node_agent_compute_plugin_host::{
    manifest_validation::is_sha256, signed_artifact_verification::jcs_sha256_hex,
};

use super::resolution::{
    SealedWindowsLoaderResolutionAuthority, WindowsLoaderApiSetHostResolution,
    WindowsLoaderModuleNode, WindowsLoaderSystemModuleBinding, WindowsLoaderSystemResolutionOrigin,
};

pub(super) fn validate_system_dependencies(
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<()> {
    let signed = &resolution.signed_system_dependencies;
    if jcs_sha256_hex(&signed.dependencies)? != signed.projection_digest
        || resolution.resolved_system_dependencies.len() != signed.dependencies.len()
    {
        bail!("COMPUTE_PLUGIN_LOADER_SIGNED_SYSTEM_DEPENDENCIES_CHANGED");
    }
    for (ordinal, resolved) in resolution.resolved_system_dependencies.iter().enumerate() {
        let expected = &signed.dependencies[ordinal];
        let duplicate_dependency = signed.dependencies[..ordinal]
            .iter()
            .any(|dependency| dependency.dependency_id == expected.dependency_id);
        let duplicate_component = resolved
            .component_identity_digests
            .iter()
            .enumerate()
            .any(|(index, digest)| resolved.component_identity_digests[..index].contains(digest));
        if resolved.dependency_ordinal != ordinal
            || resolved.dependency_id != expected.dependency_id
            || resolved.version_requirement != expected.version_requirement
            || resolved.component_identity_digests.is_empty()
            || resolved
                .component_identity_digests
                .iter()
                .any(|digest| !is_sha256(digest))
            || duplicate_dependency
            || duplicate_component
            || jcs_sha256_hex(&resolved.component_identity_digests)?
                != resolved.component_identity_set_digest
            || !is_sha256(&resolved.resolver_evidence_digest)
        {
            bail!("COMPUTE_PLUGIN_LOADER_RESOLVED_SYSTEM_DEPENDENCY_CHANGED");
        }
    }
    for (ordinal, image) in resolution
        .system_module_images
        .component_images
        .iter()
        .enumerate()
    {
        let duplicate_component = resolution.system_module_images.component_images[..ordinal]
            .iter()
            .any(|prior| {
                prior.component_identity_digest == image.component_identity_digest
                    || prior.immutable_section_identity_digest
                        == image.immutable_section_identity_digest
            });
        let resolved_component = resolution
            .resolved_system_dependencies
            .iter()
            .any(|dependency| {
                dependency
                    .component_identity_digests
                    .contains(&image.component_identity_digest)
            });
        if duplicate_component
            || !resolved_component
            || [
                &image.component_identity_digest,
                &image.image_file_identity_digest,
                &image.code_integrity_evidence_digest,
                &image.servicing_generation_digest,
                &image.immutable_section_identity_digest,
            ]
            .iter()
            .any(|digest| !is_sha256(digest))
        {
            bail!("COMPUTE_PLUGIN_LOADER_SYSTEM_IMAGE_AUTHORITY_CHANGED");
        }
    }
    if resolution.preloaded_module_authority.modules.is_empty() {
        bail!("COMPUTE_PLUGIN_LOADER_PRELOADED_MODULE_AUTHORITY_EMPTY");
    }
    for (ordinal, preloaded) in resolution
        .preloaded_module_authority
        .modules
        .iter()
        .enumerate()
    {
        let duplicate_key = resolution.preloaded_module_authority.modules[..ordinal]
            .iter()
            .any(|prior| prior.resolved_module_cache_key == preloaded.resolved_module_cache_key);
        let exact_image = resolution
            .system_module_images
            .component_images
            .iter()
            .any(|image| {
                image.component_identity_digest == preloaded.component_identity_digest
                    && image.immutable_section_identity_digest
                        == preloaded.immutable_section_identity_digest
            });
        if duplicate_key
            || !normalized_loader_module_key_valid(&preloaded.resolved_module_cache_key)
            || !exact_image
            || !is_sha256(&preloaded.component_identity_digest)
            || !is_sha256(&preloaded.immutable_section_identity_digest)
            || !is_sha256(&preloaded.preload_evidence_digest)
        {
            bail!("COMPUTE_PLUGIN_LOADER_PRELOADED_MODULE_AUTHORITY_CHANGED");
        }
    }
    for (ordinal, entry) in resolution.known_dll_authority.sections.iter().enumerate() {
        let duplicate = resolution.known_dll_authority.sections[..ordinal]
            .iter()
            .any(|prior| {
                prior.normalized_name == entry.normalized_name
                    || prior.section_identity_digest == entry.section_identity_digest
            });
        let immutable_component =
            resolution
                .system_module_images
                .component_images
                .iter()
                .any(|image| {
                    image.component_identity_digest == entry.component_identity_digest
                        && image.immutable_section_identity_digest
                            == entry.immutable_image_section_identity_digest
                });
        if !normalized_loader_module_key_valid(&entry.normalized_name)
            || !normalized_loader_module_key_valid(&entry.resolved_module_cache_key)
            || duplicate
            || !immutable_component
            || !is_sha256(&entry.section_identity_digest)
            || !is_sha256(&entry.component_identity_digest)
            || !is_sha256(&entry.immutable_image_section_identity_digest)
            || !is_sha256(&entry.section_image_mapping_receipt_digest)
        {
            bail!("COMPUTE_PLUGIN_LOADER_KNOWN_DLL_AUTHORITY_CHANGED");
        }
    }
    for (ordinal, entry) in resolution
        .api_set_authority
        .contract_host_bindings
        .iter()
        .enumerate()
    {
        let duplicate = resolution.api_set_authority.contract_host_bindings[..ordinal]
            .iter()
            .any(|prior| prior.normalized_contract_name == entry.normalized_contract_name);
        let immutable_component = resolution
            .system_module_images
            .component_images
            .iter()
            .any(|image| image.component_identity_digest == entry.host_component_identity_digest);
        if !normalized_loader_module_key_valid(&entry.normalized_contract_name)
            || !normalized_loader_module_key_valid(&entry.host_module_cache_key)
            || duplicate
            || !immutable_component
            || !is_sha256(&entry.host_component_identity_digest)
        {
            bail!("COMPUTE_PLUGIN_LOADER_API_SET_AUTHORITY_CHANGED");
        }
    }
    for (ordinal, entry) in resolution
        .side_by_side_authority
        .assembly_bindings
        .iter()
        .enumerate()
    {
        let duplicate = resolution.side_by_side_authority.assembly_bindings[..ordinal]
            .iter()
            .any(|prior| {
                prior.normalized_import_name == entry.normalized_import_name
                    || prior.assembly_identity_digest == entry.assembly_identity_digest
            });
        let immutable_component =
            resolution
                .system_module_images
                .component_images
                .iter()
                .any(|image| {
                    image.component_identity_digest == entry.component_identity_digest
                        && image.image_file_identity_digest == entry.image_file_identity_digest
                        && image.immutable_section_identity_digest
                            == entry.immutable_section_identity_digest
                });
        if !normalized_loader_module_key_valid(&entry.normalized_import_name)
            || !normalized_loader_module_key_valid(&entry.resolved_module_cache_key)
            || duplicate
            || !immutable_component
            || !is_sha256(&entry.assembly_identity_digest)
            || !is_sha256(&entry.component_identity_digest)
            || !is_sha256(&entry.image_file_identity_digest)
            || !is_sha256(&entry.immutable_section_identity_digest)
            || !is_sha256(&entry.activation_context_resolution_receipt_digest)
        {
            bail!("COMPUTE_PLUGIN_LOADER_SXS_AUTHORITY_CHANGED");
        }
    }
    Ok(())
}

pub(super) fn module_node_valid(
    node: &WindowsLoaderModuleNode,
    resolution: &SealedWindowsLoaderResolutionAuthority,
    package_file_count: usize,
) -> bool {
    match node {
        WindowsLoaderModuleNode::PackageFile {
            package_file_ordinal,
        } => *package_file_ordinal < package_file_count,
        WindowsLoaderModuleNode::SystemComponent {
            component_identity_digest,
        } => {
            is_sha256(component_identity_digest)
                && resolution
                    .resolved_system_dependencies
                    .iter()
                    .any(|dependency| {
                        dependency
                            .component_identity_digests
                            .contains(component_identity_digest)
                    })
                && resolution
                    .system_module_images
                    .component_images
                    .iter()
                    .any(|image| image.component_identity_digest == *component_identity_digest)
        }
        WindowsLoaderModuleNode::KnownDllSection {
            section_identity_digest,
        } => {
            is_sha256(section_identity_digest)
                && resolution.known_dll_authority.sections.iter().any(|entry| {
                    entry.section_identity_digest == *section_identity_digest
                        && is_sha256(&entry.immutable_image_section_identity_digest)
                        && is_sha256(&entry.section_image_mapping_receipt_digest)
                        && resolution
                            .system_module_images
                            .component_images
                            .iter()
                            .any(|image| {
                                image.component_identity_digest == entry.component_identity_digest
                                    && image.immutable_section_identity_digest
                                        == entry.immutable_image_section_identity_digest
                            })
                })
        }
        WindowsLoaderModuleNode::ApiSetHost {
            component_identity_digest,
        } => {
            is_sha256(component_identity_digest)
                && resolution
                    .api_set_authority
                    .contract_host_bindings
                    .iter()
                    .any(|entry| entry.host_component_identity_digest == *component_identity_digest)
                && resolution
                    .system_module_images
                    .component_images
                    .iter()
                    .any(|image| image.component_identity_digest == *component_identity_digest)
        }
        WindowsLoaderModuleNode::SideBySideAssembly {
            assembly_identity_digest,
        } => {
            is_sha256(assembly_identity_digest)
                && resolution
                    .side_by_side_authority
                    .assembly_bindings
                    .iter()
                    .any(|entry| entry.assembly_identity_digest == *assembly_identity_digest)
        }
    }
}

pub(super) fn system_resolution_origin_valid(
    binding: &WindowsLoaderSystemModuleBinding,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> bool {
    match &binding.resolution_origin {
        WindowsLoaderSystemResolutionOrigin::Preloaded {
            preloaded_module_ordinal,
        } => {
            binding.resolved_search_directory_ordinal.is_none()
                && binding.filesystem_image_ref.is_none()
                && resolution
                    .preloaded_module_authority
                    .modules
                    .get(*preloaded_module_ordinal)
                    .is_some_and(|entry| {
                        entry.resolved_module_cache_key == binding.normalized_import_name
                            && entry.resolved_module_cache_key == binding.resolved_module_cache_key
                            && entry.component_identity_digest
                                == binding.resolved_component_identity_digest
                            && entry.immutable_section_identity_digest
                                == binding.resolved_image_section_identity_digest
                    })
        }
        WindowsLoaderSystemResolutionOrigin::KnownDll {
            section_identity_digest,
        } => resolution.known_dll_authority.sections.iter().any(|entry| {
            binding.resolved_search_directory_ordinal.is_none()
                && binding.filesystem_image_ref.is_none()
                && entry.normalized_name == binding.normalized_import_name
                && entry.resolved_module_cache_key == binding.resolved_module_cache_key
                && entry.section_identity_digest == *section_identity_digest
                && entry.component_identity_digest == binding.resolved_component_identity_digest
                && entry.immutable_image_section_identity_digest
                    == binding.resolved_image_section_identity_digest
                && is_sha256(&entry.section_image_mapping_receipt_digest)
        }),
        WindowsLoaderSystemResolutionOrigin::ApiSet {
            normalized_contract_name,
            host_component_identity_digest,
            host_resolution,
        } => {
            normalized_contract_name == &binding.normalized_import_name
                && host_component_identity_digest == &binding.resolved_component_identity_digest
                && resolution
                    .api_set_authority
                    .contract_host_bindings
                    .iter()
                    .any(|entry| {
                        entry.normalized_contract_name == *normalized_contract_name
                            && entry.host_module_cache_key == binding.resolved_module_cache_key
                            && entry.host_component_identity_digest
                                == *host_component_identity_digest
                    })
                && api_set_host_resolution_valid(binding, host_resolution, resolution)
        }
        WindowsLoaderSystemResolutionOrigin::SideBySide {
            assembly_identity_digest,
            search_directory_ordinal,
        } => side_by_side_terminal_valid(
            binding,
            &binding.normalized_import_name,
            assembly_identity_digest,
            *search_directory_ordinal,
            resolution,
        ),
        WindowsLoaderSystemResolutionOrigin::FilesystemSearch {
            search_directory_ordinal,
        } => {
            binding.resolved_module_cache_key == binding.normalized_import_name
                && filesystem_terminal_valid(
                    binding,
                    *search_directory_ordinal,
                    &binding.normalized_import_name,
                    false,
                    resolution,
                )
        }
    }
}

fn api_set_host_resolution_valid(
    binding: &WindowsLoaderSystemModuleBinding,
    host_resolution: &WindowsLoaderApiSetHostResolution,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> bool {
    match host_resolution {
        WindowsLoaderApiSetHostResolution::Preloaded {
            preloaded_module_ordinal,
        } => {
            binding.resolved_search_directory_ordinal.is_none()
                && binding.filesystem_image_ref.is_none()
                && resolution
                    .preloaded_module_authority
                    .modules
                    .get(*preloaded_module_ordinal)
                    .is_some_and(|entry| {
                        entry.resolved_module_cache_key == binding.resolved_module_cache_key
                            && entry.component_identity_digest
                                == binding.resolved_component_identity_digest
                            && entry.immutable_section_identity_digest
                                == binding.resolved_image_section_identity_digest
                    })
        }
        WindowsLoaderApiSetHostResolution::KnownDll {
            section_identity_digest,
        } => {
            binding.resolved_search_directory_ordinal.is_none()
                && binding.filesystem_image_ref.is_none()
                && resolution.known_dll_authority.sections.iter().any(|entry| {
                    entry.normalized_name == binding.resolved_module_cache_key
                        && entry.resolved_module_cache_key == binding.resolved_module_cache_key
                        && entry.section_identity_digest == *section_identity_digest
                        && entry.component_identity_digest
                            == binding.resolved_component_identity_digest
                        && entry.immutable_image_section_identity_digest
                            == binding.resolved_image_section_identity_digest
                        && is_sha256(&entry.section_image_mapping_receipt_digest)
                })
        }
        WindowsLoaderApiSetHostResolution::FilesystemSearch {
            search_directory_ordinal,
        } => filesystem_terminal_valid(
            binding,
            *search_directory_ordinal,
            &binding.resolved_module_cache_key,
            false,
            resolution,
        ),
        WindowsLoaderApiSetHostResolution::SideBySide {
            assembly_identity_digest,
            search_directory_ordinal,
        } => side_by_side_terminal_valid(
            binding,
            &binding.resolved_module_cache_key,
            assembly_identity_digest,
            *search_directory_ordinal,
            resolution,
        ),
    }
}

fn side_by_side_terminal_valid(
    binding: &WindowsLoaderSystemModuleBinding,
    expected_requested_name: &str,
    assembly_identity_digest: &str,
    search_directory_ordinal: usize,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> bool {
    let exact_assembly =
        resolved_filesystem_system_image(binding, resolution).is_some_and(|file| {
            let (_, _, file_identity, section_identity, _, _) = file.binding();
            resolution
                .side_by_side_authority
                .assembly_bindings
                .iter()
                .any(|entry| {
                    entry.normalized_import_name == expected_requested_name
                        && entry.assembly_identity_digest == assembly_identity_digest
                        && entry.resolved_module_cache_key == binding.resolved_module_cache_key
                        && entry.component_identity_digest
                            == binding.resolved_component_identity_digest
                        && entry.image_file_identity_digest == file_identity
                        && entry.immutable_section_identity_digest == section_identity
                        && section_identity == binding.resolved_image_section_identity_digest
                        && is_sha256(&entry.activation_context_resolution_receipt_digest)
                })
        });
    exact_assembly
        && filesystem_terminal_valid(
            binding,
            search_directory_ordinal,
            &binding.resolved_module_cache_key,
            true,
            resolution,
        )
}

fn filesystem_terminal_valid(
    binding: &WindowsLoaderSystemModuleBinding,
    search_directory_ordinal: usize,
    expected_name: &str,
    require_side_by_side_directory: bool,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> bool {
    let directory = resolution.search_directories.get(search_directory_ordinal);
    let image = resolution
        .system_module_images
        .component_images
        .iter()
        .find(|image| {
            image.component_identity_digest == binding.resolved_component_identity_digest
        });
    match (
        directory,
        image,
        resolved_filesystem_system_image(binding, resolution),
    ) {
        (Some(directory), Some(image), Some(file)) => {
            let directory_kind_valid = !require_side_by_side_directory
                || matches!(
                    &directory.target,
                    super::resolution::WindowsLoaderFilesystemSearchDirectoryTarget::SideBySideAssemblyDirectory { .. }
                );
            directory_kind_valid
                && binding.resolved_search_directory_ordinal == Some(search_directory_ordinal)
                && file.matches_resolution(
                    &directory.directory_identity_digest,
                    expected_name,
                    &image.image_file_identity_digest,
                    &binding.resolved_image_section_identity_digest,
                    &image.servicing_generation_digest,
                )
        }
        _ => false,
    }
}

/// Borrow the one deduplicated final system-image owner referenced by this import edge. The
/// ordinal is checked on both the edge and owner so vector position cannot silently rebind it.
pub(super) fn resolved_filesystem_system_image<'resolution>(
    binding: &WindowsLoaderSystemModuleBinding,
    resolution: &'resolution SealedWindowsLoaderResolutionAuthority,
) -> Option<&'resolution crate::node_agent_managed_fs::PinnedWindowsLoaderSystemImageFile> {
    let image_ref = binding.filesystem_image_ref.as_ref()?;
    let custody = resolution
        .resolved_filesystem_system_images
        .get(image_ref.resolution_request_ordinal)?;
    (custody.resolution_request_ordinal == image_ref.resolution_request_ordinal)
        .then_some(custody.outcome.image())
}

pub(super) fn system_terminal_search_binding(
    binding: &WindowsLoaderSystemModuleBinding,
) -> Option<(usize, &str)> {
    match &binding.resolution_origin {
        WindowsLoaderSystemResolutionOrigin::FilesystemSearch {
            search_directory_ordinal,
        } => Some((*search_directory_ordinal, &binding.normalized_import_name)),
        WindowsLoaderSystemResolutionOrigin::SideBySide {
            search_directory_ordinal,
            ..
        } => Some((
            *search_directory_ordinal,
            &binding.resolved_module_cache_key,
        )),
        WindowsLoaderSystemResolutionOrigin::ApiSet {
            host_resolution:
                WindowsLoaderApiSetHostResolution::FilesystemSearch {
                    search_directory_ordinal,
                }
                | WindowsLoaderApiSetHostResolution::SideBySide {
                    search_directory_ordinal,
                    ..
                },
            ..
        } => Some((
            *search_directory_ordinal,
            &binding.resolved_module_cache_key,
        )),
        _ => None,
    }
}

/// Fail-closed canonical loader key accepted by this source slice. Restricting names to lowercase
/// ASCII avoids claiming Unicode/locale comparison semantics that are not yet represented while
/// making Rust equality exactly match the admitted case-insensitive Windows key domain.
pub(super) fn normalized_loader_module_key_valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && !value.starts_with('.')
        && !value.ends_with('.')
        && !value.contains("..")
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'_')
        })
}

pub(super) fn canonical_loader_module_basename(path: &str) -> Option<String> {
    let basename = path
        .rsplit(|character| character == '/' || character == '\\')
        .next()?;
    let canonical = basename.to_ascii_lowercase();
    normalized_loader_module_key_valid(&canonical).then_some(canonical)
}

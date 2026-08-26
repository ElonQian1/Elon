use anyhow::{bail, Result};
use serde_json::json;

use crate::node_agent_compute_plugin_host::{
    manifest_validation::is_sha256, signed_artifact_verification::jcs_sha256_hex,
};

use super::{
    digest::importer_edge_table_digest,
    model::SealedComputePluginRunnerImage,
    resolution::{
        SealedWindowsLoaderResolutionAuthority, WindowsLoaderImportBindingRef,
        WindowsLoaderImportEdgeKind, WindowsLoaderModuleNode, WindowsLoaderSearchedNameDisposition,
        WindowsLoaderSystemModuleBinding, WindowsLoaderSystemResolutionOrigin,
    },
    system_resolution_validation::{
        canonical_loader_module_basename, module_node_valid, normalized_loader_module_key_valid,
        system_terminal_search_binding,
    },
};

pub(super) fn validate_pe_import_graph(
    image: &SealedComputePluginRunnerImage,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<()> {
    let graph = &resolution.pe_import_graph;
    let root = WindowsLoaderModuleNode::PackageFile {
        package_file_ordinal: image.runner_ordinal,
    };
    if graph.root_package_file_ordinal != image.runner_ordinal
        || graph.expected_package_edge_count != resolution.package_module_bindings.len()
        || graph.expected_system_edge_count != resolution.system_module_bindings.len()
        || graph.expected_search_step_count != resolution.searched_names.len()
        || graph.parsed_images.is_empty()
        || graph.parsed_images[0].node != root
    {
        bail!("COMPUTE_PLUGIN_LOADER_PE_IMPORT_GRAPH_CHANGED");
    }

    let derived_closure = derive_reachable_node_closure(&root, resolution);
    for (ordinal, parsed) in graph.parsed_images.iter().enumerate() {
        let duplicate = graph.parsed_images[..ordinal]
            .iter()
            .any(|prior| prior.node == parsed.node);
        let duplicate_image_material = graph.parsed_images[..ordinal].iter().any(|prior| {
            prior.image_material_identity_digest == parsed.image_material_identity_digest
        });
        let (normal, delay, forwarder) = edge_counts_for_importer(&parsed.node, resolution);
        let expected_material = node_image_material_identity(&parsed.node, image, resolution)?;
        let expected_import_table_digest = importer_edge_table_digest(&parsed.node, resolution)?;
        if parsed.parsed_image_ordinal != ordinal
            || duplicate
            || duplicate_image_material
            || derived_closure.get(ordinal) != Some(&parsed.node)
            || !module_node_valid(&parsed.node, resolution, image.package_files.len())
            || expected_material.as_deref() != Some(parsed.image_material_identity_digest.as_str())
            || parsed.normal_import_count != normal
            || parsed.delay_import_count != delay
            || parsed.forwarder_count != forwarder
            || !importer_edge_ordinals_are_contiguous(&parsed.node, resolution)
            || !is_sha256(&parsed.image_material_identity_digest)
            || parsed.import_table_digest != expected_import_table_digest
            || !is_sha256(&parsed.import_table_digest)
        {
            bail!("COMPUTE_PLUGIN_LOADER_PARSED_IMAGE_BINDING_CHANGED");
        }
    }
    if resolution.package_module_bindings.iter().any(|binding| {
        !graph
            .parsed_images
            .iter()
            .any(|parsed| parsed.node == binding.importer)
    }) || resolution.system_module_bindings.iter().any(|binding| {
        !graph
            .parsed_images
            .iter()
            .any(|parsed| parsed.node == binding.importer)
    }) {
        bail!("COMPUTE_PLUGIN_LOADER_IMPORTER_NOT_PARSED");
    }
    if !loaded_module_cache_targets_are_consistent(image, resolution) {
        bail!("COMPUTE_PLUGIN_LOADER_MODULE_CACHE_TARGET_CHANGED");
    }

    for (ordinal, reachable) in graph.reachable_nodes.iter().enumerate() {
        let duplicate = graph.reachable_nodes[..ordinal]
            .iter()
            .any(|prior| prior.node == reachable.node);
        if reachable.reachable_node_ordinal != ordinal
            || duplicate
            || !module_node_valid(&reachable.node, resolution, image.package_files.len())
            || derived_closure.get(ordinal) != Some(&reachable.node)
            || !graph
                .parsed_images
                .iter()
                .any(|parsed| parsed.node == reachable.node)
        {
            bail!("COMPUTE_PLUGIN_LOADER_REACHABLE_NODE_BINDING_CHANGED");
        }
    }
    if graph.reachable_nodes.len() != derived_closure.len()
        || graph.parsed_images.len() != derived_closure.len()
        || graph.parsed_images.iter().any(|parsed| {
            !graph
                .reachable_nodes
                .iter()
                .any(|reachable| reachable.node == parsed.node)
        })
        || derived_closure.iter().any(|node| {
            !graph
                .parsed_images
                .iter()
                .any(|parsed| &parsed.node == node)
        })
    {
        bail!("COMPUTE_PLUGIN_LOADER_REACHABLE_NODE_SET_CHANGED");
    }

    let expected_sequence_count =
        resolution.package_module_bindings.len() + resolution.system_module_bindings.len();
    if graph.search_sequences.len() != expected_sequence_count {
        bail!("COMPUTE_PLUGIN_LOADER_SEARCH_SEQUENCE_CARDINALITY_CHANGED");
    }
    for (ordinal, sequence) in graph.search_sequences.iter().enumerate() {
        let duplicate = graph.search_sequences[..ordinal]
            .iter()
            .any(|prior| same_import_binding(&prior.import_binding, &sequence.import_binding));
        if sequence.sequence_ordinal != ordinal
            || duplicate
            || !validate_search_sequence(sequence, resolution)
        {
            bail!("COMPUTE_PLUGIN_LOADER_SEARCH_SEQUENCE_CHANGED");
        }
    }
    Ok(())
}

fn edge_counts_for_importer(
    importer: &WindowsLoaderModuleNode,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> (usize, usize, usize) {
    let kinds = resolution
        .package_module_bindings
        .iter()
        .filter(|binding| &binding.importer == importer)
        .map(|binding| binding.edge_kind)
        .chain(
            resolution
                .system_module_bindings
                .iter()
                .filter(|binding| &binding.importer == importer)
                .map(|binding| binding.edge_kind),
        )
        .collect::<Vec<_>>();
    (
        kinds
            .iter()
            .filter(|kind| **kind == WindowsLoaderImportEdgeKind::NormalImport)
            .count(),
        kinds
            .iter()
            .filter(|kind| **kind == WindowsLoaderImportEdgeKind::DelayImport)
            .count(),
        kinds
            .iter()
            .filter(|kind| **kind == WindowsLoaderImportEdgeKind::Forwarder)
            .count(),
    )
}

fn importer_edge_ordinals_are_contiguous(
    importer: &WindowsLoaderModuleNode,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> bool {
    let mut ordinals = resolution
        .package_module_bindings
        .iter()
        .filter(|binding| &binding.importer == importer)
        .map(|binding| binding.importer_edge_ordinal)
        .chain(
            resolution
                .system_module_bindings
                .iter()
                .filter(|binding| &binding.importer == importer)
                .map(|binding| binding.importer_edge_ordinal),
        )
        .collect::<Vec<_>>();
    ordinals.sort_unstable();
    ordinals
        .iter()
        .enumerate()
        .all(|(expected, actual)| expected == *actual)
}

/// Conservative Windows loaded-module cache rule: one normalized module key may never resolve to
/// two target identities anywhere in the authenticated graph. API-set hosts and SxS assemblies
/// remain origin-specific nodes, so their aliases cannot silently conflict with filesystem/package
/// resolution. This may reject a future valid exotic context, but cannot admit an impossible set.
fn loaded_module_cache_targets_are_consistent(
    image: &SealedComputePluginRunnerImage,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> bool {
    let mut observed = Vec::<(String, String)>::new();
    let Some(root_key) = canonical_loader_module_basename(&resolution.runner_relative_path) else {
        return false;
    };
    let root_target = format!("package-file:{}", image.runner_ordinal);
    if !record_cache_alias(&mut observed, &root_key, &root_target) {
        return false;
    }
    for preloaded in &resolution.preloaded_module_authority.modules {
        let target = format!(
            "immutable-system-section:{}",
            preloaded.immutable_section_identity_digest
        );
        if !record_cache_alias(&mut observed, &preloaded.resolved_module_cache_key, &target) {
            return false;
        }
    }
    for binding in &resolution.package_module_bindings {
        let target = format!("package-file:{}", binding.resolved_package_file_ordinal);
        if !record_cache_alias(&mut observed, &binding.normalized_import_name, &target)
            || !record_cache_alias(&mut observed, &binding.resolved_module_cache_key, &target)
        {
            return false;
        }
    }
    for binding in &resolution.system_module_bindings {
        let target = format!(
            "immutable-system-section:{}",
            binding.resolved_image_section_identity_digest
        );
        if !record_cache_alias(&mut observed, &binding.normalized_import_name, &target)
            || !record_cache_alias(&mut observed, &binding.resolved_module_cache_key, &target)
        {
            return false;
        }
    }
    true
}

fn record_cache_alias(observed: &mut Vec<(String, String)>, key: &str, target: &str) -> bool {
    if !normalized_loader_module_key_valid(key)
        || observed
            .iter()
            .any(|(prior_key, prior_target)| prior_key == key && prior_target != target)
    {
        return false;
    }
    observed.push((key.to_owned(), target.to_owned()));
    true
}

fn node_image_material_identity(
    node: &WindowsLoaderModuleNode,
    image: &SealedComputePluginRunnerImage,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<Option<String>> {
    let component = match node {
        WindowsLoaderModuleNode::PackageFile {
            package_file_ordinal,
        } => {
            let Some(entry) = image.package_files.get(*package_file_ordinal) else {
                return Ok(None);
            };
            let (file_identity, sealed_digest, lease_generation, immutable_policy) =
                entry.file.content_lease_binding();
            return Ok(Some(jcs_sha256_hex(&json!({
                "schema": "elon.compute_plugin.windows_package_parsed_image_material.v1",
                "file_identity_digest": file_identity,
                "sealed_content_digest": sealed_digest,
                "content_lease_generation_digest": lease_generation,
                "immutable_content_policy_digest": immutable_policy,
            }))?));
        }
        WindowsLoaderModuleNode::SystemComponent {
            component_identity_digest,
        }
        | WindowsLoaderModuleNode::ApiSetHost {
            component_identity_digest,
        } => component_identity_digest.as_str(),
        WindowsLoaderModuleNode::KnownDllSection {
            section_identity_digest,
        } => resolution
            .known_dll_authority
            .sections
            .iter()
            .find(|entry| entry.section_identity_digest == *section_identity_digest)?
            .component_identity_digest
            .as_str(),
        WindowsLoaderModuleNode::SideBySideAssembly {
            assembly_identity_digest,
        } => resolution
            .side_by_side_authority
            .assembly_bindings
            .iter()
            .find(|entry| entry.assembly_identity_digest == *assembly_identity_digest)?
            .component_identity_digest
            .as_str(),
    };
    Ok(resolution
        .system_module_images
        .component_images
        .iter()
        .find(|entry| entry.component_identity_digest == component)
        .map(|entry| entry.immutable_section_identity_digest.clone()))
}

fn derive_reachable_node_closure(
    root: &WindowsLoaderModuleNode,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Vec<WindowsLoaderModuleNode> {
    let mut closure = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(importer) = stack.pop() {
        if closure.contains(&importer) {
            continue;
        }
        closure.push(importer.clone());
        let mut targets = resolution
            .package_module_bindings
            .iter()
            .filter(|binding| binding.importer == importer)
            .map(|binding| {
                (
                    binding.importer_edge_ordinal,
                    WindowsLoaderModuleNode::PackageFile {
                        package_file_ordinal: binding.resolved_package_file_ordinal,
                    },
                )
            })
            .chain(
                resolution
                    .system_module_bindings
                    .iter()
                    .filter(|binding| binding.importer == importer)
                    .map(|binding| {
                        (
                            binding.importer_edge_ordinal,
                            system_binding_target_node(binding),
                        )
                    }),
            )
            .collect::<Vec<_>>();
        targets.sort_by_key(|(edge_ordinal, _)| *edge_ordinal);
        for (_, target) in targets.into_iter().rev() {
            if !closure.contains(&target) {
                stack.push(target);
            }
        }
    }
    closure
}

fn system_binding_target_node(
    binding: &WindowsLoaderSystemModuleBinding,
) -> WindowsLoaderModuleNode {
    match &binding.resolution_origin {
        WindowsLoaderSystemResolutionOrigin::KnownDll {
            section_identity_digest,
        } => WindowsLoaderModuleNode::KnownDllSection {
            section_identity_digest: section_identity_digest.clone(),
        },
        WindowsLoaderSystemResolutionOrigin::ApiSet {
            host_component_identity_digest,
            ..
        } => WindowsLoaderModuleNode::ApiSetHost {
            component_identity_digest: host_component_identity_digest.clone(),
        },
        WindowsLoaderSystemResolutionOrigin::SideBySide {
            assembly_identity_digest,
            ..
        } => WindowsLoaderModuleNode::SideBySideAssembly {
            assembly_identity_digest: assembly_identity_digest.clone(),
        },
        WindowsLoaderSystemResolutionOrigin::FilesystemSearch { .. } => {
            WindowsLoaderModuleNode::SystemComponent {
                component_identity_digest: binding.resolved_component_identity_digest.clone(),
            }
        }
    }
}

fn validate_search_sequence(
    sequence: &super::resolution::WindowsPeImportSearchSequenceBinding,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> bool {
    let steps = resolution
        .searched_names
        .iter()
        .filter(|entry| same_import_binding(&entry.import_binding, &sequence.import_binding))
        .collect::<Vec<_>>();
    if steps.len() != sequence.searched_name_ordinals.len()
        || steps.iter().enumerate().any(|(ordinal, step)| {
            step.search_step_ordinal != ordinal
                || sequence.searched_name_ordinals[ordinal] != step.searched_name_ordinal
        })
    {
        return false;
    }
    let prior_steps_are_absent = steps
        .iter()
        .take(steps.len().saturating_sub(1))
        .all(|step| {
            matches!(
                &step.disposition,
                WindowsLoaderSearchedNameDisposition::MustRemainAbsentShadow
            )
        });
    if !prior_steps_are_absent {
        return false;
    }
    match sequence.import_binding {
        WindowsLoaderImportBindingRef::Package { binding_ordinal } => {
            let Some(binding) = resolution.package_module_bindings.get(binding_ordinal) else {
                return false;
            };
            steps
                .iter()
                .all(|step| step.normalized_name == binding.normalized_import_name)
                && steps.last().is_some_and(|terminal| {
                    terminal.search_directory_ordinal == binding.resolved_search_directory_ordinal
                        && matches!(
                            &terminal.disposition,
                        WindowsLoaderSearchedNameDisposition::ExpectedPackage {
                            package_file_ordinal,
                            ..
                        } if *package_file_ordinal == binding.resolved_package_file_ordinal
                        )
                })
        }
        WindowsLoaderImportBindingRef::System { binding_ordinal } => {
            let Some(binding) = resolution.system_module_bindings.get(binding_ordinal) else {
                return false;
            };
            match system_terminal_search_binding(binding) {
                Some((search_directory_ordinal, searched_name)) => {
                    steps
                        .iter()
                        .all(|step| step.normalized_name == searched_name)
                        && steps.last().is_some_and(|terminal| {
                            terminal.search_directory_ordinal == search_directory_ordinal
                                && binding.resolved_search_directory_ordinal
                                    == Some(search_directory_ordinal)
                                && matches!(
                                    &terminal.disposition,
                                    WindowsLoaderSearchedNameDisposition::ExpectedSystem {
                                        resolved_component_identity_digest,
                                        ..
                                    } if resolved_component_identity_digest
                                        == &binding.resolved_component_identity_digest
                                )
                        })
                }
                None => steps.is_empty() && binding.resolved_search_directory_ordinal.is_none(),
            }
        }
    }
}

fn same_import_binding(
    left: &WindowsLoaderImportBindingRef,
    right: &WindowsLoaderImportBindingRef,
) -> bool {
    left == right
}

mod material;
mod pe_cross_binding;

use anyhow::{bail, Result};
use serde_json::json;

use crate::node_agent_compute_plugin_host::signed_artifact_verification::jcs_sha256_hex;

use super::resolution::{
    SealedWindowsLoaderNamespaceAuthority, SealedWindowsLoaderResolutionAuthority,
    WindowsLoaderLaunchPathKind, WindowsLoaderModuleNode,
};

pub(super) use material::searched_name_disposition_digest;
use material::{
    disposition_material, edge_kind_name, filesystem_system_image_ref_material,
    import_binding_ref_material, launch_path_kind_name, module_node_material,
    search_target_material, system_resolution_origin_material,
};
use pe_cross_binding::edge_locator_material;
pub(super) use pe_cross_binding::{
    pe_import_edge_cross_binding_set_digest, pe_parsed_image_cross_binding_set_digest,
    pe_pre_post_cross_binding_receipt_digest,
};

/// Canonical PE edge material for one exact importer. The authenticated parser's per-image
/// import-table digest must be derived from these complete bindings rather than accepted as an
/// independent SHA-shaped scalar.
pub(super) fn importer_edge_table_digest(
    importer: &WindowsLoaderModuleNode,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<String> {
    let package = resolution
        .package_module_bindings
        .iter()
        .enumerate()
        .filter(|(_, entry)| &entry.importer == importer)
        .map(|(binding_ordinal, entry)| {
            (
                entry.importer_graph_edge_ordinal,
                json!({
                "target_kind": "package",
                "binding_ordinal": binding_ordinal,
                "module_request_ordinal": entry.module_request_ordinal,
                "global_import_edge_ordinal": entry.global_import_edge_ordinal,
                "edge_locator": edge_locator_material(&entry.edge_locator),
                "importer_parsed_image_ordinal": entry.importer_parsed_image_ordinal,
                "importer_graph_edge_ordinal": entry.importer_graph_edge_ordinal,
                "edge_kind": edge_kind_name(&entry.edge_kind),
                "normalized_import_name": entry.normalized_import_name,
                "imported_symbol_name": entry.imported_symbol_name,
                "imported_symbol_ordinal": entry.imported_symbol_ordinal,
                "resolved_module_cache_key": entry.resolved_module_cache_key,
                "relative_path": entry.relative_path,
                "resolved_package_file_ordinal": entry.resolved_package_file_ordinal,
                "resolved_search_directory_ordinal": entry.resolved_search_directory_ordinal,
                "digest": entry.digest,
                }),
            )
        })
        .collect::<Vec<_>>();
    let system = resolution
        .system_module_bindings
        .iter()
        .enumerate()
        .filter(|(_, entry)| &entry.importer == importer)
        .map(|(binding_ordinal, entry)| {
            (
                entry.importer_graph_edge_ordinal,
                json!({
                "target_kind": "system",
                "binding_ordinal": binding_ordinal,
                "module_request_ordinal": entry.module_request_ordinal,
                "global_import_edge_ordinal": entry.global_import_edge_ordinal,
                "edge_locator": edge_locator_material(&entry.edge_locator),
                "importer_parsed_image_ordinal": entry.importer_parsed_image_ordinal,
                "importer_graph_edge_ordinal": entry.importer_graph_edge_ordinal,
                "edge_kind": edge_kind_name(&entry.edge_kind),
                "normalized_import_name": entry.normalized_import_name,
                "imported_symbol_name": entry.imported_symbol_name,
                "imported_symbol_ordinal": entry.imported_symbol_ordinal,
                "resolved_module_cache_key": entry.resolved_module_cache_key,
                "resolved_dependency_ordinal": entry.resolved_dependency_ordinal,
                "resolved_component_identity_digest": entry.resolved_component_identity_digest,
                "resolved_image_section_identity_digest": entry.resolved_image_section_identity_digest,
                "resolution_origin": system_resolution_origin_material(&entry.resolution_origin),
                "resolved_search_directory_ordinal": entry.resolved_search_directory_ordinal,
                "filesystem_image_ref": filesystem_system_image_ref_material(entry.filesystem_image_ref.as_ref()),
                }),
            )
        })
        .collect::<Vec<_>>();
    let mut ordered_edges = package.into_iter().chain(system).collect::<Vec<_>>();
    ordered_edges.sort_by_key(|(edge_ordinal, _)| *edge_ordinal);
    let ordered_edges = ordered_edges
        .into_iter()
        .map(|(_, material)| material)
        .collect::<Vec<_>>();
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_pe_import_table.v1",
        "importer": module_node_material(importer),
        "ordered_edges": ordered_edges,
    }))
}

pub(super) fn validate_aggregate_digests(
    resolution: &SealedWindowsLoaderResolutionAuthority,
    namespace: &SealedWindowsLoaderNamespaceAuthority,
) -> Result<()> {
    let prerequisite = &namespace.prerequisite;
    let searched_names = resolution
        .searched_names
        .iter()
        .map(|entry| {
            Ok(json!({
                "ordinal": entry.searched_name_ordinal,
                "import_binding": import_binding_ref_material(&entry.import_binding),
                "search_step_ordinal": entry.search_step_ordinal,
                "normalized_name": entry.normalized_name,
                "search_directory_ordinal": entry.search_directory_ordinal,
                "search_directory_authority_binding_digest": entry.search_directory_authority_binding_digest,
                "grant_request_digest": entry.grant_request_digest,
                "disposition_binding_digest": entry.disposition_binding_digest,
                "disposition": disposition_material(&entry.disposition),
            }))
        })
        .collect::<Result<Vec<_>>>()?;
    if jcs_sha256_hex(&searched_names)? != prerequisite.searched_name_set_digest {
        bail!("COMPUTE_PLUGIN_LOADER_SEARCHED_NAME_SET_DIGEST_CHANGED");
    }

    let mut grants = prerequisite
        .searched_name_grants
        .iter()
        .map(|entry| {
            let (grant_generation, parent, name, disposition, fence_generation) =
                entry.grant.binding();
            let (request, nonce, response, positive_receipt) =
                entry.grant.authenticated_positive_binding();
            json!({
                "grant_kind": "import_search_name",
                "searched_name_ordinal": entry.searched_name_ordinal,
                "search_directory_ordinal": entry.search_directory_ordinal,
                "grant_generation": grant_generation,
                "parent_directory_identity_digest": parent,
                "normalized_name": name,
                "disposition_digest": disposition,
                "fence_generation_digest": fence_generation,
                "request_digest": request,
                "query_nonce_digest": nonce,
                "authenticated_response_digest": response,
                "positive_receipt_digest": positive_receipt,
            })
        })
        .collect::<Vec<_>>();
    let launch_path_components = resolution
        .launch_path_authority
        .components
        .iter()
        .map(|entry| {
            json!({
                "path_kind": launch_path_kind_name(entry.path_kind),
                "component_ordinal": entry.component_ordinal,
                "parent_directory_identity_digest": entry.parent_directory_identity_digest,
                "normalized_component": entry.normalized_component,
                "expected_object_identity_digest": entry.expected_object_identity_digest,
            })
        })
        .collect::<Vec<_>>();
    for entry in &prerequisite.launch_path_component_grants {
        let (grant_generation, parent, name, disposition, fence_generation) = entry.grant.binding();
        let (request, nonce, response, positive_receipt) =
            entry.grant.authenticated_positive_binding();
        grants.push(json!({
            "grant_kind": "launch_path_component",
            "path_kind": launch_path_kind_name(entry.path_kind),
            "component_ordinal": entry.component_ordinal,
            "grant_generation": grant_generation,
            "parent_directory_identity_digest": parent,
            "normalized_name": name,
            "disposition_digest": disposition,
            "fence_generation_digest": fence_generation,
            "request_digest": request,
            "query_nonce_digest": nonce,
            "authenticated_response_digest": response,
            "positive_receipt_digest": positive_receipt,
        }));
    }
    if jcs_sha256_hex(&launch_path_components)?
        != resolution.launch_path_authority.component_set_digest
    {
        bail!("COMPUTE_PLUGIN_LOADER_LAUNCH_PATH_SET_DIGEST_CHANGED");
    }
    let application_components = resolution
        .launch_path_authority
        .components
        .iter()
        .filter(|entry| entry.path_kind == WindowsLoaderLaunchPathKind::Application)
        .map(|entry| {
            json!({
                "path_kind": launch_path_kind_name(entry.path_kind),
                "component_ordinal": entry.component_ordinal,
                "parent_directory_identity_digest": entry.parent_directory_identity_digest,
                "normalized_component": entry.normalized_component,
                "expected_object_identity_digest": entry.expected_object_identity_digest,
            })
        })
        .collect::<Vec<_>>();
    let working_directory_components = resolution
        .launch_path_authority
        .components
        .iter()
        .filter(|entry| entry.path_kind == WindowsLoaderLaunchPathKind::WorkingDirectory)
        .map(|entry| {
            json!({
                "path_kind": launch_path_kind_name(entry.path_kind),
                "component_ordinal": entry.component_ordinal,
                "parent_directory_identity_digest": entry.parent_directory_identity_digest,
                "normalized_component": entry.normalized_component,
                "expected_object_identity_digest": entry.expected_object_identity_digest,
            })
        })
        .collect::<Vec<_>>();
    if jcs_sha256_hex(&application_components)?
        != resolution
            .launch_path_authority
            .application_component_set_digest
        || jcs_sha256_hex(&working_directory_components)?
            != resolution
                .launch_path_authority
                .working_directory_component_set_digest
    {
        bail!("COMPUTE_PLUGIN_LOADER_LAUNCH_PATH_KIND_DIGEST_CHANGED");
    }
    if jcs_sha256_hex(&grants)? != prerequisite.fence_generation_set_digest {
        bail!("COMPUTE_PLUGIN_LOADER_FENCE_GENERATION_SET_DIGEST_CHANGED");
    }

    let search_directories = resolution
        .search_directories
        .iter()
        .map(|entry| {
            json!({
                "ordinal": entry.search_directory_ordinal,
                "target": search_target_material(&entry.target),
                "canonical_path_digest": entry.canonical_path_digest,
                "directory_identity_digest": entry.directory_identity_digest,
                "policy_source_digest": entry.policy_source_digest,
            })
        })
        .collect::<Vec<_>>();
    let package_modules = resolution
        .package_module_bindings
        .iter()
        .map(|entry| {
            json!({
                "module_request_ordinal": entry.module_request_ordinal,
                "global_import_edge_ordinal": entry.global_import_edge_ordinal,
                "edge_locator": edge_locator_material(&entry.edge_locator),
                "importer_parsed_image_ordinal": entry.importer_parsed_image_ordinal,
                "importer": module_node_material(&entry.importer),
                "importer_graph_edge_ordinal": entry.importer_graph_edge_ordinal,
                "edge_kind": edge_kind_name(&entry.edge_kind),
                "normalized_import_name": entry.normalized_import_name,
                "imported_symbol_name": entry.imported_symbol_name,
                "imported_symbol_ordinal": entry.imported_symbol_ordinal,
                "resolved_module_cache_key": entry.resolved_module_cache_key,
                "relative_path": entry.relative_path,
                "resolved_package_file_ordinal": entry.resolved_package_file_ordinal,
                "resolved_search_directory_ordinal": entry.resolved_search_directory_ordinal,
                "digest": entry.digest,
            })
        })
        .collect::<Vec<_>>();
    let system_modules = resolution
        .system_module_bindings
        .iter()
        .map(|entry| {
            json!({
                "module_request_ordinal": entry.module_request_ordinal,
                "global_import_edge_ordinal": entry.global_import_edge_ordinal,
                "edge_locator": edge_locator_material(&entry.edge_locator),
                "importer_parsed_image_ordinal": entry.importer_parsed_image_ordinal,
                "importer": module_node_material(&entry.importer),
                "importer_graph_edge_ordinal": entry.importer_graph_edge_ordinal,
                "edge_kind": edge_kind_name(&entry.edge_kind),
                "normalized_import_name": entry.normalized_import_name,
                "imported_symbol_name": entry.imported_symbol_name,
                "imported_symbol_ordinal": entry.imported_symbol_ordinal,
                "resolved_module_cache_key": entry.resolved_module_cache_key,
                "resolved_dependency_ordinal": entry.resolved_dependency_ordinal,
                "resolved_component_identity_digest": entry.resolved_component_identity_digest,
                "resolved_image_section_identity_digest": entry.resolved_image_section_identity_digest,
                "resolution_origin": system_resolution_origin_material(&entry.resolution_origin),
                "resolved_search_directory_ordinal": entry.resolved_search_directory_ordinal,
                "filesystem_image_ref": filesystem_system_image_ref_material(entry.filesystem_image_ref.as_ref()),
            })
        })
        .collect::<Vec<_>>();
    if jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_pe_import_edges.v1",
        "package": &package_modules,
        "system": &system_modules,
    }))? != resolution.pe_import_graph.import_edge_set_digest
    {
        bail!("COMPUTE_PLUGIN_LOADER_IMPORT_EDGE_SET_DIGEST_CHANGED");
    }
    let resolved_dependencies = resolution
        .resolved_system_dependencies
        .iter()
        .map(|entry| {
            json!({
                "dependency_ordinal": entry.dependency_ordinal,
                "dependency_id": entry.dependency_id,
                "version_requirement": entry.version_requirement,
                "component_identity_digests": entry.component_identity_digests,
                "component_identity_set_digest": entry.component_identity_set_digest,
                "resolver_evidence_digest": entry.resolver_evidence_digest,
            })
        })
        .collect::<Vec<_>>();
    let system_images = resolution
        .system_module_images
        .component_images
        .iter()
        .map(|entry| {
            json!({
                "component_identity_digest": entry.component_identity_digest,
                "image_file_identity_digest": entry.image_file_identity_digest,
                "code_integrity_evidence_digest": entry.code_integrity_evidence_digest,
                "servicing_generation_digest": entry.servicing_generation_digest,
                "immutable_section_identity_digest": entry.immutable_section_identity_digest,
            })
        })
        .collect::<Vec<_>>();
    if jcs_sha256_hex(&system_images)? != resolution.system_module_images.component_image_set_digest
    {
        bail!("COMPUTE_PLUGIN_LOADER_SYSTEM_IMAGE_SET_DIGEST_CHANGED");
    }
    let resolved_system_image_custodies = resolution
        .resolved_filesystem_system_images
        .iter()
        .map(|custody| {
            let (request, candidate, session, lease_request, nonce, response, receipt) =
                custody.outcome.binding();
            json!({
                "resolution_request_ordinal": custody.resolution_request_ordinal,
                "outcome_request_ordinal": request,
                "candidate_binding_digest": candidate,
                "lease_session_identity_digest": session,
                "lease_request_digest": lease_request,
                "query_nonce_digest": nonce,
                "authenticated_response_digest": response,
                "positive_receipt_digest": receipt,
            })
        })
        .collect::<Vec<_>>();
    let preloaded_modules = resolution
        .preloaded_module_authority
        .modules
        .iter()
        .map(|entry| {
            json!({
                "resolved_module_cache_key": entry.resolved_module_cache_key,
                "component_identity_digest": entry.component_identity_digest,
                "immutable_section_identity_digest": entry.immutable_section_identity_digest,
                "preload_evidence_digest": entry.preload_evidence_digest,
            })
        })
        .collect::<Vec<_>>();
    if jcs_sha256_hex(&preloaded_modules)?
        != resolution.preloaded_module_authority.module_set_digest
    {
        bail!("COMPUTE_PLUGIN_LOADER_PRELOADED_MODULE_SET_DIGEST_CHANGED");
    }

    let known_dll_sections = resolution
        .known_dll_authority
        .sections
        .iter()
        .map(|entry| {
            json!({
                "normalized_name": entry.normalized_name,
                "resolved_module_cache_key": entry.resolved_module_cache_key,
                "section_identity_digest": entry.section_identity_digest,
                "component_identity_digest": entry.component_identity_digest,
                "immutable_image_section_identity_digest": entry.immutable_image_section_identity_digest,
                "section_image_mapping_receipt_digest": entry.section_image_mapping_receipt_digest,
            })
        })
        .collect::<Vec<_>>();
    if jcs_sha256_hex(&known_dll_sections)?
        != resolution.known_dll_authority.section_binding_set_digest
    {
        bail!("COMPUTE_PLUGIN_LOADER_KNOWN_DLL_SET_DIGEST_CHANGED");
    }
    let api_set_bindings = resolution
        .api_set_authority
        .contract_host_bindings
        .iter()
        .map(|entry| {
            json!({
                "normalized_contract_name": entry.normalized_contract_name,
                "host_module_cache_key": entry.host_module_cache_key,
                "host_component_identity_digest": entry.host_component_identity_digest,
            })
        })
        .collect::<Vec<_>>();
    if jcs_sha256_hex(&api_set_bindings)?
        != resolution
            .api_set_authority
            .contract_host_binding_set_digest
    {
        bail!("COMPUTE_PLUGIN_LOADER_API_SET_BINDING_DIGEST_CHANGED");
    }
    let side_by_side_bindings = resolution
        .side_by_side_authority
        .assembly_bindings
        .iter()
        .map(|entry| {
            json!({
                "normalized_import_name": entry.normalized_import_name,
                "resolved_module_cache_key": entry.resolved_module_cache_key,
                "assembly_identity_digest": entry.assembly_identity_digest,
                "component_identity_digest": entry.component_identity_digest,
                "image_file_identity_digest": entry.image_file_identity_digest,
                "immutable_section_identity_digest": entry.immutable_section_identity_digest,
                "activation_context_resolution_receipt_digest": entry.activation_context_resolution_receipt_digest,
            })
        })
        .collect::<Vec<_>>();
    if jcs_sha256_hex(&side_by_side_bindings)?
        != resolution
            .side_by_side_authority
            .assembly_binding_set_digest
    {
        bail!("COMPUTE_PLUGIN_LOADER_SXS_BINDING_DIGEST_CHANGED");
    }

    let parsed_images = resolution
        .pe_import_graph
        .parsed_images
        .iter()
        .map(|entry| {
            json!({
                "parsed_image_ordinal": entry.parsed_image_ordinal,
                "node": module_node_material(&entry.node),
                "image_material_identity_digest": entry.image_material_identity_digest,
                "import_table_digest": entry.import_table_digest,
                "normal_import_count": entry.normal_import_count,
                "delay_import_count": entry.delay_import_count,
                "forwarder_count": entry.forwarder_count,
            })
        })
        .collect::<Vec<_>>();
    if jcs_sha256_hex(&parsed_images)? != resolution.pe_import_graph.parsed_image_set_digest {
        bail!("COMPUTE_PLUGIN_LOADER_PARSED_IMAGE_SET_DIGEST_CHANGED");
    }
    let reachable_nodes = resolution
        .pe_import_graph
        .reachable_nodes
        .iter()
        .map(|entry| {
            json!({
                "reachable_node_ordinal": entry.reachable_node_ordinal,
                "node": module_node_material(&entry.node),
            })
        })
        .collect::<Vec<_>>();
    if jcs_sha256_hex(&reachable_nodes)? != resolution.pe_import_graph.reachable_node_set_digest {
        bail!("COMPUTE_PLUGIN_LOADER_REACHABLE_NODE_SET_DIGEST_CHANGED");
    }
    let search_sequences = resolution
        .pe_import_graph
        .search_sequences
        .iter()
        .map(|entry| {
            json!({
                "sequence_ordinal": entry.sequence_ordinal,
                "import_binding": import_binding_ref_material(&entry.import_binding),
                "searched_name_ordinals": entry.searched_name_ordinals,
            })
        })
        .collect::<Vec<_>>();
    if jcs_sha256_hex(&search_sequences)? != resolution.pe_import_graph.search_sequence_set_digest {
        bail!("COMPUTE_PLUGIN_LOADER_SEARCH_SEQUENCE_SET_DIGEST_CHANGED");
    }

    let resolution_material = json!({
        "schema": "elon.compute_plugin.windows_loader_resolution_profile.v2",
        "admission_source_digest": resolution.admission_source_digest,
        "admission_receipt_digest": resolution.admission_receipt_digest,
        "extraction_plan_digest": resolution.extraction_plan_digest,
        "extraction_evidence_digest": resolution.extraction_evidence_digest,
        "runner_relative_path": resolution.runner_relative_path,
        "working_directory_relative_path": resolution.working_directory_relative_path,
        "working_directory_identity_digest": resolution.working_directory_identity_digest,
        "search_directories": search_directories,
        "known_dll": {
            "os_build_identity_digest": resolution.known_dll_authority.os_build_identity_digest,
            "object_manager_directory_identity_digest": resolution.known_dll_authority.object_manager_directory_identity_digest,
            "section_binding_set_digest": resolution.known_dll_authority.section_binding_set_digest,
            "section_namespace_generation_digest": resolution.known_dll_authority.section_namespace_generation_digest,
        },
        "api_set": {
            "os_build_identity_digest": resolution.api_set_authority.os_build_identity_digest,
            "schema_identity_digest": resolution.api_set_authority.schema_identity_digest,
            "contract_host_binding_set_digest": resolution.api_set_authority.contract_host_binding_set_digest,
        },
        "side_by_side": {
            "activation_context_identity_digest": resolution.side_by_side_authority.activation_context_identity_digest,
            "manifest_set_digest": resolution.side_by_side_authority.manifest_set_digest,
            "assembly_binding_set_digest": resolution.side_by_side_authority.assembly_binding_set_digest,
        },
        "package_module_bindings": package_modules,
        "system_module_bindings": system_modules,
        "signed_system_dependencies": {
            "manifest_digest": resolution.signed_system_dependencies.manifest_digest,
            "signed_manifest_envelope_digest": resolution.signed_system_dependencies.signed_manifest_envelope_digest,
            "projection_digest": resolution.signed_system_dependencies.projection_digest,
        },
        "resolved_system_dependencies": resolved_dependencies,
        "system_component_image_set_digest": resolution.system_module_images.component_image_set_digest,
        "resolved_filesystem_system_image_custodies": resolved_system_image_custodies,
        "preloaded_modules": {
            "process_machine_context_digest": resolution.preloaded_module_authority.process_machine_context_digest,
            "modules": preloaded_modules,
            "module_set_digest": resolution.preloaded_module_authority.module_set_digest,
        },
        "package_content_lease_set_digest": resolution.package_content_lease_set_digest,
        "system_content_lease_set_digest": resolution.system_content_lease_set_digest,
        "immutable_content_lease_set_digest": resolution.immutable_content_lease_set_digest,
        "searched_name_set_digest": prerequisite.searched_name_set_digest,
        "pe_import_graph": {
            "root_package_file_ordinal": resolution.pe_import_graph.root_package_file_ordinal,
            "parsed_images": parsed_images,
            "reachable_nodes": reachable_nodes,
            "search_sequences": search_sequences,
            "parsed_image_set_digest": resolution.pe_import_graph.parsed_image_set_digest,
            "import_edge_set_digest": resolution.pe_import_graph.import_edge_set_digest,
            "reachable_node_set_digest": resolution.pe_import_graph.reachable_node_set_digest,
            "search_sequence_set_digest": resolution.pe_import_graph.search_sequence_set_digest,
            "expected_package_edge_count": resolution.pe_import_graph.expected_package_edge_count,
            "expected_system_edge_count": resolution.pe_import_graph.expected_system_edge_count,
            "expected_search_step_count": resolution.pe_import_graph.expected_search_step_count,
            "parsed_image_cross_binding_set_digest": resolution.pe_import_graph.pre_post_cross_binding.parsed_image_cross_binding_set_digest,
            "import_edge_cross_binding_set_digest": resolution.pe_import_graph.pre_post_cross_binding.import_edge_cross_binding_set_digest,
            "pre_post_cross_binding_receipt_digest": resolution.pe_import_graph.pre_post_cross_binding.receipt_digest,
        },
        "launch_path_component_set_digest": resolution.launch_path_authority.component_set_digest,
        "application_launch_path_component_set_digest": resolution.launch_path_authority.application_component_set_digest,
        "working_directory_launch_path_component_set_digest": resolution.launch_path_authority.working_directory_component_set_digest,
        "retained_parent_chain_share_contract_set_digest": resolution.launch_path_authority.retained_parent_chain_share_contract_set_digest,
        "launch_context_selector_digest": resolution.launch_context_selector_digest,
        "selected_context_binding_digest": resolution.selected_context_binding_digest,
        "preliminary_resolution_request_plan_digest": resolution.preliminary_resolution_request_plan_digest,
        "grant_ready_resolution_plan_digest": resolution.grant_ready_resolution_plan_digest,
        "process_machine_context_digest": resolution.process_machine_context_digest,
    });
    if jcs_sha256_hex(&resolution_material)? != resolution.resolution_profile_digest {
        bail!("COMPUTE_PLUGIN_LOADER_RESOLUTION_PROFILE_DIGEST_CHANGED");
    }

    let (session_identity, grant_generation, generation_domain) = prerequisite.session.binding();
    let (
        _,
        _,
        initial_query_generation,
        _,
        initial_receipt,
        initial_request,
        initial_nonce,
        initial_fences,
        initial_content_leases,
    ) = prerequisite.initial_query_receipt.binding();
    let (
        _,
        _,
        final_query_generation,
        _,
        final_receipt,
        final_request,
        final_nonce,
        final_fences,
        final_content_leases,
    ) = namespace.final_query_receipt.binding();
    let namespace_material = json!({
        "schema": "elon.compute_plugin.windows_loader_namespace_authority.v1",
        "resolution_profile_digest": resolution.resolution_profile_digest,
        "searched_name_set_digest": prerequisite.searched_name_set_digest,
        "fence_generation_set_digest": prerequisite.fence_generation_set_digest,
        "session_identity_digest": session_identity,
        "grant_generation": grant_generation,
        "generation_domain_digest": generation_domain,
        "initial_query_generation": initial_query_generation,
        "initial_query_receipt_digest": initial_receipt,
        "initial_query_request_digest": initial_request,
        "initial_query_nonce_digest": initial_nonce,
        "initial_fence_generation_set_digest": initial_fences,
        "initial_content_lease_generation_set_digest": initial_content_leases,
        "final_query_generation": final_query_generation,
        "final_query_receipt_digest": final_receipt,
        "final_query_request_digest": final_request,
        "final_query_nonce_digest": final_nonce,
        "final_fence_generation_set_digest": final_fences,
        "final_content_lease_generation_set_digest": final_content_leases,
    });
    if jcs_sha256_hex(&namespace_material)? != namespace.namespace_authority_digest {
        bail!("COMPUTE_PLUGIN_LOADER_NAMESPACE_AUTHORITY_DIGEST_CHANGED");
    }
    Ok(())
}

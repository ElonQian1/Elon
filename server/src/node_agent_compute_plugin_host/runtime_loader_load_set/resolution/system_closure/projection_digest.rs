//! Canonical reverse projection of one recursive wave from the final sealed graph.

use anyhow::{anyhow, Result};
use serde_json::json;

use crate::node_agent_compute_plugin_host::{
    runtime_loader_load_set::digest::{
        edge_kind_name, filesystem_system_image_ref_material, final_edge_locator_material,
        module_node_material, searched_name_disposition_digest, system_resolution_origin_material,
    },
    signed_artifact_verification::jcs_sha256_hex,
};

use super::super::{SealedWindowsLoaderResolutionAuthority, WindowsLoaderImportBindingRef};
use super::WindowsRecursiveResolutionWavePlan;

pub(super) fn edge_set_digest(
    wave: &WindowsRecursiveResolutionWavePlan,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<String> {
    let end = checked_end(wave.first_module_request_ordinal, wave.module_request_count)?;
    let package = resolution
        .package_module_bindings
        .iter()
        .enumerate()
        .filter(|(_, binding)| {
            binding.module_request_ordinal >= wave.first_module_request_ordinal
                && binding.module_request_ordinal < end
        })
        .map(|(binding_ordinal, binding)| {
            (
                binding.module_request_ordinal,
                json!({
                "target_kind": "package",
                "binding_ordinal": binding_ordinal,
                "module_request_ordinal": binding.module_request_ordinal,
                "global_import_edge_ordinal": binding.global_import_edge_ordinal,
                "edge_locator": final_edge_locator_material(&binding.edge_locator),
                "importer_parsed_image_ordinal": binding.importer_parsed_image_ordinal,
                "importer": module_node_material(&binding.importer),
                "importer_graph_edge_ordinal": binding.importer_graph_edge_ordinal,
                "edge_kind": edge_kind_name(&binding.edge_kind),
                "normalized_import_name": binding.normalized_import_name,
                "imported_symbol_name": binding.imported_symbol_name,
                "imported_symbol_ordinal": binding.imported_symbol_ordinal,
                "resolved_module_cache_key": binding.resolved_module_cache_key,
                "relative_path": binding.relative_path,
                "resolved_package_file_ordinal": binding.resolved_package_file_ordinal,
                "resolved_search_directory_ordinal": binding.resolved_search_directory_ordinal,
                "digest": binding.digest,
                }),
            )
        });
    let system = resolution
        .system_module_bindings
        .iter()
        .enumerate()
        .filter(|(_, binding)| {
            binding.module_request_ordinal >= wave.first_module_request_ordinal
                && binding.module_request_ordinal < end
        })
        .map(|(binding_ordinal, binding)| {
            (
                binding.module_request_ordinal,
                json!({
                "target_kind": "system",
                "binding_ordinal": binding_ordinal,
                "module_request_ordinal": binding.module_request_ordinal,
                "global_import_edge_ordinal": binding.global_import_edge_ordinal,
                "edge_locator": final_edge_locator_material(&binding.edge_locator),
                "importer_parsed_image_ordinal": binding.importer_parsed_image_ordinal,
                "importer": module_node_material(&binding.importer),
                "importer_graph_edge_ordinal": binding.importer_graph_edge_ordinal,
                "edge_kind": edge_kind_name(&binding.edge_kind),
                "normalized_import_name": binding.normalized_import_name,
                "imported_symbol_name": binding.imported_symbol_name,
                "imported_symbol_ordinal": binding.imported_symbol_ordinal,
                "resolved_module_cache_key": binding.resolved_module_cache_key,
                "resolved_dependency_ordinal": binding.resolved_dependency_ordinal,
                "resolved_component_identity_digest": binding.resolved_component_identity_digest,
                "resolved_image_section_identity_digest": binding.resolved_image_section_identity_digest,
                "resolution_origin": system_resolution_origin_material(&binding.resolution_origin),
                "resolved_search_directory_ordinal": binding.resolved_search_directory_ordinal,
                "filesystem_image_ref": filesystem_system_image_ref_material(
                    binding.filesystem_image_ref.as_ref()
                ),
                }),
            )
        });
    let mut edges = package.chain(system).collect::<Vec<_>>();
    edges.sort_by_key(|(module_request_ordinal, _)| *module_request_ordinal);
    let edges = edges
        .into_iter()
        .map(|(_, material)| material)
        .collect::<Vec<_>>();
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_wave_edge_set.v1",
        "wave_ordinal": wave.wave_ordinal,
        "edges": edges,
    }))
}

pub(super) fn searched_name_set_digest(
    wave: &WindowsRecursiveResolutionWavePlan,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<String> {
    let end = checked_end(wave.first_searched_name_ordinal, wave.searched_name_count)?;
    let names = resolution
        .searched_names
        .get(wave.first_searched_name_ordinal..end)
        .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_SEARCH_RANGE_CHANGED"))?
        .iter()
        .map(|name| {
            let import_binding = match name.import_binding {
                WindowsLoaderImportBindingRef::Package { binding_ordinal } => {
                    json!({"kind": "package", "binding_ordinal": binding_ordinal})
                }
                WindowsLoaderImportBindingRef::System { binding_ordinal } => {
                    json!({"kind": "system", "binding_ordinal": binding_ordinal})
                }
            };
            Ok(json!({
                "searched_name_ordinal": name.searched_name_ordinal,
                "import_binding": import_binding,
                "search_step_ordinal": name.search_step_ordinal,
                "search_directory_ordinal": name.search_directory_ordinal,
                "normalized_name": name.normalized_name,
                "search_directory_authority_binding_digest": name.search_directory_authority_binding_digest,
                "grant_request_digest": name.grant_request_digest,
                "disposition_binding_digest": name.disposition_binding_digest,
                "disposition_digest": searched_name_disposition_digest(&name.disposition)?,
            }))
        })
        .collect::<Result<Vec<_>>>()?;
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_wave_searched_name_set.v1",
        "wave_ordinal": wave.wave_ordinal,
        "searched_names": names,
    }))
}

pub(super) fn system_image_set_digest(
    wave: &WindowsRecursiveResolutionWavePlan,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<String> {
    let end = checked_end(
        wave.first_system_image_request_ordinal,
        wave.system_image_request_count,
    )?;
    let owners = resolution
        .resolved_filesystem_system_images
        .get(wave.first_system_image_request_ordinal..end)
        .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_SYSTEM_OWNER_RANGE_CHANGED"))?
        .iter()
        .map(|custody| {
            let (request, candidate, session, lease_request, nonce, response, receipt) =
                custody.outcome.binding();
            let (parent, name, file, section, open_receipt, mapping_receipt) =
                custody.outcome.image().binding();
            let (_, _, servicing, generation, policy) =
                custody.outcome.image().content_lease_binding();
            json!({
                "resolution_request_ordinal": custody.resolution_request_ordinal,
                "outcome_request_ordinal": request,
                "candidate_binding_digest": candidate,
                "lease_session_identity_digest": session,
                "lease_request_digest": lease_request,
                "query_nonce_digest": nonce,
                "lease_response_digest": response,
                "lease_receipt_digest": receipt,
                "parent_directory_identity_digest": parent,
                "normalized_name": name,
                "image_file_identity_digest": file,
                "immutable_section_identity_digest": section,
                "open_receipt_digest": open_receipt,
                "mapping_receipt_digest": mapping_receipt,
                "servicing_generation_digest": servicing,
                "content_lease_generation_digest": generation,
                "immutable_content_policy_digest": policy,
            })
        })
        .collect::<Vec<_>>();
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_wave_system_image_set.v1",
        "wave_ordinal": wave.wave_ordinal,
        "owners": owners,
    }))
}

fn checked_end(first: usize, count: usize) -> Result<usize> {
    first
        .checked_add(count)
        .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_COUNT_OVERFLOW"))
}

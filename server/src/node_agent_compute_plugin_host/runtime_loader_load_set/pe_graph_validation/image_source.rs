//! Exact byte/section owner binding for every final parsed PE image.

use anyhow::Result;
use serde_json::json;

use crate::node_agent_compute_plugin_host::signed_artifact_verification::jcs_sha256_hex;

use super::super::{
    model::SealedComputePluginRunnerImage,
    resolution::{
        SealedWindowsLoaderResolutionAuthority, WindowsPeParsedImageBinding,
        WindowsPeParsedImageSource, WindowsRecursiveImageOwnerRef,
    },
};

pub(super) fn parsed_image_source_binding(
    parsed: &WindowsPeParsedImageBinding,
    image: &SealedComputePluginRunnerImage,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<Option<(String, String)>> {
    match parsed.source {
        WindowsPeParsedImageSource::BasePreleasePackage {
            prelease_parsed_image_ordinal,
        } => base_package_binding(parsed, prelease_parsed_image_ordinal, image, resolution),
        WindowsPeParsedImageSource::RecursiveExpansion {
            parse_receipt_ordinal,
        } => recursive_binding(parsed, parse_receipt_ordinal, image, resolution),
    }
}

fn base_package_binding(
    parsed: &WindowsPeParsedImageBinding,
    prelease_parsed_image_ordinal: usize,
    image: &SealedComputePluginRunnerImage,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<Option<(String, String)>> {
    let Some(cross) = resolution
        .pe_import_graph
        .pre_post_cross_binding
        .parsed_image_cross_bindings
        .iter()
        .find(|cross| {
            cross.prelease_parsed_image_ordinal == prelease_parsed_image_ordinal
                && cross.postlease_parsed_image_ordinal == parsed.parsed_image_ordinal
        })
    else {
        return Ok(None);
    };
    let Some(entry) = image.package_files.get(cross.package_file_ordinal) else {
        return Ok(None);
    };
    let (file_identity, sealed_digest, lease_generation, immutable_policy) =
        entry.file.content_lease_binding();
    let material = package_parsed_image_material_digest(
        file_identity,
        sealed_digest,
        lease_generation,
        immutable_policy,
    )?;
    let source = jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_base_prelease_parsed_image_source.v1",
        "prelease_parsed_image_ordinal": prelease_parsed_image_ordinal,
        "postlease_parsed_image_ordinal": parsed.parsed_image_ordinal,
        "package_file_ordinal": cross.package_file_ordinal,
        "file_identity_digest": file_identity,
        "sealed_content_digest": sealed_digest,
        "content_lease_generation_digest": lease_generation,
        "immutable_content_policy_digest": immutable_policy,
        "same_handle_lease_generation_digest": cross.lease_generation_digest,
    }))?;
    Ok((material == cross.postlease_image_material_identity_digest).then_some((source, material)))
}

fn recursive_binding(
    parsed: &WindowsPeParsedImageBinding,
    parse_receipt_ordinal: usize,
    image: &SealedComputePluginRunnerImage,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<Option<(String, String)>> {
    let Some(receipt) = resolution
        .pe_import_graph
        .recursive_resolution_closure
        .parse_receipts
        .get(parse_receipt_ordinal)
    else {
        return Ok(None);
    };
    if receipt.parsed_image_ordinal != parsed.parsed_image_ordinal || receipt.node != parsed.node {
        return Ok(None);
    }
    let Some((source_owner, image_material)) =
        recursive_owner_binding(&receipt.source_owner, image, resolution)?
    else {
        return Ok(None);
    };
    Ok((source_owner == receipt.source_owner_binding_digest
        && image_material == receipt.image_material_identity_digest)
        .then_some((receipt.receipt_digest.clone(), image_material)))
}

fn recursive_owner_binding(
    owner: &WindowsRecursiveImageOwnerRef,
    image: &SealedComputePluginRunnerImage,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<Option<(String, String)>> {
    let (owner_material, image_material) = match owner {
        WindowsRecursiveImageOwnerRef::PackageContentLease {
            package_file_ordinal,
        } => {
            let Some(entry) = image.package_files.get(*package_file_ordinal) else {
                return Ok(None);
            };
            let (file, sealed, generation, policy) = entry.file.content_lease_binding();
            let owner = json!({
                "kind": "package_content_lease",
                "package_file_ordinal": package_file_ordinal,
                "file_identity_digest": file,
                "sealed_content_digest": sealed,
                "content_lease_generation_digest": generation,
                "immutable_content_policy_digest": policy,
            });
            let material = package_parsed_image_material_digest(file, sealed, generation, policy)?;
            (owner, material)
        }
        WindowsRecursiveImageOwnerRef::AuthenticatedPreloadedModule {
            preloaded_module_ordinal,
        } => {
            let Some(module) = resolution
                .preloaded_module_authority
                .modules
                .get(*preloaded_module_ordinal)
            else {
                return Ok(None);
            };
            let owner = json!({
                "kind": "authenticated_preloaded_module",
                "preloaded_module_ordinal": preloaded_module_ordinal,
                "module_cache_key": module.resolved_module_cache_key,
                "component_identity_digest": module.component_identity_digest,
                "immutable_section_identity_digest": module.immutable_section_identity_digest,
                "preload_evidence_digest": module.preload_evidence_digest,
            });
            (owner, module.immutable_section_identity_digest.clone())
        }
        WindowsRecursiveImageOwnerRef::KnownDllSection {
            known_dll_authority_record_ordinal,
        } => {
            let Some(section) = resolution
                .known_dll_authority
                .sections
                .get(*known_dll_authority_record_ordinal)
            else {
                return Ok(None);
            };
            let owner = json!({
                "kind": "known_dll_section",
                "known_dll_authority_record_ordinal": known_dll_authority_record_ordinal,
                "section_identity_digest": section.section_identity_digest,
                "component_identity_digest": section.component_identity_digest,
                "immutable_section_identity_digest": section.immutable_image_section_identity_digest,
                "mapping_receipt_digest": section.section_image_mapping_receipt_digest,
                "namespace_generation_digest": resolution.known_dll_authority.section_namespace_generation_digest,
            });
            (
                owner,
                section.immutable_image_section_identity_digest.clone(),
            )
        }
        WindowsRecursiveImageOwnerRef::ResolvedFilesystemSystemImage {
            resolution_request_ordinal,
        } => {
            let Some(custody) = resolution
                .resolved_filesystem_system_images
                .get(*resolution_request_ordinal)
            else {
                return Ok(None);
            };
            let (request, candidate, session, lease_request, nonce, response, receipt) =
                custody.outcome.binding();
            let (parent, name, file, section, open_receipt, mapping_receipt) =
                custody.outcome.image().binding();
            let (_, _, servicing, generation, policy) =
                custody.outcome.image().content_lease_binding();
            let owner = json!({
                "kind": "resolved_filesystem_system_image",
                "resolution_request_ordinal": resolution_request_ordinal,
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
            });
            let material = jcs_sha256_hex(&json!({
                "schema": "elon.compute_plugin.windows_filesystem_system_image_material.v1",
                "image_file_identity_digest": file,
                "immutable_section_identity_digest": section,
                "servicing_generation_digest": servicing,
                "content_lease_generation_digest": generation,
                "immutable_content_policy_digest": policy,
            }))?;
            (owner, material)
        }
    };
    let source = jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_image_owner_binding.v1",
        "owner": owner_material,
    }))?;
    Ok(Some((source, image_material)))
}

pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn package_parsed_image_material_digest(
    file_identity_digest: &str,
    sealed_content_digest: &str,
    content_lease_generation_digest: &str,
    immutable_content_policy_digest: &str,
) -> Result<String> {
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_package_parsed_image_material.v1",
        "file_identity_digest": file_identity_digest,
        "sealed_content_digest": sealed_content_digest,
        "content_lease_generation_digest": content_lease_generation_digest,
        "immutable_content_policy_digest": immutable_content_policy_digest,
    }))
}

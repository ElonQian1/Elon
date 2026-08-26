//! Prelease-to-postlease PE image and import-edge cross-binding digests.

use anyhow::Result;
use serde_json::{json, Value};

use crate::node_agent_compute_plugin_host::signed_artifact_verification::jcs_sha256_hex;

use super::import_binding_ref_material;
use crate::node_agent_compute_plugin_host::runtime_loader_load_set::resolution::{
    SealedWindowsPePrePostCrossBindingReceipt, WindowsPeImportEdgeCrossBinding,
    WindowsPeParsedImageCrossBinding,
};

pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn pe_pre_post_cross_binding_receipt_digest(
    receipt: &SealedWindowsPePrePostCrossBindingReceipt,
) -> Result<String> {
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_pe_pre_post_cross_binding.v2",
        "prelease_material_set_digest": receipt.prelease_material_set_digest,
        "postlease_parsed_image_set_digest": receipt.postlease_parsed_image_set_digest,
        "postlease_import_edge_set_digest": receipt.postlease_import_edge_set_digest,
        "postlease_reachable_node_set_digest": receipt.postlease_reachable_node_set_digest,
        "package_content_lease_set_digest": receipt.package_content_lease_set_digest,
        "same_retained_file_handle_set_digest": receipt.same_retained_file_handle_set_digest,
        "parsed_image_cross_binding_set_digest": receipt.parsed_image_cross_binding_set_digest,
        "import_edge_cross_binding_set_digest": receipt.import_edge_cross_binding_set_digest,
    }))
}

pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn pe_import_edge_cross_binding_set_digest(
    bindings: &[WindowsPeImportEdgeCrossBinding],
) -> Result<String> {
    let material = bindings
        .iter()
        .map(|binding| {
            json!({
                "preliminary_request_ordinal": binding.preliminary_request_ordinal,
                "prelease_importer_parsed_image_ordinal": binding.prelease_importer_parsed_image_ordinal,
                "edge_locator": preliminary_edge_locator_material(&binding.edge_locator),
                "postlease_import_binding": import_binding_ref_material(&binding.postlease_import_binding),
                "postlease_importer_parsed_image_ordinal": binding.postlease_importer_parsed_image_ordinal,
            })
        })
        .collect::<Vec<_>>();
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_pe_import_edge_cross_binding_set.v2",
        "bindings": material,
    }))
}

pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn pe_parsed_image_cross_binding_set_digest(
    bindings: &[WindowsPeParsedImageCrossBinding],
) -> Result<String> {
    let material = bindings
        .iter()
        .map(|binding| {
            json!({
                "prelease_parsed_image_ordinal": binding.prelease_parsed_image_ordinal,
                "package_file_ordinal": binding.package_file_ordinal,
                "file_identity_digest": binding.file_identity_digest,
                "postlease_parsed_image_ordinal": binding.postlease_parsed_image_ordinal,
                "postlease_image_material_identity_digest": binding.postlease_image_material_identity_digest,
                "lease_generation_digest": binding.lease_generation_digest,
            })
        })
        .collect::<Vec<_>>();
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_pe_parsed_image_cross_binding_set.v1",
        "bindings": material,
    }))
}
use crate::node_agent_compute_plugin_host::runtime_loader_load_set::launch_path_discovery::WindowsPreliminaryModuleEdgeLocator;
use crate::node_agent_compute_plugin_host::runtime_loader_load_set::resolution::{
    WindowsLoaderModuleEdgeLocator, WindowsPostLeaseModuleEdgeLocator,
};

pub(super) fn preliminary_edge_locator_material(
    locator: &WindowsPreliminaryModuleEdgeLocator,
) -> Value {
    match locator {
        WindowsPreliminaryModuleEdgeLocator::Import {
            source_import_edge_ordinal,
            descriptor_ordinal,
            thunk_ordinal,
            edge_evidence_digest,
        } => json!({
            "kind": "import",
            "source_import_edge_ordinal": source_import_edge_ordinal,
            "descriptor_ordinal": descriptor_ordinal,
            "thunk_ordinal": thunk_ordinal,
            "edge_evidence_digest": edge_evidence_digest,
        }),
        WindowsPreliminaryModuleEdgeLocator::Forwarder {
            source_import_edge_ordinal,
            forwarder_hop_ordinal,
            source_export_name,
            source_export_ordinal,
            hop_evidence_digest,
        } => json!({
            "kind": "forwarder",
            "source_import_edge_ordinal": source_import_edge_ordinal,
            "forwarder_hop_ordinal": forwarder_hop_ordinal,
            "source_export_name": source_export_name,
            "source_export_ordinal": source_export_ordinal,
            "hop_evidence_digest": hop_evidence_digest,
        }),
    }
}

pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn final_edge_locator_material(
    locator: &WindowsLoaderModuleEdgeLocator,
) -> Value {
    match locator {
        WindowsLoaderModuleEdgeLocator::BasePrelease {
            preliminary_request_ordinal,
            import_edge_cross_binding_ordinal,
            locator,
        } => json!({
            "stage": "base_prelease",
            "preliminary_request_ordinal": preliminary_request_ordinal,
            "import_edge_cross_binding_ordinal": import_edge_cross_binding_ordinal,
            "locator": preliminary_edge_locator_material(locator),
        }),
        WindowsLoaderModuleEdgeLocator::SystemPostLease {
            wave_ordinal,
            source_parsed_image_ordinal,
            parse_receipt_ordinal,
            locator,
        } => json!({
            "stage": "system_postlease",
            "wave_ordinal": wave_ordinal,
            "source_parsed_image_ordinal": source_parsed_image_ordinal,
            "parse_receipt_ordinal": parse_receipt_ordinal,
            "locator": postlease_edge_locator_material(locator),
        }),
    }
}

fn postlease_edge_locator_material(locator: &WindowsPostLeaseModuleEdgeLocator) -> Value {
    match locator {
        WindowsPostLeaseModuleEdgeLocator::Import {
            source_import_edge_ordinal,
            descriptor_ordinal,
            thunk_ordinal,
            edge_evidence_digest,
        } => json!({
            "kind": "import",
            "source_import_edge_ordinal": source_import_edge_ordinal,
            "descriptor_ordinal": descriptor_ordinal,
            "thunk_ordinal": thunk_ordinal,
            "edge_evidence_digest": edge_evidence_digest,
        }),
        WindowsPostLeaseModuleEdgeLocator::Forwarder {
            source_import_edge_ordinal,
            forwarder_hop_ordinal,
            source_export_name,
            source_export_ordinal,
            hop_evidence_digest,
        } => json!({
            "kind": "forwarder",
            "source_import_edge_ordinal": source_import_edge_ordinal,
            "forwarder_hop_ordinal": forwarder_hop_ordinal,
            "source_export_name": source_export_name,
            "source_export_ordinal": source_export_ordinal,
            "hop_evidence_digest": hop_evidence_digest,
        }),
    }
}

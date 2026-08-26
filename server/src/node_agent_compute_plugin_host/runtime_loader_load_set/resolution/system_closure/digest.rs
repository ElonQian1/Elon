use anyhow::Result;
use serde_json::{json, Value};

use crate::node_agent_compute_plugin_host::signed_artifact_verification::jcs_sha256_hex;

use super::{
    SealedWindowsRecursiveResolutionClosure, WindowsPostLeaseSystemImageParseReceipt,
    WindowsRecursiveImageOwnerRef, WindowsRecursiveResolutionWavePlan,
};

pub(super) fn owner_material(owner: &WindowsRecursiveImageOwnerRef) -> Value {
    match owner {
        WindowsRecursiveImageOwnerRef::PackageContentLease {
            package_file_ordinal,
        } => json!({
            "kind": "package_content_lease",
            "package_file_ordinal": package_file_ordinal,
        }),
        WindowsRecursiveImageOwnerRef::AuthenticatedPreloadedModule {
            preloaded_module_ordinal,
        } => json!({
            "kind": "authenticated_preloaded_module",
            "preloaded_module_ordinal": preloaded_module_ordinal,
        }),
        WindowsRecursiveImageOwnerRef::KnownDllSection {
            known_dll_authority_record_ordinal,
        } => json!({
            "kind": "known_dll_section",
            "known_dll_authority_record_ordinal": known_dll_authority_record_ordinal,
        }),
        WindowsRecursiveImageOwnerRef::ResolvedFilesystemSystemImage {
            resolution_request_ordinal,
        } => json!({
            "kind": "resolved_filesystem_system_image",
            "resolution_request_ordinal": resolution_request_ordinal,
        }),
    }
}

pub(super) fn parse_receipt_digest(
    receipt: &WindowsPostLeaseSystemImageParseReceipt,
) -> Result<String> {
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_image_parse_receipt.v2",
        "parse_receipt_ordinal": receipt.parse_receipt_ordinal,
        "wave_ordinal": receipt.wave_ordinal,
        "producer_acquisition_receipt_ordinal": receipt.producer_acquisition_receipt_ordinal,
        "producer_module_request_ordinal": receipt.producer_module_request_ordinal,
        "parsed_image_ordinal": receipt.parsed_image_ordinal,
        "node": super::super::super::digest::module_node_material(&receipt.node),
        "source_owner": owner_material(&receipt.source_owner),
        "source_owner_binding_digest": receipt.source_owner_binding_digest,
        "image_material_identity_digest": receipt.image_material_identity_digest,
        "parser_policy_digest": receipt.parser_policy_digest,
        "import_table_digest": receipt.import_table_digest,
        "normal_import_count": receipt.normal_import_count,
        "delay_import_count": receipt.delay_import_count,
        "forwarder_count": receipt.forwarder_count,
        "same_owner_parse_receipt_digest": receipt.same_owner_parse_receipt_digest,
    }))
}

pub(super) fn wave_digest(wave: &WindowsRecursiveResolutionWavePlan) -> Result<String> {
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_resolution_wave.v1",
        "wave_ordinal": wave.wave_ordinal,
        "source_parse_receipt_ordinals": wave.source_parse_receipt_ordinals,
        "first_module_request_ordinal": wave.first_module_request_ordinal,
        "module_request_count": wave.module_request_count,
        "first_searched_name_ordinal": wave.first_searched_name_ordinal,
        "searched_name_count": wave.searched_name_count,
        "first_system_image_request_ordinal": wave.first_system_image_request_ordinal,
        "system_image_request_count": wave.system_image_request_count,
        "next_frontier_parse_receipt_ordinals": wave.next_frontier_parse_receipt_ordinals,
        "parsed_edge_set_digest": wave.parsed_edge_set_digest,
        "searched_name_disposition_set_digest": wave.searched_name_disposition_set_digest,
        "acquired_system_image_set_digest": wave.acquired_system_image_set_digest,
    }))
}

pub(super) fn closure_digest(closure: &SealedWindowsRecursiveResolutionClosure) -> Result<String> {
    let parse_receipts = closure
        .parse_receipts
        .iter()
        .map(|receipt| receipt.receipt_digest.as_str())
        .collect::<Vec<_>>();
    let waves = closure
        .waves
        .iter()
        .map(|wave| wave.wave_digest.as_str())
        .collect::<Vec<_>>();
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_resolution_closure.v2",
        "base_prelease_parsed_image_count": closure.base_prelease_parsed_image_count,
        "base_module_request_count": closure.base_module_request_count,
        "base_searched_name_count": closure.base_searched_name_count,
        "base_system_image_request_count": closure.base_system_image_request_count,
        "parse_receipts": parse_receipts,
        "waves": waves,
        "recursive_acquisition_chain_digest": closure.acquisition_chain.digest(),
        "file_identity_dedupe_receipt_digest": closure.file_identity_dedupe_receipt_digest,
        "module_cache_collision_closure_receipt_digest": closure.module_cache_collision_closure_receipt_digest,
        "forwarder_cycle_closure_receipt_digest": closure.forwarder_cycle_closure_receipt_digest,
        "terminal_empty_frontier_receipt_digest": closure.terminal_empty_frontier_receipt_digest,
    }))
}

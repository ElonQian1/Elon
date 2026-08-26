//! Canonical JCS/SHA-256 commitments for recursive pre-dispatch typed plans.

use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};

use crate::node_agent_compute_plugin_host::{
    runtime_loader_load_set::digest::module_node_material,
    signed_artifact_verification::jcs_sha256_hex,
};

use super::super::super::{
    SealedWindowsLoaderResolutionAuthority, WindowsLoaderModuleNode,
    WindowsPostLeaseModuleEdgeLocator,
};
use super::super::WindowsPeParsedImageSource;
use super::{plan::*, WindowsRecursiveAcquisitionPlanEvidence};

const REQUEST_PLAN_DOMAIN: &str = "ELON_WINDOWS_RECURSIVE_WAVE_REQUEST_PLAN_V1";
const TERMINAL_SET_DOMAIN: &str = "ELON_WINDOWS_RECURSIVE_WAVE_TERMINAL_SET_V1";
const DISPOSITION_SET_DOMAIN: &str = "ELON_WINDOWS_RECURSIVE_WAVE_SEARCH_DISPOSITION_SET_V1";
const FILESYSTEM_REQUEST_SET_DOMAIN: &str = "ELON_WINDOWS_RECURSIVE_WAVE_FILESYSTEM_REQUEST_SET_V1";
const ROUTE_OWNER_SET_DOMAIN: &str = "ELON_WINDOWS_RECURSIVE_WAVE_ROUTE_OWNER_SET_V1";
const RESOLUTION_PLAN_DOMAIN: &str = "ELON_WINDOWS_RECURSIVE_WAVE_RESOLUTION_PLAN_V1";
const RETAINED_FORWARDER_CHAIN_DOMAIN: &str = "ELON_WINDOWS_RECURSIVE_RETAINED_FORWARDER_CHAIN_V1";
const RETAINED_FORWARDER_CHAIN_SET_DOMAIN: &str =
    "ELON_WINDOWS_RECURSIVE_RETAINED_FORWARDER_CHAIN_SET_V1";
const BASE_PARSED_IMAGE_OWNER_SET_DOMAIN: &str =
    "ELON_WINDOWS_RECURSIVE_BASE_PARSED_IMAGE_OWNER_SET_V1";

pub(super) fn request_plan_digest(plan: &WindowsRecursiveWaveRequestPlan) -> Result<String> {
    let requests = plan
        .module_requests
        .iter()
        .map(request_material)
        .collect::<Vec<_>>();
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_wave_request_plan.v1",
        "domain": REQUEST_PLAN_DOMAIN,
        "producer_wave_ordinal": plan.producer_wave_ordinal,
        "previous_acquisition_receipt_digest": plan.previous_acquisition_receipt_digest,
        "input_custody_digest": plan.input_custody_digest,
        "authenticated_recursive_policy_digest": plan.authenticated_recursive_policy_digest,
        "source_frontier": plan.source_frontier.iter().map(source_frontier_material).collect::<Vec<_>>(),
        "first_module_request_ordinal": plan.first_module_request_ordinal,
        "first_searched_name_ordinal": plan.first_searched_name_ordinal,
        "first_system_image_request_ordinal": plan.first_system_image_request_ordinal,
        "parser_policy_digest": plan.parser_policy_digest,
        "module_requests": requests,
    }))
}

pub(super) fn request_plan_evidence_digest(
    evidence: &WindowsRecursiveWaveDispatchPlanEvidence,
) -> Result<String> {
    let requests = evidence
        .module_requests
        .iter()
        .map(request_material)
        .collect::<Vec<_>>();
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_wave_request_plan.v1",
        "domain": REQUEST_PLAN_DOMAIN,
        "producer_wave_ordinal": evidence.producer_wave_ordinal,
        "previous_acquisition_receipt_digest": evidence.previous_acquisition_receipt_digest,
        "input_custody_digest": evidence.input_custody_digest,
        "authenticated_recursive_policy_digest": evidence.authenticated_recursive_policy_digest,
        "source_frontier": evidence.source_frontier.iter().map(source_frontier_material).collect::<Vec<_>>(),
        "first_module_request_ordinal": evidence.first_module_request_ordinal,
        "first_searched_name_ordinal": evidence.first_searched_name_ordinal,
        "first_system_image_request_ordinal": evidence.first_system_image_request_ordinal,
        "parser_policy_digest": evidence.parser_policy_digest,
        "module_requests": requests,
    }))
}

pub(super) fn edge_request_binding_digest(
    request: &WindowsRecursiveParsedEdgeRequest,
) -> Result<String> {
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_edge_request_binding.v1",
        "request": request_material_without_binding(request),
    }))
}

/// Commits the cumulative state of one direct-import-rooted forwarder chain. The stored
/// `chain_binding_digest` is deliberately excluded so this commitment cannot be self-referential.
pub(super) fn retained_forwarder_chain_binding_digest(
    chain: &WindowsRecursiveRetainedForwarderChainPlanEntry,
) -> Result<String> {
    jcs_sha256_hex(&retained_forwarder_chain_material_without_binding(chain))
}

pub(super) fn retained_forwarder_chain_set_digest(
    chains: &[WindowsRecursiveRetainedForwarderChainPlanEntry],
) -> Result<String> {
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_retained_forwarder_chain_set.v1",
        "domain": RETAINED_FORWARDER_CHAIN_SET_DOMAIN,
        "chains": chains.iter().map(retained_forwarder_chain_material).collect::<Vec<_>>(),
    }))
}

pub(super) fn base_parsed_image_owner_set_digest(
    owners: &[WindowsRecursiveBaseParsedImageOwnerPlanEntry],
) -> Result<String> {
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_base_parsed_image_owner_set.v1",
        "domain": BASE_PARSED_IMAGE_OWNER_SET_DOMAIN,
        "owners": owners.iter().map(base_parsed_image_owner_material).collect::<Vec<_>>(),
    }))
}

pub(super) fn base_parsed_image_owner_set_digest_from_resolution(
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<String> {
    let mut crosses = resolution
        .pe_import_graph
        .pre_post_cross_binding
        .parsed_image_cross_bindings
        .iter()
        .collect::<Vec<_>>();
    crosses.sort_by_key(|cross| cross.postlease_parsed_image_ordinal);
    let mut owners = Vec::with_capacity(crosses.len());
    let mut previous_postlease_parsed_image_ordinal = None;
    for cross in crosses {
        let parsed = resolution
            .pe_import_graph
            .parsed_images
            .get(cross.postlease_parsed_image_ordinal)
            .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_BASE_OWNER_MISSING"))?;
        if previous_postlease_parsed_image_ordinal
            .is_some_and(|previous| previous >= cross.postlease_parsed_image_ordinal)
            || parsed.parsed_image_ordinal != cross.postlease_parsed_image_ordinal
            || parsed.image_material_identity_digest
                != cross.postlease_image_material_identity_digest
            || parsed.source
                != (WindowsPeParsedImageSource::BasePreleasePackage {
                    prelease_parsed_image_ordinal: cross.prelease_parsed_image_ordinal,
                })
            || parsed.node
                != (WindowsLoaderModuleNode::PackageFile {
                    package_file_ordinal: cross.package_file_ordinal,
                })
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_BASE_OWNER_CHANGED");
        }
        owners.push(json!({
            "prelease_parsed_image_ordinal": cross.prelease_parsed_image_ordinal,
            "package_file_ordinal": cross.package_file_ordinal,
            "file_identity_digest": cross.file_identity_digest,
            "postlease_parsed_image_ordinal": cross.postlease_parsed_image_ordinal,
            "postlease_image_material_identity_digest": cross.postlease_image_material_identity_digest,
            "lease_generation_digest": cross.lease_generation_digest,
            "source_owner_binding_digest": parsed.source_binding_digest,
        }));
        previous_postlease_parsed_image_ordinal = Some(cross.postlease_parsed_image_ordinal);
    }
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_base_parsed_image_owner_set.v1",
        "domain": BASE_PARSED_IMAGE_OWNER_SET_DOMAIN,
        "owners": owners,
    }))
}

fn retained_forwarder_chain_material_without_binding(
    chain: &WindowsRecursiveRetainedForwarderChainPlanEntry,
) -> Value {
    json!({
        "schema": "elon.compute_plugin.windows_recursive_retained_forwarder_chain.v1",
        "domain": RETAINED_FORWARDER_CHAIN_DOMAIN,
        "source_import_edge_ordinal": chain.source_import_edge_ordinal,
        "next_hop_ordinal": chain.next_hop_ordinal,
        "current_target": forwarder_target_material(&chain.current_target),
        "visited_targets": chain
            .visited_targets
            .iter()
            .map(forwarder_target_material)
            .collect::<Vec<_>>(),
    })
}

pub(super) fn terminal_resolution_set_digest(
    producer_wave_ordinal: usize,
    module_resolutions: &[WindowsRecursiveModuleResolutionPlanEntry],
) -> Result<String> {
    let resolutions = module_resolutions
        .iter()
        .map(module_resolution_material)
        .collect::<Vec<_>>();
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_wave_terminal_set.v1",
        "domain": TERMINAL_SET_DOMAIN,
        "producer_wave_ordinal": producer_wave_ordinal,
        "module_resolutions": resolutions,
    }))
}

pub(super) fn searched_name_disposition_set_digest(
    producer_wave_ordinal: usize,
    searched_names: &[WindowsRecursiveSearchedNamePlanEntry],
) -> Result<String> {
    let searched_names = searched_names
        .iter()
        .map(searched_name_material)
        .collect::<Vec<_>>();
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_wave_search_disposition_set.v1",
        "domain": DISPOSITION_SET_DOMAIN,
        "producer_wave_ordinal": producer_wave_ordinal,
        "searched_names": searched_names,
    }))
}

pub(super) fn filesystem_request_set_digest(
    producer_wave_ordinal: usize,
    requests: &[WindowsRecursiveFilesystemImageRequestPlanEntry],
) -> Result<String> {
    let requests = requests
        .iter()
        .map(filesystem_request_material)
        .collect::<Vec<_>>();
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_wave_filesystem_request_set.v1",
        "domain": FILESYSTEM_REQUEST_SET_DOMAIN,
        "producer_wave_ordinal": producer_wave_ordinal,
        "requests": requests,
    }))
}

pub(super) fn route_owner_set_digest(
    producer_wave_ordinal: usize,
    owners: &[WindowsRecursiveRouteOwnerPlanEntry],
) -> Result<String> {
    let owners = owners.iter().map(route_owner_material).collect::<Vec<_>>();
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_wave_route_owner_set.v1",
        "domain": ROUTE_OWNER_SET_DOMAIN,
        "producer_wave_ordinal": producer_wave_ordinal,
        "owners": owners,
    }))
}

pub(super) fn resolved_plan_digest(
    plan: &AuthenticatedWindowsRecursiveWaveResolutionPlan,
) -> Result<String> {
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_wave_resolution_plan.v1",
        "domain": RESOLUTION_PLAN_DOMAIN,
        "producer_wave_ordinal": plan.producer_wave_ordinal,
        "authenticated_recursive_policy_digest": plan.authenticated_recursive_policy_digest,
        "parser_policy_digest": plan.parser_policy_digest,
        "source_request_plan_digest": plan.source_request_plan_digest,
        "terminal_resolution_set_digest": plan.terminal_resolution_set_digest,
        "searched_name_disposition_set_digest": plan.searched_name_disposition_set_digest,
        "filesystem_request_set_digest": plan.filesystem_request_set_digest,
        "route_owner_set_digest": plan.route_owner_set_digest,
    }))
}

pub(super) fn resolved_plan_evidence_digest(
    evidence: &WindowsRecursiveWaveDispatchPlanEvidence,
) -> Result<String> {
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_wave_resolution_plan.v1",
        "domain": RESOLUTION_PLAN_DOMAIN,
        "producer_wave_ordinal": evidence.producer_wave_ordinal,
        "authenticated_recursive_policy_digest": evidence.authenticated_recursive_policy_digest,
        "parser_policy_digest": evidence.parser_policy_digest,
        "source_request_plan_digest": evidence.request_plan_digest,
        "terminal_resolution_set_digest": evidence.terminal_resolution_set_digest,
        "searched_name_disposition_set_digest": evidence.searched_name_disposition_set_digest,
        "filesystem_request_set_digest": evidence.filesystem_request_set_digest,
        "route_owner_set_digest": evidence.route_owner_set_digest,
    }))
}

pub(super) fn module_resolution_binding_digest(
    resolution: &WindowsRecursiveModuleResolutionPlanEntry,
) -> Result<String> {
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_module_resolution_binding.v1",
        "module_request_ordinal": resolution.module_request_ordinal,
        "searched_name_ordinals": resolution.searched_name_ordinals,
        "terminal": terminal_material(&resolution.terminal),
    }))
}

pub(super) fn api_set_terminal_binding_digest(
    terminal: &WindowsRecursiveModuleTerminalRef,
) -> Result<Option<String>> {
    let WindowsRecursiveModuleTerminalRef::ApiSetHost {
        normalized_contract_name,
        normalized_host_module_cache_key,
        host_component_identity_digest,
        host_owner,
        os_build_identity_digest,
        schema_identity_digest,
        contract_host_binding_set_digest,
        ..
    } = terminal
    else {
        return Ok(None);
    };
    Ok(Some(jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_api_set_terminal_binding.v1",
        "normalized_contract_name": normalized_contract_name,
        "normalized_host_module_cache_key": normalized_host_module_cache_key,
        "host_component_identity_digest": host_component_identity_digest,
        "host_owner": api_set_host_owner_material(host_owner),
        "os_build_identity_digest": os_build_identity_digest,
        "schema_identity_digest": schema_identity_digest,
        "contract_host_binding_set_digest": contract_host_binding_set_digest,
    }))?))
}

pub(super) fn searched_name_disposition_binding_digest(
    searched: &WindowsRecursiveSearchedNamePlanEntry,
) -> Result<String> {
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_search_disposition_binding.v1",
        "searched_name_ordinal": searched.searched_name_ordinal,
        "module_request_ordinal": searched.module_request_ordinal,
        "step_position": searched.step_position,
        "search_directory_ordinal": searched.search_directory_ordinal,
        "normalized_name": searched.normalized_name,
        "search_directory_authority_binding_digest": searched.search_directory_authority_binding_digest,
        "disposition": disposition_material(&searched.disposition),
    }))
}

pub(super) fn searched_name_grant_request_digest(
    searched: &WindowsRecursiveSearchedNamePlanEntry,
) -> Result<String> {
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_searched_name_grant_request.v1",
        "search_directory_authority_binding_digest": searched.search_directory_authority_binding_digest,
        "normalized_name": searched.normalized_name,
        "disposition_binding_digest": searched.disposition_binding_digest,
    }))
}

pub(super) fn filesystem_lease_request_digest(
    request: &WindowsRecursiveFilesystemImageRequestPlanEntry,
) -> Result<String> {
    let uses = request
        .uses
        .iter()
        .map(filesystem_use_material)
        .collect::<Vec<_>>();
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_filesystem_lease_request.v1",
        "resolution_request_ordinal": request.resolution_request_ordinal,
        "canonical_dedupe_ordinal": request.canonical_dedupe_ordinal,
        "normalized_name": request.normalized_name,
        "search_directory_authority_binding_digest": request.search_directory_authority_binding_digest,
        "resolved_component_identity_digest": request.resolved_component_identity_digest,
        "expected_file_identity_digest": request.expected_file_identity_digest,
        "concrete_servicing_generation_digest": request.concrete_servicing_generation_digest,
        "code_integrity_evidence_digest": request.code_integrity_evidence_digest,
        "servicing_resolution_receipt_digest": request.servicing_resolution_receipt_digest,
        "namespace_alias_currentness_receipt_digest": request.namespace_alias_currentness_receipt_digest,
        "candidate_binding_digest": request.candidate_binding_digest,
        "uses": uses,
    }))
}

pub(super) fn route_owner_binding_digest(
    owner: &WindowsRecursiveRouteOwnerPlanEntry,
) -> Result<String> {
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_route_owner_binding.v1",
        "route_owner_ordinal": owner.route_owner_ordinal,
        "earliest_producer_module_request_ordinal": owner.earliest_producer_module_request_ordinal,
        "target": module_node_material(&owner.target),
        "resolved_module_cache_key": owner.resolved_module_cache_key,
        "owner": route_owner_material_ref(&owner.owner),
        "expected_source_owner_binding_digest": owner.expected_source_owner_binding_digest,
        "expected_image_material_identity_digest": owner.expected_image_material_identity_digest,
        "parse_disposition": parse_disposition_material(&owner.parse_disposition),
    }))
}

pub(super) fn pre_dispatch_plan_evidence_digest(
    evidence: &WindowsRecursiveAcquisitionPlanEvidence,
) -> Result<String> {
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_pre_dispatch_plan_evidence.v1",
        "evidence": acquisition_plan_evidence_material(evidence),
    }))
}

pub(super) fn validated_dispatch_plan_evidence_digest(
    evidence: &WindowsRecursiveWaveDispatchPlanEvidence,
) -> Result<String> {
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_validated_dispatch_plan_evidence.v1",
        "producer_wave_ordinal": evidence.producer_wave_ordinal,
        "previous_acquisition_receipt_digest": evidence.previous_acquisition_receipt_digest,
        "input_custody_digest": evidence.input_custody_digest,
        "authenticated_recursive_policy_digest": evidence.authenticated_recursive_policy_digest,
        "parser_policy_digest": evidence.parser_policy_digest,
        "source_frontier": evidence.source_frontier.iter().map(source_frontier_material).collect::<Vec<_>>(),
        "first_module_request_ordinal": evidence.first_module_request_ordinal,
        "module_requests": evidence.module_requests.iter().map(request_material).collect::<Vec<_>>(),
        "first_searched_name_ordinal": evidence.first_searched_name_ordinal,
        "first_system_image_request_ordinal": evidence.first_system_image_request_ordinal,
        "module_resolutions": evidence.module_resolutions.iter().map(module_resolution_material).collect::<Vec<_>>(),
        "searched_name_dispositions": evidence.searched_name_dispositions.iter().map(searched_name_material).collect::<Vec<_>>(),
        "filesystem_image_requests": evidence.filesystem_image_requests.iter().map(filesystem_request_material).collect::<Vec<_>>(),
        "route_owners": evidence.route_owners.iter().map(route_owner_material).collect::<Vec<_>>(),
        "request_plan_digest": evidence.request_plan_digest,
        "terminal_resolution_set_digest": evidence.terminal_resolution_set_digest,
        "searched_name_disposition_set_digest": evidence.searched_name_disposition_set_digest,
        "filesystem_request_set_digest": evidence.filesystem_request_set_digest,
        "route_owner_set_digest": evidence.route_owner_set_digest,
        "resolved_plan_digest": evidence.resolved_plan_digest,
    }))
}

pub(super) fn acquisition_plan_evidence_material(
    evidence: &WindowsRecursiveAcquisitionPlanEvidence,
) -> Value {
    match evidence {
        WindowsRecursiveAcquisitionPlanEvidence::BaseGrantReady {
            grant_ready_resolution_plan_digest,
        } => json!({
            "kind": "base_grant_ready",
            "grant_ready_resolution_plan_digest": grant_ready_resolution_plan_digest,
        }),
        WindowsRecursiveAcquisitionPlanEvidence::RecursiveWave { plan } => json!({
            "kind": "recursive_wave",
            "validated_plan_evidence_digest": plan.validated_plan_evidence_digest,
            "request_plan_digest": plan.request_plan_digest,
            "terminal_resolution_set_digest": plan.terminal_resolution_set_digest,
            "searched_name_disposition_set_digest": plan.searched_name_disposition_set_digest,
            "filesystem_request_set_digest": plan.filesystem_request_set_digest,
            "route_owner_set_digest": plan.route_owner_set_digest,
            "resolved_plan_digest": plan.resolved_plan_digest,
        }),
    }
}

pub(super) fn request_material(request: &WindowsRecursiveParsedEdgeRequest) -> Value {
    let mut material = request_material_without_binding(request);
    material["edge_request_binding_digest"] = json!(request.edge_request_binding_digest);
    material
}

pub(super) fn request_material_without_binding(
    request: &WindowsRecursiveParsedEdgeRequest,
) -> Value {
    json!({
        "module_request_ordinal": request.module_request_ordinal,
        "global_import_edge_ordinal": request.global_import_edge_ordinal,
        "source_parse_receipt_ordinal": request.source_parse_receipt_ordinal,
        "importer_parsed_image_ordinal": request.importer_parsed_image_ordinal,
        "importer": module_node_material(&request.importer),
        "importer_graph_edge_ordinal": request.importer_graph_edge_ordinal,
        "edge_locator": postlease_locator_material(&request.edge_locator),
        "import_kind": import_kind_name(&request.import_kind),
        "normalized_requested_name": request.normalized_requested_name,
        "imported_symbol_name": request.imported_symbol_name,
        "imported_symbol_ordinal": request.imported_symbol_ordinal,
        "ordered_search_step_ordinals": request.ordered_search_step_ordinals,
    })
}

pub(super) fn source_frontier_material(source: &WindowsRecursiveSourceFrontierPlanEntry) -> Value {
    json!({
        "parse_receipt_ordinal": source.parse_receipt_ordinal,
        "receipt_digest": source.receipt_digest,
        "wave_ordinal": source.wave_ordinal,
        "producer_acquisition_receipt_ordinal": source.producer_acquisition_receipt_ordinal,
        "producer_module_request_ordinal": source.producer_module_request_ordinal,
        "parsed_image_ordinal": source.parsed_image_ordinal,
        "node": module_node_material(&source.node),
        "source_owner": super::super::digest::owner_material(&source.source_owner),
        "source_owner_binding_digest": source.source_owner_binding_digest,
        "image_material_identity_digest": source.image_material_identity_digest,
        "parser_policy_digest": source.parser_policy_digest,
        "import_table_digest": source.import_table_digest,
        "normal_import_count": source.normal_import_count,
        "delay_import_count": source.delay_import_count,
        "forwarder_count": source.forwarder_count,
    })
}

pub(super) fn module_resolution_material(
    resolution: &WindowsRecursiveModuleResolutionPlanEntry,
) -> Value {
    json!({
        "module_request_ordinal": resolution.module_request_ordinal,
        "searched_name_ordinals": resolution.searched_name_ordinals,
        "terminal": terminal_material(&resolution.terminal),
        "resolution_binding_digest": resolution.resolution_binding_digest,
    })
}

pub(super) fn searched_name_material(searched: &WindowsRecursiveSearchedNamePlanEntry) -> Value {
    json!({
        "searched_name_ordinal": searched.searched_name_ordinal,
        "module_request_ordinal": searched.module_request_ordinal,
        "step_position": searched.step_position,
        "search_directory_ordinal": searched.search_directory_ordinal,
        "normalized_name": searched.normalized_name,
        "search_directory_authority_binding_digest": searched.search_directory_authority_binding_digest,
        "disposition": disposition_material(&searched.disposition),
        "grant_request_digest": searched.grant_request_digest,
        "disposition_binding_digest": searched.disposition_binding_digest,
    })
}

pub(super) fn filesystem_request_material(
    request: &WindowsRecursiveFilesystemImageRequestPlanEntry,
) -> Value {
    json!({
        "resolution_request_ordinal": request.resolution_request_ordinal,
        "canonical_dedupe_ordinal": request.canonical_dedupe_ordinal,
        "primary_use_ordinal": request.primary_use_ordinal,
        "normalized_name": request.normalized_name,
        "search_directory_authority_binding_digest": request.search_directory_authority_binding_digest,
        "resolved_component_identity_digest": request.resolved_component_identity_digest,
        "expected_file_identity_digest": request.expected_file_identity_digest,
        "concrete_servicing_generation_digest": request.concrete_servicing_generation_digest,
        "code_integrity_evidence_digest": request.code_integrity_evidence_digest,
        "servicing_resolution_receipt_digest": request.servicing_resolution_receipt_digest,
        "namespace_alias_currentness_receipt_digest": request.namespace_alias_currentness_receipt_digest,
        "candidate_binding_digest": request.candidate_binding_digest,
        "uses": request.uses.iter().map(filesystem_use_material).collect::<Vec<_>>(),
        "lease_request_digest": request.lease_request_digest,
    })
}

pub(super) fn route_owner_material(owner: &WindowsRecursiveRouteOwnerPlanEntry) -> Value {
    json!({
        "route_owner_ordinal": owner.route_owner_ordinal,
        "earliest_producer_module_request_ordinal": owner.earliest_producer_module_request_ordinal,
        "target": module_node_material(&owner.target),
        "resolved_module_cache_key": owner.resolved_module_cache_key,
        "owner": route_owner_material_ref(&owner.owner),
        "expected_source_owner_binding_digest": owner.expected_source_owner_binding_digest,
        "expected_image_material_identity_digest": owner.expected_image_material_identity_digest,
        "parse_disposition": parse_disposition_material(&owner.parse_disposition),
        "route_owner_binding_digest": owner.route_owner_binding_digest,
    })
}

pub(super) fn terminal_material(terminal: &WindowsRecursiveModuleTerminalRef) -> Value {
    match terminal {
        WindowsRecursiveModuleTerminalRef::Direct { owner } => json!({
            "kind": "direct",
            "owner": route_owner_material_ref(owner),
        }),
        WindowsRecursiveModuleTerminalRef::ApiSetHost {
            normalized_contract_name,
            normalized_host_module_cache_key,
            host_component_identity_digest,
            host_owner,
            os_build_identity_digest,
            schema_identity_digest,
            contract_host_binding_set_digest,
            resolution_binding_digest,
        } => json!({
            "kind": "api_set_host",
            "normalized_contract_name": normalized_contract_name,
            "normalized_host_module_cache_key": normalized_host_module_cache_key,
            "host_component_identity_digest": host_component_identity_digest,
            "host_owner": api_set_host_owner_material(host_owner),
            "os_build_identity_digest": os_build_identity_digest,
            "schema_identity_digest": schema_identity_digest,
            "contract_host_binding_set_digest": contract_host_binding_set_digest,
            "resolution_binding_digest": resolution_binding_digest,
        }),
    }
}

pub(super) fn disposition_material(disposition: &WindowsRecursiveSearchedNameDisposition) -> Value {
    match disposition {
        WindowsRecursiveSearchedNameDisposition::MustRemainAbsent => {
            json!({"kind": "must_remain_absent"})
        }
        WindowsRecursiveSearchedNameDisposition::Terminal { terminal } => json!({
            "kind": "terminal",
            "terminal": terminal_material(terminal),
        }),
    }
}

pub(super) fn route_owner_material_ref(owner: &WindowsRecursiveRouteOwnerRef) -> Value {
    match owner {
        WindowsRecursiveRouteOwnerRef::PackageContentLease {
            package_file_ordinal,
        } => json!({
            "kind": "package_content_lease",
            "package_file_ordinal": package_file_ordinal,
        }),
        WindowsRecursiveRouteOwnerRef::AuthenticatedPreloadedModule {
            preloaded_module_ordinal,
        } => json!({
            "kind": "authenticated_preloaded_module",
            "preloaded_module_ordinal": preloaded_module_ordinal,
        }),
        WindowsRecursiveRouteOwnerRef::KnownDllSection {
            known_dll_authority_record_ordinal,
        } => json!({
            "kind": "known_dll_section",
            "known_dll_authority_record_ordinal": known_dll_authority_record_ordinal,
        }),
        WindowsRecursiveRouteOwnerRef::ResolvedFilesystemSystemImage {
            resolution_request_ordinal,
            route,
        } => json!({
            "kind": "resolved_filesystem_system_image",
            "resolution_request_ordinal": resolution_request_ordinal,
            "route": filesystem_route_name(route),
        }),
    }
}

fn api_set_host_owner_material(owner: &WindowsRecursiveApiSetHostOwnerRef) -> Value {
    match owner {
        WindowsRecursiveApiSetHostOwnerRef::AuthenticatedPreloadedModule {
            preloaded_module_ordinal,
        } => json!({
            "kind": "authenticated_preloaded_module",
            "preloaded_module_ordinal": preloaded_module_ordinal,
        }),
        WindowsRecursiveApiSetHostOwnerRef::KnownDllSection {
            known_dll_authority_record_ordinal,
        } => json!({
            "kind": "known_dll_section",
            "known_dll_authority_record_ordinal": known_dll_authority_record_ordinal,
        }),
        WindowsRecursiveApiSetHostOwnerRef::ResolvedFilesystemSystemImage {
            resolution_request_ordinal,
            route,
        } => json!({
            "kind": "resolved_filesystem_system_image",
            "resolution_request_ordinal": resolution_request_ordinal,
            "route": filesystem_route_name(route),
        }),
    }
}

fn parse_disposition_material(disposition: &WindowsRecursiveTargetParseDisposition) -> Value {
    match disposition {
        WindowsRecursiveTargetParseDisposition::AlreadyParsed {
            parsed_image_ordinal,
        } => json!({
            "kind": "already_parsed",
            "parsed_image_ordinal": parsed_image_ordinal,
        }),
        WindowsRecursiveTargetParseDisposition::NextFrontier {
            parse_receipt_ordinal,
            target_parse_wave_ordinal,
        } => json!({
            "kind": "next_frontier",
            "parse_receipt_ordinal": parse_receipt_ordinal,
            "target_parse_wave_ordinal": target_parse_wave_ordinal,
        }),
    }
}

fn filesystem_use_material(use_plan: &WindowsRecursiveFilesystemImageUse) -> Value {
    json!({
        "module_request_ordinal": use_plan.module_request_ordinal,
        "searched_name_ordinal": use_plan.searched_name_ordinal,
        "search_directory_ordinal": use_plan.search_directory_ordinal,
        "normalized_name": use_plan.normalized_name,
        "search_directory_authority_binding_digest": use_plan.search_directory_authority_binding_digest,
        "route": filesystem_route_name(&use_plan.route),
    })
}

fn forwarder_target_material(target: &WindowsRecursiveForwarderTargetRef) -> Value {
    json!({
        "node": module_node_material(&target.node),
        "symbol": forwarder_symbol_material(&target.symbol),
    })
}

fn retained_forwarder_chain_material(
    chain: &WindowsRecursiveRetainedForwarderChainPlanEntry,
) -> Value {
    let mut material = retained_forwarder_chain_material_without_binding(chain);
    material["chain_binding_digest"] = json!(chain.chain_binding_digest);
    material
}

fn base_parsed_image_owner_material(
    owner: &WindowsRecursiveBaseParsedImageOwnerPlanEntry,
) -> Value {
    json!({
        "prelease_parsed_image_ordinal": owner.prelease_parsed_image_ordinal,
        "package_file_ordinal": owner.package_file_ordinal,
        "file_identity_digest": owner.file_identity_digest,
        "postlease_parsed_image_ordinal": owner.postlease_parsed_image_ordinal,
        "postlease_image_material_identity_digest": owner.postlease_image_material_identity_digest,
        "lease_generation_digest": owner.lease_generation_digest,
        "source_owner_binding_digest": owner.source_owner_binding_digest,
    })
}

fn forwarder_symbol_material(symbol: &WindowsRecursiveForwarderSymbolRef) -> Value {
    json!({
        "name": symbol.name,
        "ordinal": symbol.ordinal,
    })
}

fn filesystem_route_name(route: &WindowsRecursiveFilesystemUseRoute) -> &'static str {
    match route {
        WindowsRecursiveFilesystemUseRoute::OrdinaryFilesystem => "ordinary_filesystem",
        WindowsRecursiveFilesystemUseRoute::SideBySide => "side_by_side",
    }
}

fn import_kind_name(kind: &WindowsRecursiveRequestImportKind) -> &'static str {
    match kind {
        WindowsRecursiveRequestImportKind::Normal => "normal",
        WindowsRecursiveRequestImportKind::Delay => "delay",
        WindowsRecursiveRequestImportKind::Forwarder => "forwarder",
    }
}

fn postlease_locator_material(locator: &WindowsPostLeaseModuleEdgeLocator) -> Value {
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

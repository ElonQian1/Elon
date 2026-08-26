//! Canonical one-way commitments for recursive acquisition custody.

use anyhow::{anyhow, Result};
use serde_json::json;

use crate::node_agent_compute_plugin_host::signed_artifact_verification::jcs_sha256_hex;

use super::super::super::{
    SealedWindowsLoaderNamespacePrerequisite, SealedWindowsLoaderResolutionAuthority,
};
use super::super::{
    SealedWindowsRecursiveResolutionClosure, WindowsPostLeaseSystemImageParseReceipt,
};
use super::{
    SealedWindowsRecursiveResolutionAcquisitionChain, WindowsRecursiveWaveAcquisitionReceipt,
};

pub(super) fn output_custody_digest(
    receipt: &WindowsRecursiveWaveAcquisitionReceipt,
) -> Result<String> {
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_wave_output_custody.v3",
        "acquisition_receipt_ordinal": receipt.acquisition_receipt_ordinal,
        "previous_acquisition_receipt_digest": receipt.previous_acquisition_receipt_digest,
        "authenticated_recursive_policy_digest": receipt.authenticated_recursive_policy_digest,
        "parser_policy_digest": receipt.parser_policy_digest,
        "producer_wave_ordinal": receipt.producer_wave_ordinal,
        "target_parse_wave_ordinal": receipt.target_parse_wave_ordinal,
        "source_frontier_parse_receipt_ordinals": receipt.source_frontier_parse_receipt_ordinals,
        "first_module_request_ordinal": receipt.first_module_request_ordinal,
        "module_request_count": receipt.module_request_count,
        "first_searched_name_ordinal": receipt.first_searched_name_ordinal,
        "searched_name_count": receipt.searched_name_count,
        "first_system_image_request_ordinal": receipt.first_system_image_request_ordinal,
        "system_image_request_count": receipt.system_image_request_count,
        "input_custody_digest": receipt.input_custody_digest,
        "base_parsed_image_owner_set_digest": receipt.base_parsed_image_owner_set_digest,
        "retained_forwarder_chain_set_digest": receipt.retained_forwarder_chain_set_digest,
        "source_request_plan_digest": receipt.source_request_plan_digest,
        "resolved_plan_digest": receipt.resolved_plan_digest,
        "pre_dispatch_plan_evidence_digest": receipt.pre_dispatch_plan_evidence_digest,
        "policy_dispatch_authorization": receipt.policy_dispatch_authorization.canonical_material(),
        "searched_name_grant_set_digest": receipt.searched_name_grant_set_digest,
        "filesystem_candidate_set_digest": receipt.filesystem_candidate_set_digest,
        "immutable_content_lease_set_digest": receipt.immutable_content_lease_set_digest,
        "same_owner_parse_set_digest": receipt.same_owner_parse_set_digest,
        "next_frontier_parse_receipt_ordinals": receipt.next_frontier_parse_receipt_ordinals,
    }))
}

pub(super) fn receipt_digest(receipt: &WindowsRecursiveWaveAcquisitionReceipt) -> Result<String> {
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_wave_acquisition_receipt.v3",
        "acquisition_receipt_ordinal": receipt.acquisition_receipt_ordinal,
        "previous_acquisition_receipt_digest": receipt.previous_acquisition_receipt_digest,
        "authenticated_recursive_policy_digest": receipt.authenticated_recursive_policy_digest,
        "parser_policy_digest": receipt.parser_policy_digest,
        "producer_wave_ordinal": receipt.producer_wave_ordinal,
        "target_parse_wave_ordinal": receipt.target_parse_wave_ordinal,
        "source_frontier_parse_receipt_ordinals": receipt.source_frontier_parse_receipt_ordinals,
        "first_module_request_ordinal": receipt.first_module_request_ordinal,
        "module_request_count": receipt.module_request_count,
        "first_searched_name_ordinal": receipt.first_searched_name_ordinal,
        "searched_name_count": receipt.searched_name_count,
        "first_system_image_request_ordinal": receipt.first_system_image_request_ordinal,
        "system_image_request_count": receipt.system_image_request_count,
        "input_custody_digest": receipt.input_custody_digest,
        "base_parsed_image_owner_set_digest": receipt.base_parsed_image_owner_set_digest,
        "retained_forwarder_chain_set_digest": receipt.retained_forwarder_chain_set_digest,
        "source_request_plan_digest": receipt.source_request_plan_digest,
        "resolved_plan_digest": receipt.resolved_plan_digest,
        "pre_dispatch_plan_evidence_digest": receipt.pre_dispatch_plan_evidence_digest,
        "policy_dispatch_authorization": receipt.policy_dispatch_authorization.canonical_material(),
        "searched_name_grant_set_digest": receipt.searched_name_grant_set_digest,
        "filesystem_candidate_set_digest": receipt.filesystem_candidate_set_digest,
        "immutable_content_lease_set_digest": receipt.immutable_content_lease_set_digest,
        "same_owner_parse_set_digest": receipt.same_owner_parse_set_digest,
        "next_frontier_parse_receipt_ordinals": receipt.next_frontier_parse_receipt_ordinals,
        "output_custody_digest": receipt.output_custody_digest,
    }))
}

pub(super) fn receipt_set_digest(
    receipts: &[WindowsRecursiveWaveAcquisitionReceipt],
) -> Result<String> {
    let receipt_digests = receipts
        .iter()
        .map(|receipt| receipt.receipt_digest.as_str())
        .collect::<Vec<_>>();
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_wave_acquisition_receipt_set.v1",
        "receipt_digests": receipt_digests,
    }))
}

pub(super) fn chain_digest(
    chain: &SealedWindowsRecursiveResolutionAcquisitionChain,
) -> Result<String> {
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_resolution_acquisition_chain.v1",
        "authenticated_recursive_policy_digest": chain.policy.digest(),
        "parser_policy_digest": chain.parser_policy_digest,
        "receipt_set_digest": chain.receipt_set_digest,
    }))
}

pub(super) fn same_owner_parse_set_digest(
    producer_wave_ordinal: usize,
    next_frontier: &[usize],
    parse_receipts: &[WindowsPostLeaseSystemImageParseReceipt],
) -> Result<String> {
    let receipts = next_frontier
        .iter()
        .map(|ordinal| {
            let receipt = parse_receipts.get(*ordinal).ok_or_else(|| {
                anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_ACQUISITION_PARSE_MISSING")
            })?;
            Ok(json!({
                "parse_receipt_ordinal": ordinal,
                "receipt_digest": receipt.receipt_digest,
            }))
        })
        .collect::<Result<Vec<_>>>()?;
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_same_owner_parse_set.v1",
        "producer_wave_ordinal": producer_wave_ordinal,
        "parse_receipts": receipts,
    }))
}

pub(super) fn base_same_owner_parse_set_digest(
    next_frontier: &[usize],
    closure: &SealedWindowsRecursiveResolutionClosure,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<String> {
    let recursive_target_parse_set_digest =
        same_owner_parse_set_digest(0, next_frontier, &closure.parse_receipts)?;
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_base_same_owner_parse_set.v1",
        "package_pre_post_cross_binding_receipt_digest": resolution
            .pe_import_graph
            .pre_post_cross_binding
            .receipt_digest,
        "recursive_target_parse_set_digest": recursive_target_parse_set_digest,
    }))
}

pub(super) fn filesystem_candidate_set_digest(
    producer_wave_ordinal: usize,
    first_resolution_request_ordinal: usize,
    resolution_request_count: usize,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<String> {
    let end = checked_end(first_resolution_request_ordinal, resolution_request_count)?;
    let candidates = resolution
        .resolved_filesystem_system_images
        .get(first_resolution_request_ordinal..end)
        .ok_or_else(|| {
            anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_ACQUISITION_CANDIDATE_RANGE_CHANGED")
        })?
        .iter()
        .map(|custody| {
            let (request, candidate, _, _, _, _, _) = custody.outcome.binding();
            let (parent, name, file, _, open_receipt, _) = custody.outcome.image().binding();
            let (
                evidence_parent,
                evidence_name,
                component,
                evidence_file,
                evidence_open_receipt,
                code_integrity,
                servicing_generation,
                servicing_resolution,
                namespace_currentness,
                evidence_candidate,
            ) = custody.outcome.candidate_resolution_binding();
            json!({
                "resolution_request_ordinal": custody.resolution_request_ordinal,
                "outcome_request_ordinal": request,
                "candidate_binding_digest": candidate,
                "parent_directory_identity_digest": parent,
                "normalized_name": name,
                "image_file_identity_digest": file,
                "parent_relative_open_receipt_digest": open_receipt,
                "candidate_resolution_evidence": {
                    "parent_directory_identity_digest": evidence_parent,
                    "normalized_name": evidence_name,
                    "resolved_component_identity_digest": component,
                    "image_file_identity_digest": evidence_file,
                    "parent_relative_open_receipt_digest": evidence_open_receipt,
                    "code_integrity_evidence_digest": code_integrity,
                    "concrete_servicing_generation_digest": servicing_generation,
                    "servicing_resolution_receipt_digest": servicing_resolution,
                    "namespace_alias_currentness_receipt_digest": namespace_currentness,
                    "candidate_binding_digest": evidence_candidate,
                },
            })
        })
        .collect::<Vec<_>>();
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_filesystem_candidate_set.v2",
        "producer_wave_ordinal": producer_wave_ordinal,
        "candidates": candidates,
    }))
}

pub(super) fn searched_name_grant_set_digest(
    producer_wave_ordinal: usize,
    first_searched_name_ordinal: usize,
    searched_name_count: usize,
    namespace: &SealedWindowsLoaderNamespacePrerequisite,
) -> Result<String> {
    let end = checked_end(first_searched_name_ordinal, searched_name_count)?;
    let (session_identity, grant_generation, generation_domain) = namespace.session.binding();
    let grants = namespace
        .searched_name_grants
        .get(first_searched_name_ordinal..end)
        .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_ACQUISITION_GRANT_RANGE_CHANGED"))?
        .iter()
        .map(|entry| {
            let (generation, parent, name, disposition, fence) = entry.grant.binding();
            let (request, nonce, response, positive_receipt) =
                entry.grant.authenticated_positive_binding();
            json!({
                "searched_name_ordinal": entry.searched_name_ordinal,
                "search_directory_ordinal": entry.search_directory_ordinal,
                "grant_generation": generation,
                "parent_directory_identity_digest": parent,
                "normalized_name": name,
                "disposition_digest": disposition,
                "fence_generation_digest": fence,
                "request_digest": request,
                "query_nonce_digest": nonce,
                "authenticated_response_digest": response,
                "positive_receipt_digest": positive_receipt,
            })
        })
        .collect::<Vec<_>>();
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_searched_name_grant_set.v1",
        "producer_wave_ordinal": producer_wave_ordinal,
        "session_identity_digest": session_identity,
        "grant_generation": grant_generation,
        "generation_domain_digest": generation_domain,
        "grants": grants,
    }))
}

pub(super) fn terminal_frontier_digest(
    closure: &SealedWindowsRecursiveResolutionClosure,
    terminal_receipt: &WindowsRecursiveWaveAcquisitionReceipt,
) -> Result<String> {
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_recursive_terminal_empty_frontier.v1",
        "acquisition_receipt_ordinal": terminal_receipt.acquisition_receipt_ordinal,
        "acquisition_receipt_digest": terminal_receipt.receipt_digest,
        "closure_wave_count": closure.waves.len(),
        "next_frontier_parse_receipt_ordinals": terminal_receipt.next_frontier_parse_receipt_ordinals,
    }))
}

fn checked_end(first: usize, count: usize) -> Result<usize> {
    first
        .checked_add(count)
        .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_COUNT_OVERFLOW"))
}

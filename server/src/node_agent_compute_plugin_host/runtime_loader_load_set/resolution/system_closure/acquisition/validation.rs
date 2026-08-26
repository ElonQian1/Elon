//! Fail-closed projection and live-owner validation for the `A0..AN` receipt chain.

use std::collections::HashSet;

use anyhow::{anyhow, bail, Result};

use crate::node_agent_compute_plugin_host::manifest_validation::is_sha256;

use super::super::super::{
    SealedWindowsLoaderNamespacePrerequisite, SealedWindowsLoaderResolutionAuthority,
};
use super::super::{
    projection_digest, SealedWindowsRecursiveResolutionClosure, WindowsRecursiveResolutionWavePlan,
};
use super::{
    digest, plan_digest, plan_forwarder_validation, plan_projection,
    SealedWindowsRecursiveResolutionAcquisitionChain, WindowsRecursiveAcquisitionPlanEvidence,
    WindowsRecursiveWaveAcquisitionReceipt,
};

pub(super) fn validate_projection_against(
    chain: &SealedWindowsRecursiveResolutionAcquisitionChain,
    closure: &SealedWindowsRecursiveResolutionClosure,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<()> {
    let expected_receipt_count = closure
        .waves
        .len()
        .checked_add(1)
        .ok_or_else(count_overflow)?;
    if chain.receipts().len() != expected_receipt_count
        || chain.parser_policy_digest != chain.policy.parser_policy_digest()
        || !is_sha256(chain.policy.digest())
        || !is_sha256(&chain.parser_policy_digest)
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_ACQUISITION_CHAIN_SHAPE_CHANGED");
    }

    let mut authorization_nonces = HashSet::with_capacity(chain.receipts().len());
    let mut previous_control_keyring_generation = None;
    let mut previous_trusted_now_ms = None;
    let mut previous_trusted_time_attestation_sequence = None;
    for (ordinal, receipt) in chain.receipts().iter().enumerate() {
        validate_receipt_projection(ordinal, receipt, chain, closure, resolution)?;
        receipt.policy_dispatch_authorization.validate_against(
            chain.policy(),
            receipt.acquisition_receipt_ordinal,
            receipt.producer_wave_ordinal,
            &receipt.input_custody_digest,
            &receipt.pre_dispatch_plan_evidence_digest,
        )?;
        if !authorization_nonces.insert(receipt.policy_dispatch_authorization.nonce_digest()) {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_POLICY_AUTHORIZATION_NONCE_REUSED");
        }
        let observed_generation = receipt
            .policy_dispatch_authorization
            .observed_control_keyring_generation();
        if previous_control_keyring_generation
            .is_some_and(|previous| observed_generation < previous)
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_POLICY_CONTROL_KEYRING_GENERATION_REGRESSED");
        }
        previous_control_keyring_generation = Some(observed_generation);

        let trusted_now_ms = receipt.policy_dispatch_authorization.trusted_now_ms();
        if previous_trusted_now_ms.is_some_and(|previous| trusted_now_ms < previous) {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_POLICY_TRUSTED_TIME_REGRESSED");
        }
        previous_trusted_now_ms = Some(trusted_now_ms);

        let trusted_time_attestation_sequence = receipt
            .policy_dispatch_authorization
            .trusted_time_attestation_sequence();
        if previous_trusted_time_attestation_sequence
            .is_some_and(|previous| trusted_time_attestation_sequence < previous)
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_POLICY_TRUSTED_TIME_SEQUENCE_REGRESSED");
        }
        previous_trusted_time_attestation_sequence = Some(trusted_time_attestation_sequence);
    }

    let Some(terminal_receipt) = chain.receipts().last() else {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_ACQUISITION_TERMINAL_MISSING");
    };
    if !terminal_receipt
        .next_frontier_parse_receipt_ordinals
        .is_empty()
        || terminal_receipt.target_parse_wave_ordinal.is_some()
        || closure.terminal_empty_frontier_receipt_digest
            != digest::terminal_frontier_digest(closure, terminal_receipt)?
        || !is_sha256(&closure.terminal_empty_frontier_receipt_digest)
        || chain.receipt_set_digest != digest::receipt_set_digest(chain.receipts())?
        || chain.acquisition_chain_digest != digest::chain_digest(chain)?
        || !is_sha256(&chain.receipt_set_digest)
        || !is_sha256(&chain.acquisition_chain_digest)
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_ACQUISITION_CHAIN_DIGEST_CHANGED");
    }
    Ok(())
}

fn validate_receipt_projection(
    ordinal: usize,
    receipt: &WindowsRecursiveWaveAcquisitionReceipt,
    chain: &SealedWindowsRecursiveResolutionAcquisitionChain,
    closure: &SealedWindowsRecursiveResolutionClosure,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<()> {
    let previous = ordinal
        .checked_sub(1)
        .and_then(|previous| chain.receipts().get(previous));
    let expected_previous_digest = previous.map(|receipt| receipt.receipt_digest.as_str());
    let expected_input_custody_digest = previous
        .map(|receipt| receipt.output_custody_digest.as_str())
        .unwrap_or(resolution.grant_ready_resolution_plan_digest.as_str());

    let (
        expected_source_frontier,
        first_module_request_ordinal,
        module_request_count,
        first_searched_name_ordinal,
        searched_name_count,
        first_system_image_request_ordinal,
        system_image_request_count,
        expected_source_request_plan_digest,
        expected_resolved_plan_digest,
        expected_lease_set_digest,
    ) = if ordinal == 0 {
        let WindowsRecursiveAcquisitionPlanEvidence::BaseGrantReady {
            grant_ready_resolution_plan_digest,
        } = &receipt.pre_dispatch_plan_evidence
        else {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_BASE_PLAN_EVIDENCE_CHANGED");
        };
        if grant_ready_resolution_plan_digest != &resolution.grant_ready_resolution_plan_digest {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_BASE_PLAN_EVIDENCE_CHANGED");
        }
        (
            &[][..],
            0,
            closure.base_module_request_count,
            0,
            closure.base_searched_name_count,
            0,
            closure.base_system_image_request_count,
            resolution.grant_ready_resolution_plan_digest.as_str(),
            resolution.grant_ready_resolution_plan_digest.as_str(),
            base_lease_set_digest(closure, resolution)?,
        )
    } else {
        let wave = closure
            .waves
            .get(ordinal - 1)
            .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_ACQUISITION_WAVE_MISSING"))?;
        let WindowsRecursiveAcquisitionPlanEvidence::RecursiveWave { plan } =
            &receipt.pre_dispatch_plan_evidence
        else {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_PLAN_EVIDENCE_KIND_CHANGED");
        };
        (
            wave.source_parse_receipt_ordinals.as_slice(),
            wave.first_module_request_ordinal,
            wave.module_request_count,
            wave.first_searched_name_ordinal,
            wave.searched_name_count,
            wave.first_system_image_request_ordinal,
            wave.system_image_request_count,
            plan.request_plan_digest.as_str(),
            plan.resolved_plan_digest.as_str(),
            wave.acquired_system_image_set_digest.clone(),
        )
    };
    let module_request_end = first_module_request_ordinal
        .checked_add(module_request_count)
        .ok_or_else(count_overflow)?;
    let expected_base_owner_set_digest =
        plan_digest::base_parsed_image_owner_set_digest_from_resolution(resolution)?;
    let expected_forwarder_chain_set_digest =
        plan_forwarder_validation::final_forwarder_chain_set_digest_through(
            module_request_end,
            resolution,
        )?;

    let expected_next_frontier = if ordinal == 0 {
        closure
            .waves
            .first()
            .map(|wave| wave.source_parse_receipt_ordinals.as_slice())
            .unwrap_or_default()
    } else {
        closure.waves[ordinal - 1]
            .next_frontier_parse_receipt_ordinals
            .as_slice()
    };
    let expected_target_wave = if expected_next_frontier.is_empty() {
        None
    } else {
        Some(ordinal.checked_add(1).ok_or_else(count_overflow)?)
    };
    let expected_candidate_set_digest = digest::filesystem_candidate_set_digest(
        ordinal,
        first_system_image_request_ordinal,
        system_image_request_count,
        resolution,
    )?;
    let expected_parse_set_digest = if ordinal == 0 {
        digest::base_same_owner_parse_set_digest(expected_next_frontier, closure, resolution)?
    } else {
        digest::same_owner_parse_set_digest(
            ordinal,
            expected_next_frontier,
            &closure.parse_receipts,
        )?
    };

    if receipt.acquisition_receipt_ordinal != ordinal
        || receipt.producer_wave_ordinal != ordinal
        || receipt.target_parse_wave_ordinal != expected_target_wave
        || receipt.previous_acquisition_receipt_digest.as_deref() != expected_previous_digest
        || receipt.authenticated_recursive_policy_digest != chain.policy.digest()
        || receipt.parser_policy_digest != chain.parser_policy_digest
        || receipt.source_frontier_parse_receipt_ordinals.as_slice() != expected_source_frontier
        || receipt.first_module_request_ordinal != first_module_request_ordinal
        || receipt.module_request_count != module_request_count
        || receipt.first_searched_name_ordinal != first_searched_name_ordinal
        || receipt.searched_name_count != searched_name_count
        || receipt.first_system_image_request_ordinal != first_system_image_request_ordinal
        || receipt.system_image_request_count != system_image_request_count
        || receipt.input_custody_digest != expected_input_custody_digest
        || receipt.base_parsed_image_owner_set_digest != expected_base_owner_set_digest
        || receipt.retained_forwarder_chain_set_digest != expected_forwarder_chain_set_digest
        || receipt.source_request_plan_digest != expected_source_request_plan_digest
        || receipt.resolved_plan_digest != expected_resolved_plan_digest
        || receipt.filesystem_candidate_set_digest != expected_candidate_set_digest
        || receipt.immutable_content_lease_set_digest != expected_lease_set_digest
        || receipt.same_owner_parse_set_digest != expected_parse_set_digest
        || receipt.next_frontier_parse_receipt_ordinals.as_slice() != expected_next_frontier
        || !strictly_increasing(&receipt.source_frontier_parse_receipt_ordinals)
        || !strictly_increasing(&receipt.next_frontier_parse_receipt_ordinals)
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_ACQUISITION_PROJECTION_CHANGED");
    }

    for parse_receipt_ordinal in expected_next_frontier {
        let parse_receipt = closure
            .parse_receipts
            .get(*parse_receipt_ordinal)
            .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_ACQUISITION_PARSE_MISSING"))?;
        if parse_receipt.producer_acquisition_receipt_ordinal != ordinal
            || parse_receipt.wave_ordinal != ordinal.checked_add(1).ok_or_else(count_overflow)?
            || parse_receipt.parser_policy_digest != chain.parser_policy_digest
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_ACQUISITION_PARSE_PROVENANCE_CHANGED");
        }
    }

    if [
        &receipt.authenticated_recursive_policy_digest,
        &receipt.parser_policy_digest,
        &receipt.input_custody_digest,
        &receipt.base_parsed_image_owner_set_digest,
        &receipt.retained_forwarder_chain_set_digest,
        &receipt.source_request_plan_digest,
        &receipt.resolved_plan_digest,
        &receipt.pre_dispatch_plan_evidence_digest,
        &receipt.searched_name_grant_set_digest,
        &receipt.filesystem_candidate_set_digest,
        &receipt.immutable_content_lease_set_digest,
        &receipt.same_owner_parse_set_digest,
        &receipt.output_custody_digest,
        &receipt.receipt_digest,
    ]
    .into_iter()
    .any(|value| !is_sha256(value))
        || !is_sha256(receipt.policy_dispatch_authorization.digest())
        || receipt
            .previous_acquisition_receipt_digest
            .as_deref()
            .is_some_and(|digest| !is_sha256(digest))
        || receipt.pre_dispatch_plan_evidence_digest
            != plan_digest::pre_dispatch_plan_evidence_digest(&receipt.pre_dispatch_plan_evidence)?
        || receipt.output_custody_digest != digest::output_custody_digest(receipt)?
        || receipt.receipt_digest != digest::receipt_digest(receipt)?
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_ACQUISITION_RECEIPT_DIGEST_CHANGED");
    }
    if ordinal > 0 {
        let wave = closure
            .waves
            .get(ordinal - 1)
            .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_ACQUISITION_WAVE_MISSING"))?;
        plan_projection::validate_recursive_plan_evidence_against(
            receipt, wave, closure, resolution,
        )?;
    }
    Ok(())
}

/// Borrowed live-owner validation. No grant is moved out of the namespace prerequisite and no
/// receipt is allowed to substitute a detached digest for the actual managed grant material.
pub(super) fn validate_namespace_grants_against(
    chain: &SealedWindowsRecursiveResolutionAcquisitionChain,
    closure: &SealedWindowsRecursiveResolutionClosure,
    namespace: &SealedWindowsLoaderNamespacePrerequisite,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<()> {
    if chain.receipts().len()
        != closure
            .waves
            .len()
            .checked_add(1)
            .ok_or_else(count_overflow)?
        || namespace.searched_name_grants.len() != resolution.searched_names.len()
        || namespace.grant_ready_resolution_plan_digest
            != resolution.grant_ready_resolution_plan_digest
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_ACQUISITION_GRANT_COVERAGE_CHANGED");
    }

    let mut next_searched_name_ordinal = 0usize;
    for receipt in chain.receipts() {
        let end = receipt
            .first_searched_name_ordinal
            .checked_add(receipt.searched_name_count)
            .ok_or_else(count_overflow)?;
        if receipt.first_searched_name_ordinal != next_searched_name_ordinal
            || end > resolution.searched_names.len()
            || receipt.searched_name_grant_set_digest
                != digest::searched_name_grant_set_digest(
                    receipt.producer_wave_ordinal,
                    receipt.first_searched_name_ordinal,
                    receipt.searched_name_count,
                    namespace,
                )?
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_ACQUISITION_GRANT_SET_CHANGED");
        }
        for ordinal in receipt.first_searched_name_ordinal..end {
            validate_one_namespace_grant(ordinal, namespace, resolution)?;
        }
        next_searched_name_ordinal = end;
    }
    if next_searched_name_ordinal != resolution.searched_names.len() {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_ACQUISITION_GRANT_COVERAGE_CHANGED");
    }
    Ok(())
}

fn validate_one_namespace_grant(
    ordinal: usize,
    namespace: &SealedWindowsLoaderNamespacePrerequisite,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<()> {
    let searched = resolution.searched_names.get(ordinal).ok_or_else(|| {
        anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_ACQUISITION_SEARCHED_NAME_MISSING")
    })?;
    let fence = namespace
        .searched_name_grants
        .get(ordinal)
        .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_ACQUISITION_GRANT_MISSING"))?;
    let expected_parent = resolution
        .search_directories
        .get(searched.search_directory_ordinal)
        .map(|directory| directory.directory_identity_digest.as_str());
    let expected_disposition =
        super::super::super::super::digest::searched_name_disposition_digest(
            &searched.disposition,
        )?;
    let (generation, parent, name, disposition, fence_digest) = fence.grant.binding();
    let (request, _, _, _) = fence.grant.authenticated_positive_binding();
    if searched.searched_name_ordinal != ordinal
        || fence.searched_name_ordinal != ordinal
        || fence.search_directory_ordinal != searched.search_directory_ordinal
        || !fence.grant.matches_session(&namespace.session)
        || generation != namespace.session.binding().1
        || Some(parent) != expected_parent
        || name != searched.normalized_name
        || disposition != expected_disposition
        || request != searched.grant_request_digest
        || !is_sha256(fence_digest)
        || !fence.grant.authenticated_positive_is_bound()
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_ACQUISITION_GRANT_OWNER_CHANGED");
    }
    Ok(())
}

fn base_lease_set_digest(
    closure: &SealedWindowsRecursiveResolutionClosure,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<String> {
    let base_projection = WindowsRecursiveResolutionWavePlan {
        wave_ordinal: 0,
        source_parse_receipt_ordinals: Vec::new(),
        first_module_request_ordinal: 0,
        module_request_count: closure.base_module_request_count,
        first_searched_name_ordinal: 0,
        searched_name_count: closure.base_searched_name_count,
        first_system_image_request_ordinal: 0,
        system_image_request_count: closure.base_system_image_request_count,
        next_frontier_parse_receipt_ordinals: Vec::new(),
        parsed_edge_set_digest: String::new(),
        searched_name_disposition_set_digest: String::new(),
        acquired_system_image_set_digest: String::new(),
        wave_digest: String::new(),
    };
    let system_image_lease_set_digest =
        projection_digest::system_image_set_digest(&base_projection, resolution)?;
    crate::node_agent_compute_plugin_host::signed_artifact_verification::jcs_sha256_hex(
        &serde_json::json!({
            "schema": "elon.compute_plugin.windows_recursive_base_immutable_content_lease_set.v1",
            "package_content_lease_set_digest": resolution.package_content_lease_set_digest,
            "system_image_lease_set_digest": system_image_lease_set_digest,
        }),
    )
}

fn strictly_increasing(values: &[usize]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

fn count_overflow() -> anyhow::Error {
    anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_COUNT_OVERFLOW")
}

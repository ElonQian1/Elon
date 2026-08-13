use anyhow::{bail, Result};

use crate::{
    compute_federation::{
        external_pool_adapter_credential_verification::{
            credential_locator_commitment, credential_ref_scheme,
        },
        external_pool_adapter_runtime_launch_profile::{
            runtime_launch_entrypoint_path_digest, ExternalPoolAdapterRuntimeLaunchProfileReceipt,
        },
        external_pool_provider_activation_candidate::ExternalPoolProviderActivationCandidateReceipt,
    },
    store::{
        compute_external_pool_adapter_registry::CurrentExternalPoolAdapterRegistryProviderBindingAuthority,
        compute_external_pool_onboarding::historical_external_pool_onboarding_application_authority_on,
    },
};

use super::build::installed_total_bytes;

pub(super) fn credential_subject_on(
    conn: &rusqlite::Connection,
    authority: &CurrentExternalPoolAdapterRegistryProviderBindingAuthority,
) -> Result<(String, String)> {
    let binding = authority.prepared().binding();
    let onboarding = historical_external_pool_onboarding_application_authority_on(
        conn,
        &binding.application_id,
        &binding.application_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("runtime launch profile lost onboarding credential root"))?;
    let locator = onboarding.non_bearer_credential_ref();
    let scheme = credential_ref_scheme(locator)?;
    let commitment = credential_locator_commitment(locator);
    if scheme != "vault_ref"
        || commitment != binding.credential_locator_commitment
        || commitment != authority.binding().binding.credential_locator_commitment
    {
        bail!("runtime launch profile credential resolver subject is unsupported or inexact");
    }
    Ok((scheme.into(), commitment))
}

pub(super) fn audit_current_roots(
    authority: &CurrentExternalPoolAdapterRegistryProviderBindingAuthority,
    candidate: &ExternalPoolProviderActivationCandidateReceipt,
    profile: &ExternalPoolAdapterRuntimeLaunchProfileReceipt,
    credential_ref_scheme: &str,
    credential_locator_commitment: &str,
) -> Result<()> {
    let c = &candidate.candidate;
    let p = &profile.profile;
    let binding_receipt = authority.binding();
    let b = &binding_receipt.binding;
    let prepared = authority.prepared();
    let installed = prepared.binding();
    if profile.profile.candidate_id != candidate.candidate_id
        || p.candidate_digest != candidate.candidate_digest
        || p.delegation_id != c.delegation_id
        || p.delegation_digest != c.delegation_digest
        || p.provider_binding_id != binding_receipt.provider_binding_id
        || p.provider_binding_digest != binding_receipt.provider_binding_digest
        || p.registry_release_id != c.registry_release_id
        || p.registry_release_digest != c.registry_release_digest
        || p.installation_receipt_id != c.installation_receipt_id
        || p.installation_receipt_digest != c.installation_receipt_digest
        || p.installation_content_digest != c.installation_content_digest
        || p.route_adapter_projection_id != c.route_adapter_projection_id
        || p.provider_id != c.provider_id
        || p.provider_owner_account_id != c.provider_owner_account_id
        || p.provider_policy_revision != c.provider_policy_revision
        || p.provider_digest != c.provider_digest
        || p.provider_status != c.provider_status
        || p.logical_adapter_id != c.logical_adapter_id
        || p.release_version != c.release_version
        || p.adapter_config_revision != c.adapter_config_revision
        || p.adapter_config_digest != c.adapter_config_digest
        || p.implementation_digest != c.implementation_digest
        || p.capability_set_digest != c.capability_set_digest
        || p.credential_verifier_digest != c.credential_verifier_digest
        || p.service_actor_id != c.service_actor_id
        || p.entrypoint_relative_path != installed.entrypoint_path
        || p.entrypoint_path_digest
            != runtime_launch_entrypoint_path_digest(&p.entrypoint_relative_path)
        || p.entrypoint_sha256 != installed.entrypoint_sha256
        || p.entrypoint_size_bytes != installed.entrypoint_size_bytes
        || p.entry_inventory_digest != installed.entry_inventory_digest
        || p.installed_file_count != installed.installed_files.len() as u64
        || p.installed_total_bytes != installed_total_bytes(prepared)
        || p.credential_ref_scheme != credential_ref_scheme
        || p.credential_locator_commitment != credential_locator_commitment
        || p.credential_locator_commitment != b.credential_locator_commitment
    {
        bail!("runtime launch profile current V249/V254 roots drifted");
    }
    Ok(())
}

pub(super) fn audit_replay_prepared(
    prepared: &crate::compute_federation::external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
    profile: &ExternalPoolAdapterRuntimeLaunchProfileReceipt,
) -> Result<()> {
    let p = &profile.profile;
    let b = prepared.binding();
    if p.installation_content_digest != b.installation_content_digest
        || p.entrypoint_relative_path != b.entrypoint_path
        || p.entrypoint_path_digest
            != runtime_launch_entrypoint_path_digest(&p.entrypoint_relative_path)
        || p.entrypoint_sha256 != b.entrypoint_sha256
        || p.entrypoint_size_bytes != b.entrypoint_size_bytes
        || p.entry_inventory_digest != b.entry_inventory_digest
        || p.installed_file_count != b.installed_files.len() as u64
        || p.installed_total_bytes != installed_total_bytes(prepared)
        || p.credential_locator_commitment != b.credential_locator_commitment
    {
        bail!("runtime launch profile replay Prepared files are not exact");
    }
    Ok(())
}

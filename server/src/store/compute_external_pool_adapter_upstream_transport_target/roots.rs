use anyhow::{bail, Result};

use crate::{
    compute_federation::{
        external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
        external_pool_adapter_runtime_launch_profile::ExternalPoolAdapterRuntimeLaunchProfileReceipt,
        external_pool_adapter_upstream_transport_target::ExternalPoolAdapterUpstreamTransportTargetReceipt,
    },
    store::compute_external_pool_adapter_runtime_launch_profile::CurrentExternalPoolAdapterRuntimeLaunchProfileAuthority,
};

pub(super) fn audit_current_roots(
    authority: &CurrentExternalPoolAdapterRuntimeLaunchProfileAuthority,
    target: &ExternalPoolAdapterUpstreamTransportTargetReceipt,
) -> Result<()> {
    audit_profile_roots(authority.profile(), target)
}

pub(super) fn audit_historical_roots(
    profile: &ExternalPoolAdapterRuntimeLaunchProfileReceipt,
    target: &ExternalPoolAdapterUpstreamTransportTargetReceipt,
) -> Result<()> {
    audit_profile_roots(profile, target)
}

fn audit_profile_roots(
    profile_receipt: &ExternalPoolAdapterRuntimeLaunchProfileReceipt,
    target_receipt: &ExternalPoolAdapterUpstreamTransportTargetReceipt,
) -> Result<()> {
    let p = &profile_receipt.profile;
    let t = &target_receipt.target;
    if t.profile_id != profile_receipt.profile_id
        || t.profile_digest != profile_receipt.profile_digest
        || t.candidate_id != p.candidate_id
        || t.candidate_digest != p.candidate_digest
        || t.delegation_id != p.delegation_id
        || t.delegation_digest != p.delegation_digest
        || t.provider_binding_id != p.provider_binding_id
        || t.provider_binding_digest != p.provider_binding_digest
        || t.registry_release_id != p.registry_release_id
        || t.registry_release_digest != p.registry_release_digest
        || t.installation_receipt_id != p.installation_receipt_id
        || t.installation_receipt_digest != p.installation_receipt_digest
        || t.installation_content_digest != p.installation_content_digest
        || t.route_adapter_projection_id != p.route_adapter_projection_id
        || t.provider_id != p.provider_id
        || t.provider_owner_account_id != p.provider_owner_account_id
        || t.provider_policy_revision != p.provider_policy_revision
        || t.provider_digest != p.provider_digest
        || t.provider_status != p.provider_status
        || t.logical_adapter_id != p.logical_adapter_id
        || t.release_version != p.release_version
        || t.adapter_config_revision != p.adapter_config_revision
        || t.adapter_config_digest != p.adapter_config_digest
        || t.implementation_digest != p.implementation_digest
        || t.capability_set_digest != p.capability_set_digest
        || t.credential_verifier_digest != p.credential_verifier_digest
        || t.launch_policy_digest != p.launch_policy_digest
        || t.network_egress_policy_id != p.launch_policy.network_egress_policy_id
        || t.network_egress_policy_revision != p.launch_policy.network_egress_policy_revision
        || t.network_egress_policy_digest != p.launch_policy.network_egress_policy_digest
        || t.service_actor_id != p.service_actor_id
    {
        bail!("upstream transport target V255 roots are not exact");
    }
    Ok(())
}

pub(in crate::store) fn audit_replay_prepared(
    prepared: &PreparedExternalPoolAdapterInstallation,
    profile: &ExternalPoolAdapterRuntimeLaunchProfileReceipt,
) -> Result<()> {
    let binding = prepared.binding();
    let p = &profile.profile;
    if binding.installation_content_digest != p.installation_content_digest
        || binding.entrypoint_path != p.entrypoint_relative_path
        || binding.entrypoint_sha256 != p.entrypoint_sha256
        || binding.entrypoint_size_bytes != p.entrypoint_size_bytes
        || binding.entry_inventory_digest != p.entry_inventory_digest
        || binding.installed_files.len() as u64 != p.installed_file_count
        || binding.credential_locator_commitment != p.credential_locator_commitment
    {
        bail!("upstream transport target replay Prepared installation is not exact");
    }
    let total = binding
        .installed_files
        .iter()
        .try_fold(0_u64, |sum, file| sum.checked_add(file.size_bytes))
        .ok_or_else(|| anyhow::anyhow!("Prepared installation byte total overflow"))?;
    if total != p.installed_total_bytes {
        bail!("upstream transport target replay Prepared byte total is not exact");
    }
    Ok(())
}

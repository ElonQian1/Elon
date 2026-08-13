use anyhow::{bail, Result};

use crate::{
    compute_federation::{
        external_pool_adapter_runtime_launch_profile::ExternalPoolAdapterRuntimeLaunchProfileReceipt,
        external_pool_adapter_supervisor_session_policy_companion::ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt,
        external_pool_adapter_upstream_transport_target::ExternalPoolAdapterUpstreamTransportTargetReceipt,
    },
    store::compute_external_pool_adapter_upstream_transport_target::CurrentExternalPoolAdapterUpstreamTransportTargetAuthority,
};

pub(super) fn audit_current_roots(
    authority: &CurrentExternalPoolAdapterUpstreamTransportTargetAuthority,
    companion: &ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt,
) -> Result<()> {
    audit_roots(authority.target(), authority.profile().profile(), companion)
}
pub(super) fn audit_historical_roots(
    target: &ExternalPoolAdapterUpstreamTransportTargetReceipt,
    profile: &ExternalPoolAdapterRuntimeLaunchProfileReceipt,
    companion: &ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt,
) -> Result<()> {
    audit_roots(target, profile, companion)
}
fn audit_roots(
    target: &ExternalPoolAdapterUpstreamTransportTargetReceipt,
    profile: &ExternalPoolAdapterRuntimeLaunchProfileReceipt,
    receipt: &ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt,
) -> Result<()> {
    let t = &target.target;
    let p = &profile.profile;
    let c = &receipt.companion;
    let launch = &p.launch_policy;
    if c.target_id != target.target_id
        || c.target_digest != target.target_digest
        || c.target_policy_digest != t.target_policy_digest
        || c.profile_id != profile.profile_id
        || c.profile_digest != profile.profile_digest
        || c.candidate_id != p.candidate_id
        || c.candidate_digest != p.candidate_digest
        || c.delegation_id != p.delegation_id
        || c.delegation_digest != p.delegation_digest
        || c.provider_binding_id != p.provider_binding_id
        || c.provider_binding_digest != p.provider_binding_digest
        || c.registry_release_id != p.registry_release_id
        || c.registry_release_digest != p.registry_release_digest
        || c.installation_receipt_id != p.installation_receipt_id
        || c.installation_receipt_digest != p.installation_receipt_digest
        || c.installation_content_digest != p.installation_content_digest
        || c.route_adapter_projection_id != p.route_adapter_projection_id
        || c.provider_id != p.provider_id
        || c.provider_owner_account_id != p.provider_owner_account_id
        || c.provider_policy_revision != p.provider_policy_revision
        || c.provider_digest != p.provider_digest
        || c.provider_status != p.provider_status
        || c.logical_adapter_id != p.logical_adapter_id
        || c.release_version != p.release_version
        || c.adapter_config_revision != p.adapter_config_revision
        || c.adapter_config_digest != p.adapter_config_digest
        || c.implementation_digest != p.implementation_digest
        || c.capability_set_digest != p.capability_set_digest
        || c.credential_verifier_digest != p.credential_verifier_digest
        || c.service_actor_id != p.service_actor_id
        || c.launch_policy_digest != p.launch_policy_digest
        || c.process_isolation_policy_id != launch.process_isolation_policy_id
        || c.process_isolation_policy_revision != launch.process_isolation_policy_revision
        || c.process_isolation_policy_digest != launch.process_isolation_policy_digest
        || c.resource_policy_id != launch.resource_policy_id
        || c.resource_policy_revision != launch.resource_policy_revision
        || c.resource_policy_digest != launch.resource_policy_digest
        || c.network_egress_policy_id != launch.network_egress_policy_id
        || c.network_egress_policy_revision != launch.network_egress_policy_revision
        || c.network_egress_policy_digest != launch.network_egress_policy_digest
    {
        bail!("supervisor session companion V255/V258 roots are not exact")
    }
    Ok(())
}

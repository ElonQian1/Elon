//! Stable root binding retained across transaction-free child and network work.

use std::time::Duration;

use anyhow::{bail, Result};
use ring::constant_time::verify_slices_are_equal;

use super::super::types::{
    CurrentExternalPoolAdapterProbePreparationAuthority,
    CurrentExternalPoolAdapterRuntimeBundleAuthority,
};
use super::super::{
    entrypoint_capsule::PreparedExternalPoolAdapterEntrypointCapsule,
    projected_active_bundle::CurrentExternalPoolAdapterProjectedActiveRuntimeBundleAuthority,
};
use crate::{
    compute_federation::{
        external_pool_adapter_installation::ExternalPoolAdapterInstallationBinding,
        external_pool_adapter_task_protocol_conformance::ExternalPoolAdapterTaskProtocolConformanceRunReceipt,
        external_pool_adapter_upstream_transport_target::ExternalPoolAdapterUpstreamTransportTargetReceipt,
    },
    store::{
        compute_external_pool_adapter_provider_active_successor::CurrentExternalPoolAdapterRenewedRouteRuntimeCarrierAuthority,
        compute_external_pool_adapter_sandbox_reattestation::current_external_pool_adapter_sandbox_reattestation_authority_on,
        compute_external_pool_adapter_supervisor_session_policy_companion::CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority,
        compute_external_pool_adapter_vulnerability_reattestation::current_external_pool_adapter_vulnerability_reattestation_authority_on,
    },
};

/// Stable non-secret roots retained across transaction-free network waits.
/// It is intentionally neither Clone, Debug, nor serializable.
pub(in crate::store::compute_external_pool_adapter_runtime_bundle) struct ExternalPoolAdapterEphemeralSecretDeliveryBinding
{
    policy_digest: String,
    profile_digest: String,
    target_digest: String,
    companion_digest: String,
    source_capsule_digest: String,
    launch_capsule_digest: String,
    launch_capsule_size_bytes: u64,
    delivery_root: String,
    bundle_material_digest: [u8; 32],
    runtime_bundle_identity_commitment: String,
    vulnerability_reattestation_receipt_id: String,
    vulnerability_reattestation_receipt_digest: String,
    sandbox_reattestation_receipt_id: String,
    sandbox_reattestation_receipt_digest: String,
    credential_reattestation_receipt_id: String,
    credential_reattestation_receipt_digest: String,
    installation: ExternalPoolAdapterInstallationBinding,
    upstream_target: ExternalPoolAdapterUpstreamTransportTargetReceipt,
    projected_active_evidence: Option<ProjectedActiveDeliveryEvidence>,
    probe_timeout_ms: u64,
}

#[derive(Eq, PartialEq)]
struct ProjectedActiveDeliveryEvidence {
    route_renewal_receipt_id: String,
    route_renewal_receipt_digest: String,
    route_effective_expires_at: String,
    runtime_compatibility_verification_receipt_id: String,
    runtime_compatibility_verification_receipt_digest: String,
    task_protocol_run_receipt_id: String,
    task_protocol_run_receipt_digest: String,
}

pub(in crate::store::compute_external_pool_adapter_runtime_bundle) fn delivery_binding(
    preparation: &CurrentExternalPoolAdapterProbePreparationAuthority<'_, '_, '_>,
    companion: &CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority,
    delivery_root: &str,
    session_root_arguments: &[String],
    runtime_bundle_identity_commitment: String,
) -> Result<ExternalPoolAdapterEphemeralSecretDeliveryBinding> {
    use sha2::{Digest, Sha256};

    let bundle = preparation.bundle();
    let roots = bundle.roots();
    let companion_receipt = companion.companion();
    let material = &companion_receipt.companion;
    let profile = &bundle.launch_profile().profile().profile;
    let capsule = preparation.capsule();
    let vulnerability = preparation.vulnerability().receipt();
    let sandbox = preparation.sandbox().receipt();
    let credential = bundle.credential().receipt();
    let probe_timeout_ms = profile.launch_policy.probe_timeout_ms;
    if session_root_arguments.len() != 6
        || profile.launch_policy.probe_contract != "authenticated_no_work_readiness_v1"
        || probe_timeout_ms == 0
        || probe_timeout_ms != material.supervisor_session_policy.state.probe_timeout_ms
        || session_root_arguments[0] != material.supervisor_session_policy_digest
        || session_root_arguments[1] != material.profile_digest
        || session_root_arguments[2] != material.target_digest
        || session_root_arguments[3] != companion_receipt.companion_digest
        || capsule.launch_size_bytes() == 0
        || capsule.launch_sha256() == capsule.entrypoint_sha256()
        || session_root_arguments[4] != capsule.launch_sha256()
        || session_root_arguments[5] != delivery_root
    {
        bail!("ephemeral secret delivery no-work roots rejected");
    }
    let mut digest = Sha256::new();
    digest.update(b"elon.external_pool_adapter.bundle_material.v1\0");
    digest.update(roots.bundle_generation().to_be_bytes());
    digest.update(roots.config_size_bytes().to_be_bytes());
    digest.update(roots.config_sha256());
    digest.update(roots.credential_size_bytes().to_be_bytes());
    digest.update(roots.credential_sha256());
    Ok(ExternalPoolAdapterEphemeralSecretDeliveryBinding {
        policy_digest: session_root_arguments[0].clone(),
        profile_digest: session_root_arguments[1].clone(),
        target_digest: session_root_arguments[2].clone(),
        companion_digest: session_root_arguments[3].clone(),
        source_capsule_digest: capsule.entrypoint_sha256().to_string(),
        launch_capsule_digest: session_root_arguments[4].clone(),
        launch_capsule_size_bytes: capsule.launch_size_bytes(),
        delivery_root: session_root_arguments[5].clone(),
        bundle_material_digest: digest.finalize().into(),
        runtime_bundle_identity_commitment,
        vulnerability_reattestation_receipt_id: vulnerability.reattestation_receipt_id.clone(),
        vulnerability_reattestation_receipt_digest: vulnerability
            .reattestation_receipt_digest
            .clone(),
        sandbox_reattestation_receipt_id: sandbox.reattestation_receipt_id.clone(),
        sandbox_reattestation_receipt_digest: sandbox.reattestation_receipt_digest.clone(),
        credential_reattestation_receipt_id: credential.reattestation_receipt_id.clone(),
        credential_reattestation_receipt_digest: credential.reattestation_receipt_digest.clone(),
        installation: bundle
            .launch_profile()
            .candidate()
            .registry()
            .prepared()
            .binding()
            .clone(),
        upstream_target: companion.target().target().clone(),
        projected_active_evidence: None,
        probe_timeout_ms,
    })
}

pub(in crate::store::compute_external_pool_adapter_runtime_bundle) fn projected_active_delivery_binding(
    bundle: &CurrentExternalPoolAdapterProjectedActiveRuntimeBundleAuthority<'_, '_>,
    capsule: &PreparedExternalPoolAdapterEntrypointCapsule,
    delivery_root: &str,
    session_root_arguments: &[String],
    runtime_bundle_identity_commitment: String,
    task_protocol: &ExternalPoolAdapterTaskProtocolConformanceRunReceipt,
) -> Result<ExternalPoolAdapterEphemeralSecretDeliveryBinding> {
    use sha2::{Digest, Sha256};

    let carrier = bundle.carrier();
    let profile = &carrier.profile().profile;
    let companion = carrier.companion();
    let target = carrier.target();
    let credential = carrier.credential().receipt();
    let vulnerability = bundle.vulnerability().receipt();
    let sandbox = bundle.sandbox().receipt();
    let route = carrier.renewed_route();
    let runtime = carrier.runtime_compatibility().verification();
    let roots = bundle.roots();
    let probe_timeout_ms = profile.launch_policy.probe_timeout_ms;
    if session_root_arguments.len() != 6
        || profile.launch_policy.probe_contract != "authenticated_no_work_readiness_v1"
        || probe_timeout_ms == 0
        || probe_timeout_ms
            != companion
                .companion
                .supervisor_session_policy
                .state
                .probe_timeout_ms
        || session_root_arguments[0] != companion.companion.supervisor_session_policy_digest
        || session_root_arguments[1] != companion.companion.profile_digest
        || session_root_arguments[2] != target.target_digest
        || session_root_arguments[3] != companion.companion_digest
        || session_root_arguments[4] != capsule.launch_sha256()
        || session_root_arguments[5] != delivery_root
        || capsule.launch_size_bytes() == 0
        || capsule.launch_sha256() == capsule.entrypoint_sha256()
    {
        bail!("projected-active secret delivery roots rejected");
    }
    let mut digest = Sha256::new();
    digest.update(b"elon.external_pool_adapter.bundle_material.v1\0");
    digest.update(roots.bundle_generation().to_be_bytes());
    digest.update(roots.config_size_bytes().to_be_bytes());
    digest.update(roots.config_sha256());
    digest.update(roots.credential_size_bytes().to_be_bytes());
    digest.update(roots.credential_sha256());
    Ok(ExternalPoolAdapterEphemeralSecretDeliveryBinding {
        policy_digest: session_root_arguments[0].clone(),
        profile_digest: session_root_arguments[1].clone(),
        target_digest: session_root_arguments[2].clone(),
        companion_digest: session_root_arguments[3].clone(),
        source_capsule_digest: capsule.entrypoint_sha256().into(),
        launch_capsule_digest: session_root_arguments[4].clone(),
        launch_capsule_size_bytes: capsule.launch_size_bytes(),
        delivery_root: session_root_arguments[5].clone(),
        bundle_material_digest: digest.finalize().into(),
        runtime_bundle_identity_commitment,
        vulnerability_reattestation_receipt_id: vulnerability.reattestation_receipt_id.clone(),
        vulnerability_reattestation_receipt_digest: vulnerability
            .reattestation_receipt_digest
            .clone(),
        sandbox_reattestation_receipt_id: sandbox.reattestation_receipt_id.clone(),
        sandbox_reattestation_receipt_digest: sandbox.reattestation_receipt_digest.clone(),
        credential_reattestation_receipt_id: credential.reattestation_receipt_id.clone(),
        credential_reattestation_receipt_digest: credential.reattestation_receipt_digest.clone(),
        installation: carrier.prepared().binding().clone(),
        upstream_target: target.clone(),
        projected_active_evidence: Some(ProjectedActiveDeliveryEvidence {
            route_renewal_receipt_id: route.receipt().route_renewal_receipt_id.clone(),
            route_renewal_receipt_digest: route.receipt().route_renewal_receipt_digest.clone(),
            route_effective_expires_at: route.effective_expires_at().into(),
            runtime_compatibility_verification_receipt_id: runtime.verification_receipt_id.clone(),
            runtime_compatibility_verification_receipt_digest: runtime
                .verification_receipt_digest
                .clone(),
            task_protocol_run_receipt_id: task_protocol.run_receipt_id.clone(),
            task_protocol_run_receipt_digest: task_protocol.run_receipt_digest.clone(),
        }),
        probe_timeout_ms,
    })
}

pub(in crate::store::compute_external_pool_adapter_runtime_bundle) fn audit_projected_active_delivery_binding_on(
    transaction: &rusqlite::Transaction<'_>,
    binding: &ExternalPoolAdapterEphemeralSecretDeliveryBinding,
    carrier: &CurrentExternalPoolAdapterRenewedRouteRuntimeCarrierAuthority<'_, '_>,
    task_protocol: &ExternalPoolAdapterTaskProtocolConformanceRunReceipt,
    bundle: Option<&CurrentExternalPoolAdapterProjectedActiveRuntimeBundleAuthority<'_, '_>>,
    checked_at: &str,
) -> Result<()> {
    let root = &carrier
        .historical_activation()
        .activation_root()
        .activation_root;
    let vulnerability = current_external_pool_adapter_vulnerability_reattestation_authority_on(
        transaction,
        &root.registry_release_id,
        &binding.vulnerability_reattestation_receipt_id,
        &binding.vulnerability_reattestation_receipt_digest,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("active secret session lost exact current V250"))?;
    let sandbox = current_external_pool_adapter_sandbox_reattestation_authority_on(
        transaction,
        &root.registry_release_id,
        &binding.sandbox_reattestation_receipt_id,
        &binding.sandbox_reattestation_receipt_digest,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("active secret session lost exact current V252"))?;
    let credential = carrier.credential().receipt();
    let compatibility = carrier.runtime_compatibility().verification();
    let route = carrier.renewed_route();
    let active = binding
        .projected_active_evidence
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("active secret session lacks projected-active evidence"))?;
    if carrier.checked_at() != checked_at
        || carrier.renewed_route().checked_at() != checked_at
        || carrier.renewed_route().effective_expires_at() <= checked_at
        || vulnerability.checked_at() != checked_at
        || sandbox.checked_at() != checked_at
        || credential.reattestation_receipt_id != binding.credential_reattestation_receipt_id
        || credential.reattestation_receipt_digest
            != binding.credential_reattestation_receipt_digest
        || carrier.prepared().binding() != &binding.installation
        || carrier.target() != &binding.upstream_target
        || carrier.companion().companion_digest != binding.companion_digest
        || carrier.profile().profile_digest != binding.profile_digest
        || route.receipt().route_renewal_receipt_id != active.route_renewal_receipt_id
        || route.receipt().route_renewal_receipt_digest != active.route_renewal_receipt_digest
        || route.effective_expires_at() != active.route_effective_expires_at
        || compatibility.verification_receipt_id
            != active.runtime_compatibility_verification_receipt_id
        || compatibility.verification_receipt_digest
            != active.runtime_compatibility_verification_receipt_digest
        || task_protocol.run_receipt_id != active.task_protocol_run_receipt_id
        || task_protocol.run_receipt_digest != active.task_protocol_run_receipt_digest
    {
        bail!("projected-active secret session roots changed after bundle preparation");
    }
    if let Some(bundle) = bundle {
        if bundle.checked_at() != checked_at
            || bundle.vulnerability().receipt().reattestation_receipt_id
                != binding.vulnerability_reattestation_receipt_id
            || bundle
                .vulnerability()
                .receipt()
                .reattestation_receipt_digest
                != binding.vulnerability_reattestation_receipt_digest
            || bundle.sandbox().receipt().reattestation_receipt_id
                != binding.sandbox_reattestation_receipt_id
            || bundle.sandbox().receipt().reattestation_receipt_digest
                != binding.sandbox_reattestation_receipt_digest
        {
            bail!("projected-active final bundle changed V250/V252 delivery evidence");
        }
    }
    Ok(())
}

pub(in crate::store::compute_external_pool_adapter_runtime_bundle) fn audit_delivery_roots(
    bundle: &CurrentExternalPoolAdapterRuntimeBundleAuthority<'_, '_>,
    companion: &CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority,
    checked_at: &str,
) -> Result<()> {
    let bundle_profile = bundle.launch_profile().profile();
    let companion_receipt = companion.companion();
    let companion_material = &companion_receipt.companion;
    let target = companion.target();
    let target_profile = target.profile().profile();
    let bundle_installation = bundle
        .launch_profile()
        .candidate()
        .registry()
        .prepared()
        .binding();
    let session_installation = target.profile().candidate().registry().prepared().binding();
    if bundle.checked_at() != checked_at
        || companion.checked_at() != checked_at
        || target.checked_at() != checked_at
        || companion_material.profile_id != bundle_profile.profile_id
        || companion_material.profile_digest != bundle_profile.profile_digest
        || target_profile.profile_id != bundle_profile.profile_id
        || target_profile.profile_digest != bundle_profile.profile_digest
        || companion_material.target_id != target.target().target_id
        || companion_material.target_digest != target.target().target_digest
        || companion_material.provider_binding_id != bundle_profile.profile.provider_binding_id
        || companion_material.provider_binding_digest
            != bundle_profile.profile.provider_binding_digest
        || bundle_installation != session_installation
    {
        bail!("ephemeral secret delivery roots drifted");
    }
    Ok(())
}

impl PartialEq for ExternalPoolAdapterEphemeralSecretDeliveryBinding {
    fn eq(&self, other: &Self) -> bool {
        self.policy_digest == other.policy_digest
            && self.profile_digest == other.profile_digest
            && self.target_digest == other.target_digest
            && self.companion_digest == other.companion_digest
            && self.source_capsule_digest == other.source_capsule_digest
            && self.launch_capsule_digest == other.launch_capsule_digest
            && self.launch_capsule_size_bytes == other.launch_capsule_size_bytes
            && self.delivery_root == other.delivery_root
            && self.bundle_material_digest == other.bundle_material_digest
            && verify_slices_are_equal(
                self.runtime_bundle_identity_commitment.as_bytes(),
                other.runtime_bundle_identity_commitment.as_bytes(),
            )
            .is_ok()
            && self.vulnerability_reattestation_receipt_id
                == other.vulnerability_reattestation_receipt_id
            && self.vulnerability_reattestation_receipt_digest
                == other.vulnerability_reattestation_receipt_digest
            && self.sandbox_reattestation_receipt_id == other.sandbox_reattestation_receipt_id
            && self.sandbox_reattestation_receipt_digest
                == other.sandbox_reattestation_receipt_digest
            && self.credential_reattestation_receipt_id == other.credential_reattestation_receipt_id
            && self.credential_reattestation_receipt_digest
                == other.credential_reattestation_receipt_digest
            && self.installation == other.installation
            && self.upstream_target == other.upstream_target
            && self.projected_active_evidence == other.projected_active_evidence
            && self.probe_timeout_ms == other.probe_timeout_ms
    }
}

impl Eq for ExternalPoolAdapterEphemeralSecretDeliveryBinding {}

impl ExternalPoolAdapterEphemeralSecretDeliveryBinding {
    pub(in crate::store::compute_external_pool_adapter_runtime_bundle) fn upstream_target(
        &self,
    ) -> &ExternalPoolAdapterUpstreamTransportTargetReceipt {
        &self.upstream_target
    }

    pub(in crate::store::compute_external_pool_adapter_runtime_bundle) fn delivery_root(
        &self,
    ) -> &str {
        &self.delivery_root
    }

    pub(in crate::store::compute_external_pool_adapter_runtime_bundle) fn target_digest(
        &self,
    ) -> &str {
        &self.target_digest
    }

    pub(in crate::store::compute_external_pool_adapter_runtime_bundle) fn source_capsule_digest(
        &self,
    ) -> &str {
        &self.source_capsule_digest
    }

    pub(in crate::store::compute_external_pool_adapter_runtime_bundle) fn launch_capsule_digest(
        &self,
    ) -> &str {
        &self.launch_capsule_digest
    }

    pub(in crate::store::compute_external_pool_adapter_runtime_bundle) fn runtime_bundle_identity_commitment(
        &self,
    ) -> &str {
        &self.runtime_bundle_identity_commitment
    }

    pub(in crate::store::compute_external_pool_adapter_runtime_bundle) fn session_root_arguments(
        &self,
    ) -> [String; 6] {
        [
            self.policy_digest.clone(),
            self.profile_digest.clone(),
            self.target_digest.clone(),
            self.companion_digest.clone(),
            self.launch_capsule_digest.clone(),
            self.delivery_root.clone(),
        ]
    }

    pub(in crate::store::compute_external_pool_adapter_runtime_bundle) fn probe_timeout(
        &self,
    ) -> Duration {
        Duration::from_millis(self.probe_timeout_ms)
    }
}

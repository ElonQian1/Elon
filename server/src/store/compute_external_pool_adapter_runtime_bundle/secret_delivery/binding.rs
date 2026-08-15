//! Stable root binding retained across transaction-free child and network work.

use std::time::Duration;

use anyhow::{bail, Result};
use ring::constant_time::verify_slices_are_equal;

use super::super::types::{
    CurrentExternalPoolAdapterProbePreparationAuthority,
    CurrentExternalPoolAdapterRuntimeBundleAuthority,
};
use crate::{
    compute_federation::{
        external_pool_adapter_installation::ExternalPoolAdapterInstallationBinding,
        external_pool_adapter_upstream_transport_target::ExternalPoolAdapterUpstreamTransportTargetReceipt,
    },
    store::compute_external_pool_adapter_supervisor_session_policy_companion::CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority,
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
    probe_timeout_ms: u64,
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
        probe_timeout_ms,
    })
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

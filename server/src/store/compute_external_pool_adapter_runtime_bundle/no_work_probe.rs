use std::marker::PhantomData;

use anyhow::{bail, Result};
use rusqlite::Transaction;

mod reproof;

use super::{
    probe_preparation::{audit_credential_roots, audit_sandbox_roots, audit_vulnerability_roots},
    runtime::ExternalPoolAdapterProviderRuntimeReadinessRuntime,
    secret_delivery::{
        CleanedExternalPoolAdapterEphemeralSecretDeliveryAuthority,
        ExternalPoolAdapterEphemeralSecretDeliveryBinding,
    },
};
use crate::{
    compute_federation::{
        external_pool_adapter_broker_tls::ExternalPoolAdapterBrokerTlsTarget,
        external_pool_adapter_credential_reattestation::ExternalPoolAdapterCredentialReattestationReceipt,
        external_pool_adapter_runtime_compatibility_verification::server_runtime_compatibility_v2_profile_catalog,
        external_pool_adapter_runtime_launch_profile::ExternalPoolAdapterRuntimeLaunchProfileReceipt,
        external_pool_adapter_sandbox_reattestation::ExternalPoolAdapterSandboxReattestationReceipt,
        external_pool_adapter_vulnerability_reattestation::ExternalPoolAdapterVulnerabilityReattestationReceipt,
    },
    store::{
        compute_external_pool_adapter_credential_reattestation::{
            current_external_pool_adapter_credential_reattestation_head_authority_on,
            CurrentExternalPoolAdapterCredentialReattestationAuthority,
        },
        compute_external_pool_adapter_runtime_compatibility_verification::{
            current_external_pool_adapter_runtime_compatibility_verification_authority_on,
            CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority,
        },
        compute_external_pool_adapter_sandbox_reattestation::{
            current_external_pool_adapter_sandbox_reattestation_head_authority_on,
            CurrentExternalPoolAdapterSandboxReattestationAuthority,
        },
        compute_external_pool_adapter_supervisor_session_policy_companion::CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority,
        compute_external_pool_adapter_upstream_transport_target::{
            CurrentExternalPoolAdapterUpstreamTransportTargetAuthority,
            ExternalPoolAdapterInstallationReopener,
        },
        compute_external_pool_adapter_vulnerability_reattestation::{
            current_external_pool_adapter_vulnerability_reattestation_head_authority_on,
            CurrentExternalPoolAdapterVulnerabilityReattestationAuthority,
        },
        Store,
    },
};
use elon_external_pool_adapter_session_core::ExternalPoolAdapterNoWorkProbeHostReceipt;

const MAX_PROVIDER_READINESS_PROBE_TIMEOUT_MS: u64 = 15_000;

/// Process-private proof of one exact post-cleanup no-task response.
///
/// Every database-backed field is borrowed from the final IMMEDIATE transaction. The authority is
/// neither Clone, Debug, nor serializable and cannot survive the final callback.
pub(in crate::store) struct CurrentExternalPoolAdapterNoWorkProbeObservationAuthority<
    'authority,
    'tx,
    'conn,
> {
    receipt: &'authority ExternalPoolAdapterNoWorkProbeHostReceipt,
    binding: &'authority ExternalPoolAdapterEphemeralSecretDeliveryBinding,
    selected_address: std::net::SocketAddr,
    launch_profile: &'authority ExternalPoolAdapterRuntimeLaunchProfileReceipt,
    vulnerability: &'authority CurrentExternalPoolAdapterVulnerabilityReattestationAuthority,
    sandbox: &'authority CurrentExternalPoolAdapterSandboxReattestationAuthority,
    credential: &'authority CurrentExternalPoolAdapterCredentialReattestationAuthority,
    companion: &'authority CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority,
    runtime_compatibility:
        &'authority CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority<'tx, 'conn>,
    cleaned: &'authority CleanedExternalPoolAdapterEphemeralSecretDeliveryAuthority,
    runtime_bundle_identity_commitment: &'authority str,
    post_cleanup_observation_commitment: String,
    custody_epoch_digest: &'authority str,
    checked_at: String,
    expires_at: String,
    transaction: PhantomData<&'tx Transaction<'conn>>,
}

impl<'authority, 'tx, 'conn>
    CurrentExternalPoolAdapterNoWorkProbeObservationAuthority<'authority, 'tx, 'conn>
{
    pub(in crate::store) fn no_work_observed(&self) -> bool {
        let _retained_exact_authority = (
            self.receipt,
            self.binding,
            self.selected_address,
            self.launch_profile,
            self.vulnerability,
            self.sandbox,
            self.credential,
            self.companion,
            self.runtime_compatibility,
            self.cleaned,
            self.runtime_bundle_identity_commitment,
            &self.post_cleanup_observation_commitment,
            self.custody_epoch_digest,
            &self.checked_at,
            &self.expires_at,
            &self.transaction,
        );
        true
    }

    pub(in crate::store) fn request_bytes(&self) -> u32 {
        self.receipt.request_bytes()
    }

    pub(in crate::store) fn response_bytes(&self) -> u32 {
        self.receipt.response_bytes()
    }

    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }

    /// The child authenticated the no-work exchange at this earlier timestamp. The final
    /// `checked_at` is intentionally a distinct post-reap/post-cleanup transaction anchor.
    pub(in crate::store) fn probe_checked_at(&self) -> &str {
        self.cleaned.delivery_checked_at()
    }

    pub(in crate::store) fn expires_at(&self) -> &str {
        &self.expires_at
    }

    pub(in crate::store) fn launch_profile(
        &self,
    ) -> &ExternalPoolAdapterRuntimeLaunchProfileReceipt {
        self.launch_profile
    }

    pub(in crate::store) fn vulnerability(
        &self,
    ) -> &ExternalPoolAdapterVulnerabilityReattestationReceipt {
        self.vulnerability.receipt()
    }

    pub(in crate::store) fn sandbox(&self) -> &ExternalPoolAdapterSandboxReattestationReceipt {
        self.sandbox.receipt()
    }

    pub(in crate::store) fn credential(
        &self,
    ) -> &ExternalPoolAdapterCredentialReattestationReceipt {
        self.credential.receipt()
    }

    pub(in crate::store) fn companion(
        &self,
    ) -> &CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority {
        self.companion
    }

    pub(in crate::store) fn runtime_compatibility(
        &self,
    ) -> &CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority<'tx, 'conn> {
        self.runtime_compatibility
    }

    pub(in crate::store) fn runtime_bundle_identity_commitment(&self) -> &str {
        self.runtime_bundle_identity_commitment
    }

    pub(in crate::store) fn post_cleanup_observation_commitment(&self) -> &str {
        &self.post_cleanup_observation_commitment
    }

    pub(in crate::store) fn custody_epoch_digest(&self) -> &str {
        self.custody_epoch_digest
    }

    pub(in crate::store) fn source_capsule_digest(&self) -> &str {
        self.binding.source_capsule_digest()
    }

    pub(in crate::store) fn launch_capsule_digest(&self) -> &str {
        self.binding.launch_capsule_digest()
    }

    pub(in crate::store) fn authenticated_shutdown_completed(&self) -> bool {
        self.cleaned.authenticated_shutdown_completed()
    }

    pub(in crate::store) fn pidfd_reaped(&self) -> bool {
        self.cleaned.pidfd_reaped()
    }

    pub(in crate::store) fn cgroup_cleaned(&self) -> bool {
        self.cleaned.cgroup_cleaned()
    }

    pub(in crate::store) fn scratch_cleaned(&self) -> bool {
        self.cleaned.scratch_cleaned()
    }
}

impl Store {
    #[allow(clippy::too_many_arguments, dead_code)]
    pub(in crate::store) async fn with_current_external_pool_adapter_no_work_probe_observation(
        &self,
        profile_id: &str,
        companion_id: &str,
        expected_companion_digest: &str,
        target_id: &str,
        expected_target_digest: &str,
        runtime_compatibility_verification_receipt_id: &str,
        expected_runtime_compatibility_verification_receipt_digest: &str,
        reopen_prepared: &mut ExternalPoolAdapterInstallationReopener<'_>,
        runtime: &ExternalPoolAdapterProviderRuntimeReadinessRuntime,
        preflight_consume: impl FnOnce(
                &Transaction<'_>,
                &CurrentExternalPoolAdapterUpstreamTransportTargetAuthority,
                &str,
            ) -> Result<()>
            + Send,
        consume: impl FnOnce(
                &Transaction<'_>,
                &CurrentExternalPoolAdapterNoWorkProbeObservationAuthority<'_, '_, '_>,
            ) -> Result<()>
            + Send,
    ) -> Result<bool> {
        let Some(mut broker) = self
            .prepare_current_external_pool_adapter_broker_tls_channel(
                target_id,
                expected_target_digest,
                reopen_prepared,
                |transaction, target, checked_at| {
                    require_preflight_dynamic_and_compatibility_roots(
                        transaction,
                        target,
                        runtime_compatibility_verification_receipt_id,
                        expected_runtime_compatibility_verification_receipt_digest,
                        checked_at,
                    )?;
                    preflight_consume(transaction, target, checked_at)
                },
            )
            .await?
        else {
            return Ok(false);
        };

        // Successful execution uses exactly six independently reopened installation audits. These
        // are #3 and #4; neither is opened before the broker network await completes.
        let delivery_bundle_prepared = reopen_prepared().map_err(anyhow::Error::new)?;
        let delivery_session_prepared = reopen_prepared().map_err(anyhow::Error::new)?;
        let Some(mut delivery) = self
            .prepare_current_external_pool_adapter_ephemeral_secret_delivery(
                profile_id,
                companion_id,
                expected_companion_digest,
                delivery_bundle_prepared,
                delivery_session_prepared,
                runtime.bundle_root(),
                runtime.cgroup_parent(),
                runtime.process_custody(),
            )?
        else {
            return Ok(false);
        };
        let delivery_target =
            ExternalPoolAdapterBrokerTlsTarget::from_receipt(delivery.binding().upstream_target())?;
        if broker.target() != &delivery_target
            || broker.target().target_id() != target_id
            || broker.target().target_digest() != expected_target_digest
        {
            bail!("no-work probe broker and child roots diverged");
        }

        let request = delivery.receive_no_work_request()?;
        let response = broker
            .exchange_no_work(
                request.request(),
                request.expected_response_bytes(),
                delivery.binding().probe_timeout(),
            )
            .await?;
        let selected_address = broker.selected_address();
        let receipt = delivery.complete_no_work_request(request, &response)?;
        drop(response);
        drop(broker);

        // No final callback or durable write is reachable until this consuming transition returns.
        let cleaned = delivery.shutdown_and_reap()?;

        // These are successful-path reopens #5 and #6, deliberately obtained only after cleanup.
        let reproof_bundle_prepared = reopen_prepared().map_err(anyhow::Error::new)?;
        let reproof_session_prepared = reopen_prepared().map_err(anyhow::Error::new)?;
        self.with_reproved_external_pool_adapter_no_work_roots(
            profile_id,
            companion_id,
            expected_companion_digest,
            runtime_compatibility_verification_receipt_id,
            expected_runtime_compatibility_verification_receipt_digest,
            reproof_bundle_prepared,
            reproof_session_prepared,
            runtime,
            receipt,
            selected_address,
            cleaned,
            consume,
        )
    }
}

fn require_preflight_dynamic_and_compatibility_roots(
    transaction: &Transaction<'_>,
    target: &CurrentExternalPoolAdapterUpstreamTransportTargetAuthority,
    verification_receipt_id: &str,
    expected_verification_receipt_digest: &str,
    checked_at: &str,
) -> Result<()> {
    let profile_authority = target.profile();
    let profile = &profile_authority.profile().profile;
    let release_receipt = profile_authority.candidate().registry().release();
    let release = &release_receipt.release;
    let vulnerability =
        current_external_pool_adapter_vulnerability_reattestation_head_authority_on(
            transaction,
            &profile.registry_release_id,
            checked_at,
        )?
        .ok_or_else(|| anyhow::anyhow!("no-work preflight lacks current V250"))?;
    let sandbox = current_external_pool_adapter_sandbox_reattestation_head_authority_on(
        transaction,
        &profile.registry_release_id,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("no-work preflight lacks current V252"))?;
    let credential = current_external_pool_adapter_credential_reattestation_head_authority_on(
        transaction,
        &profile.provider_binding_id,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("no-work preflight lacks current V253"))?;
    audit_vulnerability_roots(
        profile,
        release,
        &vulnerability.receipt().reattestation.binding,
    )?;
    audit_sandbox_roots(
        profile,
        release,
        vulnerability.receipt(),
        &sandbox.receipt().reattestation.binding,
    )?;
    audit_credential_roots(profile, &credential.receipt().reattestation.binding)?;
    let compatibility =
        current_external_pool_adapter_runtime_compatibility_verification_authority_on(
            transaction,
            verification_receipt_id,
            expected_verification_receipt_digest,
            checked_at,
        )?
        .ok_or_else(|| anyhow::anyhow!("no-work preflight lacks exact current V268"))?;
    audit_runtime_compatibility_static_roots(target, &compatibility)
}

fn audit_runtime_compatibility_static_roots(
    target: &CurrentExternalPoolAdapterUpstreamTransportTargetAuthority,
    compatibility: &CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority<'_, '_>,
) -> Result<()> {
    let profile = &target.profile().profile().profile;
    let target_receipt = target.target();
    let target_material = &target_receipt.target;
    let release = target.profile().candidate().registry().release();
    let verification = &compatibility.verification().verification;
    let run = &compatibility.run_observation().observation;
    let catalog = server_runtime_compatibility_v2_profile_catalog()?;
    if compatibility.release() != release
        || &verification.registry_release != release
        || &run.registry_release != release
        || verification.profile_id != catalog.profile.profile_id
        || verification.profile_revision != catalog.profile.profile_revision
        || verification.profile_digest != catalog.profile_digest
        || run.profile_id != catalog.profile.profile_id
        || run.profile_revision != catalog.profile.profile_revision
        || run.profile_digest != catalog.profile_digest
        || catalog.profile.runtime_launch_policy.policy_digest != profile.launch_policy_digest
        || catalog.profile.upstream_transport_policy.policy_digest
            != target_material.target_policy_digest
        || profile.installation_content_digest != release.release.installation_content_digest
        || target_material.installation_content_digest
            != release.release.installation_content_digest
        || compatibility.checked_at() != target.checked_at()
    {
        bail!("V268 and Provider runtime preflight roots diverged");
    }
    Ok(())
}

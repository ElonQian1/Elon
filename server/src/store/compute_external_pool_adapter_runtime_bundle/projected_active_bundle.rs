//! Renewed-route active bundle composition without reusing registering Provider authority.

use std::marker::PhantomData;

use anyhow::{bail, Result};
use rusqlite::Transaction;

use crate::{
    compute_federation::{
        external_pool_adapter_credential_verification::{
            credential_locator_commitment, credential_ref_scheme,
        },
        external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
    },
    store::{
        compute_external_pool_adapter_provider_active_successor::{
            current_external_pool_adapter_renewed_route_runtime_carrier_for_binding_on,
            CurrentExternalPoolAdapterRenewedRouteRuntimeCarrierAuthority,
        },
        compute_external_pool_adapter_sandbox_reattestation::{
            current_external_pool_adapter_sandbox_reattestation_head_authority_on,
            CurrentExternalPoolAdapterSandboxReattestationAuthority,
        },
        compute_external_pool_adapter_vulnerability_reattestation::{
            current_external_pool_adapter_vulnerability_reattestation_head_authority_on,
            CurrentExternalPoolAdapterVulnerabilityReattestationAuthority,
        },
        compute_external_pool_onboarding::historical_external_pool_onboarding_application_authority_on,
    },
};

use super::{
    filesystem::resolve_external_pool_adapter_runtime_bundle,
    types::{
        ExpectedExternalPoolAdapterRuntimeBundle, ExternalPoolAdapterRuntimeBundleRoot,
        ExternalPoolAdapterRuntimeBundleRoots, PreparedExternalPoolAdapterRuntimeBundle,
    },
};

/// Transaction-bound active V256 snapshot. The renewed route remains inseparable from every
/// current V250/V252/V253/V268 root and the retained installation/bundle handles.
pub(in crate::store) struct CurrentExternalPoolAdapterProjectedActiveRuntimeBundleAuthority<
    'tx,
    'conn,
> {
    carrier: CurrentExternalPoolAdapterRenewedRouteRuntimeCarrierAuthority<'tx, 'conn>,
    vulnerability: CurrentExternalPoolAdapterVulnerabilityReattestationAuthority,
    sandbox: CurrentExternalPoolAdapterSandboxReattestationAuthority,
    bundle: PreparedExternalPoolAdapterRuntimeBundle,
    checked_at: String,
    transaction: PhantomData<&'tx Transaction<'conn>>,
}

impl<'tx, 'conn> CurrentExternalPoolAdapterProjectedActiveRuntimeBundleAuthority<'tx, 'conn> {
    pub(in crate::store) fn carrier(
        &self,
    ) -> &CurrentExternalPoolAdapterRenewedRouteRuntimeCarrierAuthority<'tx, 'conn> {
        &self.carrier
    }

    pub(in crate::store) fn vulnerability(
        &self,
    ) -> &CurrentExternalPoolAdapterVulnerabilityReattestationAuthority {
        &self.vulnerability
    }

    pub(in crate::store) fn sandbox(
        &self,
    ) -> &CurrentExternalPoolAdapterSandboxReattestationAuthority {
        &self.sandbox
    }

    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }

    pub(super) fn roots(&self) -> ExternalPoolAdapterRuntimeBundleRoots<'_> {
        self.bundle.roots()
    }

    pub(super) fn revalidate(&self) -> Result<()> {
        self.bundle.revalidate()
    }

    pub(super) fn with_sensitive_bytes(
        &self,
        consume: impl FnOnce(&[u8], &[u8]) -> Result<()>,
    ) -> Result<()> {
        self.bundle.with_sensitive_bytes(consume)
    }

    pub(super) fn into_prepared_bundle(self) -> PreparedExternalPoolAdapterRuntimeBundle {
        self.bundle
    }
}

#[allow(clippy::too_many_arguments)]
pub(in crate::store) fn current_external_pool_adapter_projected_active_runtime_bundle_authority_on<
    'tx,
    'conn,
>(
    transaction: &'tx Transaction<'conn>,
    provider_binding_id: &str,
    expected_activation_receipt_id: &str,
    expected_activation_receipt_digest: &str,
    prepared: PreparedExternalPoolAdapterInstallation,
    bundle_root: &ExternalPoolAdapterRuntimeBundleRoot,
    checked_at: &str,
) -> Result<Option<CurrentExternalPoolAdapterProjectedActiveRuntimeBundleAuthority<'tx, 'conn>>> {
    let Some(carrier) = current_external_pool_adapter_renewed_route_runtime_carrier_for_binding_on(
        transaction,
        provider_binding_id,
        expected_activation_receipt_id,
        expected_activation_receipt_digest,
        prepared,
        checked_at,
    )?
    else {
        return Ok(None);
    };
    let root = &carrier
        .historical_activation()
        .activation_root()
        .activation_root;
    let profile_receipt = carrier.profile();
    let profile = &profile_receipt.profile;
    let credential = carrier.credential();
    let credential_receipt = credential.receipt();
    let binding = &credential_receipt.reattestation.binding;
    audit_active_bundle_roots(&carrier, checked_at)?;
    let locator_commitment = credential_subject_commitment(transaction, binding)?;
    let expected = ExpectedExternalPoolAdapterRuntimeBundle {
        profile_id: profile_receipt.profile_id.clone(),
        profile_digest: profile_receipt.profile_digest.clone(),
        launch_policy_digest: profile.launch_policy_digest.clone(),
        candidate_id: profile.candidate_id.clone(),
        candidate_digest: profile.candidate_digest.clone(),
        provider_binding_id: root.provider_binding_id.clone(),
        provider_binding_digest: root.provider_binding_digest.clone(),
        provider_id: carrier
            .historical_activation()
            .active_provider()
            .provider_id
            .clone(),
        provider_owner_account_id: root.provider_owner_account_id.clone(),
        logical_adapter_id: root.logical_adapter_id.clone(),
        release_version: binding.release_version.clone(),
        adapter_config_revision: binding.adapter_config_revision,
        adapter_config_digest: binding.adapter_config_digest.clone(),
        credential_locator_commitment: locator_commitment,
        credential_reattestation_receipt_id: credential_receipt.reattestation_receipt_id.clone(),
        credential_reattestation_receipt_digest: credential_receipt
            .reattestation_receipt_digest
            .clone(),
        credential_reattestation_material_digest: credential_receipt
            .reattestation_material_digest
            .clone(),
        credential_report_expires_at: binding.report_expires_at.clone(),
    };
    let bundle = resolve_external_pool_adapter_runtime_bundle(bundle_root, &expected)
        .map_err(|_| anyhow::anyhow!("projected-active bundle custody resolution failed"))?;
    let vulnerability =
        current_external_pool_adapter_vulnerability_reattestation_head_authority_on(
            transaction,
            &root.registry_release_id,
            checked_at,
        )?
        .ok_or_else(|| anyhow::anyhow!("projected-active bundle lacks current V250"))?;
    let sandbox = current_external_pool_adapter_sandbox_reattestation_head_authority_on(
        transaction,
        &root.registry_release_id,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("projected-active bundle lacks current V252"))?;
    if vulnerability.checked_at() != checked_at || sandbox.checked_at() != checked_at {
        bail!("projected-active bundle current roots use different time anchors");
    }
    Ok(Some(
        CurrentExternalPoolAdapterProjectedActiveRuntimeBundleAuthority {
            carrier,
            vulnerability,
            sandbox,
            bundle,
            checked_at: checked_at.into(),
            transaction: PhantomData,
        },
    ))
}

fn audit_active_bundle_roots(
    carrier: &CurrentExternalPoolAdapterRenewedRouteRuntimeCarrierAuthority<'_, '_>,
    checked_at: &str,
) -> Result<()> {
    let root = &carrier
        .historical_activation()
        .activation_root()
        .activation_root;
    let binding = &carrier.credential().receipt().reattestation.binding;
    let active = carrier.historical_activation().active_provider();
    if carrier.checked_at() != checked_at
        || carrier.renewed_route().checked_at() != checked_at
        || carrier.runtime_compatibility().checked_at() != checked_at
        || binding.provider_binding_id != root.provider_binding_id
        || binding.provider_binding_digest != root.provider_binding_digest
        || binding.provider_id != active.provider_id
        || binding.observed_provider_policy_revision != active.policy_revision
        || binding.observed_provider_status != active.status
        || binding.route_adapter_projection_id != root.route_adapter_projection_id
    {
        bail!("projected-active bundle roots diverged from its renewed-route carrier");
    }
    Ok(())
}

fn credential_subject_commitment(
    transaction: &rusqlite::Connection,
    binding: &crate::compute_federation::external_pool_adapter_credential_reattestation::ExternalPoolAdapterCredentialReattestationBinding,
) -> Result<String> {
    let onboarding = historical_external_pool_onboarding_application_authority_on(
        transaction,
        &binding.application_id,
        &binding.application_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("projected-active bundle lost credential subject"))?;
    let locator = onboarding.non_bearer_credential_ref();
    let scheme = credential_ref_scheme(locator)
        .map_err(|_| anyhow::anyhow!("projected-active credential subject is unsupported"))?;
    let commitment = credential_locator_commitment(locator);
    let suffix = locator.strip_prefix("vault-ref:").unwrap_or_default();
    if scheme != "vault_ref"
        || suffix.is_empty()
        || suffix.len() > 160
        || !suffix.as_bytes()[0].is_ascii_alphanumeric()
        || !suffix
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        || commitment != binding.credential_locator_commitment
    {
        bail!("projected-active credential locator is inexact");
    }
    Ok(commitment)
}

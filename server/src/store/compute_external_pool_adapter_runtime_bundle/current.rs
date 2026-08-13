use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{Transaction, TransactionBehavior};

use crate::{
    compute_federation::{
        external_pool_adapter_credential_verification::{
            credential_locator_commitment, credential_ref_scheme,
        },
        external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
    },
    store::{
        compute_external_pool_adapter_credential_reattestation::current_external_pool_adapter_credential_reattestation_head_authority_on,
        compute_external_pool_adapter_runtime_launch_profile::current_external_pool_adapter_runtime_launch_profile_authority_on,
        compute_external_pool_onboarding::historical_external_pool_onboarding_application_authority_on,
        Store,
    },
};

use super::{
    filesystem::resolve_external_pool_adapter_runtime_bundle,
    types::{
        CurrentExternalPoolAdapterRuntimeBundleAuthority, ExpectedExternalPoolAdapterRuntimeBundle,
        ExternalPoolAdapterRuntimeBundleRoot,
    },
};

impl Store {
    /// Owns the mutex connection, write-serializing transaction, fresh time anchor, and snapshot.
    pub(in crate::store) fn with_current_external_pool_adapter_runtime_bundle_authority(
        &self,
        profile_id: &str,
        prepared: PreparedExternalPoolAdapterInstallation,
        bundle_root: &ExternalPoolAdapterRuntimeBundleRoot,
        consume: impl FnOnce(
            &Transaction<'_>,
            &CurrentExternalPoolAdapterRuntimeBundleAuthority<'_, '_>,
        ) -> Result<()>,
    ) -> Result<bool> {
        let mut conn = self.conn()?;
        let transaction = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let checked_at = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
        let Some(authority) = current_external_pool_adapter_runtime_bundle_authority_on(
            &transaction,
            profile_id,
            prepared,
            bundle_root,
            &checked_at,
        )?
        else {
            return Ok(false);
        };
        consume(&transaction, &authority)?;
        drop(authority);
        transaction.commit()?;
        Ok(true)
    }
}

/// Composes current V255/V253 roots with one filesystem snapshot inside the owning transaction.
pub(super) fn current_external_pool_adapter_runtime_bundle_authority_on<'tx, 'conn>(
    transaction: &'tx Transaction<'conn>,
    profile_id: &str,
    prepared: PreparedExternalPoolAdapterInstallation,
    bundle_root: &ExternalPoolAdapterRuntimeBundleRoot,
    checked_at: &str,
) -> Result<Option<CurrentExternalPoolAdapterRuntimeBundleAuthority<'tx, 'conn>>> {
    let Some(launch_profile) = current_external_pool_adapter_runtime_launch_profile_authority_on(
        transaction,
        profile_id,
        prepared,
        checked_at,
    )?
    else {
        return Ok(None);
    };
    let profile_receipt = launch_profile.profile();
    let profile = &profile_receipt.profile;
    let credential = current_external_pool_adapter_credential_reattestation_head_authority_on(
        transaction,
        &profile.provider_binding_id,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("runtime bundle lacks a current credential authority"))?;
    let credential_receipt = credential.receipt();
    let credential_binding = &credential_receipt.reattestation.binding;
    audit_credential_roots(
        profile,
        credential_binding,
        checked_at,
        credential.checked_at(),
    )?;
    let locator_commitment = credential_subject_commitment(transaction, credential_binding)?;
    let expected = ExpectedExternalPoolAdapterRuntimeBundle {
        profile_id: profile_receipt.profile_id.clone(),
        profile_digest: profile_receipt.profile_digest.clone(),
        launch_policy_digest: profile.launch_policy_digest.clone(),
        candidate_id: profile.candidate_id.clone(),
        candidate_digest: profile.candidate_digest.clone(),
        provider_binding_id: profile.provider_binding_id.clone(),
        provider_binding_digest: profile.provider_binding_digest.clone(),
        provider_id: profile.provider_id.clone(),
        provider_owner_account_id: profile.provider_owner_account_id.clone(),
        logical_adapter_id: profile.logical_adapter_id.clone(),
        release_version: profile.release_version.clone(),
        adapter_config_revision: profile.adapter_config_revision,
        adapter_config_digest: profile.adapter_config_digest.clone(),
        credential_locator_commitment: locator_commitment,
        credential_reattestation_receipt_id: credential_receipt.reattestation_receipt_id.clone(),
        credential_reattestation_receipt_digest: credential_receipt
            .reattestation_receipt_digest
            .clone(),
        credential_reattestation_material_digest: credential_receipt
            .reattestation_material_digest
            .clone(),
        credential_report_expires_at: credential_binding.report_expires_at.clone(),
    };
    let bundle = resolve_external_pool_adapter_runtime_bundle(bundle_root, &expected)
        .map_err(|_| anyhow::anyhow!("runtime bundle custody resolution failed"))?;
    Ok(Some(CurrentExternalPoolAdapterRuntimeBundleAuthority::new(
        transaction,
        launch_profile,
        credential,
        bundle,
        checked_at.to_string(),
    )))
}

fn credential_subject_commitment(
    conn: &rusqlite::Connection,
    binding: &crate::compute_federation::external_pool_adapter_credential_reattestation::ExternalPoolAdapterCredentialReattestationBinding,
) -> Result<String> {
    let onboarding = historical_external_pool_onboarding_application_authority_on(
        conn,
        &binding.application_id,
        &binding.application_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("runtime bundle lost credential subject authority"))?;
    let locator = onboarding.non_bearer_credential_ref();
    let scheme = credential_ref_scheme(locator)
        .map_err(|_| anyhow::anyhow!("runtime bundle credential subject is unsupported"))?;
    let commitment = credential_locator_commitment(locator);
    let suffix = locator.strip_prefix("vault-ref:").unwrap_or_default();
    if scheme != "vault_ref"
        || suffix.is_empty()
        || suffix.len() > 160
        || suffix == "."
        || suffix == ".."
        || !suffix.as_bytes()[0].is_ascii_alphanumeric()
        || !suffix
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        || commitment != binding.credential_locator_commitment
    {
        bail!("runtime bundle credential subject is unsupported or inexact");
    }
    Ok(commitment)
}

fn audit_credential_roots(
    profile: &crate::compute_federation::external_pool_adapter_runtime_launch_profile::ExternalPoolAdapterRuntimeLaunchProfileMaterial,
    credential: &crate::compute_federation::external_pool_adapter_credential_reattestation::ExternalPoolAdapterCredentialReattestationBinding,
    checked_at: &str,
    credential_checked_at: &str,
) -> Result<()> {
    if credential.provider_binding_id != profile.provider_binding_id
        || credential.provider_binding_digest != profile.provider_binding_digest
        || credential.registry_release_id != profile.registry_release_id
        || credential.registry_release_digest != profile.registry_release_digest
        || credential.installation_receipt_id != profile.installation_receipt_id
        || credential.installation_receipt_digest != profile.installation_receipt_digest
        || credential.route_adapter_projection_id != profile.route_adapter_projection_id
        || credential.provider_id != profile.provider_id
        || credential.provider_owner_account_id != profile.provider_owner_account_id
        || credential.observed_provider_policy_revision != profile.provider_policy_revision
        || credential.observed_provider_digest != profile.provider_digest
        || credential.observed_provider_status != profile.provider_status
        || credential.adapter_id != profile.logical_adapter_id
        || credential.release_version != profile.release_version
        || credential.adapter_config_revision != profile.adapter_config_revision
        || credential.adapter_config_digest != profile.adapter_config_digest
        || credential.credential_ref_scheme != "vault_ref"
        || credential.credential_locator_commitment != profile.credential_locator_commitment
        || credential.credential_verifier_digest != profile.credential_verifier_digest
        || credential_checked_at != checked_at
    {
        bail!("runtime bundle current authority roots drifted");
    }
    Ok(())
}

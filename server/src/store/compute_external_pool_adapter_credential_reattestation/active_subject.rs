use anyhow::{bail, Result};
use rusqlite::Connection;

use crate::{
    compute_federation::{
        external_pool_adapter_credential_reattestation::ExternalPoolAdapterCredentialReattestationBinding,
        external_pool_adapter_registry::ExternalPoolAdapterRegistryProviderBindingReceipt,
        provider::{PROVIDER_KIND_EXTERNAL_POOL, PROVIDER_STATUS_ACTIVE},
    },
    store::compute_external_pool_adapter_provider_active_successor::{
        historical_external_pool_adapter_atomic_activation_for_binding_on,
        historical_external_pool_adapter_atomic_activation_for_observed_provider_on,
        HistoricalExternalPoolAdapterAtomicActivationAuthority,
    },
};

use super::{
    current::currentness_on, read::receipt_by_id_on,
    types::CurrentExternalPoolAdapterCredentialReattestationAuthority,
};

pub(super) fn current_projected_active_registry_subject_on(
    conn: &Connection,
    provider_binding: &ExternalPoolAdapterRegistryProviderBindingReceipt,
    provider: &crate::store::compute_provider_registry::ComputeProviderRegistrationReceipt,
    checked_at: &str,
) -> Result<Option<HistoricalExternalPoolAdapterAtomicActivationAuthority>> {
    let binding = &provider_binding.binding;
    let Some(activation) = historical_external_pool_adapter_atomic_activation_for_binding_on(
        conn,
        &provider_binding.provider_binding_id,
        checked_at,
    )?
    else {
        return Ok(None);
    };
    let root = &activation.activation_root().activation_root;
    let active = activation.active_provider();
    let adapter = active.adapter.as_ref();
    if provider_binding.provider_binding_digest != root.provider_binding_digest
        || binding.provider_id != root.provider_id
        || binding.provider_owner_account_id != root.provider_owner_account_id
        || binding.adapter_id != root.logical_adapter_id
        || binding.route_adapter_projection_id != root.route_adapter_projection_id
        || &provider.provider != active
        || adapter.map(|item| item.adapter_id.as_str())
            != Some(root.route_adapter_projection_id.as_str())
        || adapter.map(|item| item.adapter_version.as_str())
            != Some(binding.release_version.as_str())
        || adapter.map(|item| item.config_revision) != Some(binding.adapter_config_revision)
        || adapter.map(|item| item.config_digest.as_str())
            != Some(binding.adapter_config_digest.as_str())
    {
        bail!("live Provider is not the exact V277 projected-active subject");
    }
    Ok(Some(activation))
}

pub(super) fn current_projected_active_subject_on(
    conn: &Connection,
    binding: &ExternalPoolAdapterCredentialReattestationBinding,
    checked_at: &str,
) -> Result<Option<HistoricalExternalPoolAdapterAtomicActivationAuthority>> {
    let Some(activation) =
        historical_external_pool_adapter_atomic_activation_for_observed_provider_on(
            conn,
            &binding.provider_binding_id,
            binding.observed_provider_policy_revision,
            &binding.observed_provider_digest,
        )?
    else {
        return Ok(None);
    };
    validate_projected_active_binding(binding, &activation)?;
    Ok(Some(activation))
}

pub(super) fn historical_projected_active_subject_on(
    conn: &Connection,
    binding: &ExternalPoolAdapterCredentialReattestationBinding,
) -> Result<Option<HistoricalExternalPoolAdapterAtomicActivationAuthority>> {
    let Some(activation) =
        historical_external_pool_adapter_atomic_activation_for_observed_provider_on(
            conn,
            &binding.provider_binding_id,
            binding.observed_provider_policy_revision,
            &binding.observed_provider_digest,
        )?
    else {
        return Ok(None);
    };
    validate_projected_active_binding(binding, &activation)?;
    Ok(Some(activation))
}

pub(in crate::store) fn current_external_pool_adapter_projected_active_credential_reattestation_authority_on(
    conn: &Connection,
    provider_binding_id: &str,
    expected_receipt_id: &str,
    expected_receipt_digest: &str,
    checked_at: &str,
) -> Result<Option<CurrentExternalPoolAdapterCredentialReattestationAuthority>> {
    let Some(currentness) = currentness_on(conn, provider_binding_id, checked_at)? else {
        return Ok(None);
    };
    let Some(stored) = receipt_by_id_on(conn, expected_receipt_id)? else {
        return Ok(None);
    };
    let receipt = &stored.receipt;
    let binding = &receipt.reattestation.binding;
    if currentness.current_status != "verified_current"
        || currentness.provider_revision_status != "exact_projected_active"
        || currentness.reattestation.reattestation_receipt_id != expected_receipt_id
        || currentness.reattestation.reattestation_receipt_digest != expected_receipt_digest
        || receipt.reattestation_receipt_digest != expected_receipt_digest
        || binding.provider_binding_id != provider_binding_id
        || binding.observed_provider_status != PROVIDER_STATUS_ACTIVE
        || current_projected_active_subject_on(conn, binding, checked_at)?.is_none()
    {
        bail!("projected-active credential re-attestation is not current and exact");
    }
    Ok(Some(
        CurrentExternalPoolAdapterCredentialReattestationAuthority::new(
            stored.receipt,
            checked_at.into(),
        ),
    ))
}

fn validate_projected_active_binding(
    binding: &ExternalPoolAdapterCredentialReattestationBinding,
    activation: &HistoricalExternalPoolAdapterAtomicActivationAuthority,
) -> Result<()> {
    let receipt = activation.receipt();
    let identity = &receipt.activation.identity;
    let root = &activation.activation_root().activation_root;
    let provider = activation.active_provider();
    let adapter = provider.adapter.as_ref();
    let provider_json = serde_json::to_string(provider)?;
    let provider_digest = sha256_hex(provider_json.as_bytes());
    if binding.observed_provider_status != PROVIDER_STATUS_ACTIVE
        || binding.provider_binding_id != identity.provider_binding_id
        || binding.provider_binding_digest != identity.provider_binding_digest
        || identity.activation_root_digest != activation.activation_root().activation_root_digest
        || binding.provider_id != provider.provider_id
        || binding.observed_provider_policy_revision != provider.policy_revision
        || binding.observed_provider_digest != provider_digest
        || binding.provider_owner_account_id != root.provider_owner_account_id
        || binding.adapter_id != root.logical_adapter_id
        || binding.route_adapter_projection_id != root.route_adapter_projection_id
        || provider.provider_kind != PROVIDER_KIND_EXTERNAL_POOL
        || provider.status != PROVIDER_STATUS_ACTIVE
        || adapter.map(|item| item.adapter_id.as_str())
            != Some(root.route_adapter_projection_id.as_str())
        || adapter.map(|item| item.adapter_version.as_str())
            != Some(binding.release_version.as_str())
        || adapter.map(|item| item.config_revision) != Some(binding.adapter_config_revision)
        || adapter.map(|item| item.config_digest.as_str())
            != Some(binding.adapter_config_digest.as_str())
        || activation.route_closure().route_adapter_projection_id
            != root.route_adapter_projection_id
    {
        bail!("credential re-attestation is not rooted in the exact projected-active subject");
    }
    Ok(())
}

fn sha256_hex(value: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(value))
}

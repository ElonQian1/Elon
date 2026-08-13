use anyhow::{bail, Result};

use crate::{
    compute_federation::{
        external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
        external_pool_provider_activation_candidate::ACTIVATION_DELEGATION_REVOCATION_CONFIRMATION,
    },
    store::{
        compute_external_pool_adapter_installation::external_pool_adapter_installation_receipt_authority_on,
        compute_external_pool_adapter_registry::{
            historical_external_pool_adapter_registry_provider_binding_authority_on,
            historical_external_pool_adapter_registry_release_authority_on,
        },
    },
};

use super::types::*;

pub(super) fn ensure_create_replay(
    conn: &rusqlite::Connection,
    prepared: &PreparedExternalPoolAdapterInstallation,
    delegation: &StoredDelegation,
    candidate: &StoredCandidate,
    binding_id: &str,
    binding_digest: &str,
    release_digest: &str,
    owner: &str,
    scope: &str,
    key: &str,
    confirmation: &str,
) -> Result<()> {
    let d = &delegation.receipt.delegation;
    let c = &candidate.receipt.candidate;
    if d.provider_binding_id != binding_id
        || d.provider_binding_digest != binding_digest
        || d.registry_release_digest != release_digest
        || d.issued_by_owner_user_id != owner
        || d.idempotency_scope != scope
        || d.idempotency_key != key
        || d.confirmation != confirmation
        || c.delegation_id != delegation.receipt.delegation_id
        || c.delegation_digest != delegation.receipt.delegation_digest
    {
        bail!("activation candidate idempotency replay conflicts with sealed input");
    }
    let binding = historical_external_pool_adapter_registry_provider_binding_authority_on(
        conn,
        binding_id,
        binding_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("activation replay lost V249 binding history"))?;
    let release = historical_external_pool_adapter_registry_release_authority_on(
        conn,
        &d.registry_release_id,
        release_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("activation replay lost V249 release history"))?;
    let installation = external_pool_adapter_installation_receipt_authority_on(
        conn,
        &c.installation_receipt_id,
        &c.installation_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("activation replay lost installation history"))?;
    let b = &binding.binding().binding;
    let r = &release.release().release;
    let installed = &installation.receipt().installation.binding;
    if prepared.binding() != installed
        || prepared.installation_content_digest() != c.installation_content_digest
        || b.registry_release_id != d.registry_release_id
        || b.registry_release_digest != d.registry_release_digest
        || b.route_adapter_projection_id != d.route_adapter_projection_id
        || b.provider_id != d.provider_id
        || b.provider_owner_account_id != d.provider_owner_account_id
        || b.provider_policy_revision != d.provider_policy_revision
        || b.provider_digest != d.provider_digest
        || b.adapter_id != d.logical_adapter_id
        || b.release_version != d.release_version
        || b.adapter_config_revision != d.adapter_config_revision
        || b.adapter_config_digest != d.adapter_config_digest
        || c.provider_binding_id != binding_id
        || c.provider_binding_digest != binding_digest
        || c.registry_release_id != release.release().registry_release_id
        || c.registry_release_digest != release.release().registry_release_digest
        || c.installation_receipt_id != installation.receipt().installation_receipt_id
        || c.installation_receipt_digest != installation.receipt().installation_receipt_digest
        || c.installation_content_digest != installed.installation_content_digest
        || c.route_adapter_projection_id != b.route_adapter_projection_id
        || c.provider_id != b.provider_id
        || c.provider_owner_account_id != b.provider_owner_account_id
        || c.provider_policy_revision != b.provider_policy_revision
        || c.provider_digest != b.provider_digest
        || c.logical_adapter_id != b.adapter_id
        || c.release_version != b.release_version
        || c.adapter_config_revision != b.adapter_config_revision
        || c.adapter_config_digest != b.adapter_config_digest
        || c.implementation_digest != r.implementation_digest
        || c.capability_set_digest != r.capability_set_digest
        || c.credential_verifier_digest != r.credential_verifier_digest
        || c.service_actor_id != d.service_actor_id
    {
        bail!("activation candidate replay static history or Prepared files are not exact");
    }
    Ok(())
}

pub(super) fn ensure_revocation_target(
    input: &RevokeExternalPoolProviderActivationDelegation,
    delegation: &StoredDelegation,
    candidate: &StoredCandidate,
) -> Result<()> {
    let d = &delegation.receipt.delegation;
    if delegation.receipt.delegation_digest != input.expected_delegation_digest
        || candidate.receipt.candidate_digest != input.expected_candidate_digest
        || candidate.receipt.candidate.delegation_id != input.delegation_id
        || d.provider_owner_account_id != input.revoked_by_owner_user_id
        || input.confirmation != ACTIVATION_DELEGATION_REVOCATION_CONFIRMATION
    {
        bail!("activation revocation target or owner confirmation is not exact");
    }
    Ok(())
}

pub(super) fn ensure_revocation_replay(
    input: &RevokeExternalPoolProviderActivationDelegation,
    delegation: &StoredDelegation,
    candidate: &StoredCandidate,
    revocation: &StoredRevocation,
) -> Result<()> {
    ensure_revocation_target(input, delegation, candidate)?;
    let r = &revocation.receipt.revocation;
    if r.reason != input.reason
        || r.idempotency_scope != input.idempotency_scope
        || r.idempotency_key != input.idempotency_key
        || r.confirmation != input.confirmation
        || r.revoked_by_owner_user_id != input.revoked_by_owner_user_id
    {
        bail!("activation revocation idempotency replay conflicts with sealed input");
    }
    Ok(())
}

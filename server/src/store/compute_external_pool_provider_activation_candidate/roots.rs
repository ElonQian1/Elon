use anyhow::{bail, Result};

use crate::{
    compute_federation::{
        attempt_gateway::{
            canonical_adapter_binding_json_and_digest, ComputeAttemptAdapterBinding,
            COMPUTE_ATTEMPT_ADAPTER_BINDING_SCHEMA, COMPUTE_ATTEMPT_ROUTE_SERVER_ADAPTER,
        },
        external_pool_provider_activation_candidate::{
            external_pool_activation_service_actor_id, logical_projection_compatibility_digest,
            ExternalPoolProviderActivationCandidateReceipt,
            ExternalPoolProviderActivationDelegationReceipt, ACTIVATION_CANDIDATE_STATUS,
            ACTIVATION_CLOSURE_NOT_IMPLEMENTED, ACTIVATION_SERVICE_ACTOR_KIND,
        },
        provider::{PROVIDER_KIND_EXTERNAL_POOL, PROVIDER_STATUS_REGISTERING},
    },
    store::compute_external_pool_adapter_registry::CurrentExternalPoolAdapterRegistryProviderBindingAuthority,
};

pub(super) fn audit_static_roots(
    authority: &CurrentExternalPoolAdapterRegistryProviderBindingAuthority,
    delegation: &ExternalPoolProviderActivationDelegationReceipt,
    candidate: &ExternalPoolProviderActivationCandidateReceipt,
) -> Result<()> {
    let release = authority.release();
    let release_item = &release.release;
    let binding = authority.binding();
    let b = &binding.binding;
    let prepared = authority.prepared().binding();
    let d = &delegation.delegation;
    let c = &candidate.candidate;
    audit_delegation_derived_identity(delegation)?;
    audit_candidate_derived_identity(candidate)?;
    if delegation.delegation_id != c.delegation_id
        || delegation.delegation_digest != c.delegation_digest
        || d.provider_binding_id != binding.provider_binding_id
        || d.provider_binding_digest != binding.provider_binding_digest
        || d.registry_release_id != release.registry_release_id
        || d.registry_release_digest != release.registry_release_digest
        || d.route_adapter_projection_id != b.route_adapter_projection_id
        || d.provider_id != b.provider_id
        || d.provider_owner_account_id != b.provider_owner_account_id
        || d.provider_policy_revision != b.provider_policy_revision
        || d.provider_digest != b.provider_digest
        || d.provider_status != PROVIDER_STATUS_REGISTERING
        || d.logical_adapter_id != b.adapter_id
        || d.release_version != b.release_version
        || d.adapter_config_revision != b.adapter_config_revision
        || d.adapter_config_digest != b.adapter_config_digest
        || d.service_actor_id != c.service_actor_id
        || d.service_actor_kind != ACTIVATION_SERVICE_ACTOR_KIND
        || d.sequence != c.sequence
        || d.issued_by_owner_user_id != b.provider_owner_account_id
        || c.provider_binding_id != binding.provider_binding_id
        || c.provider_binding_digest != binding.provider_binding_digest
        || c.registry_release_id != release.registry_release_id
        || c.registry_release_digest != release.registry_release_digest
        || c.installation_receipt_id != b.installation_receipt_id
        || c.installation_receipt_digest != b.installation_receipt_digest
        || c.installation_content_digest != b.installation_content_digest
        || c.route_adapter_projection_id != b.route_adapter_projection_id
        || c.provider_id != b.provider_id
        || c.provider_owner_account_id != b.provider_owner_account_id
        || c.provider_policy_revision != b.provider_policy_revision
        || c.provider_digest != b.provider_digest
        || c.provider_status != PROVIDER_STATUS_REGISTERING
        || c.logical_adapter_id != b.adapter_id
        || c.release_version != b.release_version
        || c.adapter_config_revision != b.adapter_config_revision
        || c.adapter_config_digest != b.adapter_config_digest
        || c.implementation_digest != release_item.implementation_digest
        || c.capability_set_digest != release_item.capability_set_digest
        || c.credential_verifier_digest != release_item.credential_verifier_digest
        || c.candidate_status != ACTIVATION_CANDIDATE_STATUS
        || c.activation_closure_status != ACTIVATION_CLOSURE_NOT_IMPLEMENTED
        || authority.checked_at() < c.checked_at.as_str()
        || prepared.provider_id != b.provider_id
        || prepared.provider_owner_account_id != b.provider_owner_account_id
        || prepared.provider_policy_revision != b.provider_policy_revision
        || prepared.provider_digest != b.provider_digest
        || prepared.adapter_id != b.adapter_id
        || prepared.adapter_release_version != b.release_version
        || prepared.adapter_config_revision != b.adapter_config_revision
        || prepared.adapter_config_digest != b.adapter_config_digest
        || prepared.installation_content_digest != b.installation_content_digest
    {
        bail!("activation candidate static V249 roots drifted");
    }
    Ok(())
}

pub(super) fn audit_delegation_derived_identity(
    receipt: &ExternalPoolProviderActivationDelegationReceipt,
) -> Result<()> {
    let d = &receipt.delegation;
    let expected = external_pool_activation_service_actor_id(
        &d.provider_id,
        &d.provider_binding_id,
        &d.provider_binding_digest,
        &d.route_adapter_projection_id,
    )?;
    if d.service_actor_id != expected {
        bail!("activation delegation service actor identity is not deterministic and exact");
    }
    Ok(())
}

pub(super) fn audit_candidate_derived_identity(
    receipt: &ExternalPoolProviderActivationCandidateReceipt,
) -> Result<()> {
    let c = &receipt.candidate;
    let expected_actor = external_pool_activation_service_actor_id(
        &c.provider_id,
        &c.provider_binding_id,
        &c.provider_binding_digest,
        &c.route_adapter_projection_id,
    )?;
    let logical = ComputeAttemptAdapterBinding {
        schema: COMPUTE_ATTEMPT_ADAPTER_BINDING_SCHEMA.into(),
        provider_id: c.provider_id.clone(),
        provider_kind: PROVIDER_KIND_EXTERNAL_POOL.into(),
        route_kind: COMPUTE_ATTEMPT_ROUTE_SERVER_ADAPTER.into(),
        endpoint_id: None,
        endpoint_transport: None,
        adapter_id: c.logical_adapter_id.clone(),
        adapter_version: c.release_version.clone(),
        config_revision: c.adapter_config_revision,
        config_digest: c.adapter_config_digest.clone(),
    };
    let logical_digest = canonical_adapter_binding_json_and_digest(&logical)?.1;
    let compatibility = logical_projection_compatibility_digest(
        &c.provider_binding_id,
        &c.provider_binding_digest,
        &c.registry_release_id,
        &c.registry_release_digest,
        &logical_digest,
        &c.route_adapter_projection_id,
    )?;
    if c.service_actor_id != expected_actor
        || c.logical_adapter_binding_digest != logical_digest
        || c.logical_projection_compatibility_digest != compatibility
    {
        bail!("activation candidate derived logical/projection identities are not exact");
    }
    Ok(())
}

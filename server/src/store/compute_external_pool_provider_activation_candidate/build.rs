use anyhow::Result;

use crate::{
    compute_federation::{
        attempt_gateway::{
            canonical_adapter_binding_json_and_digest, ComputeAttemptAdapterBinding,
            COMPUTE_ATTEMPT_ADAPTER_BINDING_SCHEMA, COMPUTE_ATTEMPT_ROUTE_SERVER_ADAPTER,
        },
        external_pool_provider_activation_candidate::*,
        provider::{PROVIDER_KIND_EXTERNAL_POOL, PROVIDER_STATUS_REGISTERING},
    },
    store::{
        compute_external_pool_adapter_registry::CurrentExternalPoolAdapterRegistryProviderBindingAuthority,
        new_id,
    },
};

use super::types::{
    RevokeExternalPoolProviderActivationDelegation, StoredCandidate, StoredDelegation,
};

pub(super) fn build_delegation(
    authority: &CurrentExternalPoolAdapterRegistryProviderBindingAuthority,
    predecessor: Option<&StoredDelegation>,
    sequence: u64,
    owner: &str,
    scope: &str,
    key: &str,
    confirmation: &str,
    now: &str,
) -> Result<ExternalPoolProviderActivationDelegationReceipt> {
    let binding = authority.binding();
    let b = &binding.binding;
    let actor_id = external_pool_activation_service_actor_id(
        &b.provider_id,
        &binding.provider_binding_id,
        &binding.provider_binding_digest,
        &b.route_adapter_projection_id,
    )?;
    let material = ExternalPoolProviderActivationDelegationMaterial {
        provider_binding_id: binding.provider_binding_id.clone(),
        provider_binding_digest: binding.provider_binding_digest.clone(),
        registry_release_id: b.registry_release_id.clone(),
        registry_release_digest: b.registry_release_digest.clone(),
        route_adapter_projection_id: b.route_adapter_projection_id.clone(),
        provider_id: b.provider_id.clone(),
        provider_owner_account_id: b.provider_owner_account_id.clone(),
        provider_policy_revision: b.provider_policy_revision,
        provider_digest: b.provider_digest.clone(),
        provider_status: PROVIDER_STATUS_REGISTERING.into(),
        logical_adapter_id: b.adapter_id.clone(),
        release_version: b.release_version.clone(),
        adapter_config_revision: b.adapter_config_revision,
        adapter_config_digest: b.adapter_config_digest.clone(),
        service_actor_id: actor_id,
        service_actor_kind: ACTIVATION_SERVICE_ACTOR_KIND.into(),
        allowed_route_kinds: vec![COMPUTE_ATTEMPT_ROUTE_SERVER_ADAPTER.into()],
        allowed_actor_phases: vec!["application".into(), "dispatch".into()],
        issued_by_owner_user_id: owner.into(),
        issued_at: now.into(),
        recorded_at: now.into(),
        sequence,
        predecessor_delegation_id: predecessor.map(|x| x.receipt.delegation_id.clone()),
        predecessor_delegation_digest: predecessor.map(|x| x.receipt.delegation_digest.clone()),
        idempotency_scope: scope.into(),
        idempotency_key: key.into(),
        confirmation: confirmation.into(),
        delegation_effect: ACTIVATION_DELEGATION_EFFECT.into(),
        provider_effect: ACTIVATION_NO_EFFECT.into(),
        credential_effect: ACTIVATION_NO_EFFECT.into(),
        route_effect: ACTIVATION_ROUTE_CANDIDATE_ONLY.into(),
        execution_effect: ACTIVATION_NO_EFFECT.into(),
        market_effect: ACTIVATION_NO_EFFECT.into(),
        settlement_effect: ACTIVATION_NO_EFFECT.into(),
    };
    seal_delegation(material)
}

pub(super) fn build_candidate(
    authority: &CurrentExternalPoolAdapterRegistryProviderBindingAuthority,
    delegation: &ExternalPoolProviderActivationDelegationReceipt,
    predecessor: Option<&StoredCandidate>,
    sequence: u64,
    now: &str,
) -> Result<ExternalPoolProviderActivationCandidateReceipt> {
    let release = authority.release();
    let r = &release.release;
    let binding = authority.binding();
    let b = &binding.binding;
    let logical = ComputeAttemptAdapterBinding {
        schema: COMPUTE_ATTEMPT_ADAPTER_BINDING_SCHEMA.into(),
        provider_id: b.provider_id.clone(),
        provider_kind: PROVIDER_KIND_EXTERNAL_POOL.into(),
        route_kind: COMPUTE_ATTEMPT_ROUTE_SERVER_ADAPTER.into(),
        endpoint_id: None,
        endpoint_transport: None,
        adapter_id: b.adapter_id.clone(),
        adapter_version: b.release_version.clone(),
        config_revision: b.adapter_config_revision,
        config_digest: b.adapter_config_digest.clone(),
    };
    let logical_digest = canonical_adapter_binding_json_and_digest(&logical)?.1;
    let compatibility = logical_projection_compatibility_digest(
        &binding.provider_binding_id,
        &binding.provider_binding_digest,
        &release.registry_release_id,
        &release.registry_release_digest,
        &logical_digest,
        &b.route_adapter_projection_id,
    )?;
    let material = ExternalPoolProviderActivationCandidateMaterial {
        delegation_id: delegation.delegation_id.clone(),
        delegation_digest: delegation.delegation_digest.clone(),
        provider_binding_id: binding.provider_binding_id.clone(),
        provider_binding_digest: binding.provider_binding_digest.clone(),
        registry_release_id: release.registry_release_id.clone(),
        registry_release_digest: release.registry_release_digest.clone(),
        installation_receipt_id: b.installation_receipt_id.clone(),
        installation_receipt_digest: b.installation_receipt_digest.clone(),
        installation_content_digest: b.installation_content_digest.clone(),
        route_adapter_projection_id: b.route_adapter_projection_id.clone(),
        provider_id: b.provider_id.clone(),
        provider_owner_account_id: b.provider_owner_account_id.clone(),
        provider_policy_revision: b.provider_policy_revision,
        provider_digest: b.provider_digest.clone(),
        provider_status: PROVIDER_STATUS_REGISTERING.into(),
        logical_adapter_id: b.adapter_id.clone(),
        release_version: b.release_version.clone(),
        adapter_config_revision: b.adapter_config_revision,
        adapter_config_digest: b.adapter_config_digest.clone(),
        implementation_digest: r.implementation_digest.clone(),
        capability_set_digest: r.capability_set_digest.clone(),
        credential_verifier_digest: r.credential_verifier_digest.clone(),
        logical_adapter_binding_digest: logical_digest,
        logical_projection_compatibility_digest: compatibility,
        service_actor_id: delegation.delegation.service_actor_id.clone(),
        sequence,
        predecessor_candidate_id: predecessor.map(|x| x.receipt.candidate_id.clone()),
        predecessor_candidate_digest: predecessor.map(|x| x.receipt.candidate_digest.clone()),
        checked_at: now.into(),
        recorded_at: now.into(),
        candidate_status: ACTIVATION_CANDIDATE_STATUS.into(),
        activation_closure_status: ACTIVATION_CLOSURE_NOT_IMPLEMENTED.into(),
        candidate_effect: ACTIVATION_CANDIDATE_EFFECT.into(),
        provider_effect: ACTIVATION_NO_EFFECT.into(),
        credential_effect: ACTIVATION_NO_EFFECT.into(),
        route_effect: ACTIVATION_ROUTE_CANDIDATE_ONLY.into(),
        execution_effect: ACTIVATION_NO_EFFECT.into(),
        market_effect: ACTIVATION_NO_EFFECT.into(),
        settlement_effect: ACTIVATION_NO_EFFECT.into(),
    };
    seal_candidate(material)
}

pub(super) fn build_revocation(
    input: &RevokeExternalPoolProviderActivationDelegation,
    delegation: &StoredDelegation,
    candidate: &StoredCandidate,
    now: &str,
) -> Result<ExternalPoolProviderActivationDelegationRevocationReceipt> {
    let d = &delegation.receipt.delegation;
    let material = ExternalPoolProviderActivationDelegationRevocationMaterial {
        delegation_id: delegation.receipt.delegation_id.clone(),
        delegation_digest: delegation.receipt.delegation_digest.clone(),
        candidate_id: candidate.receipt.candidate_id.clone(),
        candidate_digest: candidate.receipt.candidate_digest.clone(),
        provider_binding_id: d.provider_binding_id.clone(),
        provider_binding_digest: d.provider_binding_digest.clone(),
        provider_id: d.provider_id.clone(),
        revoked_by_owner_user_id: input.revoked_by_owner_user_id.clone(),
        reason: input.reason.clone(),
        revoked_at: now.into(),
        recorded_at: now.into(),
        idempotency_scope: input.idempotency_scope.clone(),
        idempotency_key: input.idempotency_key.clone(),
        confirmation: input.confirmation.clone(),
        revocation_effect: ACTIVATION_DELEGATION_REVOCATION_EFFECT.into(),
        provider_effect: ACTIVATION_NO_EFFECT.into(),
        credential_effect: ACTIVATION_NO_EFFECT.into(),
        route_effect: ACTIVATION_NO_EFFECT.into(),
        execution_effect: ACTIVATION_NO_EFFECT.into(),
        market_effect: ACTIVATION_NO_EFFECT.into(),
        settlement_effect: ACTIVATION_NO_EFFECT.into(),
    };
    seal_revocation(material)
}

fn seal_delegation(
    material: ExternalPoolProviderActivationDelegationMaterial,
) -> Result<ExternalPoolProviderActivationDelegationReceipt> {
    let mut r = ExternalPoolProviderActivationDelegationReceipt {
        schema: ACTIVATION_DELEGATION_SCHEMA.into(),
        delegation_id: new_id("external_pool_provider_activation_delegation"),
        delegation_digest: String::new(),
        delegation_material_digest: activation_delegation_material_digest(&material)?,
        canonicalization: ACTIVATION_CANONICALIZATION.into(),
        digest_algorithm: ACTIVATION_DIGEST_ALGORITHM.into(),
        delegation: material,
    };
    r.delegation_digest = canonical_activation_delegation_json_and_digest(&r)?.1;
    validate_activation_delegation_receipt(&r)?;
    Ok(r)
}
fn seal_candidate(
    material: ExternalPoolProviderActivationCandidateMaterial,
) -> Result<ExternalPoolProviderActivationCandidateReceipt> {
    let mut r = ExternalPoolProviderActivationCandidateReceipt {
        schema: ACTIVATION_CANDIDATE_SCHEMA.into(),
        candidate_id: new_id("external_pool_provider_activation_candidate"),
        candidate_digest: String::new(),
        candidate_material_digest: activation_candidate_material_digest(&material)?,
        canonicalization: ACTIVATION_CANONICALIZATION.into(),
        digest_algorithm: ACTIVATION_DIGEST_ALGORITHM.into(),
        candidate: material,
    };
    r.candidate_digest = canonical_activation_candidate_json_and_digest(&r)?.1;
    validate_activation_candidate_receipt(&r)?;
    Ok(r)
}
fn seal_revocation(
    material: ExternalPoolProviderActivationDelegationRevocationMaterial,
) -> Result<ExternalPoolProviderActivationDelegationRevocationReceipt> {
    let mut r = ExternalPoolProviderActivationDelegationRevocationReceipt {
        schema: ACTIVATION_DELEGATION_REVOCATION_SCHEMA.into(),
        revocation_id: new_id("external_pool_provider_activation_delegation_revocation"),
        revocation_digest: String::new(),
        revocation_material_digest: activation_delegation_revocation_material_digest(&material)?,
        canonicalization: ACTIVATION_CANONICALIZATION.into(),
        digest_algorithm: ACTIVATION_DIGEST_ALGORITHM.into(),
        revocation: material,
    };
    r.revocation_digest = canonical_activation_delegation_revocation_json_and_digest(&r)?.1;
    validate_activation_delegation_revocation_receipt(&r)?;
    Ok(r)
}

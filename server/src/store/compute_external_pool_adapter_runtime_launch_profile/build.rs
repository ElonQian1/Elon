use anyhow::Result;

use crate::{
    compute_federation::{
        external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
        external_pool_adapter_runtime_launch_profile::*,
    },
    store::{
        compute_external_pool_provider_activation_candidate::CurrentExternalPoolProviderActivationCandidateStaticAuthority,
        new_id,
    },
};

use super::{
    policy::runtime_launch_policy_catalog,
    types::{RevokeExternalPoolAdapterRuntimeLaunchProfile, StoredRuntimeLaunchProfile},
};

pub(super) fn build_profile(
    authority: &CurrentExternalPoolProviderActivationCandidateStaticAuthority,
    predecessor: Option<&StoredRuntimeLaunchProfile>,
    credential_ref_scheme: &str,
    credential_locator_commitment: &str,
    sequence: u64,
    now: &str,
    recorded_by_actor_kind: &str,
    recorded_by_actor_user_id: &str,
    idempotency_scope: &str,
    idempotency_key: &str,
    confirmation: &str,
) -> Result<ExternalPoolAdapterRuntimeLaunchProfileReceipt> {
    let policy = runtime_launch_policy_catalog()?;
    let candidate_receipt = authority.candidate();
    let candidate = &candidate_receipt.candidate;
    let binding = authority.registry().prepared().binding();
    let material = ExternalPoolAdapterRuntimeLaunchProfileMaterial {
        candidate_id: candidate_receipt.candidate_id.clone(),
        candidate_digest: candidate_receipt.candidate_digest.clone(),
        delegation_id: candidate.delegation_id.clone(),
        delegation_digest: candidate.delegation_digest.clone(),
        provider_binding_id: candidate.provider_binding_id.clone(),
        provider_binding_digest: candidate.provider_binding_digest.clone(),
        registry_release_id: candidate.registry_release_id.clone(),
        registry_release_digest: candidate.registry_release_digest.clone(),
        installation_receipt_id: candidate.installation_receipt_id.clone(),
        installation_receipt_digest: candidate.installation_receipt_digest.clone(),
        installation_content_digest: candidate.installation_content_digest.clone(),
        route_adapter_projection_id: candidate.route_adapter_projection_id.clone(),
        provider_id: candidate.provider_id.clone(),
        provider_owner_account_id: candidate.provider_owner_account_id.clone(),
        provider_policy_revision: candidate.provider_policy_revision,
        provider_digest: candidate.provider_digest.clone(),
        provider_status: candidate.provider_status.clone(),
        logical_adapter_id: candidate.logical_adapter_id.clone(),
        release_version: candidate.release_version.clone(),
        adapter_config_revision: candidate.adapter_config_revision,
        adapter_config_digest: candidate.adapter_config_digest.clone(),
        implementation_digest: candidate.implementation_digest.clone(),
        capability_set_digest: candidate.capability_set_digest.clone(),
        credential_verifier_digest: candidate.credential_verifier_digest.clone(),
        credential_ref_scheme: credential_ref_scheme.into(),
        credential_locator_commitment: credential_locator_commitment.into(),
        service_actor_id: candidate.service_actor_id.clone(),
        entrypoint_relative_path: binding.entrypoint_path.clone(),
        entrypoint_path_digest: runtime_launch_entrypoint_path_digest(&binding.entrypoint_path),
        entrypoint_sha256: binding.entrypoint_sha256.clone(),
        entrypoint_size_bytes: binding.entrypoint_size_bytes,
        entry_inventory_digest: binding.entry_inventory_digest.clone(),
        installed_file_count: binding.installed_files.len() as u64,
        installed_total_bytes: installed_total_bytes(authority.registry().prepared()),
        launch_policy_digest: policy.digest,
        launch_policy: policy.policy,
        sequence,
        predecessor_profile_id: predecessor.map(|x| x.receipt.profile_id.clone()),
        predecessor_profile_digest: predecessor.map(|x| x.receipt.profile_digest.clone()),
        recorded_by_actor_kind: recorded_by_actor_kind.into(),
        recorded_by_actor_user_id: recorded_by_actor_user_id.into(),
        recorded_at: now.into(),
        idempotency_scope: idempotency_scope.into(),
        idempotency_key: idempotency_key.into(),
        confirmation: confirmation.into(),
        profile_status: RUNTIME_LAUNCH_PROFILE_STATUS.into(),
        profile_effect: RUNTIME_LAUNCH_PROFILE_EFFECT.into(),
        adapter_effect: RUNTIME_LAUNCH_PROFILE_NO_EFFECT.into(),
        runtime_effect: RUNTIME_LAUNCH_PROFILE_NO_EFFECT.into(),
        provider_effect: RUNTIME_LAUNCH_PROFILE_NO_EFFECT.into(),
        credential_effect: RUNTIME_LAUNCH_PROFILE_NO_EFFECT.into(),
        route_effect: RUNTIME_LAUNCH_PROFILE_NO_EFFECT.into(),
        execution_effect: RUNTIME_LAUNCH_PROFILE_NO_EFFECT.into(),
        usage_effect: RUNTIME_LAUNCH_PROFILE_NO_EFFECT.into(),
        market_effect: RUNTIME_LAUNCH_PROFILE_NO_EFFECT.into(),
        settlement_effect: RUNTIME_LAUNCH_PROFILE_NO_EFFECT.into(),
    };
    seal_profile(material)
}

pub(super) fn build_revocation(
    input: &RevokeExternalPoolAdapterRuntimeLaunchProfile,
    profile: &StoredRuntimeLaunchProfile,
    now: &str,
) -> Result<ExternalPoolAdapterRuntimeLaunchProfileRevocationReceipt> {
    let p = &profile.receipt.profile;
    let material = ExternalPoolAdapterRuntimeLaunchProfileRevocationMaterial {
        profile_id: profile.receipt.profile_id.clone(),
        profile_digest: profile.receipt.profile_digest.clone(),
        provider_binding_id: p.provider_binding_id.clone(),
        provider_binding_digest: p.provider_binding_digest.clone(),
        candidate_id: p.candidate_id.clone(),
        candidate_digest: p.candidate_digest.clone(),
        revoked_by_actor_kind: input.revoked_by_actor_kind.clone(),
        revoked_by_actor_user_id: input.revoked_by_actor_user_id.clone(),
        reason: input.reason.clone(),
        revoked_at: now.into(),
        recorded_at: now.into(),
        idempotency_scope: input.idempotency_scope.clone(),
        idempotency_key: input.idempotency_key.clone(),
        confirmation: input.confirmation.clone(),
        revocation_effect: RUNTIME_LAUNCH_PROFILE_REVOCATION_EFFECT.into(),
        adapter_effect: RUNTIME_LAUNCH_PROFILE_NO_EFFECT.into(),
        runtime_effect: RUNTIME_LAUNCH_PROFILE_NO_EFFECT.into(),
        provider_effect: RUNTIME_LAUNCH_PROFILE_NO_EFFECT.into(),
        credential_effect: RUNTIME_LAUNCH_PROFILE_NO_EFFECT.into(),
        route_effect: RUNTIME_LAUNCH_PROFILE_NO_EFFECT.into(),
        execution_effect: RUNTIME_LAUNCH_PROFILE_NO_EFFECT.into(),
        usage_effect: RUNTIME_LAUNCH_PROFILE_NO_EFFECT.into(),
        market_effect: RUNTIME_LAUNCH_PROFILE_NO_EFFECT.into(),
        settlement_effect: RUNTIME_LAUNCH_PROFILE_NO_EFFECT.into(),
    };
    seal_revocation(material)
}

fn seal_profile(
    material: ExternalPoolAdapterRuntimeLaunchProfileMaterial,
) -> Result<ExternalPoolAdapterRuntimeLaunchProfileReceipt> {
    let mut receipt = ExternalPoolAdapterRuntimeLaunchProfileReceipt {
        schema: RUNTIME_LAUNCH_PROFILE_SCHEMA.into(),
        profile_id: new_id("external_pool_adapter_runtime_launch_profile"),
        profile_digest: String::new(),
        profile_material_digest: runtime_launch_profile_material_digest(&material)?,
        canonicalization: RUNTIME_LAUNCH_PROFILE_CANONICALIZATION.into(),
        digest_algorithm: RUNTIME_LAUNCH_PROFILE_DIGEST_ALGORITHM.into(),
        profile: material,
    };
    receipt.profile_digest = canonical_runtime_launch_profile_json_and_digest(&receipt)?.1;
    validate_runtime_launch_profile_receipt(&receipt)?;
    Ok(receipt)
}

fn seal_revocation(
    material: ExternalPoolAdapterRuntimeLaunchProfileRevocationMaterial,
) -> Result<ExternalPoolAdapterRuntimeLaunchProfileRevocationReceipt> {
    let mut receipt = ExternalPoolAdapterRuntimeLaunchProfileRevocationReceipt {
        schema: RUNTIME_LAUNCH_PROFILE_REVOCATION_SCHEMA.into(),
        revocation_id: new_id("external_pool_adapter_runtime_launch_profile_revocation"),
        revocation_digest: String::new(),
        revocation_material_digest: runtime_launch_profile_revocation_material_digest(&material)?,
        canonicalization: RUNTIME_LAUNCH_PROFILE_CANONICALIZATION.into(),
        digest_algorithm: RUNTIME_LAUNCH_PROFILE_DIGEST_ALGORITHM.into(),
        revocation: material,
    };
    receipt.revocation_digest =
        canonical_runtime_launch_profile_revocation_json_and_digest(&receipt)?.1;
    validate_runtime_launch_profile_revocation_receipt(&receipt)?;
    Ok(receipt)
}

pub(super) fn installed_total_bytes(prepared: &PreparedExternalPoolAdapterInstallation) -> u64 {
    prepared
        .installed_files()
        .iter()
        .map(|file| file.size_bytes)
        .sum()
}

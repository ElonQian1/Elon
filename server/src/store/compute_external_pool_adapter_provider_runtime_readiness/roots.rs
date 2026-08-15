use anyhow::{bail, Result};
use rusqlite::{params, Transaction};

use crate::{
    compute_federation::external_pool_adapter_provider_runtime_readiness::*,
    store::{
        compute_external_pool_adapter_runtime_bundle::CurrentExternalPoolAdapterNoWorkProbeObservationAuthority,
        compute_external_pool_adapter_upstream_transport_target::CurrentExternalPoolAdapterUpstreamTransportTargetAuthority,
    },
};

use super::types::CreateExternalPoolAdapterProviderRuntimeReadiness;

pub(super) fn audit_create_preflight(
    transaction: &Transaction<'_>,
    input: &CreateExternalPoolAdapterProviderRuntimeReadiness,
    target: &CurrentExternalPoolAdapterUpstreamTransportTargetAuthority,
    checked_at: &str,
) -> Result<()> {
    let target_receipt = target.target();
    let t = &target_receipt.target;
    let profile_receipt = target.profile().profile();
    let p = &profile_receipt.profile;
    if target.checked_at() != checked_at
        || target_receipt.target_id != input.target_id
        || target_receipt.target_digest != input.expected_target_digest
        || profile_receipt.profile_id != input.profile_id
        || profile_receipt.profile_digest != input.expected_profile_digest
        || p.candidate_id != input.candidate_id
        || p.candidate_digest != input.expected_candidate_digest
        || p.provider_binding_id != input.provider_binding_id
        || p.provider_binding_digest != input.expected_provider_binding_digest
        || p.installation_receipt_id != input.expected_installation_receipt_id
        || p.installation_receipt_digest != input.expected_installation_receipt_digest
        || t.profile_id != input.profile_id
        || t.profile_digest != input.expected_profile_digest
        || t.provider_binding_id != input.provider_binding_id
        || t.provider_binding_digest != input.expected_provider_binding_digest
    {
        bail!("provider runtime readiness preflight roots are not exact")
    }
    let actor_ok: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM users
          WHERE id=?1 AND status='active' AND role IN ('admin','owner'))",
        params![input.recorded_by_actor_user_id],
        |row| row.get(0),
    )?;
    if input.recorded_by_actor_kind != PROVIDER_RUNTIME_READINESS_ACTOR_PLATFORM_ADMIN || !actor_ok
    {
        bail!("provider runtime readiness trigger actor is not a platform administrator")
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_material_from_observation(
    input: &CreateExternalPoolAdapterProviderRuntimeReadiness,
    observation: &CurrentExternalPoolAdapterNoWorkProbeObservationAuthority<'_, '_, '_>,
    sequence: u64,
    predecessor_readiness_receipt_id: Option<String>,
    predecessor_readiness_receipt_digest: Option<String>,
    probe_execution_id: String,
) -> Result<ExternalPoolAdapterProviderRuntimeReadinessMaterial> {
    audit_final_observation(input, observation)?;
    let policy = server_provider_runtime_readiness_policy_catalog()?;
    let companion_receipt = observation.companion().companion();
    let c = &companion_receipt.companion;
    let vulnerability = observation.vulnerability();
    let sandbox = observation.sandbox();
    let credential = observation.credential();
    let compatibility = observation.runtime_compatibility();
    let verification_receipt = compatibility.verification();
    let run_receipt = compatibility.run_observation();
    let release = compatibility.release();
    Ok(ExternalPoolAdapterProviderRuntimeReadinessMaterial {
        policy_id: policy.policy.policy_id,
        policy_revision: policy.policy.policy_revision,
        policy_digest: policy.policy_digest,
        provider_binding_id: c.provider_binding_id.clone(),
        provider_binding_digest: c.provider_binding_digest.clone(),
        registry_release_id: c.registry_release_id.clone(),
        registry_release_digest: c.registry_release_digest.clone(),
        registry_release_material_digest: release.registry_release_material_digest.clone(),
        installation_receipt_id: c.installation_receipt_id.clone(),
        installation_receipt_digest: c.installation_receipt_digest.clone(),
        installation_content_digest: c.installation_content_digest.clone(),
        candidate_id: c.candidate_id.clone(),
        candidate_digest: c.candidate_digest.clone(),
        delegation_id: c.delegation_id.clone(),
        delegation_digest: c.delegation_digest.clone(),
        profile_id: companion_receipt.companion.profile_id.clone(),
        profile_digest: companion_receipt.companion.profile_digest.clone(),
        target_id: c.target_id.clone(),
        target_digest: c.target_digest.clone(),
        companion_id: companion_receipt.companion_id.clone(),
        companion_digest: companion_receipt.companion_digest.clone(),
        provider_id: c.provider_id.clone(),
        provider_policy_revision: c.provider_policy_revision,
        provider_digest: c.provider_digest.clone(),
        provider_status: c.provider_status.clone(),
        vulnerability_reattestation_receipt_id: vulnerability.reattestation_receipt_id.clone(),
        vulnerability_reattestation_receipt_digest: vulnerability
            .reattestation_receipt_digest
            .clone(),
        sandbox_reattestation_receipt_id: sandbox.reattestation_receipt_id.clone(),
        sandbox_reattestation_receipt_digest: sandbox.reattestation_receipt_digest.clone(),
        credential_reattestation_receipt_id: credential.reattestation_receipt_id.clone(),
        credential_reattestation_receipt_digest: credential.reattestation_receipt_digest.clone(),
        runtime_compatibility_verification_receipt_id: verification_receipt
            .verification_receipt_id
            .clone(),
        runtime_compatibility_verification_receipt_digest: verification_receipt
            .verification_receipt_digest
            .clone(),
        launch_policy_digest: c.launch_policy_digest.clone(),
        target_policy_digest: c.target_policy_digest.clone(),
        entrypoint_capsule_policy_digest: c.entrypoint_capsule_policy_digest.clone(),
        supervisor_session_policy_digest: c.supervisor_session_policy_digest.clone(),
        source_capsule_sha256: run_receipt.observation.source_capsule_sha256.clone(),
        source_capsule_size_bytes: run_receipt.observation.source_capsule_size_bytes,
        launch_image_sha256: run_receipt.observation.launch_image_sha256.clone(),
        launch_image_size_bytes: run_receipt.observation.launch_image_size_bytes,
        sealed_bindings: ExternalPoolAdapterProviderRuntimeReadinessSealedBindings {
            runtime_custody_epoch_digest: observation.custody_epoch_digest().into(),
            runtime_bundle_identity_commitment: observation
                .runtime_bundle_identity_commitment()
                .into(),
            post_cleanup_observation_commitment: observation
                .post_cleanup_observation_commitment()
                .into(),
        },
        probe_execution_id,
        request_bytes: u64::from(observation.request_bytes()),
        response_bytes: u64::from(observation.response_bytes()),
        probe_checked_at: observation.probe_checked_at().into(),
        cleanup_completed_at: observation.checked_at().into(),
        checked_at: observation.checked_at().into(),
        expires_at: observation.expires_at().into(),
        sequence,
        predecessor_readiness_receipt_id,
        predecessor_readiness_receipt_digest,
        recorded_by_actor_kind: input.recorded_by_actor_kind.clone(),
        recorded_by_actor_user_id: input.recorded_by_actor_user_id.clone(),
        recorded_at: observation.checked_at().into(),
        idempotency_scope: input.idempotency_scope.clone(),
        idempotency_key: input.idempotency_key.clone(),
        confirmation: input.confirmation.clone(),
        evidence_scope: PROVIDER_RUNTIME_READINESS_EVIDENCE_SCOPE.into(),
        receipt_status: PROVIDER_RUNTIME_READINESS_RECEIPT_STATUS.into(),
        effects: provider_runtime_readiness_no_effects(),
        observed_readiness: provider_runtime_readiness_observed_readiness(),
    })
}

fn audit_final_observation(
    input: &CreateExternalPoolAdapterProviderRuntimeReadiness,
    observation: &CurrentExternalPoolAdapterNoWorkProbeObservationAuthority<'_, '_, '_>,
) -> Result<()> {
    let companion_receipt = observation.companion().companion();
    let c = &companion_receipt.companion;
    let profile = observation.launch_profile();
    let compatibility = observation.runtime_compatibility();
    let verification = compatibility.verification();
    let run = compatibility.run_observation();
    let release = compatibility.release();
    if !observation.no_work_observed()
        || !observation.authenticated_shutdown_completed()
        || !observation.pidfd_reaped()
        || !observation.cgroup_cleaned()
        || !observation.scratch_cleaned()
        || c.provider_binding_id != input.provider_binding_id
        || c.provider_binding_digest != input.expected_provider_binding_digest
        || c.installation_receipt_id != input.expected_installation_receipt_id
        || c.installation_receipt_digest != input.expected_installation_receipt_digest
        || c.candidate_id != input.candidate_id
        || c.candidate_digest != input.expected_candidate_digest
        || profile.profile_id != input.profile_id
        || profile.profile_digest != input.expected_profile_digest
        || c.target_id != input.target_id
        || c.target_digest != input.expected_target_digest
        || companion_receipt.companion_id != input.companion_id
        || companion_receipt.companion_digest != input.expected_companion_digest
        || verification.verification_receipt_id
            != input.runtime_compatibility_verification_receipt_id
        || verification.verification_receipt_digest
            != input.expected_runtime_compatibility_verification_receipt_digest
        || release.registry_release_id != c.registry_release_id
        || release.registry_release_digest != c.registry_release_digest
        || release.release.installation_content_digest != c.installation_content_digest
        || run.observation.source_capsule_sha256 != observation.source_capsule_digest()
        || run.observation.launch_image_sha256 != observation.launch_capsule_digest()
        || compatibility.checked_at() != observation.checked_at()
    {
        bail!("provider runtime readiness final observation roots are not exact")
    }
    Ok(())
}

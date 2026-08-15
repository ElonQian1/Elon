use anyhow::{bail, Result};

use crate::compute_federation::{
    external_pool_adapter_release::ComputeExternalPoolAdapterReleaseCapability,
    external_pool_adapter_runtime_launch_profile::server_linux_runtime_launch_policy_catalog,
    external_pool_adapter_supervisor_session_policy_companion::server_supervisor_session_policy_catalog,
    external_pool_adapter_upstream_transport_target::server_upstream_transport_target_policy_catalog,
};

use super::*;

pub(crate) fn server_runtime_compatibility_v2_profile_catalog(
) -> Result<ExternalPoolAdapterRuntimeCompatibilityProfileV2Envelope> {
    let profile = runtime_compatibility_v2_profile_for_validation()?;
    validate_runtime_compatibility_v2_profile(&profile)?;
    Ok(ExternalPoolAdapterRuntimeCompatibilityProfileV2Envelope {
        schema: RUNTIME_COMPATIBILITY_V2_PROFILE_ENVELOPE_SCHEMA.into(),
        canonicalization: RUNTIME_COMPATIBILITY_VERIFICATION_CANONICALIZATION.into(),
        digest_algorithm: RUNTIME_COMPATIBILITY_VERIFICATION_DIGEST_ALGORITHM.into(),
        profile_digest: runtime_compatibility_profile_digest(&profile)?,
        profile,
    })
}

pub(crate) fn build_runtime_compatibility_challenge_receipt(
    challenge: ExternalPoolAdapterRuntimeCompatibilityChallengeMaterial,
) -> Result<ExternalPoolAdapterRuntimeCompatibilityChallengeReceipt> {
    validate_runtime_compatibility_challenge_material(&challenge)?;
    validate_runtime_compatibility_challenge_current_roots(&challenge)?;
    let challenge_material_digest = runtime_compatibility_challenge_material_digest(&challenge)?;
    let mut receipt = ExternalPoolAdapterRuntimeCompatibilityChallengeReceipt {
        schema: RUNTIME_COMPATIBILITY_VERIFICATION_CHALLENGE_SCHEMA.into(),
        challenge_digest: String::new(),
        challenge_material_digest,
        canonicalization: RUNTIME_COMPATIBILITY_VERIFICATION_CANONICALIZATION.into(),
        digest_algorithm: RUNTIME_COMPATIBILITY_VERIFICATION_DIGEST_ALGORITHM.into(),
        challenge,
    };
    receipt.challenge_digest = runtime_compatibility_challenge_json_and_digest(&receipt)?.1;
    validate_runtime_compatibility_challenge_receipt(&receipt)?;
    Ok(receipt)
}

pub(crate) fn prepare_runtime_compatibility_server_run_observation(
    challenge: &ExternalPoolAdapterRuntimeCompatibilityChallengeReceipt,
    material: ExternalPoolAdapterRuntimeCompatibilityServerRunObservationMaterial,
) -> Result<PreparedExternalPoolAdapterRuntimeCompatibilityServerRunObservation> {
    validate_runtime_compatibility_challenge_receipt(challenge)?;
    validate_runtime_compatibility_server_run_observation_material(&material)?;
    validate_runtime_compatibility_observation_against_challenge(&material, challenge)?;
    Ok(PreparedExternalPoolAdapterRuntimeCompatibilityServerRunObservation { material })
}

pub(crate) fn build_runtime_compatibility_run_observation_receipt(
    run_observation_id: String,
    prepared: PreparedExternalPoolAdapterRuntimeCompatibilityServerRunObservation,
) -> Result<ExternalPoolAdapterRuntimeCompatibilityRunObservationReceipt> {
    let run_observation_material_digest =
        runtime_compatibility_observation_material_digest(&prepared.material)?;
    let mut receipt = ExternalPoolAdapterRuntimeCompatibilityRunObservationReceipt {
        schema: RUNTIME_COMPATIBILITY_VERIFICATION_OBSERVATION_SCHEMA.into(),
        run_observation_id,
        run_observation_digest: String::new(),
        run_observation_material_digest,
        canonicalization: RUNTIME_COMPATIBILITY_VERIFICATION_CANONICALIZATION.into(),
        digest_algorithm: RUNTIME_COMPATIBILITY_VERIFICATION_DIGEST_ALGORITHM.into(),
        observation: prepared.material,
    };
    receipt.run_observation_digest = runtime_compatibility_observation_json_and_digest(&receipt)?.1;
    validate_runtime_compatibility_run_observation_receipt(&receipt)?;
    Ok(receipt)
}

pub(crate) fn build_runtime_compatibility_verification_receipt(
    verification_receipt_id: String,
    material: ExternalPoolAdapterRuntimeCompatibilityVerificationMaterial,
    challenge: &ExternalPoolAdapterRuntimeCompatibilityChallengeReceipt,
    observation: &ExternalPoolAdapterRuntimeCompatibilityRunObservationReceipt,
) -> Result<ExternalPoolAdapterRuntimeCompatibilityVerificationReceipt> {
    validate_runtime_compatibility_verification_material(&material, challenge, observation)?;
    let verification_material_digest =
        runtime_compatibility_verification_material_digest(&material)?;
    let mut receipt = ExternalPoolAdapterRuntimeCompatibilityVerificationReceipt {
        schema: RUNTIME_COMPATIBILITY_VERIFICATION_RECEIPT_SCHEMA.into(),
        verification_receipt_id,
        verification_receipt_digest: String::new(),
        verification_material_digest,
        canonicalization: RUNTIME_COMPATIBILITY_VERIFICATION_CANONICALIZATION.into(),
        digest_algorithm: RUNTIME_COMPATIBILITY_VERIFICATION_DIGEST_ALGORITHM.into(),
        verification: material,
    };
    receipt.verification_receipt_digest =
        runtime_compatibility_verification_receipt_json_and_digest(&receipt)?.1;
    validate_runtime_compatibility_verification_receipt(&receipt, challenge, observation)?;
    Ok(receipt)
}

pub(crate) fn build_runtime_compatibility_revocation_receipt(
    revocation_receipt_id: String,
    material: ExternalPoolAdapterRuntimeCompatibilityRevocationMaterial,
) -> Result<ExternalPoolAdapterRuntimeCompatibilityRevocationReceipt> {
    validate_runtime_compatibility_revocation_material(&material)?;
    let revocation_material_digest = runtime_compatibility_revocation_material_digest(&material)?;
    let mut receipt = ExternalPoolAdapterRuntimeCompatibilityRevocationReceipt {
        schema: RUNTIME_COMPATIBILITY_VERIFICATION_REVOCATION_RECEIPT_SCHEMA.into(),
        revocation_receipt_id,
        revocation_receipt_digest: String::new(),
        revocation_material_digest,
        canonicalization: RUNTIME_COMPATIBILITY_VERIFICATION_CANONICALIZATION.into(),
        digest_algorithm: RUNTIME_COMPATIBILITY_VERIFICATION_DIGEST_ALGORITHM.into(),
        revocation: material,
    };
    receipt.revocation_receipt_digest =
        runtime_compatibility_revocation_receipt_json_and_digest(&receipt)?.1;
    validate_runtime_compatibility_revocation_receipt(&receipt)?;
    Ok(receipt)
}

pub(super) fn runtime_compatibility_v2_profile_for_validation(
) -> Result<ExternalPoolAdapterRuntimeCompatibilityProfileV2> {
    let (runtime, runtime_digest) = server_linux_runtime_launch_policy_catalog()?;
    let (transport, transport_digest) = server_upstream_transport_target_policy_catalog()?;
    let (session, session_digest) = server_supervisor_session_policy_catalog()?;
    let (runner, runner_digest) = server_runtime_compatibility_runner_policy_catalog()?;
    let (fixtures, fixture_digest) = server_runtime_compatibility_public_fixture_catalog()?;
    if runtime.policy_id != "external_pool_adapter_runtime_launch_policy_v1"
        || runtime.policy_revision != 1
        || transport.policy_id != "external_pool_adapter_upstream_transport_target_policy_v1"
        || transport.policy_revision != 1
        || session.policy_id != "external_pool_adapter_supervisor_session_policy_v2"
        || session.policy_revision != 2
    {
        bail!("runtime compatibility V2 policy versions require a profile revision");
    }
    Ok(ExternalPoolAdapterRuntimeCompatibilityProfileV2 {
        schema: RUNTIME_COMPATIBILITY_V2_PROFILE_SCHEMA.into(),
        profile_id: RUNTIME_COMPATIBILITY_V2_PROFILE_ID.into(),
        profile_revision: RUNTIME_COMPATIBILITY_V2_PROFILE_REVISION,
        host_os: "linux".into(),
        host_arch: "x86_64".into(),
        release_capabilities: release_capabilities(),
        runtime_launch_policy: policy_ref(
            &runtime.policy_id,
            runtime.policy_revision,
            runtime_digest,
        ),
        upstream_transport_policy: policy_ref(
            &transport.policy_id,
            transport.policy_revision,
            transport_digest,
        ),
        supervisor_session_policy: policy_ref(
            &session.policy_id,
            session.policy_revision,
            session_digest,
        ),
        source_capsule_policy: runtime_compatibility_source_capsule_policy_ref(),
        runner_policy: policy_ref(&runner.policy_id, runner.policy_revision, runner_digest),
        fixture_catalog: policy_ref(
            &fixtures.catalog_id,
            fixtures.catalog_revision,
            fixture_digest,
        ),
        evidence_scope: RUNTIME_COMPATIBILITY_VERIFICATION_EVIDENCE_SCOPE.into(),
        effects: runtime_compatibility_no_effects(),
        readiness: runtime_compatibility_no_readiness(),
    })
}

fn policy_ref(
    policy_id: &str,
    policy_revision: u64,
    policy_digest: String,
) -> ExternalPoolAdapterRuntimeCompatibilityPolicyRef {
    ExternalPoolAdapterRuntimeCompatibilityPolicyRef {
        policy_id: policy_id.into(),
        policy_revision,
        policy_digest,
    }
}

fn release_capabilities() -> Vec<ComputeExternalPoolAdapterReleaseCapability> {
    [
        "authenticated_ack",
        "authenticated_events",
        "cancel_no_start",
        "idempotent_commit",
        "prepare",
        "reconcile",
    ]
    .into_iter()
    .map(
        |capability_id| ComputeExternalPoolAdapterReleaseCapability {
            capability_id: capability_id.into(),
            capability_revision: 1,
        },
    )
    .collect()
}

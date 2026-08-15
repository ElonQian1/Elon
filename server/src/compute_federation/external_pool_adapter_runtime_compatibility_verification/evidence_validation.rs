use anyhow::{bail, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::Duration;
use rsa::{
    pkcs1v15::{Signature as RsaSignature, VerifyingKey},
    pkcs8::DecodePublicKey,
    signature::Verifier,
    RsaPublicKey,
};
use sha2::{Digest, Sha256};

use crate::compute_federation::external_pool_adapter_registry::validate_registry_release_receipt;

use super::{validation_support::*, *};

pub(crate) fn validate_runtime_compatibility_server_run_observation_material(
    value: &ExternalPoolAdapterRuntimeCompatibilityServerRunObservationMaterial,
) -> Result<()> {
    validate_registry_release_receipt(&value.registry_release)?;
    identifiers([
        &value.runner_execution_id,
        &value.challenge_id,
        &value.profile_id,
        &value.observation_status,
    ])?;
    digests([
        &value.challenge_digest,
        &value.challenge_nonce_digest,
        &value.profile_digest,
        &value.runner_policy_digest,
        &value.fixture_catalog_digest,
        &value.source_capsule_sha256,
        &value.source_capsule_policy_digest,
        &value.launch_image_sha256,
        &value.public_fixture_delivery_root,
    ])?;
    let started = canonical_timestamp(&value.run_started_at)?;
    let completed = canonical_timestamp(&value.run_completed_at)?;
    canonical_timestamp(&value.recorded_at)?;
    if value.profile_id != RUNTIME_COMPATIBILITY_V2_PROFILE_ID
        || value.profile_revision != RUNTIME_COMPATIBILITY_V2_PROFILE_REVISION
        || value.source_capsule_size_bytes == 0
        || value.launch_image_size_bytes == 0
        || value.source_capsule_sha256 == value.launch_image_sha256
        || value.source_capsule_policy_digest != RUNTIME_COMPATIBILITY_SOURCE_CAPSULE_POLICY_DIGEST
        || completed < started
        || completed - started > Duration::seconds(RUNTIME_COMPATIBILITY_MAX_RUN_SECONDS as i64)
        || value.recorded_at != value.run_completed_at
        || value.child_network_attempt_count != 0
        || value.upstream_connect_attempt_count != 0
        || value.write_outside_ephemeral_count != 0
        || value.additional_process_attempt_count != 0
        || value.policy_violation_count != 0
        || value.observation_status != RUNTIME_COMPATIBILITY_VERIFICATION_OBSERVATION_STATUS
        || !no_effects(&value.effects)
        || !no_readiness(&value.readiness)
    {
        bail!("runtime compatibility server-run observation is not exact");
    }
    validate_fixture_resources(
        &value.fixture_resources,
        &value.registry_release.release.manifest.files,
    )?;
    validate_observation_inventory(&value.observations)?;
    validate_no_work(value)?;
    Ok(())
}

pub(crate) fn validate_runtime_compatibility_observation_against_challenge(
    value: &ExternalPoolAdapterRuntimeCompatibilityServerRunObservationMaterial,
    challenge: &ExternalPoolAdapterRuntimeCompatibilityChallengeReceipt,
) -> Result<()> {
    let selected = &challenge.challenge;
    let started = canonical_timestamp(&value.run_started_at)?;
    let completed = canonical_timestamp(&value.run_completed_at)?;
    if value.challenge_id != selected.challenge_id
        || value.challenge_digest != challenge.challenge_digest
        || value.challenge_nonce_digest != selected.challenge_nonce_digest
        || value.registry_release != selected.registry_release
        || value.profile_id != selected.profile_id
        || value.profile_revision != selected.profile_revision
        || value.profile_digest != selected.profile_digest
        || value.runner_policy_digest != selected.runner_policy.policy_digest
        || value.fixture_catalog_digest != selected.fixture_catalog.policy_digest
        || value.source_capsule_sha256 != selected.entrypoint_sha256
        || value.source_capsule_size_bytes != selected.entrypoint_size_bytes
        || value.source_capsule_policy_digest != selected.source_capsule_policy.policy_digest
        || value.fixture_resources != selected.fixture_resources
        || started < canonical_timestamp(&selected.issued_at)?
        || completed >= canonical_timestamp(&selected.expires_at)?
    {
        bail!("runtime compatibility observation does not consume its exact challenge");
    }
    Ok(())
}

pub(crate) fn validate_runtime_compatibility_run_observation_receipt(
    value: &ExternalPoolAdapterRuntimeCompatibilityRunObservationReceipt,
) -> Result<()> {
    if value.schema != RUNTIME_COMPATIBILITY_VERIFICATION_OBSERVATION_SCHEMA
        || value.canonicalization != RUNTIME_COMPATIBILITY_VERIFICATION_CANONICALIZATION
        || value.digest_algorithm != RUNTIME_COMPATIBILITY_VERIFICATION_DIGEST_ALGORITHM
    {
        bail!("runtime compatibility run-observation metadata is unsupported");
    }
    identifier(&value.run_observation_id, 200)?;
    digests([
        &value.run_observation_digest,
        &value.run_observation_material_digest,
    ])?;
    validate_runtime_compatibility_server_run_observation_material(&value.observation)?;
    if runtime_compatibility_observation_material_digest(&value.observation)?
        != value.run_observation_material_digest
        || runtime_compatibility_observation_json_and_digest(value)?.1
            != value.run_observation_digest
    {
        bail!("runtime compatibility run-observation receipt is not canonical");
    }
    Ok(())
}

pub(crate) fn validate_runtime_compatibility_verification_material(
    value: &ExternalPoolAdapterRuntimeCompatibilityVerificationMaterial,
    challenge: &ExternalPoolAdapterRuntimeCompatibilityChallengeReceipt,
    observation: &ExternalPoolAdapterRuntimeCompatibilityRunObservationReceipt,
) -> Result<()> {
    validate_registry_release_receipt(&value.registry_release)?;
    identifiers([
        &value.runner_execution_id,
        &value.challenge_id,
        &value.run_observation_id,
        &value.profile_id,
        &value.sandbox_verifier_key_record_id,
        &value.sandbox_verifier_operator,
        &value.sandbox_verifier_product,
        &value.verified_by_admin_user_id,
        &value.idempotency_scope,
        &value.idempotency_key,
    ])?;
    digests([
        &value.challenge_digest,
        &value.run_observation_digest,
        &value.run_observation_material_digest,
        &value.profile_digest,
        &value.runner_policy_digest,
        &value.fixture_catalog_digest,
        &value.public_fixture_delivery_root,
        &value.sandbox_verifier_key_record_digest,
        &value.sandbox_verifier_key_id,
        &value.signature_message_digest,
        &value.signature_digest,
    ])?;
    let signature = STANDARD.decode(&value.signature_base64)?;
    let expected_message = runtime_compatibility_signature_challenge(challenge, observation)?;
    let verified = canonical_timestamp(&value.verified_at)?;
    let expires = canonical_timestamp(&value.expires_at)?;
    if value.runner_execution_id != observation.observation.runner_execution_id
        || value.challenge_id != challenge.challenge.challenge_id
        || value.challenge_digest != challenge.challenge_digest
        || value.run_observation_id != observation.run_observation_id
        || value.run_observation_digest != observation.run_observation_digest
        || value.run_observation_material_digest != observation.run_observation_material_digest
        || value.registry_release != challenge.challenge.registry_release
        || value.profile_id != challenge.challenge.profile_id
        || value.profile_revision != challenge.challenge.profile_revision
        || value.profile_digest != challenge.challenge.profile_digest
        || value.runner_policy_digest != challenge.challenge.runner_policy.policy_digest
        || value.fixture_catalog_digest != challenge.challenge.fixture_catalog.policy_digest
        || value.public_fixture_delivery_root
            != observation.observation.public_fixture_delivery_root
        || value.sandbox_verifier_key_record_id
            != challenge.challenge.sandbox_verifier_key_record_id
        || value.sandbox_verifier_key_record_digest
            != challenge.challenge.sandbox_verifier_key_record_digest
        || value.sandbox_verifier_key_id != challenge.challenge.sandbox_verifier_key_id
        || value.sandbox_verifier_operator != challenge.challenge.sandbox_verifier_operator
        || value.sandbox_verifier_product != challenge.challenge.sandbox_verifier_product
        || value.signature_algorithm != RUNTIME_COMPATIBILITY_VERIFICATION_SIGNATURE_ALGORITHM
        || value.sequence != challenge.challenge.sequence
        || value.predecessor_verification_receipt_id
            != challenge.challenge.predecessor_verification_receipt_id
        || value.predecessor_verification_receipt_digest
            != challenge.challenge.predecessor_verification_receipt_digest
        || value.signature_message_digest != expected_message.signature_message_digest
        || signature.is_empty()
        || signature.len() > 1024
        || STANDARD.encode(&signature) != value.signature_base64
        || hex::encode(Sha256::digest(&signature)) != value.signature_digest
        || value.confirmation != RUNTIME_COMPATIBILITY_VERIFICATION_CONFIRMATION
        || value.recorded_at != value.verified_at
        || verified < canonical_timestamp(&observation.observation.run_completed_at)?
        || verified >= canonical_timestamp(&challenge.challenge.expires_at)?
        || expires - verified
            != Duration::hours(RUNTIME_COMPATIBILITY_VERIFICATION_RECEIPT_VALIDITY_HOURS)
        || value.evidence_scope != RUNTIME_COMPATIBILITY_VERIFICATION_EVIDENCE_SCOPE
        || value.receipt_status != RUNTIME_COMPATIBILITY_VERIFICATION_SIGNED_RECEIPT_STATUS
        || !no_effects(&value.effects)
        || !no_readiness(&value.readiness)
    {
        bail!("runtime compatibility verification material is not exact");
    }
    Ok(())
}

pub(crate) fn validate_runtime_compatibility_verification_receipt(
    value: &ExternalPoolAdapterRuntimeCompatibilityVerificationReceipt,
    challenge: &ExternalPoolAdapterRuntimeCompatibilityChallengeReceipt,
    observation: &ExternalPoolAdapterRuntimeCompatibilityRunObservationReceipt,
) -> Result<()> {
    if value.schema != RUNTIME_COMPATIBILITY_VERIFICATION_RECEIPT_SCHEMA
        || value.canonicalization != RUNTIME_COMPATIBILITY_VERIFICATION_CANONICALIZATION
        || value.digest_algorithm != RUNTIME_COMPATIBILITY_VERIFICATION_DIGEST_ALGORITHM
    {
        bail!("runtime compatibility verification receipt metadata is unsupported");
    }
    identifier(&value.verification_receipt_id, 200)?;
    digests([
        &value.verification_receipt_digest,
        &value.verification_material_digest,
    ])?;
    validate_runtime_compatibility_verification_material(
        &value.verification,
        challenge,
        observation,
    )?;
    if runtime_compatibility_verification_material_digest(&value.verification)?
        != value.verification_material_digest
        || runtime_compatibility_verification_receipt_json_and_digest(value)?.1
            != value.verification_receipt_digest
    {
        bail!("runtime compatibility verification receipt is not canonical");
    }
    Ok(())
}

pub(crate) fn verify_runtime_compatibility_signature(
    public_key_pem: &str,
    challenge: &ExternalPoolAdapterRuntimeCompatibilitySignatureChallenge,
    signature_base64: &str,
) -> Result<()> {
    let message = STANDARD.decode(&challenge.signature_message_base64)?;
    let signature_bytes = STANDARD.decode(signature_base64)?;
    if challenge.schema != RUNTIME_COMPATIBILITY_VERIFICATION_SIGNATURE_CHALLENGE_SCHEMA
        || challenge.canonicalization != RUNTIME_COMPATIBILITY_VERIFICATION_CANONICALIZATION
        || challenge.digest_algorithm != RUNTIME_COMPATIBILITY_VERIFICATION_DIGEST_ALGORITHM
        || challenge.signature_algorithm != RUNTIME_COMPATIBILITY_VERIFICATION_SIGNATURE_ALGORITHM
        || STANDARD.encode(&message) != challenge.signature_message_base64
        || hex::encode(Sha256::digest(&message)) != challenge.signature_message_digest
        || STANDARD.encode(&signature_bytes) != signature_base64
        || signature_bytes.is_empty()
        || signature_bytes.len() > 1024
    {
        bail!("runtime compatibility signature encoding is invalid");
    }
    let public = RsaPublicKey::from_public_key_pem(public_key_pem)?;
    let signature = RsaSignature::try_from(signature_bytes.as_slice())?;
    VerifyingKey::<Sha256>::new(public)
        .verify(&message, &signature)
        .map_err(|_| anyhow::anyhow!("runtime compatibility signature verification failed"))
}

pub(crate) fn validate_runtime_compatibility_revocation_material(
    value: &ExternalPoolAdapterRuntimeCompatibilityRevocationMaterial,
) -> Result<()> {
    identifiers([
        &value.verification_receipt_id,
        &value.registry_release_id,
        &value.revoked_by_admin_user_id,
        &value.idempotency_scope,
        &value.idempotency_key,
    ])?;
    digests([
        &value.verification_receipt_digest,
        &value.registry_release_digest,
    ])?;
    canonical_timestamp(&value.revoked_at)?;
    if value.recorded_at != value.revoked_at
        || value.reason.trim() != value.reason
        || !(12..=500).contains(&value.reason.chars().count())
        || value.reason.chars().any(char::is_control)
        || value.confirmation != RUNTIME_COMPATIBILITY_VERIFICATION_REVOCATION_CONFIRMATION
        || value.revocation_status != RUNTIME_COMPATIBILITY_VERIFICATION_REVOCATION_STATUS
        || !no_effects(&value.effects)
        || !no_readiness(&value.readiness)
    {
        bail!("runtime compatibility revocation material is invalid");
    }
    Ok(())
}

pub(crate) fn validate_runtime_compatibility_revocation_receipt(
    value: &ExternalPoolAdapterRuntimeCompatibilityRevocationReceipt,
) -> Result<()> {
    if value.schema != RUNTIME_COMPATIBILITY_VERIFICATION_REVOCATION_RECEIPT_SCHEMA
        || value.canonicalization != RUNTIME_COMPATIBILITY_VERIFICATION_CANONICALIZATION
        || value.digest_algorithm != RUNTIME_COMPATIBILITY_VERIFICATION_DIGEST_ALGORITHM
    {
        bail!("runtime compatibility revocation receipt metadata is unsupported");
    }
    identifier(&value.revocation_receipt_id, 200)?;
    digests([
        &value.revocation_receipt_digest,
        &value.revocation_material_digest,
    ])?;
    validate_runtime_compatibility_revocation_material(&value.revocation)?;
    if runtime_compatibility_revocation_material_digest(&value.revocation)?
        != value.revocation_material_digest
        || runtime_compatibility_revocation_receipt_json_and_digest(value)?.1
            != value.revocation_receipt_digest
    {
        bail!("runtime compatibility revocation receipt is not canonical");
    }
    Ok(())
}

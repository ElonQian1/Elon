use anyhow::{bail, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::Duration;
use sha2::{Digest, Sha256};

use crate::compute_federation::external_pool_adapter_registry::validate_registry_release_receipt;

use super::{validation_support::*, *};

pub(crate) fn validate_runtime_compatibility_runner_policy(
    value: &ExternalPoolAdapterRuntimeCompatibilityRunnerPolicy,
) -> Result<()> {
    if value != &runtime_compatibility_runner_policy_for_validation() {
        bail!("runtime compatibility runner policy is not exact");
    }
    Ok(())
}

pub(crate) fn validate_runtime_compatibility_public_fixture_catalog(
    value: &ExternalPoolAdapterRuntimeCompatibilityPublicFixtureCatalog,
) -> Result<()> {
    if value != &runtime_compatibility_public_fixture_catalog_for_validation() {
        bail!("runtime compatibility public fixture catalog is not exact");
    }
    Ok(())
}

pub(crate) fn validate_runtime_compatibility_v2_profile(
    value: &ExternalPoolAdapterRuntimeCompatibilityProfileV2,
) -> Result<()> {
    if value != &runtime_compatibility_v2_profile_for_validation()?
        || !no_effects(&value.effects)
        || !no_readiness(&value.readiness)
    {
        bail!("runtime compatibility V2 profile is not the current exact catalog");
    }
    Ok(())
}

pub(crate) fn validate_runtime_compatibility_challenge_material(
    value: &ExternalPoolAdapterRuntimeCompatibilityChallengeMaterial,
) -> Result<()> {
    validate_registry_release_receipt(&value.registry_release)?;
    let release = &value.registry_release.release;
    if value.schema != RUNTIME_COMPATIBILITY_VERIFICATION_CHALLENGE_SCHEMA
        || value.profile_id != RUNTIME_COMPATIBILITY_V2_PROFILE_ID
        || value.profile_revision != RUNTIME_COMPATIBILITY_V2_PROFILE_REVISION
        || value.signature_algorithm != RUNTIME_COMPATIBILITY_VERIFICATION_SIGNATURE_ALGORITHM
        || value.confirmation != RUNTIME_COMPATIBILITY_VERIFICATION_CHALLENGE_CONFIRMATION
        || value.runtime_kind != release.manifest.runtime.kind
        || value.entrypoint_path != release.manifest.runtime.entrypoint
        || value.sequence == 0
        || value.predecessor_verification_receipt_id.is_some()
            != value.predecessor_verification_receipt_digest.is_some()
        || (value.sequence == 1) != value.predecessor_verification_receipt_id.is_none()
    {
        bail!("runtime compatibility challenge authority is invalid");
    }
    identifiers([
        &value.challenge_id,
        &value.created_by_admin_user_id,
        &value.idempotency_scope,
        &value.idempotency_key,
        &value.sandbox_verifier_key_record_id,
        &value.sandbox_verifier_operator,
        &value.sandbox_verifier_product,
    ])?;
    digests([
        &value.challenge_nonce_digest,
        &value.profile_digest,
        &value.sandbox_verifier_key_record_digest,
        &value.sandbox_verifier_key_id,
        &value.entrypoint_sha256,
    ])?;
    for reference in policy_refs(value) {
        identifier(&reference.policy_id, 240)?;
        digest(&reference.policy_digest)?;
        if reference.policy_revision == 0 {
            bail!("runtime compatibility policy revision is invalid");
        }
    }
    if value.source_capsule_policy != runtime_compatibility_source_capsule_policy_ref() {
        bail!("runtime compatibility source capsule policy is not V257 V1");
    }
    let nonce = STANDARD.decode(&value.challenge_nonce_base64)?;
    let issued = canonical_timestamp(&value.issued_at)?;
    let expires = canonical_timestamp(&value.expires_at)?;
    if nonce.len() != 32
        || STANDARD.encode(&nonce) != value.challenge_nonce_base64
        || hex::encode(Sha256::digest(&nonce)) != value.challenge_nonce_digest
        || expires - issued
            != Duration::minutes(RUNTIME_COMPATIBILITY_VERIFICATION_CHALLENGE_VALIDITY_MINUTES)
        || value.recorded_at != value.issued_at
    {
        bail!("runtime compatibility challenge window or nonce is invalid");
    }
    validate_entrypoint(value)?;
    validate_fixture_resources(&value.fixture_resources, &release.manifest.files)?;
    optional_id_digest(
        value.predecessor_verification_receipt_id.as_deref(),
        value.predecessor_verification_receipt_digest.as_deref(),
    )?;
    Ok(())
}

pub(crate) fn validate_runtime_compatibility_challenge_current_roots(
    value: &ExternalPoolAdapterRuntimeCompatibilityChallengeMaterial,
) -> Result<()> {
    let current = server_runtime_compatibility_v2_profile_catalog()?;
    if value.profile_digest != current.profile_digest
        || value.registry_release.release.supported_capabilities
            != current.profile.release_capabilities
        || value.runtime_launch_policy != current.profile.runtime_launch_policy
        || value.upstream_transport_policy != current.profile.upstream_transport_policy
        || value.supervisor_session_policy != current.profile.supervisor_session_policy
        || value.source_capsule_policy != current.profile.source_capsule_policy
        || value.runner_policy != current.profile.runner_policy
        || value.fixture_catalog != current.profile.fixture_catalog
    {
        bail!("runtime compatibility challenge roots are not current");
    }
    Ok(())
}

pub(crate) fn validate_runtime_compatibility_challenge_receipt(
    value: &ExternalPoolAdapterRuntimeCompatibilityChallengeReceipt,
) -> Result<()> {
    if value.schema != RUNTIME_COMPATIBILITY_VERIFICATION_CHALLENGE_SCHEMA
        || value.canonicalization != RUNTIME_COMPATIBILITY_VERIFICATION_CANONICALIZATION
        || value.digest_algorithm != RUNTIME_COMPATIBILITY_VERIFICATION_DIGEST_ALGORITHM
    {
        bail!("runtime compatibility challenge receipt metadata is unsupported");
    }
    validate_runtime_compatibility_challenge_material(&value.challenge)?;
    digests([&value.challenge_digest, &value.challenge_material_digest])?;
    if runtime_compatibility_challenge_material_digest(&value.challenge)?
        != value.challenge_material_digest
        || runtime_compatibility_challenge_json_and_digest(value)?.1 != value.challenge_digest
    {
        bail!("runtime compatibility challenge receipt is not canonical");
    }
    Ok(())
}

pub(crate) fn validate_create_runtime_compatibility_challenge_input(
    value: &CreateExternalPoolAdapterRuntimeCompatibilityChallengeInput,
) -> Result<()> {
    identifiers([
        &value.registry_release_id,
        &value.sandbox_verifier_key_record_id,
        &value.idempotency_key,
    ])?;
    digests([
        &value.expected_registry_release_digest,
        &value.expected_profile_digest,
        &value.expected_runner_policy_digest,
        &value.expected_fixture_catalog_digest,
        &value.expected_sandbox_verifier_key_record_digest,
        &value.expected_sandbox_verifier_key_id,
    ])?;
    optional_id_digest(
        value.predecessor_verification_receipt_id.as_deref(),
        value.predecessor_verification_receipt_digest.as_deref(),
    )?;
    if value.confirmation != RUNTIME_COMPATIBILITY_VERIFICATION_CHALLENGE_CONFIRMATION {
        bail!("runtime compatibility challenge confirmation is invalid");
    }
    Ok(())
}

pub(crate) fn validate_record_runtime_compatibility_verification_input(
    value: &RecordExternalPoolAdapterRuntimeCompatibilityVerificationReceiptInput,
) -> Result<()> {
    identifiers([&value.run_observation_id, &value.idempotency_key])?;
    digests([
        &value.expected_run_observation_digest,
        &value.expected_signature_message_digest,
    ])?;
    let signature = STANDARD.decode(&value.signature_base64)?;
    if signature.is_empty()
        || signature.len() > 1024
        || STANDARD.encode(&signature) != value.signature_base64
        || value.confirmation != RUNTIME_COMPATIBILITY_VERIFICATION_CONFIRMATION
    {
        bail!("runtime compatibility verification input is invalid");
    }
    Ok(())
}

pub(crate) fn validate_revoke_runtime_compatibility_verification_input(
    value: &RevokeExternalPoolAdapterRuntimeCompatibilityVerificationReceiptInput,
) -> Result<()> {
    identifiers([&value.verification_receipt_id, &value.idempotency_key])?;
    digest(&value.expected_verification_receipt_digest)?;
    if value.reason.trim() != value.reason
        || !(12..=500).contains(&value.reason.chars().count())
        || value.reason.chars().any(char::is_control)
        || value.confirmation != RUNTIME_COMPATIBILITY_VERIFICATION_REVOCATION_CONFIRMATION
    {
        bail!("runtime compatibility revocation input is invalid");
    }
    Ok(())
}

pub(crate) fn validate_runtime_compatibility_currentness_summary(
    value: &ExternalPoolAdapterRuntimeCompatibilityCurrentnessSummary,
) -> Result<()> {
    identifiers([
        &value.registry_release_id,
        &value.adapter_id,
        &value.release_version,
        &value.profile_id,
        &value.verification_receipt_id,
        &value.currentness_status,
    ])?;
    digest(&value.verification_receipt_digest)?;
    let verified = canonical_timestamp(&value.verified_at)?;
    let expires = canonical_timestamp(&value.expires_at)?;
    if let Some(revoked_at) = &value.revoked_at {
        canonical_timestamp(revoked_at)?;
    }
    if value.schema != RUNTIME_COMPATIBILITY_VERIFICATION_CURRENTNESS_SCHEMA
        || value.profile_id != RUNTIME_COMPATIBILITY_V2_PROFILE_ID
        || value.profile_revision != RUNTIME_COMPATIBILITY_V2_PROFILE_REVISION
        || value.sequence == 0
        || expires - verified
            != Duration::hours(RUNTIME_COMPATIBILITY_VERIFICATION_RECEIPT_VALIDITY_HOURS)
        || !matches!(
            value.currentness_status.as_str(),
            RUNTIME_COMPATIBILITY_VERIFICATION_CURRENT_STATUS
                | RUNTIME_COMPATIBILITY_VERIFICATION_HISTORICAL_STATUS
        )
        || (value.currentness_status == RUNTIME_COMPATIBILITY_VERIFICATION_CURRENT_STATUS
            && (value.revoked_at.is_some() || !value.historical_reasons.is_empty()))
        || !no_effects(&value.effects)
        || !no_readiness(&value.readiness)
    {
        bail!("runtime compatibility currentness summary is invalid");
    }
    for reason in &value.historical_reasons {
        identifier(reason, 240)?;
    }
    Ok(())
}

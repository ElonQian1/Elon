use anyhow::{bail, Result};
use chrono::{DateTime, Duration, FixedOffset, SecondsFormat};

use crate::compute_federation::provider::PROVIDER_STATUS_REGISTERING;

use super::*;

pub(crate) fn validate_create_provider_runtime_readiness_receipt_body(
    value: &CreateProviderRuntimeReadinessReceiptBody,
) -> Result<()> {
    digests([
        &value.expected_provider_binding_digest,
        &value.expected_installation_receipt_digest,
        &value.expected_candidate_digest,
        &value.expected_profile_digest,
        &value.expected_target_digest,
        &value.expected_companion_digest,
        &value.expected_runtime_compatibility_verification_receipt_digest,
    ])?;
    identifiers([
        &value.expected_installation_receipt_id,
        &value.runtime_compatibility_verification_receipt_id,
        &value.idempotency_key,
    ])?;
    if let Some(predecessor) = &value.expected_predecessor {
        identifiers([&predecessor.readiness_receipt_id])?;
        digests([&predecessor.readiness_receipt_digest])?;
    }
    if !value.confirm_provider_runtime_readiness {
        bail!("provider runtime readiness confirmation is required")
    }
    Ok(())
}

pub(crate) fn validate_revoke_provider_runtime_readiness_receipt_body(
    value: &RevokeProviderRuntimeReadinessReceiptBody,
) -> Result<()> {
    digests([&value.expected_readiness_receipt_digest])?;
    identifiers([&value.idempotency_key])?;
    reason(&value.reason)?;
    if !value.confirm_revocation {
        bail!("provider runtime readiness revocation confirmation is required")
    }
    Ok(())
}

pub(crate) fn validate_provider_runtime_readiness_policy(
    value: &ExternalPoolAdapterProviderRuntimeReadinessPolicy,
) -> Result<()> {
    if value != &provider_runtime_readiness_policy_for_validation() {
        bail!("provider runtime readiness policy is not the exact server catalog entry")
    }
    Ok(())
}

pub(crate) fn validate_provider_runtime_readiness_policy_envelope(
    value: &ExternalPoolAdapterProviderRuntimeReadinessPolicyEnvelope,
) -> Result<()> {
    validate_provider_runtime_readiness_policy(&value.policy)?;
    digests([&value.policy_digest])?;
    if value.schema != PROVIDER_RUNTIME_READINESS_POLICY_ENVELOPE_SCHEMA
        || value.canonicalization != PROVIDER_RUNTIME_READINESS_CANONICALIZATION
        || value.digest_algorithm != PROVIDER_RUNTIME_READINESS_DIGEST_ALGORITHM
        || provider_runtime_readiness_policy_digest(&value.policy)? != value.policy_digest
    {
        bail!("provider runtime readiness policy envelope is not exact")
    }
    Ok(())
}

pub(crate) fn validate_provider_runtime_readiness_material(
    value: &ExternalPoolAdapterProviderRuntimeReadinessMaterial,
) -> Result<()> {
    identifiers([
        &value.provider_binding_id,
        &value.registry_release_id,
        &value.installation_receipt_id,
        &value.candidate_id,
        &value.delegation_id,
        &value.profile_id,
        &value.target_id,
        &value.companion_id,
        &value.provider_id,
        &value.vulnerability_reattestation_receipt_id,
        &value.sandbox_reattestation_receipt_id,
        &value.credential_reattestation_receipt_id,
        &value.runtime_compatibility_verification_receipt_id,
        &value.probe_execution_id,
        &value.recorded_by_actor_user_id,
        &value.idempotency_scope,
        &value.idempotency_key,
    ])?;
    digests([
        &value.policy_digest,
        &value.provider_binding_digest,
        &value.registry_release_digest,
        &value.registry_release_material_digest,
        &value.installation_receipt_digest,
        &value.installation_content_digest,
        &value.candidate_digest,
        &value.delegation_digest,
        &value.profile_digest,
        &value.target_digest,
        &value.companion_digest,
        &value.provider_digest,
        &value.vulnerability_reattestation_receipt_digest,
        &value.sandbox_reattestation_receipt_digest,
        &value.credential_reattestation_receipt_digest,
        &value.runtime_compatibility_verification_receipt_digest,
        &value.launch_policy_digest,
        &value.target_policy_digest,
        &value.entrypoint_capsule_policy_digest,
        &value.supervisor_session_policy_digest,
        &value.source_capsule_sha256,
        &value.launch_image_sha256,
        &value.sealed_bindings.runtime_custody_epoch_digest,
        &value.sealed_bindings.runtime_bundle_identity_commitment,
        &value.sealed_bindings.post_cleanup_observation_commitment,
    ])?;
    optional_predecessor(value)?;
    validate_material_policy_and_constants(value)?;
    validate_material_timestamps(value)?;
    Ok(())
}

pub(crate) fn validate_provider_runtime_readiness_receipt(
    receipt: &ExternalPoolAdapterProviderRuntimeReadinessReceipt,
) -> Result<()> {
    metadata(
        &receipt.schema,
        PROVIDER_RUNTIME_READINESS_RECEIPT_SCHEMA,
        &receipt.readiness_receipt_id,
        &receipt.readiness_receipt_digest,
        &receipt.readiness_material_digest,
        &receipt.canonicalization,
        &receipt.digest_algorithm,
    )?;
    validate_provider_runtime_readiness_material(&receipt.readiness)?;
    if provider_runtime_readiness_material_digest(&receipt.readiness)?
        != receipt.readiness_material_digest
        || canonical_provider_runtime_readiness_receipt_json_and_digest(receipt)?.1
            != receipt.readiness_receipt_digest
    {
        bail!("provider runtime readiness receipt digest is not exact")
    }
    Ok(())
}

pub(crate) fn validate_provider_runtime_readiness_revocation_material(
    value: &ExternalPoolAdapterProviderRuntimeReadinessRevocationMaterial,
) -> Result<()> {
    identifiers([
        &value.readiness_receipt_id,
        &value.provider_binding_id,
        &value.candidate_id,
        &value.profile_id,
        &value.target_id,
        &value.companion_id,
        &value.provider_id,
        &value.revoked_by_actor_user_id,
        &value.idempotency_scope,
        &value.idempotency_key,
    ])?;
    digests([
        &value.readiness_receipt_digest,
        &value.provider_binding_digest,
        &value.candidate_digest,
        &value.profile_digest,
        &value.target_digest,
        &value.companion_digest,
    ])?;
    reason(&value.reason)?;
    let expected_scope = format!(
        "v270:provider-runtime-readiness:revoke:{}:{}",
        value.revoked_by_actor_kind, value.revoked_by_actor_user_id
    );
    if !matches!(
        value.revoked_by_actor_kind.as_str(),
        PROVIDER_RUNTIME_READINESS_ACTOR_PROVIDER_OWNER
            | PROVIDER_RUNTIME_READINESS_ACTOR_PLATFORM_ADMIN
    ) || value.idempotency_scope != expected_scope
        || value.revoked_at != value.recorded_at
        || value.confirmation != PROVIDER_RUNTIME_READINESS_REVOCATION_CONFIRMATION
        || value.revocation_status != PROVIDER_RUNTIME_READINESS_REVOCATION_STATUS
        || value.effects != provider_runtime_readiness_no_effects()
        || value.readiness != provider_runtime_readiness_no_readiness()
    {
        bail!("provider runtime readiness revocation material is not exact")
    }
    canonical_nanos(&value.revoked_at)?;
    Ok(())
}

pub(crate) fn validate_provider_runtime_readiness_revocation_receipt(
    receipt: &ExternalPoolAdapterProviderRuntimeReadinessRevocationReceipt,
) -> Result<()> {
    metadata(
        &receipt.schema,
        PROVIDER_RUNTIME_READINESS_REVOCATION_RECEIPT_SCHEMA,
        &receipt.revocation_receipt_id,
        &receipt.revocation_receipt_digest,
        &receipt.revocation_material_digest,
        &receipt.canonicalization,
        &receipt.digest_algorithm,
    )?;
    validate_provider_runtime_readiness_revocation_material(&receipt.revocation)?;
    if provider_runtime_readiness_revocation_material_digest(&receipt.revocation)?
        != receipt.revocation_material_digest
        || canonical_provider_runtime_readiness_revocation_json_and_digest(receipt)?.1
            != receipt.revocation_receipt_digest
    {
        bail!("provider runtime readiness revocation receipt digest is not exact")
    }
    Ok(())
}

fn validate_material_policy_and_constants(
    value: &ExternalPoolAdapterProviderRuntimeReadinessMaterial,
) -> Result<()> {
    let policy = server_provider_runtime_readiness_policy_catalog()?;
    let expected_scope = format!(
        "v270:provider-runtime-readiness:create:{}",
        value.recorded_by_actor_user_id
    );
    let seals = &value.sealed_bindings;
    if value.policy_id != policy.policy.policy_id
        || value.policy_revision != policy.policy.policy_revision
        || value.policy_digest != policy.policy_digest
        || value.provider_status != PROVIDER_STATUS_REGISTERING
        || value.provider_policy_revision <= 0
        || value.source_capsule_size_bytes == 0
        || value.launch_image_size_bytes == 0
        || value.source_capsule_sha256 == value.launch_image_sha256
        || !(1..=PROVIDER_RUNTIME_READINESS_MAX_REQUEST_BYTES).contains(&value.request_bytes)
        || !(1..=PROVIDER_RUNTIME_READINESS_MAX_RESPONSE_BYTES).contains(&value.response_bytes)
        || value.sequence == 0
        || value.recorded_by_actor_kind != PROVIDER_RUNTIME_READINESS_ACTOR_PLATFORM_ADMIN
        || value.idempotency_scope != expected_scope
        || value.confirmation != PROVIDER_RUNTIME_READINESS_CONFIRMATION
        || value.evidence_scope != PROVIDER_RUNTIME_READINESS_EVIDENCE_SCOPE
        || value.receipt_status != PROVIDER_RUNTIME_READINESS_RECEIPT_STATUS
        || value.effects != provider_runtime_readiness_no_effects()
        || value.observed_readiness != provider_runtime_readiness_observed_readiness()
        || seals.runtime_custody_epoch_digest == seals.runtime_bundle_identity_commitment
        || seals.runtime_custody_epoch_digest == seals.post_cleanup_observation_commitment
        || seals.runtime_bundle_identity_commitment == seals.post_cleanup_observation_commitment
    {
        bail!("provider runtime readiness material constants are not exact")
    }
    Ok(())
}

fn validate_material_timestamps(
    value: &ExternalPoolAdapterProviderRuntimeReadinessMaterial,
) -> Result<()> {
    let probe = canonical_nanos(&value.probe_checked_at)?;
    let cleanup = canonical_nanos(&value.cleanup_completed_at)?;
    let checked = canonical_nanos(&value.checked_at)?;
    let expires = canonical_nanos(&value.expires_at)?;
    if value.recorded_at != value.checked_at
        || probe > cleanup
        || cleanup > checked
        || expires <= checked
        || expires
            > checked
                + Duration::milliseconds(PROVIDER_RUNTIME_READINESS_MAX_PROBE_TIMEOUT_MS as i64)
    {
        bail!("provider runtime readiness timestamps exceed the frozen observation window")
    }
    Ok(())
}

fn optional_predecessor(value: &ExternalPoolAdapterProviderRuntimeReadinessMaterial) -> Result<()> {
    let id = &value.predecessor_readiness_receipt_id;
    let digest = &value.predecessor_readiness_receipt_digest;
    if id.is_some() != digest.is_some() || (value.sequence == 1) != id.is_none() {
        bail!("provider runtime readiness predecessor pair is incomplete")
    }
    if let Some(id) = id {
        identifiers([id])?;
    }
    if let Some(digest) = digest {
        digests([digest])?;
    }
    Ok(())
}

fn reason(value: &str) -> Result<()> {
    if !(12..=500).contains(&value.chars().count())
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        bail!("provider runtime readiness revocation reason is invalid")
    }
    Ok(())
}

fn identifiers<const N: usize>(values: [&str; N]) -> Result<()> {
    if values.into_iter().any(|value| {
        value.is_empty()
            || value.trim() != value
            || value.chars().count() > 240
            || value.chars().any(char::is_control)
    }) {
        bail!("provider runtime readiness identifier is invalid")
    }
    Ok(())
}

fn digests<const N: usize>(values: [&str; N]) -> Result<()> {
    if values.into_iter().any(|value| {
        value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    }) {
        bail!("provider runtime readiness digest is invalid")
    }
    Ok(())
}

fn canonical_nanos(value: &str) -> Result<DateTime<FixedOffset>> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("provider runtime readiness timestamp is not canonical UTC nanos")
    }
    Ok(parsed)
}

fn metadata(
    schema: &str,
    expected_schema: &str,
    id: &str,
    digest: &str,
    material_digest: &str,
    canonicalization: &str,
    digest_algorithm: &str,
) -> Result<()> {
    identifiers([id])?;
    digests([digest, material_digest])?;
    if schema != expected_schema
        || canonicalization != PROVIDER_RUNTIME_READINESS_CANONICALIZATION
        || digest_algorithm != PROVIDER_RUNTIME_READINESS_DIGEST_ALGORITHM
    {
        bail!("provider runtime readiness receipt metadata is invalid")
    }
    Ok(())
}

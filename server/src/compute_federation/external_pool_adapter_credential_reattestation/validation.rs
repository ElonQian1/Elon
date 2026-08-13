use anyhow::{bail, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{DateTime, Duration, SecondsFormat};
use sha2::{Digest, Sha256};

use crate::compute_federation::{
    external_pool_adapter_credential_verification::{
        validate_credential_verification_draft, ExternalPoolAdapterCredentialVerificationDraft,
    },
    provider::{PROVIDER_KIND_EXTERNAL_POOL, PROVIDER_STATUS_ACTIVE, PROVIDER_STATUS_REGISTERING},
};

use super::{canonical::*, types::*};

pub(crate) fn validate_credential_reattestation_binding(
    binding: &ExternalPoolAdapterCredentialReattestationBinding,
) -> Result<()> {
    if binding.schema != CREDENTIAL_REATTESTATION_BINDING_SCHEMA
        || binding.signature_algorithm != CREDENTIAL_REATTESTATION_SIGNATURE_ALGORITHM
        || binding.verification_policy_id != CREDENTIAL_REATTESTATION_POLICY_ID
        || binding.provider_kind != PROVIDER_KIND_EXTERNAL_POOL
        || binding.observed_provider_status != PROVIDER_STATUS_REGISTERING
            && binding.observed_provider_status != PROVIDER_STATUS_ACTIVE
        || binding.observed_provider_policy_revision < 1
        || binding.adapter_config_revision < 1
        || binding.sequence == 0
        || binding.predecessor_receipt_id.is_some() != binding.predecessor_receipt_digest.is_some()
        || (binding.sequence == 1) != binding.predecessor_receipt_id.is_none()
        || binding.credential_ref_scheme != "vault_ref"
            && binding.credential_ref_scheme != "gateway_ref"
    {
        bail!("credential re-attestation binding policy is invalid");
    }
    for value in identifiers(binding) {
        identifier(value, 200)?;
    }
    for value in digests(binding) {
        digest(value)?;
    }
    text(&binding.adapter_config_digest, 1, 512)?;
    if let Some(value) = binding.predecessor_receipt_id.as_deref() {
        identifier(value, 200)?;
    }
    if let Some(value) = binding.predecessor_receipt_digest.as_deref() {
        digest(value)?;
    }
    validate_challenge_material(binding)?;
    validate_credential_verification_draft(&draft(binding))?;
    if binding.credential_verifier_digest != binding.expected_credential_verifier.verifier_digest {
        bail!("credential re-attestation verifier digest lineage is not exact");
    }
    Ok(())
}

pub(crate) fn validate_credential_reattestation_receipt(
    receipt: &ExternalPoolAdapterCredentialReattestationReceipt,
) -> Result<()> {
    if receipt.schema != CREDENTIAL_REATTESTATION_RECEIPT_SCHEMA
        || receipt.canonicalization != CREDENTIAL_REATTESTATION_CANONICALIZATION
        || receipt.digest_algorithm != CREDENTIAL_REATTESTATION_DIGEST_ALGORITHM
    {
        bail!("credential re-attestation receipt metadata is unsupported");
    }
    identifier(&receipt.reattestation_receipt_id, 200)?;
    digest(&receipt.reattestation_receipt_digest)?;
    digest(&receipt.reattestation_material_digest)?;
    validate_credential_reattestation_binding(&receipt.reattestation.binding)?;
    let item = &receipt.reattestation;
    digest(&item.signature_message_digest)?;
    validate_signature(&item.signature_base64, &item.signature_digest)?;
    identifier(&item.recorded_by_admin_user_id, 200)?;
    identifier(&item.idempotency_scope, 240)?;
    identifier(&item.idempotency_key, 240)?;
    let verified = canonical_nanos(&item.verified_at)?;
    let recorded = canonical_nanos(&item.recorded_at)?;
    let generated = canonical_nanos(&item.binding.report_generated_at)?;
    let challenge_expires = canonical_nanos(&item.binding.challenge_expires_at)?;
    let report_expires = canonical_nanos(&item.binding.report_expires_at)?;
    let challenge = credential_reattestation_challenge(item.binding.clone())?;
    if recorded != verified
        || item.recorded_at != item.verified_at
        || verified < generated
        || verified >= challenge_expires
        || verified >= report_expires
        || item.signature_message_digest != challenge.signature_message_digest
        || item.confirmation != CREDENTIAL_REATTESTATION_CONFIRMATION
        || item.evidence_scope != CREDENTIAL_REATTESTATION_EVIDENCE_SCOPE
        || item.credential_reattestation_effect != CREDENTIAL_REATTESTATION_EFFECT
        || !effects_are_none(item)
        || credential_reattestation_material_digest(item)? != receipt.reattestation_material_digest
    {
        bail!("credential re-attestation receipt effects or material are invalid");
    }
    let (_, digest_value) = credential_reattestation_receipt_json_and_digest(receipt)?;
    if digest_value != receipt.reattestation_receipt_digest {
        bail!("credential re-attestation receipt digest is not self-rooting");
    }
    Ok(())
}

pub(crate) fn validate_credential_reattestation_revocation_receipt(
    receipt: &ExternalPoolAdapterCredentialReattestationRevocationReceipt,
) -> Result<()> {
    if receipt.schema != CREDENTIAL_REATTESTATION_REVOCATION_RECEIPT_SCHEMA
        || receipt.canonicalization != CREDENTIAL_REATTESTATION_CANONICALIZATION
        || receipt.digest_algorithm != CREDENTIAL_REATTESTATION_DIGEST_ALGORITHM
    {
        bail!("credential re-attestation revocation metadata is unsupported");
    }
    identifier(&receipt.revocation_receipt_id, 200)?;
    digest(&receipt.revocation_receipt_digest)?;
    digest(&receipt.revocation_material_digest)?;
    let item = &receipt.revocation;
    identifier(&item.reattestation_receipt_id, 200)?;
    identifier(&item.provider_binding_id, 200)?;
    identifier(&item.revoked_by_admin_user_id, 200)?;
    identifier(&item.idempotency_scope, 240)?;
    identifier(&item.idempotency_key, 240)?;
    for value in [
        &item.reattestation_receipt_digest,
        &item.provider_binding_digest,
    ] {
        digest(value)?;
    }
    if item.reason.trim() != item.reason
        || item.reason.chars().count() < 12
        || item.reason.chars().count() > 500
        || item.reason.chars().any(char::is_control)
    {
        bail!("credential re-attestation revocation reason is invalid");
    }
    canonical_nanos(&item.revoked_at)?;
    canonical_nanos(&item.recorded_at)?;
    if item.recorded_at != item.revoked_at
        || item.confirmation != CREDENTIAL_REATTESTATION_REVOCATION_CONFIRMATION
        || item.revocation_effect != CREDENTIAL_REATTESTATION_REVOCATION_EFFECT
        || !revocation_effects_are_none(item)
        || credential_reattestation_revocation_material_digest(item)?
            != receipt.revocation_material_digest
    {
        bail!("credential re-attestation revocation effects or material are invalid");
    }
    let (_, digest_value) = credential_reattestation_revocation_receipt_json_and_digest(receipt)?;
    if digest_value != receipt.revocation_receipt_digest {
        bail!("credential re-attestation revocation digest is not self-rooting");
    }
    Ok(())
}

fn validate_challenge_material(
    binding: &ExternalPoolAdapterCredentialReattestationBinding,
) -> Result<()> {
    let nonce = STANDARD.decode(&binding.challenge_nonce_base64)?;
    let issued = canonical_nanos(&binding.challenge_issued_at)?;
    let expires = canonical_nanos(&binding.challenge_expires_at)?;
    if nonce.len() != 32
        || STANDARD.encode(&nonce) != binding.challenge_nonce_base64
        || hex::encode(Sha256::digest(&nonce)) != binding.challenge_nonce_digest
        || expires - issued
            != Duration::minutes(CREDENTIAL_REATTESTATION_CHALLENGE_VALIDITY_MINUTES)
    {
        bail!("credential re-attestation challenge material is invalid");
    }
    Ok(())
}

fn draft(
    binding: &ExternalPoolAdapterCredentialReattestationBinding,
) -> ExternalPoolAdapterCredentialVerificationDraft {
    ExternalPoolAdapterCredentialVerificationDraft {
        verifier_report_id: binding.verifier_report_id.clone(),
        verification_started_at: binding.verification_started_at.clone(),
        verification_completed_at: binding.verification_completed_at.clone(),
        report_generated_at: binding.report_generated_at.clone(),
        report_expires_at: binding.report_expires_at.clone(),
        credential_resolution_outcome: binding.credential_resolution_outcome.clone(),
        provider_authentication_outcome: binding.provider_authentication_outcome.clone(),
        provider_response_evidence_digest: binding.provider_response_evidence_digest.clone(),
    }
}

fn identifiers(binding: &ExternalPoolAdapterCredentialReattestationBinding) -> [&str; 25] {
    [
        &binding.challenge_id,
        &binding.provider_binding_id,
        &binding.registry_release_id,
        &binding.route_adapter_projection_id,
        &binding.installation_receipt_id,
        &binding.application_id,
        &binding.adoption_receipt_id,
        &binding.provider_id,
        &binding.provider_kind,
        &binding.provider_owner_account_id,
        &binding.observed_settlement_account_id,
        &binding.observed_provider_status,
        &binding.adapter_id,
        &binding.release_version,
        &binding.admission_id,
        &binding.legacy_credential_verification_receipt_id,
        &binding.credential_ref_scheme,
        &binding.credential_verifier_key_record_id,
        &binding.credential_verifier_key_id,
        &binding.credential_verifier_record_id,
        &binding.signature_algorithm,
        &binding.verification_policy_id,
        &binding.verifier_report_id,
        &binding.credential_resolution_outcome,
        &binding.provider_authentication_outcome,
    ]
}

fn digests(binding: &ExternalPoolAdapterCredentialReattestationBinding) -> [&str; 19] {
    [
        &binding.challenge_nonce_digest,
        &binding.provider_binding_digest,
        &binding.provider_binding_material_digest,
        &binding.registry_release_digest,
        &binding.registry_release_material_digest,
        &binding.installation_receipt_digest,
        &binding.installation_content_digest,
        &binding.application_digest,
        &binding.adoption_receipt_digest,
        &binding.observed_provider_digest,
        &binding.admission_digest,
        &binding.legacy_credential_verification_receipt_digest,
        &binding.credential_locator_commitment,
        &binding.expected_credential_verifier.verifier_digest,
        &binding.credential_verifier_digest,
        &binding.credential_verifier_key_record_digest,
        &binding.credential_verifier_key_id,
        &binding.credential_verifier_record_digest,
        &binding.provider_response_evidence_digest,
    ]
}

fn validate_signature(value: &str, expected_digest: &str) -> Result<()> {
    digest(expected_digest)?;
    let bytes = STANDARD.decode(value)?;
    if bytes.is_empty()
        || bytes.len() > 1024
        || STANDARD.encode(&bytes) != value
        || hex::encode(Sha256::digest(&bytes)) != expected_digest
    {
        bail!("credential re-attestation signature material is invalid");
    }
    Ok(())
}

fn effects_are_none(item: &ExternalPoolAdapterCredentialReattestationMaterial) -> bool {
    [
        &item.adapter_effect,
        &item.provider_effect,
        &item.route_effect,
        &item.execution_effect,
        &item.usage_effect,
        &item.settlement_effect,
    ]
    .into_iter()
    .all(|value| value == CREDENTIAL_REATTESTATION_NO_EFFECT)
}

fn revocation_effects_are_none(
    item: &ExternalPoolAdapterCredentialReattestationRevocationMaterial,
) -> bool {
    [
        &item.adapter_effect,
        &item.provider_effect,
        &item.route_effect,
        &item.execution_effect,
        &item.usage_effect,
        &item.settlement_effect,
    ]
    .into_iter()
    .all(|value| value == CREDENTIAL_REATTESTATION_NO_EFFECT)
}

fn identifier(value: &str, max: usize) -> Result<()> {
    text(value, 1, max)
}

fn text(value: &str, min: usize, max: usize) -> Result<()> {
    if value.trim() != value
        || !(min..=max).contains(&value.chars().count())
        || value.chars().any(char::is_control)
    {
        bail!("credential re-attestation text is invalid");
    }
    Ok(())
}

fn digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("credential re-attestation digest is invalid");
    }
    Ok(())
}

fn canonical_nanos(value: &str) -> Result<DateTime<chrono::FixedOffset>> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("credential re-attestation timestamp is not canonical UTC nanoseconds");
    }
    Ok(parsed)
}

use anyhow::{bail, Result};
use chrono::{DateTime, Duration, SecondsFormat};

use super::{canonical::*, types::*};

pub(crate) fn validate_credential_verification_draft(
    draft: &ExternalPoolAdapterCredentialVerificationDraft,
) -> Result<()> {
    identifier(&draft.verifier_report_id, 200)?;
    digest(&draft.provider_response_evidence_digest)?;
    if draft.credential_resolution_outcome != "passed"
        || draft.provider_authentication_outcome != "passed"
    {
        bail!("credential verification draft contains a non-passing outcome");
    }
    let started = canonical_nanos(&draft.verification_started_at)?;
    let completed = canonical_nanos(&draft.verification_completed_at)?;
    let generated = canonical_nanos(&draft.report_generated_at)?;
    let expires = canonical_nanos(&draft.report_expires_at)?;
    if completed < started
        || completed - started > Duration::minutes(MAX_CREDENTIAL_VERIFICATION_RUN_MINUTES)
        || generated < completed
        || generated - completed
            > Duration::minutes(MAX_CREDENTIAL_VERIFICATION_REPORT_DELAY_MINUTES)
        || expires <= generated
        || expires - generated > Duration::minutes(MAX_CREDENTIAL_VERIFICATION_VALIDITY_MINUTES)
    {
        bail!("credential verification runtime/report window is invalid");
    }
    Ok(())
}

pub(crate) fn validate_credential_verification_binding(
    binding: &ExternalPoolAdapterCredentialVerificationBinding,
) -> Result<()> {
    if binding.schema != CREDENTIAL_VERIFICATION_BINDING_SCHEMA
        || binding.signature_algorithm != CREDENTIAL_VERIFICATION_SIGNATURE_ALGORITHM
        || binding.verification_policy_id != CREDENTIAL_VERIFICATION_POLICY_ID
        || binding.provider_kind != "external_pool"
        || binding.provider_status != "registering"
        || binding.credential_ref_scheme != "vault_ref"
            && binding.credential_ref_scheme != "gateway_ref"
        || binding.provider_policy_revision < 1
        || binding.adapter_config_revision < 1
    {
        bail!("credential verification binding policy is invalid");
    }
    for value in [
        &binding.application_id,
        &binding.provider_id,
        &binding.provider_owner_account_id,
        &binding.settlement_account_id,
        &binding.adapter_id,
        &binding.adapter_release_version,
        &binding.admission_id,
        &binding.credential_verifier_key_record_id,
        &binding.credential_verifier_key_id,
        &binding.credential_verifier_record_id,
    ] {
        identifier(value, 200)?;
    }
    for value in [
        &binding.application_digest,
        &binding.provider_digest,
        &binding.credential_locator_commitment,
        &binding.admission_digest,
        &binding.declared_implementation_sha256,
        &binding.capability_set_digest,
        &binding.expected_credential_verifier.verifier_digest,
        &binding.credential_verifier_key_record_digest,
        &binding.credential_verifier_key_id,
        &binding.credential_verifier_record_digest,
        &binding.provider_response_evidence_digest,
    ] {
        digest(value)?;
    }
    canonical_nanos(&binding.onboarding_applied_at)?;
    canonical_nanos(&binding.admission_applied_at)?;
    validate_credential_verification_draft(&ExternalPoolAdapterCredentialVerificationDraft {
        verifier_report_id: binding.verifier_report_id.clone(),
        verification_started_at: binding.verification_started_at.clone(),
        verification_completed_at: binding.verification_completed_at.clone(),
        report_generated_at: binding.report_generated_at.clone(),
        report_expires_at: binding.report_expires_at.clone(),
        credential_resolution_outcome: binding.credential_resolution_outcome.clone(),
        provider_authentication_outcome: binding.provider_authentication_outcome.clone(),
        provider_response_evidence_digest: binding.provider_response_evidence_digest.clone(),
    })
}

pub(crate) fn validate_credential_verification_receipt(
    receipt: &ExternalPoolAdapterCredentialVerificationReceipt,
) -> Result<()> {
    if receipt.schema != CREDENTIAL_VERIFICATION_RECEIPT_SCHEMA
        || receipt.canonicalization != CREDENTIAL_VERIFICATION_CANONICALIZATION
        || receipt.digest_algorithm != CREDENTIAL_VERIFICATION_DIGEST_ALGORITHM
    {
        bail!("credential verification receipt metadata is unsupported");
    }
    identifier(&receipt.credential_verification_receipt_id, 200)?;
    digest(&receipt.credential_verification_receipt_digest)?;
    digest(&receipt.verification_material_digest)?;
    validate_credential_verification_binding(&receipt.verification.binding)?;
    let material = &receipt.verification;
    digest(&material.signature_message_digest)?;
    digest(&material.signature_digest)?;
    identifier(&material.recorded_by_admin_user_id, 200)?;
    identifier(&material.idempotency_scope, 240)?;
    identifier(&material.idempotency_key, 240)?;
    canonical_nanos(&material.verified_at)?;
    canonical_nanos(&material.recorded_at)?;
    if material.recorded_at != material.verified_at
        || material.confirmation != CREDENTIAL_VERIFICATION_CONFIRMATION
        || material.evidence_scope != CREDENTIAL_VERIFICATION_EVIDENCE_SCOPE
        || material.credential_effect != CREDENTIAL_VERIFICATION_EFFECT
        || material.adapter_effect != CREDENTIAL_VERIFICATION_NO_EFFECT
        || material.route_effect != CREDENTIAL_VERIFICATION_NO_EFFECT
        || material.execution_effect != CREDENTIAL_VERIFICATION_NO_EFFECT
        || material.settlement_effect != CREDENTIAL_VERIFICATION_NO_EFFECT
        || credential_verification_material_digest(material)?
            != receipt.verification_material_digest
    {
        bail!("credential verification receipt effects or material digest are invalid");
    }
    Ok(())
}

fn identifier(value: &str, max: usize) -> Result<()> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > max
        || value.chars().any(char::is_control)
    {
        bail!("credential verification identifier is invalid");
    }
    Ok(())
}

fn digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("credential verification digest is invalid");
    }
    Ok(())
}

fn canonical_nanos(value: &str) -> Result<DateTime<chrono::FixedOffset>> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value {
        bail!("credential verification timestamp is not canonical UTC nanoseconds");
    }
    Ok(parsed)
}

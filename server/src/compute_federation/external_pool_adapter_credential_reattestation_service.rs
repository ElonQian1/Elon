//! Administrator orchestration and public redaction for V253 credential re-attestation.

use anyhow::Error as AnyError;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::store::{
    CreateExternalPoolAdapterCredentialReattestation,
    GetExternalPoolAdapterCredentialReattestationChallenge,
    RevokeExternalPoolAdapterCredentialReattestation, Store,
};

use super::{
    external_pool_adapter_credential_reattestation::{
        ExternalPoolAdapterCredentialReattestationChallenge, CREDENTIAL_REATTESTATION_CONFIRMATION,
        CREDENTIAL_REATTESTATION_REVOCATION_CONFIRMATION,
    },
    external_pool_adapter_credential_verification::{
        validate_credential_verification_draft, ExternalPoolAdapterCredentialVerificationDraft,
    },
};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CredentialReattestationChallengeBody {
    pub expected_provider_binding_digest: String,
    pub expected_registry_release_digest: String,
    pub credential_verifier_key_record_id: String,
    pub expected_credential_verifier_key_record_digest: String,
    pub expected_credential_verifier_key_id: String,
    pub verifier_report_id: String,
    pub verification_started_at: String,
    pub verification_completed_at: String,
    pub report_generated_at: String,
    pub report_expires_at: String,
    pub credential_resolution_outcome: String,
    pub provider_authentication_outcome: String,
    pub provider_response_evidence_digest: String,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecordCredentialReattestationBody {
    pub challenge_id: String,
    pub expected_signature_message_digest: String,
    pub signature_base64: String,
    pub idempotency_key: String,
    pub confirm_reattestation: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RevokeCredentialReattestationBody {
    pub expected_reattestation_receipt_digest: String,
    pub reason: String,
    pub idempotency_key: String,
    pub confirm_revocation: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct CredentialReattestationChallengeResponse {
    pub schema: &'static str,
    pub canonicalization: &'static str,
    pub digest_algorithm: &'static str,
    pub signature_algorithm: &'static str,
    pub signature_message_base64: String,
    pub signature_message_digest: String,
    pub binding: Value,
}

#[derive(Debug, Error)]
pub(crate) enum CredentialReattestationServiceError {
    #[error("external-pool Adapter credential re-attestation was not found")]
    NotFound,
    #[error("external-pool Adapter credential re-attestation request is invalid")]
    Invalid(#[source] AnyError),
    #[error("external-pool Adapter credential re-attestation authority conflicts")]
    Conflict(#[source] AnyError),
}

pub(crate) fn challenge_for_admin(
    store: &Store,
    provider_binding_id: &str,
    body: CredentialReattestationChallengeBody,
) -> Result<CredentialReattestationChallengeResponse, CredentialReattestationServiceError> {
    validate_identifier(provider_binding_id, 200, "Provider binding ID")?;
    validate_identifier(
        &body.credential_verifier_key_record_id,
        200,
        "credential verifier key record ID",
    )?;
    for (value, label) in [
        (
            &body.expected_provider_binding_digest,
            "Provider binding digest",
        ),
        (
            &body.expected_registry_release_digest,
            "registry release digest",
        ),
        (
            &body.expected_credential_verifier_key_record_digest,
            "credential verifier key record digest",
        ),
        (
            &body.expected_credential_verifier_key_id,
            "credential verifier key ID",
        ),
    ] {
        validate_digest(value, label)?;
    }
    let draft = draft_from_body(&body);
    validate_credential_verification_draft(&draft)
        .map_err(CredentialReattestationServiceError::Invalid)?;
    require_provider_binding(store, provider_binding_id)?;
    let challenge = store
        .issue_external_pool_adapter_credential_reattestation_challenge(
            GetExternalPoolAdapterCredentialReattestationChallenge {
                provider_binding_id: provider_binding_id.to_string(),
                expected_provider_binding_digest: body.expected_provider_binding_digest,
                expected_registry_release_digest: body.expected_registry_release_digest,
                credential_verifier_key_record_id: body.credential_verifier_key_record_id,
                expected_credential_verifier_key_record_digest: body
                    .expected_credential_verifier_key_record_digest,
                expected_credential_verifier_key_id: body.expected_credential_verifier_key_id,
                draft,
            },
        )
        .map_err(CredentialReattestationServiceError::Conflict)?;
    public_challenge(challenge)
}

pub(crate) fn record_for_admin(
    store: &Store,
    admin_user_id: &str,
    provider_binding_id: &str,
    body: RecordCredentialReattestationBody,
) -> Result<Value, CredentialReattestationServiceError> {
    if !body.confirm_reattestation {
        return Err(invalid(
            "credential re-attestation requires explicit confirmation",
        ));
    }
    for (value, maximum, label) in [
        (admin_user_id, 200, "administrator user ID"),
        (provider_binding_id, 200, "Provider binding ID"),
        (body.challenge_id.as_str(), 200, "challenge ID"),
        (body.idempotency_key.as_str(), 240, "idempotency key"),
    ] {
        validate_identifier(value, maximum, label)?;
    }
    validate_digest(
        &body.expected_signature_message_digest,
        "signature message digest",
    )?;
    validate_signature(&body.signature_base64)?;
    require_provider_binding(store, provider_binding_id)?;
    if !store
        .external_pool_adapter_credential_reattestation_challenge_exists(
            &body.challenge_id,
            provider_binding_id,
        )
        .map_err(CredentialReattestationServiceError::Conflict)?
    {
        return Err(CredentialReattestationServiceError::NotFound);
    }
    let receipt = store
        .create_external_pool_adapter_credential_reattestation(
            CreateExternalPoolAdapterCredentialReattestation {
                challenge_id: body.challenge_id,
                expected_signature_message_digest: body.expected_signature_message_digest,
                signature_base64: body.signature_base64,
                recorded_by_admin_user_id: admin_user_id.to_string(),
                confirmation: CREDENTIAL_REATTESTATION_CONFIRMATION.to_string(),
                idempotency_scope: format!("v253:credential-reattest:{admin_user_id}"),
                idempotency_key: body.idempotency_key,
            },
        )
        .map_err(CredentialReattestationServiceError::Conflict)?;
    redacted_json(receipt)
}

pub(crate) fn currentness_for_admin(
    store: &Store,
    provider_binding_id: &str,
) -> Result<Value, CredentialReattestationServiceError> {
    validate_identifier(provider_binding_id, 200, "Provider binding ID")?;
    require_provider_binding(store, provider_binding_id)?;
    let current = store
        .external_pool_adapter_credential_reattestation_currentness(provider_binding_id)
        .map_err(CredentialReattestationServiceError::Conflict)?
        .ok_or(CredentialReattestationServiceError::NotFound)?;
    if current.current_status != "verified_current" {
        return Err(CredentialReattestationServiceError::Conflict(
            anyhow::anyhow!("credential re-attestation is not current"),
        ));
    }
    redacted_json(current)
}

pub(crate) fn revoke_for_admin(
    store: &Store,
    admin_user_id: &str,
    provider_binding_id: &str,
    reattestation_receipt_id: &str,
    body: RevokeCredentialReattestationBody,
) -> Result<Value, CredentialReattestationServiceError> {
    if !body.confirm_revocation {
        return Err(invalid(
            "credential re-attestation revocation requires confirmation",
        ));
    }
    for (value, maximum, label) in [
        (admin_user_id, 200, "administrator user ID"),
        (provider_binding_id, 200, "Provider binding ID"),
        (reattestation_receipt_id, 200, "re-attestation receipt ID"),
        (body.idempotency_key.as_str(), 240, "idempotency key"),
    ] {
        validate_identifier(value, maximum, label)?;
    }
    validate_digest(
        &body.expected_reattestation_receipt_digest,
        "re-attestation receipt digest",
    )?;
    if body.reason.trim() != body.reason
        || !(12..=500).contains(&body.reason.chars().count())
        || body.reason.chars().any(char::is_control)
    {
        return Err(invalid(
            "credential re-attestation revocation reason is invalid",
        ));
    }
    require_provider_binding(store, provider_binding_id)?;
    if !store
        .external_pool_adapter_credential_reattestation_exists(
            reattestation_receipt_id,
            provider_binding_id,
        )
        .map_err(CredentialReattestationServiceError::Conflict)?
    {
        return Err(CredentialReattestationServiceError::NotFound);
    }
    let receipt = store
        .revoke_external_pool_adapter_credential_reattestation(
            RevokeExternalPoolAdapterCredentialReattestation {
                reattestation_receipt_id: reattestation_receipt_id.to_string(),
                expected_reattestation_receipt_digest: body.expected_reattestation_receipt_digest,
                revoked_by_admin_user_id: admin_user_id.to_string(),
                reason: body.reason,
                confirmation: CREDENTIAL_REATTESTATION_REVOCATION_CONFIRMATION.to_string(),
                idempotency_scope: format!("v253:credential-reattest-revoke:{admin_user_id}"),
                idempotency_key: body.idempotency_key,
            },
        )
        .map_err(CredentialReattestationServiceError::Conflict)?;
    redacted_json(receipt)
}

fn draft_from_body(
    body: &CredentialReattestationChallengeBody,
) -> ExternalPoolAdapterCredentialVerificationDraft {
    ExternalPoolAdapterCredentialVerificationDraft {
        verifier_report_id: body.verifier_report_id.clone(),
        verification_started_at: body.verification_started_at.clone(),
        verification_completed_at: body.verification_completed_at.clone(),
        report_generated_at: body.report_generated_at.clone(),
        report_expires_at: body.report_expires_at.clone(),
        credential_resolution_outcome: body.credential_resolution_outcome.clone(),
        provider_authentication_outcome: body.provider_authentication_outcome.clone(),
        provider_response_evidence_digest: body.provider_response_evidence_digest.clone(),
    }
}

fn public_challenge(
    challenge: ExternalPoolAdapterCredentialReattestationChallenge,
) -> Result<CredentialReattestationChallengeResponse, CredentialReattestationServiceError> {
    let mut binding = serde_json::to_value(challenge.binding)
        .map_err(|error| CredentialReattestationServiceError::Conflict(AnyError::new(error)))?;
    let nonce = binding.get("challenge_nonce_base64").cloned();
    let nonce_digest = binding.get("challenge_nonce_digest").cloned();
    redact(&mut binding);
    if let Value::Object(map) = &mut binding {
        if let Some(value) = nonce {
            map.insert("nonce_base64".to_string(), value);
        }
        if let Some(value) = nonce_digest {
            map.insert("nonce_digest".to_string(), value);
        }
    }
    Ok(CredentialReattestationChallengeResponse {
        schema: challenge.schema,
        canonicalization: challenge.canonicalization,
        digest_algorithm: challenge.digest_algorithm,
        signature_algorithm: challenge.signature_algorithm,
        signature_message_base64: challenge.signature_message_base64,
        signature_message_digest: challenge.signature_message_digest,
        binding,
    })
}

fn redacted_json<T: Serialize>(value: T) -> Result<Value, CredentialReattestationServiceError> {
    let mut value = serde_json::to_value(value)
        .map_err(|error| CredentialReattestationServiceError::Conflict(AnyError::new(error)))?;
    redact(&mut value);
    Ok(value)
}

fn redact(value: &mut Value) {
    const ALWAYS: &[&str] = &[
        "credential_ref",
        "non_bearer_credential_ref",
        "credential_ref_scheme",
        "credential_locator_commitment",
        "public_key_pem",
        "recorded_by_admin_user_id",
        "revoked_by_admin_user_id",
        "idempotency_scope",
        "idempotency_key",
        "confirmation",
        "receipt_json",
        "installation_path",
        "installation_root",
        "install_root",
        "installed_path",
        "entrypoint_path",
        "filesystem_path",
        "source_path",
        "archive_path",
        "path",
    ];
    const SIGNING_MATERIAL: &[&str] = &[
        "challenge_nonce_base64",
        "challenge_nonce_digest",
        "nonce_base64",
        "nonce_digest",
        "signature_message_base64",
        "signature_message_digest",
        "signature_base64",
        "signature_digest",
    ];
    match value {
        Value::Object(map) => {
            for key in ALWAYS {
                map.remove(*key);
            }
            for key in SIGNING_MATERIAL {
                map.remove(*key);
            }
            for child in map.values_mut() {
                redact(child);
            }
        }
        Value::Array(values) => values.iter_mut().for_each(redact),
        _ => {}
    }
}

fn validate_identifier(
    value: &str,
    maximum: usize,
    label: &'static str,
) -> Result<(), CredentialReattestationServiceError> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > maximum
        || value.chars().any(char::is_control)
    {
        return Err(invalid(format!("{label} is invalid")));
    }
    Ok(())
}

fn validate_digest(
    value: &str,
    label: &'static str,
) -> Result<(), CredentialReattestationServiceError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(invalid(format!("{label} is invalid")));
    }
    Ok(())
}

fn validate_signature(value: &str) -> Result<(), CredentialReattestationServiceError> {
    if value.is_empty() || value.len() > 1_368 {
        return Err(invalid("credential verifier signature Base64 is invalid"));
    }
    let signature = STANDARD
        .decode(value)
        .map_err(|error| CredentialReattestationServiceError::Invalid(AnyError::new(error)))?;
    if signature.is_empty() || signature.len() > 1024 || STANDARD.encode(&signature) != value {
        return Err(invalid("credential verifier signature Base64 is invalid"));
    }
    Ok(())
}

fn require_provider_binding(
    store: &Store,
    provider_binding_id: &str,
) -> Result<(), CredentialReattestationServiceError> {
    if store
        .external_pool_adapter_registry_provider_binding_audit_target(provider_binding_id)
        .map_err(CredentialReattestationServiceError::Conflict)?
        .is_some()
    {
        Ok(())
    } else {
        Err(CredentialReattestationServiceError::NotFound)
    }
}

fn invalid(message: impl Into<String>) -> CredentialReattestationServiceError {
    CredentialReattestationServiceError::Invalid(anyhow::anyhow!(message.into()))
}

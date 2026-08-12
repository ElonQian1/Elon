//! Administrator orchestration for short-lived signed external-pool credential evidence.

use anyhow::Error as AnyError;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Deserialize;
use thiserror::Error;

use crate::store::{
    CreateExternalPoolAdapterCredentialVerification,
    ExternalPoolAdapterCredentialVerificationCurrentness,
    ExternalPoolAdapterCredentialVerificationWriteReceipt,
    GetExternalPoolAdapterCredentialVerificationChallenge, Store,
};

use super::external_pool_adapter_credential_verification::{
    validate_credential_verification_draft, ExternalPoolAdapterCredentialVerificationChallenge,
    ExternalPoolAdapterCredentialVerificationDraft, CREDENTIAL_VERIFICATION_CONFIRMATION,
};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CredentialVerificationChallengeBody {
    pub application_id: String,
    pub expected_application_digest: String,
    pub admission_id: String,
    pub expected_admission_digest: String,
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
pub(crate) struct RecordCredentialVerificationBody {
    pub application_id: String,
    pub expected_application_digest: String,
    pub admission_id: String,
    pub expected_admission_digest: String,
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
    pub expected_signature_message_digest: String,
    pub signature_base64: String,
    pub idempotency_key: String,
    pub confirm_verification: bool,
}

#[derive(Debug, Error)]
pub(crate) enum CredentialVerificationServiceError {
    #[error("external-pool Adapter credential verification was not found")]
    NotFound,
    #[error("external-pool Adapter credential-verification request is invalid")]
    Invalid(#[source] AnyError),
    #[error("external-pool Adapter credential-verification lineage conflicts")]
    Conflict(#[source] AnyError),
}

pub(crate) fn challenge_for_admin(
    store: &Store,
    body: CredentialVerificationChallengeBody,
) -> Result<ExternalPoolAdapterCredentialVerificationChallenge, CredentialVerificationServiceError>
{
    let input = challenge_input(
        &body.application_id,
        &body.expected_application_digest,
        &body.admission_id,
        &body.expected_admission_digest,
        &body.credential_verifier_key_record_id,
        &body.expected_credential_verifier_key_record_digest,
        &body.expected_credential_verifier_key_id,
        draft_from_challenge(&body),
    );
    validate_challenge_request(&input)?;
    store
        .external_pool_adapter_credential_verification_challenge(input)
        .map_err(CredentialVerificationServiceError::Conflict)
}

pub(crate) fn record_for_admin(
    store: &Store,
    admin_user_id: &str,
    body: RecordCredentialVerificationBody,
) -> Result<ExternalPoolAdapterCredentialVerificationWriteReceipt, CredentialVerificationServiceError>
{
    if !body.confirm_verification {
        return Err(invalid(
            "recording credential verification requires explicit confirmation",
        ));
    }
    let challenge = challenge_input(
        &body.application_id,
        &body.expected_application_digest,
        &body.admission_id,
        &body.expected_admission_digest,
        &body.credential_verifier_key_record_id,
        &body.expected_credential_verifier_key_record_digest,
        &body.expected_credential_verifier_key_id,
        draft_from_record(&body),
    );
    let idempotency_scope =
        format!("external-pool-adapter-credential-verification:{admin_user_id}");
    validate_challenge_request(&challenge)?;
    validate_identifier(admin_user_id, 200)?;
    validate_identifier(&idempotency_scope, 240)?;
    validate_identifier(&body.idempotency_key, 240)?;
    validate_digest(&body.expected_signature_message_digest)?;
    validate_signature(&body.signature_base64)?;
    store
        .create_external_pool_adapter_credential_verification(
            CreateExternalPoolAdapterCredentialVerification {
                challenge,
                expected_signature_message_digest: body.expected_signature_message_digest,
                signature_base64: body.signature_base64,
                recorded_by_admin_user_id: admin_user_id.to_string(),
                confirmation: CREDENTIAL_VERIFICATION_CONFIRMATION.to_string(),
                idempotency_scope,
                idempotency_key: body.idempotency_key,
            },
        )
        .map_err(CredentialVerificationServiceError::Conflict)
}

pub(crate) fn currentness_for_admin(
    store: &Store,
    receipt_id: &str,
) -> Result<ExternalPoolAdapterCredentialVerificationCurrentness, CredentialVerificationServiceError>
{
    validate_identifier(receipt_id, 200)?;
    store
        .external_pool_adapter_credential_verification_currentness(receipt_id)
        .map_err(CredentialVerificationServiceError::Conflict)?
        .ok_or(CredentialVerificationServiceError::NotFound)
}

#[allow(clippy::too_many_arguments)]
fn challenge_input(
    application_id: &str,
    application_digest: &str,
    admission_id: &str,
    admission_digest: &str,
    key_record_id: &str,
    key_record_digest: &str,
    key_id: &str,
    draft: ExternalPoolAdapterCredentialVerificationDraft,
) -> GetExternalPoolAdapterCredentialVerificationChallenge {
    GetExternalPoolAdapterCredentialVerificationChallenge {
        application_id: application_id.to_string(),
        expected_application_digest: application_digest.to_string(),
        admission_id: admission_id.to_string(),
        expected_admission_digest: admission_digest.to_string(),
        credential_verifier_key_record_id: key_record_id.to_string(),
        expected_credential_verifier_key_record_digest: key_record_digest.to_string(),
        expected_credential_verifier_key_id: key_id.to_string(),
        draft,
    }
}

fn draft_from_challenge(
    body: &CredentialVerificationChallengeBody,
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

fn draft_from_record(
    body: &RecordCredentialVerificationBody,
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

fn validate_challenge_request(
    input: &GetExternalPoolAdapterCredentialVerificationChallenge,
) -> Result<(), CredentialVerificationServiceError> {
    for value in [
        &input.application_id,
        &input.admission_id,
        &input.credential_verifier_key_record_id,
    ] {
        validate_identifier(value, 200)?;
    }
    for value in [
        &input.expected_application_digest,
        &input.expected_admission_digest,
        &input.expected_credential_verifier_key_record_digest,
        &input.expected_credential_verifier_key_id,
    ] {
        validate_digest(value)?;
    }
    validate_credential_verification_draft(&input.draft)
        .map_err(CredentialVerificationServiceError::Invalid)
}

fn validate_identifier(value: &str, max: usize) -> Result<(), CredentialVerificationServiceError> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > max
        || value.chars().any(char::is_control)
    {
        return Err(invalid("credential verification identifier is invalid"));
    }
    Ok(())
}

fn validate_digest(value: &str) -> Result<(), CredentialVerificationServiceError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(invalid("credential verification digest is invalid"));
    }
    Ok(())
}

fn validate_signature(value: &str) -> Result<(), CredentialVerificationServiceError> {
    let signature = STANDARD
        .decode(value)
        .map_err(|error| CredentialVerificationServiceError::Invalid(AnyError::new(error)))?;
    if signature.is_empty() || signature.len() > 1024 || STANDARD.encode(&signature) != value {
        return Err(invalid("credential verifier signature Base64 is invalid"));
    }
    Ok(())
}

fn invalid(message: &'static str) -> CredentialVerificationServiceError {
    CredentialVerificationServiceError::Invalid(anyhow::anyhow!(message))
}

//! Strict caller-input validation for V268 runtime compatibility administrator routes.

use anyhow::Error as AnyError;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Deserialize;

use super::external_pool_adapter_runtime_compatibility_verification_service::RuntimeCompatibilityVerificationServiceError;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExpectedRuntimeCompatibilityVerificationPredecessor {
    pub verification_receipt_id: String,
    pub verification_receipt_digest: String,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateRuntimeCompatibilityChallengeBody {
    pub expected_registry_release_digest: String,
    pub sandbox_verifier_key_record_id: String,
    pub expected_sandbox_verifier_key_record_digest: String,
    pub expected_sandbox_verifier_key_id: String,
    pub expected_profile_digest: String,
    pub expected_runner_policy_digest: String,
    pub expected_fixture_catalog_digest: String,
    pub expected_predecessor: Option<ExpectedRuntimeCompatibilityVerificationPredecessor>,
    pub idempotency_key: String,
    pub confirm_challenge: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecordRuntimeCompatibilityVerificationBody {
    pub run_observation_id: String,
    pub expected_run_observation_digest: String,
    pub expected_signature_message_digest: String,
    pub signature_base64: String,
    pub idempotency_key: String,
    pub confirm_verification: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RevokeRuntimeCompatibilityVerificationBody {
    pub expected_verification_receipt_digest: String,
    pub reason: String,
    pub idempotency_key: String,
    pub confirm_revocation: bool,
}

pub(super) fn validate_challenge(
    admin_user_id: &str,
    registry_release_id: &str,
    body: &CreateRuntimeCompatibilityChallengeBody,
) -> Result<(), RuntimeCompatibilityVerificationServiceError> {
    if !body.confirm_challenge {
        return Err(invalid(
            "runtime compatibility challenge requires explicit confirmation",
        ));
    }
    validate_common(admin_user_id, registry_release_id, &body.idempotency_key)?;
    validate_identifier(
        &body.sandbox_verifier_key_record_id,
        160,
        "sandbox verifier key record ID",
    )?;
    for (value, label) in [
        (
            &body.expected_registry_release_digest,
            "registry release digest",
        ),
        (
            &body.expected_profile_digest,
            "runtime compatibility profile digest",
        ),
        (&body.expected_runner_policy_digest, "runner policy digest"),
        (
            &body.expected_fixture_catalog_digest,
            "fixture catalog digest",
        ),
        (
            &body.expected_sandbox_verifier_key_record_digest,
            "sandbox verifier key record digest",
        ),
        (
            &body.expected_sandbox_verifier_key_id,
            "sandbox verifier key ID",
        ),
    ] {
        validate_digest(value, label)?;
    }
    if let Some(predecessor) = &body.expected_predecessor {
        validate_identifier(
            &predecessor.verification_receipt_id,
            200,
            "predecessor verification receipt ID",
        )?;
        validate_digest(
            &predecessor.verification_receipt_digest,
            "predecessor verification receipt digest",
        )?;
    }
    Ok(())
}

pub(super) fn validate_record(
    admin_user_id: &str,
    registry_release_id: &str,
    body: &RecordRuntimeCompatibilityVerificationBody,
) -> Result<(), RuntimeCompatibilityVerificationServiceError> {
    if !body.confirm_verification {
        return Err(invalid(
            "runtime compatibility verification requires explicit confirmation",
        ));
    }
    validate_common(admin_user_id, registry_release_id, &body.idempotency_key)?;
    validate_identifier(&body.run_observation_id, 200, "run observation ID")?;
    validate_digest(
        &body.expected_run_observation_digest,
        "run observation digest",
    )?;
    validate_digest(
        &body.expected_signature_message_digest,
        "signature message digest",
    )?;
    validate_signature(&body.signature_base64)
}

pub(super) fn validate_revoke(
    admin_user_id: &str,
    registry_release_id: &str,
    verification_receipt_id: &str,
    body: &RevokeRuntimeCompatibilityVerificationBody,
) -> Result<(), RuntimeCompatibilityVerificationServiceError> {
    if !body.confirm_revocation {
        return Err(invalid(
            "runtime compatibility revocation requires explicit confirmation",
        ));
    }
    validate_common(admin_user_id, registry_release_id, &body.idempotency_key)?;
    validate_identifier(verification_receipt_id, 200, "verification receipt ID")?;
    validate_digest(
        &body.expected_verification_receipt_digest,
        "verification receipt digest",
    )?;
    if body.reason.trim() != body.reason
        || !(12..=500).contains(&body.reason.chars().count())
        || body.reason.chars().any(char::is_control)
    {
        return Err(invalid(
            "runtime compatibility revocation reason is invalid",
        ));
    }
    Ok(())
}

pub(super) fn validate_identifier(
    value: &str,
    maximum: usize,
    label: &'static str,
) -> Result<(), RuntimeCompatibilityVerificationServiceError> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > maximum
        || value.chars().any(char::is_control)
    {
        return Err(invalid(format!("{label} is invalid")));
    }
    Ok(())
}

fn validate_common(
    admin_user_id: &str,
    registry_release_id: &str,
    idempotency_key: &str,
) -> Result<(), RuntimeCompatibilityVerificationServiceError> {
    validate_identifier(admin_user_id, 200, "administrator user ID")?;
    validate_identifier(registry_release_id, 200, "registry release ID")?;
    validate_identifier(idempotency_key, 240, "idempotency key")
}

fn validate_digest(
    value: &str,
    label: &'static str,
) -> Result<(), RuntimeCompatibilityVerificationServiceError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(invalid(format!("{label} is invalid")));
    }
    Ok(())
}

fn validate_signature(value: &str) -> Result<(), RuntimeCompatibilityVerificationServiceError> {
    if value.is_empty() || value.len() > 1_368 {
        return Err(invalid("runtime compatibility signature Base64 is invalid"));
    }
    let decoded = STANDARD.decode(value).map_err(|error| {
        RuntimeCompatibilityVerificationServiceError::Invalid(AnyError::new(error))
    })?;
    if decoded.is_empty() || decoded.len() > 1024 || STANDARD.encode(&decoded) != value {
        return Err(invalid("runtime compatibility signature Base64 is invalid"));
    }
    Ok(())
}

fn invalid(message: impl Into<String>) -> RuntimeCompatibilityVerificationServiceError {
    RuntimeCompatibilityVerificationServiceError::Invalid(anyhow::anyhow!(message.into()))
}

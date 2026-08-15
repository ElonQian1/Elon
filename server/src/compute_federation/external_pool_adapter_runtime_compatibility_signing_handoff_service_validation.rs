//! Strict caller-input validation for the V269 runtime compatibility signing handoff.

use anyhow::{bail, Result as AnyResult};
use serde::Deserialize;

use super::external_pool_adapter_runtime_compatibility_signing_handoff_service::RuntimeCompatibilitySigningHandoffServiceError;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RuntimeCompatibilitySigningHandoffBody {
    pub expected_challenge_digest: String,
    pub provider_binding_id: String,
    pub expected_provider_binding_digest: String,
    pub expected_installation_receipt_id: String,
    pub expected_installation_receipt_digest: String,
    pub confirm_signing_handoff: bool,
}

pub(super) fn validate_signing_handoff(
    admin_user_id: &str,
    registry_release_id: &str,
    challenge_id: &str,
    body: &RuntimeCompatibilitySigningHandoffBody,
) -> Result<(), RuntimeCompatibilitySigningHandoffServiceError> {
    if !body.confirm_signing_handoff {
        return Err(invalid(
            "runtime compatibility signing handoff requires explicit confirmation",
        ));
    }
    for (value, maximum, label) in [
        (admin_user_id, 200, "administrator user ID"),
        (registry_release_id, 200, "registry release ID"),
        (challenge_id, 200, "runtime compatibility challenge ID"),
        (&body.provider_binding_id, 200, "provider binding ID"),
        (
            &body.expected_installation_receipt_id,
            200,
            "installation receipt ID",
        ),
    ] {
        validate_identifier(value, maximum, label)?;
    }
    for (value, label) in [
        (&body.expected_challenge_digest, "challenge digest"),
        (
            &body.expected_provider_binding_digest,
            "provider binding digest",
        ),
        (
            &body.expected_installation_receipt_digest,
            "installation receipt digest",
        ),
    ] {
        validate_digest(value, label)?;
    }
    Ok(())
}

fn validate_identifier(
    value: &str,
    maximum: usize,
    label: &'static str,
) -> Result<(), RuntimeCompatibilitySigningHandoffServiceError> {
    let result: AnyResult<()> = (|| {
        if value.is_empty()
            || value.trim() != value
            || value.chars().count() > maximum
            || value.chars().any(char::is_control)
        {
            bail!("{label} is invalid");
        }
        Ok(())
    })();
    result.map_err(RuntimeCompatibilitySigningHandoffServiceError::Invalid)
}

fn validate_digest(
    value: &str,
    label: &'static str,
) -> Result<(), RuntimeCompatibilitySigningHandoffServiceError> {
    let result: AnyResult<()> = (|| {
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            bail!("{label} must be exact lowercase SHA-256 hex");
        }
        Ok(())
    })();
    result.map_err(RuntimeCompatibilitySigningHandoffServiceError::Invalid)
}

fn invalid(message: &'static str) -> RuntimeCompatibilitySigningHandoffServiceError {
    RuntimeCompatibilitySigningHandoffServiceError::Invalid(anyhow::anyhow!(message))
}

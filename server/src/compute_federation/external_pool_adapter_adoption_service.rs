//! Administrator orchestration for exact, revocable external-pool Adapter adoption authority.

use anyhow::{bail, Error as AnyError, Result as AnyResult};
use serde::Deserialize;
use thiserror::Error;

use crate::store::{
    AdoptExternalPoolAdapter, ExternalPoolAdapterAdoptionCurrentness,
    ExternalPoolAdapterAdoptionWriteReceipt, RevokeExternalPoolAdapterAdoption, Store,
};

use super::external_pool_adapter_adoption::{
    ADOPTION_CONFIRMATION, ADOPTION_REVOCATION_CONFIRMATION,
};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AdoptExternalPoolAdapterBody {
    pub application_id: String,
    pub expected_application_digest: String,
    pub admission_id: String,
    pub expected_admission_digest: String,
    pub expected_sandbox_conformance_receipt_digest: String,
    pub credential_verification_receipt_id: String,
    pub expected_credential_verification_receipt_digest: String,
    pub idempotency_key: String,
    pub confirm_adoption: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RevokeExternalPoolAdapterAdoptionBody {
    pub expected_adoption_receipt_digest: String,
    pub reason: String,
    pub idempotency_key: String,
    pub confirm_revocation: bool,
}

#[derive(Debug, Error)]
pub(crate) enum AdapterAdoptionServiceError {
    #[error("external-pool Adapter adoption was not found")]
    NotFound,
    #[error("external-pool Adapter adoption request is invalid")]
    Invalid(#[source] AnyError),
    #[error("external-pool Adapter adoption lineage conflicts")]
    Conflict(#[source] AnyError),
}

pub(crate) fn adopt_for_admin(
    store: &Store,
    admin_user_id: &str,
    body: AdoptExternalPoolAdapterBody,
) -> Result<ExternalPoolAdapterAdoptionWriteReceipt, AdapterAdoptionServiceError> {
    if !body.confirm_adoption {
        return Err(invalid("Adapter adoption requires explicit confirmation"));
    }
    validate_identifier(admin_user_id, 200)?;
    validate_identifier(&body.application_id, 200)?;
    validate_identifier(&body.admission_id, 200)?;
    validate_identifier(&body.credential_verification_receipt_id, 200)?;
    validate_identifier(&body.idempotency_key, 240)?;
    for digest in [
        &body.expected_application_digest,
        &body.expected_admission_digest,
        &body.expected_sandbox_conformance_receipt_digest,
        &body.expected_credential_verification_receipt_digest,
    ] {
        validate_digest(digest)?;
    }
    store
        .adopt_external_pool_adapter(AdoptExternalPoolAdapter {
            application_id: body.application_id,
            expected_application_digest: body.expected_application_digest,
            admission_id: body.admission_id,
            expected_admission_digest: body.expected_admission_digest,
            expected_sandbox_conformance_receipt_digest: body
                .expected_sandbox_conformance_receipt_digest,
            credential_verification_receipt_id: body.credential_verification_receipt_id,
            expected_credential_verification_receipt_digest: body
                .expected_credential_verification_receipt_digest,
            adopted_by_admin_user_id: admin_user_id.to_string(),
            confirmation: ADOPTION_CONFIRMATION.to_string(),
            idempotency_scope: format!("external-pool-adapter-adoption:{admin_user_id}"),
            idempotency_key: body.idempotency_key,
        })
        .map_err(AdapterAdoptionServiceError::Conflict)
}

pub(crate) fn revoke_for_admin(
    store: &Store,
    admin_user_id: &str,
    receipt_id: &str,
    body: RevokeExternalPoolAdapterAdoptionBody,
) -> Result<ExternalPoolAdapterAdoptionWriteReceipt, AdapterAdoptionServiceError> {
    if !body.confirm_revocation {
        return Err(invalid(
            "Adapter adoption revocation requires explicit confirmation",
        ));
    }
    validate_identifier(admin_user_id, 200)?;
    validate_identifier(receipt_id, 200)?;
    validate_identifier(&body.reason, 1000)?;
    validate_identifier(&body.idempotency_key, 240)?;
    validate_digest(&body.expected_adoption_receipt_digest)?;
    store
        .revoke_external_pool_adapter_adoption(RevokeExternalPoolAdapterAdoption {
            adoption_receipt_id: receipt_id.to_string(),
            expected_adoption_receipt_digest: body.expected_adoption_receipt_digest,
            revoked_by_admin_user_id: admin_user_id.to_string(),
            reason: body.reason,
            confirmation: ADOPTION_REVOCATION_CONFIRMATION.to_string(),
            idempotency_scope: format!("external-pool-adapter-adoption-revocation:{admin_user_id}"),
            idempotency_key: body.idempotency_key,
        })
        .map_err(AdapterAdoptionServiceError::Conflict)
}

pub(crate) fn currentness_for_admin(
    store: &Store,
    receipt_id: &str,
) -> Result<ExternalPoolAdapterAdoptionCurrentness, AdapterAdoptionServiceError> {
    validate_identifier(receipt_id, 200)?;
    store
        .external_pool_adapter_adoption_currentness(receipt_id)
        .map_err(AdapterAdoptionServiceError::Conflict)?
        .ok_or(AdapterAdoptionServiceError::NotFound)
}

fn validate_identifier(value: &str, max: usize) -> Result<(), AdapterAdoptionServiceError> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > max
        || value.chars().any(char::is_control)
    {
        return Err(invalid("Adapter adoption identifier is invalid"));
    }
    Ok(())
}

fn validate_digest(value: &str) -> Result<(), AdapterAdoptionServiceError> {
    let result: AnyResult<()> = (|| {
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            bail!("Adapter adoption digest is invalid");
        }
        Ok(())
    })();
    result.map_err(AdapterAdoptionServiceError::Invalid)
}

fn invalid(message: &'static str) -> AdapterAdoptionServiceError {
    AdapterAdoptionServiceError::Invalid(anyhow::anyhow!(message))
}

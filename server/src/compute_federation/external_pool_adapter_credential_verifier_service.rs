//! Administrator orchestration for immutable credential-verifier identities.

use anyhow::Error as AnyError;
use serde::Deserialize;
use thiserror::Error;

use crate::{
    compute_federation::external_pool_adapter_credential_verifier::{
        CREDENTIAL_VERIFIER_ACTIVATE_CONFIRMATION, CREDENTIAL_VERIFIER_REGISTER_CONFIRMATION,
        CREDENTIAL_VERIFIER_REVOKE_CONFIRMATION,
    },
    store::{
        ActivateExternalPoolAdapterCredentialVerifier,
        ExternalPoolAdapterCredentialVerifierCurrentnessReceipt,
        ExternalPoolAdapterCredentialVerifierRegistrationWriteReceipt,
        ExternalPoolAdapterCredentialVerifierTransitionWriteReceipt,
        RegisterExternalPoolAdapterCredentialVerifier, RevokeExternalPoolAdapterCredentialVerifier,
        Store,
    },
};

const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegisterCredentialVerifierBody {
    pub verifier_operator: String,
    pub verifier_product: String,
    pub verification_kind: String,
    pub verifier_id: String,
    pub verifier_revision: i64,
    pub verifier_digest: String,
    pub idempotency_key: String,
    pub confirm_registration: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActivateCredentialVerifierBody {
    pub expected_verifier_record_digest: String,
    pub idempotency_key: String,
    pub confirm_activation: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RevokeCredentialVerifierBody {
    pub expected_verifier_record_digest: String,
    pub idempotency_key: String,
    pub reason: String,
    pub confirm_revocation: bool,
}

#[derive(Debug, Error)]
pub(crate) enum CredentialVerifierServiceError {
    #[error("external-pool Adapter credential verifier was not found")]
    NotFound,
    #[error("external-pool Adapter credential-verifier request is invalid")]
    Invalid(#[source] AnyError),
    #[error("external-pool Adapter credential-verifier state conflicts with immutable history")]
    Conflict(#[source] AnyError),
}

pub(crate) fn register_for_admin(
    store: &Store,
    actor: &str,
    body: RegisterCredentialVerifierBody,
) -> Result<
    ExternalPoolAdapterCredentialVerifierRegistrationWriteReceipt,
    CredentialVerifierServiceError,
> {
    if !body.confirm_registration || !(1..=MAX_SAFE_INTEGER).contains(&body.verifier_revision) {
        return Err(invalid("登记 credential verifier 前必须确认有效版本"));
    }
    let verifier_digest = normalize_digest(&body.verifier_digest)?;
    store
        .register_external_pool_adapter_credential_verifier(
            RegisterExternalPoolAdapterCredentialVerifier {
                verifier_operator: normalize(&body.verifier_operator, 160)?,
                verifier_product: normalize(&body.verifier_product, 160)?,
                verification_kind: normalize(&body.verification_kind, 80)?,
                verifier_id: normalize(&body.verifier_id, 160)?,
                verifier_revision: body.verifier_revision,
                verifier_digest,
                created_by_admin_user_id: actor.into(),
                confirmation: CREDENTIAL_VERIFIER_REGISTER_CONFIRMATION.into(),
                idempotency_scope: scope("register", actor),
                idempotency_key: normalize(&body.idempotency_key, 160)?,
            },
        )
        .map_err(CredentialVerifierServiceError::Conflict)
}

pub(crate) fn activate_for_admin(
    store: &Store,
    actor: &str,
    id: &str,
    body: ActivateCredentialVerifierBody,
) -> Result<
    ExternalPoolAdapterCredentialVerifierTransitionWriteReceipt,
    CredentialVerifierServiceError,
> {
    if !body.confirm_activation {
        return Err(invalid("激活 credential verifier 前必须显式确认"));
    }
    require_verifier(store, id)?;
    store
        .activate_external_pool_adapter_credential_verifier(
            ActivateExternalPoolAdapterCredentialVerifier {
                verifier_record_id: normalize(id, 160)?,
                expected_verifier_record_digest: normalize_digest(
                    &body.expected_verifier_record_digest,
                )?,
                activated_by_admin_user_id: actor.into(),
                confirmation: CREDENTIAL_VERIFIER_ACTIVATE_CONFIRMATION.into(),
                idempotency_scope: scope("activate", actor),
                idempotency_key: normalize(&body.idempotency_key, 160)?,
            },
        )
        .map_err(CredentialVerifierServiceError::Conflict)
}

pub(crate) fn revoke_for_admin(
    store: &Store,
    actor: &str,
    id: &str,
    body: RevokeCredentialVerifierBody,
) -> Result<
    ExternalPoolAdapterCredentialVerifierTransitionWriteReceipt,
    CredentialVerifierServiceError,
> {
    if !body.confirm_revocation {
        return Err(invalid("撤销 credential verifier 前必须显式确认"));
    }
    require_verifier(store, id)?;
    store
        .revoke_external_pool_adapter_credential_verifier(
            RevokeExternalPoolAdapterCredentialVerifier {
                verifier_record_id: normalize(id, 160)?,
                expected_verifier_record_digest: normalize_digest(
                    &body.expected_verifier_record_digest,
                )?,
                revoked_by_admin_user_id: actor.into(),
                reason: normalize(&body.reason, 2_000)?,
                confirmation: CREDENTIAL_VERIFIER_REVOKE_CONFIRMATION.into(),
                idempotency_scope: scope("revoke", actor),
                idempotency_key: normalize(&body.idempotency_key, 160)?,
            },
        )
        .map_err(CredentialVerifierServiceError::Conflict)
}

pub(crate) fn currentness_for_admin(
    store: &Store,
    id: &str,
) -> Result<ExternalPoolAdapterCredentialVerifierCurrentnessReceipt, CredentialVerifierServiceError>
{
    let id = normalize(id, 160)?;
    store
        .external_pool_adapter_credential_verifier_currentness(&id)
        .map_err(CredentialVerifierServiceError::Conflict)?
        .ok_or(CredentialVerifierServiceError::NotFound)
}

fn require_verifier(store: &Store, id: &str) -> Result<(), CredentialVerifierServiceError> {
    currentness_for_admin(store, id).map(|_| ())
}

fn normalize(value: &str, max: usize) -> Result<String, CredentialVerifierServiceError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max || value.chars().any(char::is_control) {
        return Err(invalid("credential verifier identity is invalid"));
    }
    Ok(value.into())
}

fn normalize_digest(value: &str) -> Result<String, CredentialVerifierServiceError> {
    let value = value.trim().to_ascii_lowercase();
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(invalid("credential verifier digest is invalid"));
    }
    Ok(value)
}

fn scope(operation: &str, actor: &str) -> String {
    format!("external-pool-adapter-credential-verifier:{operation}:{actor}")
}

fn invalid(message: &'static str) -> CredentialVerifierServiceError {
    CredentialVerifierServiceError::Invalid(anyhow::anyhow!(message))
}

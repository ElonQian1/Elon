//! Administrator orchestration for V242 credential-verifier signing keys.

use anyhow::{Context, Error as AnyError};
use rsa::{
    pkcs1::DecodeRsaPublicKey,
    pkcs8::{DecodePublicKey, EncodePublicKey, LineEnding},
    traits::PublicKeyParts,
    RsaPublicKey,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    compute_federation::external_pool_adapter_credential_verifier_key::{
        KEY_ALGORITHM, REGISTER_CONFIRMATION, REVOKE_CONFIRMATION,
    },
    store::{
        CredentialVerifierKeyCurrentnessReceipt, CredentialVerifierKeyRegistrationWriteReceipt,
        CredentialVerifierKeyRevocationWriteReceipt, RegisterCredentialVerifierKey,
        RevokeCredentialVerifierKey, Store,
    },
};

const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegisterCredentialVerifierKeyBody {
    pub verifier_record_id: String,
    pub expected_verifier_record_digest: String,
    pub verification_kind: String,
    pub verifier_id: String,
    pub verifier_revision: i64,
    pub expected_verifier_digest: String,
    pub algorithm: String,
    pub public_key_pem: String,
    pub idempotency_key: String,
    pub confirm_registration: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RevokeCredentialVerifierKeyBody {
    pub expected_key_record_digest: String,
    pub idempotency_key: String,
    pub reason: String,
    pub confirm_revocation: bool,
}

#[derive(Debug, Error)]
pub(crate) enum CredentialVerifierKeyServiceError {
    #[error("external-pool Adapter credential verifier key was not found")]
    NotFound,
    #[error("external-pool Adapter credential-verifier-key request is invalid")]
    Invalid(#[source] AnyError),
    #[error("external-pool Adapter credential-verifier-key conflicts with immutable history")]
    Conflict(#[source] AnyError),
}

pub(crate) fn register_for_admin(
    store: &Store,
    actor: &str,
    body: RegisterCredentialVerifierKeyBody,
) -> Result<CredentialVerifierKeyRegistrationWriteReceipt, CredentialVerifierKeyServiceError> {
    if !body.confirm_registration
        || body.algorithm != KEY_ALGORITHM
        || !(1..=MAX_SAFE_INTEGER).contains(&body.verifier_revision)
    {
        return Err(invalid(
            "登记 credential verifier key 前必须确认有效版本和受支持算法",
        ));
    }
    let public = parse_public_key(&body.public_key_pem)?;
    let public_key_pem = public
        .to_public_key_pem(LineEnding::LF)
        .context("规范化 credential verifier RSA 公钥失败")
        .map_err(CredentialVerifierKeyServiceError::Invalid)?;
    let der = public
        .to_public_key_der()
        .context("编码 credential verifier RSA 公钥失败")
        .map_err(CredentialVerifierKeyServiceError::Invalid)?;

    store
        .register_external_pool_adapter_credential_verifier_key(RegisterCredentialVerifierKey {
            verifier_record_id: normalize(&body.verifier_record_id, 160)?,
            expected_verifier_record_digest: normalize_digest(
                &body.expected_verifier_record_digest,
            )?,
            verification_kind: normalize(&body.verification_kind, 80)?,
            verifier_id: normalize(&body.verifier_id, 160)?,
            verifier_revision: body.verifier_revision,
            expected_verifier_digest: normalize_digest(&body.expected_verifier_digest)?,
            key_id: hex::encode(Sha256::digest(der.as_bytes())),
            public_key_pem,
            created_by_admin_user_id: actor.into(),
            confirmation: REGISTER_CONFIRMATION.into(),
            idempotency_scope: scope("register", actor),
            idempotency_key: normalize(&body.idempotency_key, 160)?,
        })
        .map_err(CredentialVerifierKeyServiceError::Conflict)
}

pub(crate) fn revoke_for_admin(
    store: &Store,
    actor: &str,
    id: &str,
    body: RevokeCredentialVerifierKeyBody,
) -> Result<CredentialVerifierKeyRevocationWriteReceipt, CredentialVerifierKeyServiceError> {
    if !body.confirm_revocation {
        return Err(invalid("撤销 credential verifier key 前必须显式确认"));
    }
    require_key(store, id)?;
    store
        .revoke_external_pool_adapter_credential_verifier_key(RevokeCredentialVerifierKey {
            key_record_id: normalize(id, 160)?,
            expected_key_record_digest: normalize_digest(&body.expected_key_record_digest)?,
            revoked_by_admin_user_id: actor.into(),
            reason: normalize_reason(&body.reason)?,
            confirmation: REVOKE_CONFIRMATION.into(),
            idempotency_scope: scope("revoke", actor),
            idempotency_key: normalize(&body.idempotency_key, 160)?,
        })
        .map_err(CredentialVerifierKeyServiceError::Conflict)
}

pub(crate) fn currentness_for_admin(
    store: &Store,
    id: &str,
) -> Result<CredentialVerifierKeyCurrentnessReceipt, CredentialVerifierKeyServiceError> {
    let id = normalize(id, 160)?;
    store
        .external_pool_adapter_credential_verifier_key_currentness(&id)
        .map_err(CredentialVerifierKeyServiceError::Conflict)?
        .ok_or(CredentialVerifierKeyServiceError::NotFound)
}

fn require_key(store: &Store, id: &str) -> Result<(), CredentialVerifierKeyServiceError> {
    currentness_for_admin(store, id).map(|_| ())
}

fn parse_public_key(value: &str) -> Result<RsaPublicKey, CredentialVerifierKeyServiceError> {
    let value = value.trim();
    if value.is_empty() || value.len() > 16 * 1024 {
        return Err(invalid("credential verifier RSA 公钥长度无效"));
    }
    let key = RsaPublicKey::from_public_key_pem(value)
        .or_else(|_| RsaPublicKey::from_pkcs1_pem(value))
        .context("credential verifier 公钥必须是 SPKI 或 PKCS#1 RSA PEM")
        .map_err(CredentialVerifierKeyServiceError::Invalid)?;
    if !(2048..=8192).contains(&key.n().bits()) {
        return Err(invalid(
            "credential verifier RSA 公钥必须为 2048 到 8192 位",
        ));
    }
    Ok(key)
}

fn normalize(value: &str, max: usize) -> Result<String, CredentialVerifierKeyServiceError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max || value.chars().any(char::is_control) {
        return Err(invalid("credential verifier key identity is invalid"));
    }
    Ok(value.into())
}

fn normalize_reason(value: &str) -> Result<String, CredentialVerifierKeyServiceError> {
    let value = normalize(value, 2_000)?;
    if value.chars().count() < 8 {
        return Err(invalid(
            "credential verifier key revocation reason is too short",
        ));
    }
    Ok(value)
}

fn normalize_digest(value: &str) -> Result<String, CredentialVerifierKeyServiceError> {
    let value = value.trim().to_ascii_lowercase();
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(invalid("credential verifier key digest is invalid"));
    }
    Ok(value)
}

fn scope(operation: &str, actor: &str) -> String {
    format!("external-pool-adapter-credential-verifier-key:{operation}:{actor}")
}

fn invalid(message: &'static str) -> CredentialVerifierKeyServiceError {
    CredentialVerifierKeyServiceError::Invalid(anyhow::anyhow!(message))
}

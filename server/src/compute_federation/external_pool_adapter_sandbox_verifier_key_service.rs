//! Administrator orchestration for independent sandbox-verifier trust keys.

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
    compute_federation::external_pool_adapter_sandbox_verifier_key::{
        SANDBOX_VERIFIER_KEY_ACTIVATE_CONFIRMATION, SANDBOX_VERIFIER_KEY_ALGORITHM,
        SANDBOX_VERIFIER_KEY_REGISTER_CONFIRMATION, SANDBOX_VERIFIER_KEY_REVOKE_CONFIRMATION,
    },
    store::{
        ActivateExternalPoolAdapterSandboxVerifierKey,
        ExternalPoolAdapterSandboxVerifierKeyCurrentnessReceipt,
        ExternalPoolAdapterSandboxVerifierKeyRegistrationWriteReceipt,
        ExternalPoolAdapterSandboxVerifierKeyTransitionWriteReceipt,
        RegisterExternalPoolAdapterSandboxVerifierKey, RevokeExternalPoolAdapterSandboxVerifierKey,
        Store,
    },
};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegisterSandboxVerifierKeyBody {
    pub verifier_operator: String,
    pub verifier_product: String,
    pub algorithm: String,
    pub public_key_pem: String,
    pub idempotency_key: String,
    pub confirm_registration: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActivateSandboxVerifierKeyBody {
    pub expected_key_record_digest: String,
    pub idempotency_key: String,
    pub confirm_activation: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RevokeSandboxVerifierKeyBody {
    pub expected_key_record_digest: String,
    pub idempotency_key: String,
    pub reason: String,
    pub confirm_revocation: bool,
}

#[derive(Debug, Error)]
pub(crate) enum SandboxVerifierKeyServiceError {
    #[error("external-pool Adapter sandbox verifier key was not found")]
    NotFound,
    #[error("external-pool Adapter sandbox-verifier-key request is invalid")]
    Invalid(#[source] AnyError),
    #[error("external-pool Adapter sandbox-verifier-key state conflicts with immutable history")]
    Conflict(#[source] AnyError),
}

pub(crate) fn register_for_admin(
    store: &Store,
    actor: &str,
    body: RegisterSandboxVerifierKeyBody,
) -> Result<
    ExternalPoolAdapterSandboxVerifierKeyRegistrationWriteReceipt,
    SandboxVerifierKeyServiceError,
> {
    if !body.confirm_registration || body.algorithm != SANDBOX_VERIFIER_KEY_ALGORITHM {
        return Err(invalid(
            "登记 sandbox verifier key 前必须确认并使用受支持算法",
        ));
    }
    let verifier_operator = normalize(&body.verifier_operator, 160)?;
    let verifier_product = normalize(&body.verifier_product, 160)?;
    let public = parse_public_key(&body.public_key_pem)?;
    let public_key_pem = public
        .to_public_key_pem(LineEnding::LF)
        .context("规范化 sandbox verifier RSA 公钥失败")
        .map_err(SandboxVerifierKeyServiceError::Invalid)?;
    let der = public
        .to_public_key_der()
        .context("编码 sandbox verifier RSA 公钥失败")
        .map_err(SandboxVerifierKeyServiceError::Invalid)?;
    store
        .register_external_pool_adapter_sandbox_verifier_key(
            RegisterExternalPoolAdapterSandboxVerifierKey {
                verifier_operator,
                verifier_product,
                key_id: hex::encode(Sha256::digest(der.as_bytes())),
                public_key_pem,
                created_by_admin_user_id: actor.into(),
                confirmation: SANDBOX_VERIFIER_KEY_REGISTER_CONFIRMATION.into(),
                idempotency_scope: scope("register", actor),
                idempotency_key: body.idempotency_key,
            },
        )
        .map_err(SandboxVerifierKeyServiceError::Conflict)
}

pub(crate) fn activate_for_admin(
    store: &Store,
    actor: &str,
    id: &str,
    body: ActivateSandboxVerifierKeyBody,
) -> Result<
    ExternalPoolAdapterSandboxVerifierKeyTransitionWriteReceipt,
    SandboxVerifierKeyServiceError,
> {
    if !body.confirm_activation {
        return Err(invalid("激活 sandbox verifier key 前必须显式确认"));
    }
    require_key(store, id)?;
    store
        .activate_external_pool_adapter_sandbox_verifier_key(
            ActivateExternalPoolAdapterSandboxVerifierKey {
                key_record_id: id.into(),
                expected_key_record_digest: body.expected_key_record_digest,
                activated_by_admin_user_id: actor.into(),
                confirmation: SANDBOX_VERIFIER_KEY_ACTIVATE_CONFIRMATION.into(),
                idempotency_scope: scope("activate", actor),
                idempotency_key: body.idempotency_key,
            },
        )
        .map_err(SandboxVerifierKeyServiceError::Conflict)
}

pub(crate) fn revoke_for_admin(
    store: &Store,
    actor: &str,
    id: &str,
    body: RevokeSandboxVerifierKeyBody,
) -> Result<
    ExternalPoolAdapterSandboxVerifierKeyTransitionWriteReceipt,
    SandboxVerifierKeyServiceError,
> {
    if !body.confirm_revocation {
        return Err(invalid("撤销 sandbox verifier key 前必须显式确认"));
    }
    require_key(store, id)?;
    store
        .revoke_external_pool_adapter_sandbox_verifier_key(
            RevokeExternalPoolAdapterSandboxVerifierKey {
                key_record_id: id.into(),
                expected_key_record_digest: body.expected_key_record_digest,
                revoked_by_admin_user_id: actor.into(),
                reason: body.reason,
                confirmation: SANDBOX_VERIFIER_KEY_REVOKE_CONFIRMATION.into(),
                idempotency_scope: scope("revoke", actor),
                idempotency_key: body.idempotency_key,
            },
        )
        .map_err(SandboxVerifierKeyServiceError::Conflict)
}

pub(crate) fn currentness_for_admin(
    store: &Store,
    id: &str,
) -> Result<ExternalPoolAdapterSandboxVerifierKeyCurrentnessReceipt, SandboxVerifierKeyServiceError>
{
    store
        .external_pool_adapter_sandbox_verifier_key_currentness(id)
        .map_err(SandboxVerifierKeyServiceError::Conflict)?
        .ok_or(SandboxVerifierKeyServiceError::NotFound)
}

fn require_key(store: &Store, id: &str) -> Result<(), SandboxVerifierKeyServiceError> {
    currentness_for_admin(store, id).map(|_| ())
}

fn parse_public_key(value: &str) -> Result<RsaPublicKey, SandboxVerifierKeyServiceError> {
    let value = value.trim();
    if value.is_empty() || value.len() > 16 * 1024 {
        return Err(invalid("sandbox verifier RSA 公钥长度无效"));
    }
    let key = RsaPublicKey::from_public_key_pem(value)
        .or_else(|_| RsaPublicKey::from_pkcs1_pem(value))
        .context("sandbox verifier 公钥必须是 SPKI 或 PKCS#1 RSA PEM")
        .map_err(SandboxVerifierKeyServiceError::Invalid)?;
    if !(2048..=8192).contains(&key.n().bits()) {
        return Err(invalid("sandbox verifier RSA 公钥必须为 2048 到 8192 位"));
    }
    Ok(key)
}

fn normalize(value: &str, max: usize) -> Result<String, SandboxVerifierKeyServiceError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max || value.chars().any(char::is_control) {
        return Err(invalid("sandbox verifier identity is invalid"));
    }
    Ok(value.into())
}

fn scope(operation: &str, actor: &str) -> String {
    format!("external-pool-adapter-sandbox-verifier-key:{operation}:{actor}")
}

fn invalid(message: &'static str) -> SandboxVerifierKeyServiceError {
    SandboxVerifierKeyServiceError::Invalid(anyhow::anyhow!(message))
}

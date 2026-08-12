//! Administrator orchestration for the Artifact signer trust-key registry.

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
    compute_federation::external_pool_adapter_artifact_signing_key::{
        SIGNING_KEY_ACTIVATION_CONFIRMATION, SIGNING_KEY_ALGORITHM,
        SIGNING_KEY_REGISTRATION_CONFIRMATION, SIGNING_KEY_REVOCATION_CONFIRMATION,
    },
    store::{
        ActivateExternalPoolAdapterArtifactSigningKey,
        ExternalPoolAdapterArtifactSigningKeyActivationWriteReceipt,
        ExternalPoolAdapterArtifactSigningKeyCurrentnessReceipt,
        ExternalPoolAdapterArtifactSigningKeyRegistrationWriteReceipt,
        ExternalPoolAdapterArtifactSigningKeyRevocationWriteReceipt,
        RegisterExternalPoolAdapterArtifactSigningKey, RevokeExternalPoolAdapterArtifactSigningKey,
        Store,
    },
};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegisterExternalPoolAdapterArtifactSigningKeyBody {
    pub source_operator: String,
    pub algorithm: String,
    pub public_key_pem: String,
    pub idempotency_key: String,
    pub confirm_registration: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActivateExternalPoolAdapterArtifactSigningKeyBody {
    pub expected_key_record_digest: String,
    pub idempotency_key: String,
    pub confirm_activation: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RevokeExternalPoolAdapterArtifactSigningKeyBody {
    pub expected_key_record_digest: String,
    pub idempotency_key: String,
    pub reason: String,
    pub confirm_revocation: bool,
}

#[derive(Debug, Error)]
pub(crate) enum ExternalPoolAdapterArtifactSigningKeyServiceError {
    #[error("external-pool Adapter Artifact signing key was not found")]
    NotFound,
    #[error("external-pool Adapter Artifact signing-key request is invalid")]
    Invalid(#[source] AnyError),
    #[error("external-pool Adapter Artifact signing-key state conflicts with immutable history")]
    Conflict(#[source] AnyError),
}

pub(crate) fn register_for_admin(
    store: &Store,
    admin_user_id: &str,
    body: RegisterExternalPoolAdapterArtifactSigningKeyBody,
) -> Result<
    ExternalPoolAdapterArtifactSigningKeyRegistrationWriteReceipt,
    ExternalPoolAdapterArtifactSigningKeyServiceError,
> {
    if !body.confirm_registration || body.algorithm != SIGNING_KEY_ALGORITHM {
        return Err(invalid("登记签名密钥前必须确认并使用受支持的算法"));
    }
    let source_operator = normalize_source_operator(&body.source_operator)
        .map_err(ExternalPoolAdapterArtifactSigningKeyServiceError::Invalid)?;
    let public_key = parse_public_key(&body.public_key_pem)
        .map_err(ExternalPoolAdapterArtifactSigningKeyServiceError::Invalid)?;
    let public_key_pem = public_key
        .to_public_key_pem(LineEnding::LF)
        .context("规范化 Artifact signer RSA 公钥失败")
        .map_err(ExternalPoolAdapterArtifactSigningKeyServiceError::Invalid)?;
    let der = public_key
        .to_public_key_der()
        .context("编码 Artifact signer RSA 公钥失败")
        .map_err(ExternalPoolAdapterArtifactSigningKeyServiceError::Invalid)?;
    let key_id = hex::encode(Sha256::digest(der.as_bytes()));
    store
        .register_external_pool_adapter_artifact_signing_key(
            RegisterExternalPoolAdapterArtifactSigningKey {
                source_operator,
                key_id,
                public_key_pem,
                created_by_admin_user_id: admin_user_id.to_string(),
                confirmation: SIGNING_KEY_REGISTRATION_CONFIRMATION.to_string(),
                idempotency_scope: operation_scope("register", admin_user_id),
                idempotency_key: body.idempotency_key,
            },
        )
        .map_err(ExternalPoolAdapterArtifactSigningKeyServiceError::Conflict)
}

pub(crate) fn activate_for_admin(
    store: &Store,
    admin_user_id: &str,
    key_record_id: &str,
    body: ActivateExternalPoolAdapterArtifactSigningKeyBody,
) -> Result<
    ExternalPoolAdapterArtifactSigningKeyActivationWriteReceipt,
    ExternalPoolAdapterArtifactSigningKeyServiceError,
> {
    if !body.confirm_activation {
        return Err(invalid("激活签名密钥前必须显式确认"));
    }
    require_key(store, key_record_id)?;
    store
        .activate_external_pool_adapter_artifact_signing_key(
            ActivateExternalPoolAdapterArtifactSigningKey {
                key_record_id: key_record_id.to_string(),
                expected_key_record_digest: body.expected_key_record_digest,
                activated_by_admin_user_id: admin_user_id.to_string(),
                confirmation: SIGNING_KEY_ACTIVATION_CONFIRMATION.to_string(),
                idempotency_scope: operation_scope("activate", admin_user_id),
                idempotency_key: body.idempotency_key,
            },
        )
        .map_err(ExternalPoolAdapterArtifactSigningKeyServiceError::Conflict)
}

pub(crate) fn revoke_for_admin(
    store: &Store,
    admin_user_id: &str,
    key_record_id: &str,
    body: RevokeExternalPoolAdapterArtifactSigningKeyBody,
) -> Result<
    ExternalPoolAdapterArtifactSigningKeyRevocationWriteReceipt,
    ExternalPoolAdapterArtifactSigningKeyServiceError,
> {
    if !body.confirm_revocation {
        return Err(invalid("撤销签名密钥前必须显式确认"));
    }
    require_key(store, key_record_id)?;
    store
        .revoke_external_pool_adapter_artifact_signing_key(
            RevokeExternalPoolAdapterArtifactSigningKey {
                key_record_id: key_record_id.to_string(),
                expected_key_record_digest: body.expected_key_record_digest,
                revoked_by_admin_user_id: admin_user_id.to_string(),
                reason: body.reason,
                confirmation: SIGNING_KEY_REVOCATION_CONFIRMATION.to_string(),
                idempotency_scope: operation_scope("revoke", admin_user_id),
                idempotency_key: body.idempotency_key,
            },
        )
        .map_err(ExternalPoolAdapterArtifactSigningKeyServiceError::Conflict)
}

pub(crate) fn currentness_for_admin(
    store: &Store,
    key_record_id: &str,
) -> Result<
    ExternalPoolAdapterArtifactSigningKeyCurrentnessReceipt,
    ExternalPoolAdapterArtifactSigningKeyServiceError,
> {
    store
        .external_pool_adapter_artifact_signing_key_currentness(key_record_id)
        .map_err(ExternalPoolAdapterArtifactSigningKeyServiceError::Conflict)?
        .ok_or(ExternalPoolAdapterArtifactSigningKeyServiceError::NotFound)
}

fn require_key(
    store: &Store,
    key_record_id: &str,
) -> Result<(), ExternalPoolAdapterArtifactSigningKeyServiceError> {
    currentness_for_admin(store, key_record_id).map(|_| ())
}

fn parse_public_key(value: &str) -> anyhow::Result<RsaPublicKey> {
    let value = value.trim();
    if value.is_empty() || value.len() > 16 * 1024 {
        anyhow::bail!("Artifact signer RSA 公钥长度无效");
    }
    let key = RsaPublicKey::from_public_key_pem(value)
        .or_else(|_| RsaPublicKey::from_pkcs1_pem(value))
        .context("Artifact signer 公钥必须是 SPKI 或 PKCS#1 RSA PEM")?;
    if !(2048..=8192).contains(&key.n().bits()) {
        anyhow::bail!("Artifact signer RSA 公钥必须为 2048 到 8192 位");
    }
    Ok(key)
}

fn normalize_source_operator(value: &str) -> anyhow::Result<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 160 || value.chars().any(char::is_control) {
        anyhow::bail!("来源运营方标识长度必须为 1 到 160 个字符且不含控制字符");
    }
    Ok(value.to_string())
}

fn operation_scope(operation: &str, admin_user_id: &str) -> String {
    format!("external-pool-adapter-artifact-signing-key:{operation}:{admin_user_id}")
}

fn invalid(message: &'static str) -> ExternalPoolAdapterArtifactSigningKeyServiceError {
    ExternalPoolAdapterArtifactSigningKeyServiceError::Invalid(anyhow::anyhow!(message))
}

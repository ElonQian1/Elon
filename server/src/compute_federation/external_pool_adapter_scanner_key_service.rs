//! Administrator orchestration for independent vulnerability-scanner trust keys.

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
    compute_federation::external_pool_adapter_scanner_key::{
        SCANNER_KEY_ACTIVATE_CONFIRMATION, SCANNER_KEY_ALGORITHM,
        SCANNER_KEY_REGISTER_CONFIRMATION, SCANNER_KEY_REVOKE_CONFIRMATION,
    },
    store::{
        ActivateExternalPoolAdapterScannerKey, ExternalPoolAdapterScannerKeyActivationWriteReceipt,
        ExternalPoolAdapterScannerKeyCurrentnessReceipt,
        ExternalPoolAdapterScannerKeyRegistrationWriteReceipt,
        ExternalPoolAdapterScannerKeyRevocationWriteReceipt, RegisterExternalPoolAdapterScannerKey,
        RevokeExternalPoolAdapterScannerKey, Store,
    },
};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegisterScannerKeyBody {
    pub scanner_operator: String,
    pub scanner_product: String,
    pub algorithm: String,
    pub public_key_pem: String,
    pub idempotency_key: String,
    pub confirm_registration: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActivateScannerKeyBody {
    pub expected_key_record_digest: String,
    pub idempotency_key: String,
    pub confirm_activation: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RevokeScannerKeyBody {
    pub expected_key_record_digest: String,
    pub idempotency_key: String,
    pub reason: String,
    pub confirm_revocation: bool,
}

#[derive(Debug, Error)]
pub(crate) enum ScannerKeyServiceError {
    #[error("external-pool Adapter scanner key was not found")]
    NotFound,
    #[error("external-pool Adapter scanner-key request is invalid")]
    Invalid(#[source] AnyError),
    #[error("external-pool Adapter scanner-key state conflicts with immutable history")]
    Conflict(#[source] AnyError),
}

pub(crate) fn register_for_admin(
    store: &Store,
    actor: &str,
    body: RegisterScannerKeyBody,
) -> Result<ExternalPoolAdapterScannerKeyRegistrationWriteReceipt, ScannerKeyServiceError> {
    if !body.confirm_registration || body.algorithm != SCANNER_KEY_ALGORITHM {
        return Err(invalid("登记 scanner key 前必须确认并使用受支持算法"));
    }
    let scanner_operator = normalize(&body.scanner_operator, 160)?;
    let scanner_product = normalize(&body.scanner_product, 160)?;
    let public = parse_public_key(&body.public_key_pem)?;
    let public_key_pem = public
        .to_public_key_pem(LineEnding::LF)
        .context("规范化 scanner RSA 公钥失败")
        .map_err(ScannerKeyServiceError::Invalid)?;
    let der = public
        .to_public_key_der()
        .context("编码 scanner RSA 公钥失败")
        .map_err(ScannerKeyServiceError::Invalid)?;
    store
        .register_external_pool_adapter_scanner_key(RegisterExternalPoolAdapterScannerKey {
            scanner_operator,
            scanner_product,
            key_id: hex::encode(Sha256::digest(der.as_bytes())),
            public_key_pem,
            created_by_admin_user_id: actor.to_string(),
            confirmation: SCANNER_KEY_REGISTER_CONFIRMATION.to_string(),
            idempotency_scope: scope("register", actor),
            idempotency_key: body.idempotency_key,
        })
        .map_err(ScannerKeyServiceError::Conflict)
}

pub(crate) fn activate_for_admin(
    store: &Store,
    actor: &str,
    id: &str,
    body: ActivateScannerKeyBody,
) -> Result<ExternalPoolAdapterScannerKeyActivationWriteReceipt, ScannerKeyServiceError> {
    if !body.confirm_activation {
        return Err(invalid("激活 scanner key 前必须显式确认"));
    }
    require_key(store, id)?;
    store
        .activate_external_pool_adapter_scanner_key(ActivateExternalPoolAdapterScannerKey {
            key_record_id: id.to_string(),
            expected_key_record_digest: body.expected_key_record_digest,
            activated_by_admin_user_id: actor.to_string(),
            confirmation: SCANNER_KEY_ACTIVATE_CONFIRMATION.to_string(),
            idempotency_scope: scope("activate", actor),
            idempotency_key: body.idempotency_key,
        })
        .map_err(ScannerKeyServiceError::Conflict)
}

pub(crate) fn revoke_for_admin(
    store: &Store,
    actor: &str,
    id: &str,
    body: RevokeScannerKeyBody,
) -> Result<ExternalPoolAdapterScannerKeyRevocationWriteReceipt, ScannerKeyServiceError> {
    if !body.confirm_revocation {
        return Err(invalid("撤销 scanner key 前必须显式确认"));
    }
    require_key(store, id)?;
    store
        .revoke_external_pool_adapter_scanner_key(RevokeExternalPoolAdapterScannerKey {
            key_record_id: id.to_string(),
            expected_key_record_digest: body.expected_key_record_digest,
            revoked_by_admin_user_id: actor.to_string(),
            reason: body.reason,
            confirmation: SCANNER_KEY_REVOKE_CONFIRMATION.to_string(),
            idempotency_scope: scope("revoke", actor),
            idempotency_key: body.idempotency_key,
        })
        .map_err(ScannerKeyServiceError::Conflict)
}

pub(crate) fn currentness_for_admin(
    store: &Store,
    id: &str,
) -> Result<ExternalPoolAdapterScannerKeyCurrentnessReceipt, ScannerKeyServiceError> {
    store
        .external_pool_adapter_scanner_key_currentness(id)
        .map_err(ScannerKeyServiceError::Conflict)?
        .ok_or(ScannerKeyServiceError::NotFound)
}

fn require_key(store: &Store, id: &str) -> Result<(), ScannerKeyServiceError> {
    currentness_for_admin(store, id).map(|_| ())
}

fn parse_public_key(value: &str) -> Result<RsaPublicKey, ScannerKeyServiceError> {
    let value = value.trim();
    if value.is_empty() || value.len() > 16 * 1024 {
        return Err(invalid("scanner RSA 公钥长度无效"));
    }
    let key = RsaPublicKey::from_public_key_pem(value)
        .or_else(|_| RsaPublicKey::from_pkcs1_pem(value))
        .context("scanner 公钥必须是 SPKI 或 PKCS#1 RSA PEM")
        .map_err(ScannerKeyServiceError::Invalid)?;
    if !(2048..=8192).contains(&key.n().bits()) {
        return Err(invalid("scanner RSA 公钥必须为 2048 到 8192 位"));
    }
    Ok(key)
}

fn normalize(value: &str, max: usize) -> Result<String, ScannerKeyServiceError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max || value.chars().any(char::is_control) {
        return Err(invalid("scanner identity is invalid"));
    }
    Ok(value.to_string())
}

fn scope(operation: &str, actor: &str) -> String {
    format!("external-pool-adapter-scanner-key:{operation}:{actor}")
}

fn invalid(message: &'static str) -> ScannerKeyServiceError {
    ScannerKeyServiceError::Invalid(anyhow::anyhow!(message))
}

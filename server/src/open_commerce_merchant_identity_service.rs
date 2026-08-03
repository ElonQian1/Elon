use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use rsa::{
    pkcs1::DecodeRsaPublicKey,
    pkcs1v15::{Signature as RsaSignature, VerifyingKey},
    pkcs8::{DecodePublicKey, EncodePublicKey, LineEnding},
    signature::Verifier,
    traits::PublicKeyParts,
    RsaPublicKey,
};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{
    open_commerce_merchant_identity_model::{
        CreateMerchantIdentityKeyRequest, OpenCommerceMerchantIdentityKey,
        MERCHANT_IDENTITY_KEY_ALGORITHM, MERCHANT_IDENTITY_KEY_SCHEMA,
        MERCHANT_IDENTITY_PROOF_PROTOCOL,
    },
    open_commerce_service::OpenCommerceActor,
    project_auth::can_edit,
    store::Store,
};

pub(crate) fn create_identity_key(
    store: &Store,
    project_id: &str,
    merchant_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: CreateMerchantIdentityKeyRequest,
) -> Result<OpenCommerceMerchantIdentityKey> {
    require_editor(actor)?;
    let merchant = store.open_commerce_merchant_for_project(project_id, merchant_id)?;
    let public_key = parse_public_key(&request.public_key_pem)?;
    validate_public_key(&public_key)?;
    let public_key_pem = public_key
        .to_public_key_pem(LineEnding::LF)
        .context("规范化商户可携带身份公钥失败")?;
    let public_key_der = public_key
        .to_public_key_der()
        .context("编码商户可携带身份公钥失败")?;
    let key_id = hex::encode(Sha256::digest(public_key_der.as_bytes()));
    verify_possession_proof(
        &public_key,
        &request.proof_signature_base64,
        project_id,
        &merchant.id,
        &key_id,
    )?;
    let existing = store.list_open_commerce_merchant_identity_keys(project_id, merchant_id, 100)?;
    if let Some(record) = existing.iter().find(|record| record.key_id == key_id) {
        verify_identity_key_record(record)?;
        if record.status != "active" {
            bail!("该商户身份公钥已撤销；请生成新密钥完成轮换");
        }
        return Ok(record.clone());
    }
    if existing
        .iter()
        .filter(|record| record.status == "active")
        .count()
        >= 3
    {
        bail!("每个商户最多保留 3 个有效身份公钥，请先撤销旧密钥");
    }
    let verified_at = chrono::Utc::now().to_rfc3339();
    let (record, created) = store.save_open_commerce_merchant_identity_key(
        project_id,
        merchant_id,
        &key_id,
        MERCHANT_IDENTITY_KEY_ALGORITHM,
        &public_key_pem,
        &request.proof_signature_base64,
        actor.user_id,
        &verified_at,
    )?;
    verify_identity_key_record(&record)?;
    if created {
        store.record_open_commerce_audit(
            project_id,
            actor.user_id,
            Some(actor.app_id),
            "merchant_identity.key_created",
            "merchant_identity_key",
            &record.id,
            &json!({
                "merchant_id": merchant_id,
                "key_id": record.key_id,
                "algorithm": record.algorithm,
                "proof_authority": "merchant_private_key_possession",
            }),
        )?;
    }
    Ok(record)
}

pub(crate) fn list_identity_keys(
    store: &Store,
    project_id: &str,
    merchant_id: &str,
    actor: &OpenCommerceActor<'_>,
) -> Result<Vec<OpenCommerceMerchantIdentityKey>> {
    require_editor(actor)?;
    store.open_commerce_merchant_for_project(project_id, merchant_id)?;
    store
        .list_open_commerce_merchant_identity_keys(project_id, merchant_id, 100)?
        .into_iter()
        .map(|record| verify_identity_key_record(&record).map(|_| record))
        .collect()
}

pub(crate) fn revoke_identity_key(
    store: &Store,
    project_id: &str,
    merchant_id: &str,
    record_id: &str,
    actor: &OpenCommerceActor<'_>,
) -> Result<OpenCommerceMerchantIdentityKey> {
    require_editor(actor)?;
    store.open_commerce_merchant_for_project(project_id, merchant_id)?;
    let record = store
        .revoke_open_commerce_merchant_identity_key(
            project_id,
            merchant_id,
            normalize_record_id(record_id)?,
        )?
        .ok_or_else(|| anyhow!("商户可携带身份公钥不存在"))?;
    verify_identity_key_record(&record)?;
    store.record_open_commerce_audit(
        project_id,
        actor.user_id,
        Some(actor.app_id),
        "merchant_identity.key_revoked",
        "merchant_identity_key",
        &record.id,
        &json!({"merchant_id": merchant_id, "key_id": record.key_id}),
    )?;
    Ok(record)
}

pub(crate) fn proof_message(project_id: &str, merchant_id: &str, key_id: &str) -> String {
    format!(
        "{MERCHANT_IDENTITY_PROOF_PROTOCOL}\n{}\n{}\n{}",
        project_id.trim(),
        merchant_id.trim(),
        key_id
    )
}

fn verify_possession_proof(
    public_key: &RsaPublicKey,
    signature_base64: &str,
    project_id: &str,
    merchant_id: &str,
    key_id: &str,
) -> Result<()> {
    if signature_base64.is_empty() || signature_base64.len() > 2048 {
        bail!("商户身份所有权签名长度无效");
    }
    let signature_bytes = BASE64
        .decode(signature_base64)
        .context("商户身份所有权签名不是有效 Base64")?;
    let signature =
        RsaSignature::try_from(signature_bytes.as_slice()).context("商户身份所有权签名字节无效")?;
    VerifyingKey::<Sha256>::new(public_key.clone())
        .verify(
            proof_message(project_id, merchant_id, key_id).as_bytes(),
            &signature,
        )
        .context("商户身份私钥持有证明验证失败")
}

fn parse_public_key(value: &str) -> Result<RsaPublicKey> {
    let value = value.trim();
    if value.is_empty() || value.len() > 16 * 1024 {
        bail!("商户可携带身份公钥长度无效");
    }
    RsaPublicKey::from_public_key_pem(value)
        .or_else(|_| RsaPublicKey::from_pkcs1_pem(value))
        .context("商户可携带身份公钥必须是 RSA PEM")
}

fn validate_public_key(public_key: &RsaPublicKey) -> Result<()> {
    if !(2048..=8192).contains(&public_key.n().bits()) {
        bail!("商户可携带身份公钥必须为 2048 到 8192 位 RSA");
    }
    Ok(())
}

fn verify_identity_key_record(record: &OpenCommerceMerchantIdentityKey) -> Result<()> {
    if record.schema != MERCHANT_IDENTITY_KEY_SCHEMA
        || record.algorithm != MERCHANT_IDENTITY_KEY_ALGORITHM
        || !matches!(record.status.as_str(), "active" | "revoked")
    {
        bail!("商户可携带身份公钥记录无效");
    }
    let public_key = parse_public_key(&record.public_key_pem)?;
    validate_public_key(&public_key)?;
    let der = public_key
        .to_public_key_der()
        .context("编码商户可携带身份公钥失败")?;
    if hex::encode(Sha256::digest(der.as_bytes())) != record.key_id {
        bail!("商户可携带身份公钥摘要不一致");
    }
    verify_possession_proof(
        &public_key,
        &record.proof_signature_base64,
        &record.project_id,
        &record.merchant_id,
        &record.key_id,
    )?;
    Ok(())
}

fn normalize_record_id(value: &str) -> Result<&str> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 120 {
        bail!("商户身份公钥记录 ID 长度必须为 1 到 120 个字符");
    }
    Ok(value)
}

fn require_editor(actor: &OpenCommerceActor<'_>) -> Result<()> {
    if !actor.project_role.is_some_and(can_edit) {
        bail!("当前调用方没有项目编辑权限");
    }
    Ok(())
}

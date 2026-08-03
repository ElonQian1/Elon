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
    open_commerce_portability_import_model::{
        ConsumerPortabilityPackageSignature, VerifiedConsumerPortabilitySignature,
        CONSUMER_PORTABILITY_SIGNATURE_ALGORITHM,
    },
    open_commerce_portability_model::ConsumerPortabilityExport,
    open_commerce_portability_trust_model::{
        ConsumerPortabilityTrustKey, CreateConsumerPortabilityTrustKeyRequest,
        CONSUMER_PORTABILITY_TRUST_KEY_ALGORITHM, CONSUMER_PORTABILITY_TRUST_KEY_SCHEMA,
    },
    open_commerce_service::OpenCommerceActor,
    store::Store,
};

const SIGNATURE_PROTOCOL: &str = "open_commerce.consumer_portability_signature.v1";

pub(crate) fn create_trust_key(
    store: &Store,
    destination_project_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: CreateConsumerPortabilityTrustKeyRequest,
) -> Result<ConsumerPortabilityTrustKey> {
    ensure_consumer_project_actor(actor)?;
    let source_operator = normalize_source_operator(&request.source_operator)?;
    let public_key = parse_public_key(&request.public_key_pem)?;
    validate_public_key(&public_key)?;
    let public_key_pem = public_key
        .to_public_key_pem(LineEnding::LF)
        .context("规范化消费者可携带数据包信任公钥失败")?;
    let public_key_der = public_key
        .to_public_key_der()
        .context("编码消费者可携带数据包信任公钥失败")?;
    let key_id = hex::encode(Sha256::digest(public_key_der.as_bytes()));
    let (record, created) = store.save_consumer_portability_trust_key(
        destination_project_id,
        actor.user_id,
        &source_operator,
        &key_id,
        CONSUMER_PORTABILITY_TRUST_KEY_ALGORITHM,
        &public_key_pem,
    )?;
    verify_trust_key_record(&record)?;
    if !created && record.status != "active" {
        bail!("该运营方公钥已撤销；请使用新的公钥完成轮换");
    }
    if created {
        store.record_open_commerce_audit(
            destination_project_id,
            actor.user_id,
            Some(actor.app_id),
            "consumer_portability.trust_key_created",
            "consumer_portability_trust_key",
            &record.id,
            &json!({
                "source_operator": record.source_operator,
                "key_id": record.key_id,
                "algorithm": record.algorithm,
            }),
        )?;
    }
    Ok(record)
}

pub(crate) fn list_trust_keys(
    store: &Store,
    destination_project_id: &str,
    actor: &OpenCommerceActor<'_>,
    limit: usize,
) -> Result<Vec<ConsumerPortabilityTrustKey>> {
    ensure_consumer_project_actor(actor)?;
    store
        .list_consumer_portability_trust_keys(destination_project_id, actor.user_id, limit)?
        .into_iter()
        .map(|record| verify_trust_key_record(&record).map(|_| record))
        .collect()
}

pub(crate) fn revoke_trust_key(
    store: &Store,
    destination_project_id: &str,
    record_id: &str,
    actor: &OpenCommerceActor<'_>,
) -> Result<ConsumerPortabilityTrustKey> {
    ensure_consumer_project_actor(actor)?;
    let record_id = normalize_record_id(record_id)?;
    let record = store
        .revoke_consumer_portability_trust_key(destination_project_id, actor.user_id, &record_id)?
        .ok_or_else(|| anyhow!("消费者可携带数据包信任公钥不存在"))?;
    verify_trust_key_record(&record)?;
    store.record_open_commerce_audit(
        destination_project_id,
        actor.user_id,
        Some(actor.app_id),
        "consumer_portability.trust_key_revoked",
        "consumer_portability_trust_key",
        &record.id,
        &json!({
            "source_operator": record.source_operator,
            "key_id": record.key_id,
        }),
    )?;
    Ok(record)
}

pub(crate) fn verify_package_signature(
    store: &Store,
    destination_project_id: &str,
    consumer_user_id: &str,
    source_operator: &str,
    package: &ConsumerPortabilityExport,
    signature: ConsumerPortabilityPackageSignature,
) -> Result<VerifiedConsumerPortabilitySignature> {
    if signature.algorithm != CONSUMER_PORTABILITY_SIGNATURE_ALGORITHM {
        bail!("消费者可携带数据包签名算法不受支持");
    }
    validate_key_id(&signature.key_id)?;
    if signature.signature_base64.is_empty() || signature.signature_base64.len() > 2048 {
        bail!("消费者可携带数据包签名长度无效");
    }
    let key_record = store
        .active_consumer_portability_trust_key(
            destination_project_id,
            consumer_user_id,
            source_operator,
            &signature.key_id,
        )?
        .ok_or_else(|| anyhow!("未找到该来源运营方的有效信任公钥"))?;
    verify_trust_key_record(&key_record)?;
    let public_key = parse_public_key(&key_record.public_key_pem)?;
    let signature_bytes = BASE64
        .decode(&signature.signature_base64)
        .context("消费者可携带数据包签名不是有效 Base64")?;
    let rsa_signature = RsaSignature::try_from(signature_bytes.as_slice())
        .context("消费者可携带数据包签名字节无效")?;
    VerifyingKey::<Sha256>::new(public_key)
        .verify(
            signature_message(source_operator, &signature.key_id, package).as_bytes(),
            &rsa_signature,
        )
        .context("消费者可携带数据包运营方签名验证失败")?;
    Ok(VerifiedConsumerPortabilitySignature {
        key_record_id: key_record.id,
        signature,
        verified_at: chrono::Utc::now().to_rfc3339(),
    })
}

pub(crate) fn signature_message(
    source_operator: &str,
    key_id: &str,
    package: &ConsumerPortabilityExport,
) -> String {
    format!(
        "{SIGNATURE_PROTOCOL}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        source_operator,
        key_id,
        package.schema,
        package.id,
        package.source_project_id,
        package.idempotency_key,
        package.payload_sha256,
        package.created_at,
    )
}

pub(crate) fn normalize_source_operator(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 160 {
        bail!("来源运营方标识长度必须为 1 到 160 个字符");
    }
    if value.chars().any(char::is_control) {
        bail!("来源运营方标识不能包含控制字符");
    }
    Ok(value.to_string())
}

fn parse_public_key(value: &str) -> Result<RsaPublicKey> {
    let value = value.trim();
    if value.is_empty() || value.len() > 16 * 1024 {
        bail!("消费者可携带数据包信任公钥长度无效");
    }
    RsaPublicKey::from_public_key_pem(value)
        .or_else(|_| RsaPublicKey::from_pkcs1_pem(value))
        .context("消费者可携带数据包信任公钥必须是 RSA PEM")
}

fn validate_public_key(public_key: &RsaPublicKey) -> Result<()> {
    if !(2048..=8192).contains(&public_key.n().bits()) {
        bail!("消费者可携带数据包信任公钥必须为 2048 到 8192 位 RSA");
    }
    Ok(())
}

fn verify_trust_key_record(record: &ConsumerPortabilityTrustKey) -> Result<()> {
    if record.schema != CONSUMER_PORTABILITY_TRUST_KEY_SCHEMA
        || record.algorithm != CONSUMER_PORTABILITY_TRUST_KEY_ALGORITHM
        || !matches!(record.status.as_str(), "active" | "revoked")
    {
        bail!("消费者可携带数据包信任公钥记录无效");
    }
    normalize_source_operator(&record.source_operator)?;
    validate_key_id(&record.key_id)?;
    let public_key = parse_public_key(&record.public_key_pem)?;
    validate_public_key(&public_key)?;
    let der = public_key
        .to_public_key_der()
        .context("编码消费者可携带数据包信任公钥失败")?;
    if hex::encode(Sha256::digest(der.as_bytes())) != record.key_id {
        bail!("消费者可携带数据包信任公钥摘要不一致");
    }
    Ok(())
}

fn validate_key_id(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        bail!("消费者可携带数据包签名公钥 ID 格式无效");
    }
    Ok(())
}

fn normalize_record_id(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 120 {
        bail!("消费者可携带数据包信任公钥记录 ID 长度必须为 1 到 120 个字符");
    }
    Ok(value.to_string())
}

fn ensure_consumer_project_actor(actor: &OpenCommerceActor<'_>) -> Result<()> {
    if actor.project_role.is_none() {
        bail!("当前调用方不属于消费者项目");
    }
    Ok(())
}

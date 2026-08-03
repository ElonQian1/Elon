use anyhow::{anyhow, bail, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::DateTime;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{
    open_commerce_consumer_vault_model::{
        ConsumerDataVaultEnvelope, ConsumerDataVaultItem, ConsumerDataVaultItemSummary,
        CreateConsumerDataVaultItemRequest, DeleteConsumerDataVaultItemRequest,
        UpdateConsumerDataVaultItemRequest, CONSUMER_DATA_VAULT_ENVELOPE_SCHEMA,
    },
    open_commerce_service::OpenCommerceActor,
    store::Store,
};

const VAULT_KDF_ITERATIONS: u32 = 310_000;
const MAX_CIPHERTEXT_BYTES: usize = 1024 * 1024;
const ALLOWED_ITEM_KINDS: [&str; 6] = [
    "private_note",
    "identity",
    "health",
    "finance",
    "credential_reference",
    "custom",
];

pub(crate) fn create_item(
    store: &Store,
    consumer_project_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: CreateConsumerDataVaultItemRequest,
) -> Result<ConsumerDataVaultItem> {
    ensure_consumer_project_actor(actor)?;
    let id = normalize_id(&request.id)?;
    let label = normalize_label(&request.label)?;
    let item_kind = normalize_item_kind(&request.item_kind)?;
    let ciphertext = validate_envelope(&request.envelope, &id, 1)?;
    let ciphertext_sha256 = hex::encode(Sha256::digest(&ciphertext));
    let item = store.create_open_commerce_consumer_vault_item(
        &id,
        consumer_project_id,
        actor.user_id,
        &label,
        &item_kind,
        &request.envelope,
        &ciphertext_sha256,
        ciphertext.len() as i64,
    )?;
    record_audit(store, consumer_project_id, actor, "created", &item)?;
    Ok(item)
}

pub(crate) fn update_item(
    store: &Store,
    consumer_project_id: &str,
    item_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: UpdateConsumerDataVaultItemRequest,
) -> Result<ConsumerDataVaultItem> {
    ensure_consumer_project_actor(actor)?;
    let id = normalize_id(item_id)?;
    if request.expected_revision < 1 {
        bail!("保险箱条目预期修订必须大于零");
    }
    let next_revision = request
        .expected_revision
        .checked_add(1)
        .ok_or_else(|| anyhow!("保险箱条目修订号超出范围"))?;
    let label = normalize_label(&request.label)?;
    let item_kind = normalize_item_kind(&request.item_kind)?;
    let ciphertext = validate_envelope(&request.envelope, &id, next_revision)?;
    let ciphertext_sha256 = hex::encode(Sha256::digest(&ciphertext));
    let item = store.update_open_commerce_consumer_vault_item(
        &id,
        consumer_project_id,
        actor.user_id,
        request.expected_revision,
        &label,
        &item_kind,
        &request.envelope,
        &ciphertext_sha256,
        ciphertext.len() as i64,
    )?;
    record_audit(store, consumer_project_id, actor, "updated", &item)?;
    Ok(item)
}

pub(crate) fn get_item(
    store: &Store,
    consumer_project_id: &str,
    item_id: &str,
    actor: &OpenCommerceActor<'_>,
) -> Result<ConsumerDataVaultItem> {
    ensure_consumer_project_actor(actor)?;
    let id = normalize_id(item_id)?;
    store
        .open_commerce_consumer_vault_item(consumer_project_id, actor.user_id, &id)?
        .ok_or_else(|| anyhow!("消费者数据保险箱条目不存在"))
}

pub(crate) fn list_items(
    store: &Store,
    consumer_project_id: &str,
    actor: &OpenCommerceActor<'_>,
    limit: usize,
) -> Result<Vec<ConsumerDataVaultItemSummary>> {
    ensure_consumer_project_actor(actor)?;
    store.list_open_commerce_consumer_vault_items(consumer_project_id, actor.user_id, limit)
}

pub(crate) fn delete_item(
    store: &Store,
    consumer_project_id: &str,
    item_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: DeleteConsumerDataVaultItemRequest,
) -> Result<ConsumerDataVaultItemSummary> {
    ensure_consumer_project_actor(actor)?;
    if !request.confirmed_by_user {
        bail!("删除保险箱条目必须由用户明确确认");
    }
    if request.expected_revision < 1 {
        bail!("保险箱条目预期修订必须大于零");
    }
    let id = normalize_id(item_id)?;
    let item = store.delete_open_commerce_consumer_vault_item(
        consumer_project_id,
        actor.user_id,
        &id,
        request.expected_revision,
    )?;
    store.record_open_commerce_audit(
        consumer_project_id,
        actor.user_id,
        Some(actor.app_id),
        "consumer_data_vault.deleted",
        "consumer_data_vault_item",
        &item.id,
        &json!({
            "item_kind": item.item_kind,
            "revision": item.revision,
            "ciphertext_sha256": item.ciphertext_sha256,
            "ciphertext_bytes": item.ciphertext_bytes,
            "server_can_decrypt": false,
        }),
    )?;
    Ok(item)
}

fn record_audit(
    store: &Store,
    consumer_project_id: &str,
    actor: &OpenCommerceActor<'_>,
    action: &str,
    item: &ConsumerDataVaultItem,
) -> Result<()> {
    store.record_open_commerce_audit(
        consumer_project_id,
        actor.user_id,
        Some(actor.app_id),
        &format!("consumer_data_vault.{action}"),
        "consumer_data_vault_item",
        &item.id,
        &json!({
            "item_kind": item.item_kind,
            "revision": item.revision,
            "ciphertext_sha256": item.ciphertext_sha256,
            "ciphertext_bytes": item.ciphertext_bytes,
            "server_can_decrypt": false,
        }),
    )
}

fn validate_envelope(
    envelope: &ConsumerDataVaultEnvelope,
    expected_id: &str,
    expected_revision: i64,
) -> Result<Vec<u8>> {
    if envelope.schema != CONSUMER_DATA_VAULT_ENVELOPE_SCHEMA
        || envelope.record_id != expected_id
        || envelope.revision != expected_revision
    {
        bail!("加密信封版本、记录 ID 或修订号不匹配");
    }
    if envelope.kdf.name != "PBKDF2"
        || envelope.kdf.hash != "SHA-256"
        || envelope.kdf.iterations != VAULT_KDF_ITERATIONS
        || envelope.cipher.name != "AES-256-GCM"
        || envelope.cipher.auth_tag_length_bits != 128
    {
        bail!("消费者数据保险箱加密参数不受支持");
    }
    let salt = BASE64
        .decode(envelope.kdf.salt_base64.trim())
        .map_err(|_| anyhow!("保险箱 KDF 盐值不是有效 Base64"))?;
    if salt.len() != 16 {
        bail!("保险箱 KDF 盐值必须为 16 字节");
    }
    let nonce = BASE64
        .decode(envelope.cipher.nonce_base64.trim())
        .map_err(|_| anyhow!("保险箱随机数不是有效 Base64"))?;
    if nonce.len() != 12 {
        bail!("保险箱 AES-GCM 随机数必须为 12 字节");
    }
    let ciphertext = BASE64
        .decode(envelope.ciphertext_base64.trim())
        .map_err(|_| anyhow!("保险箱密文不是有效 Base64"))?;
    if ciphertext.len() < 17 || ciphertext.len() > MAX_CIPHERTEXT_BYTES {
        bail!("保险箱密文大小必须为 17 字节到 1 MiB");
    }
    DateTime::parse_from_rfc3339(envelope.created_at.trim())
        .map_err(|_| anyhow!("保险箱加密时间必须为 RFC3339"))?;
    Ok(ciphertext)
}

fn normalize_id(value: &str) -> Result<String> {
    let value = value.trim();
    if value.chars().count() < 8
        || value.chars().count() > 120
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        bail!("保险箱条目 ID 必须为 8 到 120 个字母、数字、短横线或下划线");
    }
    Ok(value.to_string())
}

fn normalize_label(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 120 {
        bail!("保险箱条目标签长度必须为 1 到 120 个字符");
    }
    Ok(value.to_string())
}

fn normalize_item_kind(value: &str) -> Result<String> {
    let value = value.trim().to_ascii_lowercase();
    if !ALLOWED_ITEM_KINDS.contains(&value.as_str()) {
        bail!("保险箱条目类型不受支持");
    }
    Ok(value)
}

fn ensure_consumer_project_actor(actor: &OpenCommerceActor<'_>) -> Result<()> {
    if actor.project_role.is_none() {
        bail!("当前调用方不属于消费者项目");
    }
    Ok(())
}

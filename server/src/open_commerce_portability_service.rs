use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{
    open_commerce_portability_model::{
        ConsumerPortabilityExport, ConsumerPortabilityExportSummary, ConsumerPortabilityPayload,
        CreateConsumerPortabilityExportRequest, CONSUMER_PORTABILITY_EXPORT_SCHEMA,
        CONSUMER_PORTABILITY_PAYLOAD_SCHEMA,
    },
    open_commerce_service::OpenCommerceActor,
    store::Store,
};

const MAX_PORTABILITY_PAYLOAD_BYTES: usize = 5 * 1024 * 1024;

pub(crate) fn create_export(
    store: &Store,
    consumer_project_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: CreateConsumerPortabilityExportRequest,
) -> Result<ConsumerPortabilityExport> {
    ensure_consumer_project_actor(actor)?;
    let idempotency_key = normalize_idempotency_key(&request.idempotency_key)?;
    if let Some(existing) = store.consumer_portability_export_by_key(
        consumer_project_id,
        actor.user_id,
        &idempotency_key,
    )? {
        return verify_export(existing, consumer_project_id);
    }

    let sources =
        store.consumer_portability_snapshot_sources(consumer_project_id, actor.user_id)?;
    let payload = ConsumerPortabilityPayload {
        schema: CONSUMER_PORTABILITY_PAYLOAD_SCHEMA.to_string(),
        source_project_id: consumer_project_id.trim().to_string(),
        generated_at: Utc::now().to_rfc3339(),
        relationships: sources.relationships,
        relationship_renewals: sources.relationship_renewals,
        data_requests: sources.data_requests,
    };
    let payload_json = canonical_payload_json(&payload)?;
    let payload_sha256 = digest_payload(payload_json.as_bytes());
    let (export, created) = store.save_consumer_portability_export(
        consumer_project_id,
        actor.user_id,
        &idempotency_key,
        CONSUMER_PORTABILITY_EXPORT_SCHEMA,
        &payload_json,
        &payload_sha256,
    )?;
    let export = verify_export(export, consumer_project_id)?;
    if created {
        store.record_open_commerce_audit(
            consumer_project_id,
            actor.user_id,
            Some(actor.app_id),
            "consumer_portability.export_created",
            "consumer_portability_export",
            &export.id,
            &json!({
                "payload_sha256": export.payload_sha256,
                "relationship_count": export.payload.relationships.len(),
                "renewal_count": export.payload.relationship_renewals.len(),
                "data_request_count": export.payload.data_requests.len()
            }),
        )?;
    }
    Ok(export)
}

pub(crate) fn list_exports(
    store: &Store,
    consumer_project_id: &str,
    actor: &OpenCommerceActor<'_>,
    limit: usize,
) -> Result<Vec<ConsumerPortabilityExportSummary>> {
    ensure_consumer_project_actor(actor)?;
    store
        .list_consumer_portability_exports(consumer_project_id, actor.user_id, limit)?
        .into_iter()
        .map(|export| verify_export(export, consumer_project_id).map(|value| value.summary()))
        .collect()
}

pub(crate) fn get_export(
    store: &Store,
    consumer_project_id: &str,
    export_id: &str,
    actor: &OpenCommerceActor<'_>,
) -> Result<ConsumerPortabilityExport> {
    ensure_consumer_project_actor(actor)?;
    let export_id = export_id.trim();
    if export_id.is_empty() || export_id.chars().count() > 120 {
        bail!("消费者可携带数据包 ID 长度必须为 1 到 120 个字符");
    }
    let export = store
        .consumer_portability_export(consumer_project_id, actor.user_id, export_id)?
        .ok_or_else(|| anyhow!("消费者可携带数据包不存在"))?;
    verify_export(export, consumer_project_id)
}

fn ensure_consumer_project_actor(actor: &OpenCommerceActor<'_>) -> Result<()> {
    if actor.project_role.is_none() {
        bail!("当前调用方不属于消费者项目");
    }
    Ok(())
}

fn normalize_idempotency_key(value: &str) -> Result<String> {
    let value = value.trim();
    if !(8..=120).contains(&value.chars().count()) {
        bail!("幂等键长度必须为 8 到 120 个字符");
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "-_.:".contains(character))
    {
        bail!("幂等键只能包含英文字母、数字、横线、下划线、点和冒号");
    }
    Ok(value.to_string())
}

fn verify_export(
    export: ConsumerPortabilityExport,
    expected_project_id: &str,
) -> Result<ConsumerPortabilityExport> {
    if export.schema != CONSUMER_PORTABILITY_EXPORT_SCHEMA {
        bail!("消费者可携带数据包版本不受支持");
    }
    if export.payload.schema != CONSUMER_PORTABILITY_PAYLOAD_SCHEMA {
        bail!("消费者可携带数据负载版本不受支持");
    }
    if export.source_project_id != expected_project_id.trim()
        || export.payload.source_project_id != expected_project_id.trim()
    {
        bail!("消费者可携带数据包来源项目不一致");
    }
    if export.payload_sha256.len() != 64
        || !export
            .payload_sha256
            .chars()
            .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase())
    {
        bail!("消费者可携带数据包摘要格式无效");
    }
    let payload_json = canonical_payload_json(&export.payload)?;
    let actual_digest = digest_payload(payload_json.as_bytes());
    if actual_digest != export.payload_sha256 {
        bail!("消费者可携带数据包完整性校验失败");
    }
    Ok(export)
}

fn canonical_payload_json(payload: &ConsumerPortabilityPayload) -> Result<String> {
    let payload_json = serde_json::to_string(payload).context("序列化消费者可携带数据失败")?;
    if payload_json.len() > MAX_PORTABILITY_PAYLOAD_BYTES {
        bail!("消费者可携带数据包超过 V1 的 5 MiB 上限");
    }
    Ok(payload_json)
}

fn digest_payload(payload: &[u8]) -> String {
    hex::encode(Sha256::digest(payload))
}

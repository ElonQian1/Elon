use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    open_commerce_consumer_receipt_model::{
        CONSUMER_RECEIPT_PAYLOAD_SCHEMA, CONSUMER_RECEIPT_SCHEMA,
    },
    open_commerce_consumer_receipt_service,
    open_commerce_data_erasure_evidence_model::{
        ERASURE_EVIDENCE_KIND_EXTERNAL_RECEIPT, ERASURE_EVIDENCE_KIND_MERCHANT_ATTESTATION,
        ERASURE_EVIDENCE_SOURCE_AUTHORITY,
    },
    open_commerce_model::SETTLEMENT_RECORDED_NOT_CHARGED,
    open_commerce_portability_model::{
        ConsumerPortabilityExport, ConsumerPortabilityExportSummary, ConsumerPortabilityPayload,
        ConsumerPortableDataErasureEvidence, ConsumerPortableInvocationReceipt,
        ConsumerPortableMerchantIdentityClaim, CreateConsumerPortabilityExportRequest,
        CONSUMER_PORTABILITY_EXPORT_SCHEMA, CONSUMER_PORTABILITY_EXPORT_SCHEMA_V4,
        CONSUMER_PORTABILITY_PAYLOAD_SCHEMA, CONSUMER_PORTABILITY_PAYLOAD_SCHEMA_V4,
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
    let invocation_receipts = sources
        .terminal_invocations
        .into_iter()
        .map(open_commerce_consumer_receipt_service::receipt_from_invocation)
        .map(|receipt| {
            receipt.map(|receipt| ConsumerPortableInvocationReceipt {
                schema: receipt.schema,
                payload_sha256: receipt.payload_sha256,
                payload_json: receipt.payload_json,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let merchant_identity_claims =
        portable_merchant_identity_claims(store, &sources.relationships)?;
    let data_erasure_evidence = sources
        .data_erasure_evidence
        .into_iter()
        .map(portable_data_erasure_evidence)
        .collect();
    let payload = ConsumerPortabilityPayload {
        schema: CONSUMER_PORTABILITY_PAYLOAD_SCHEMA.to_string(),
        source_project_id: consumer_project_id.trim().to_string(),
        generated_at: Utc::now().to_rfc3339(),
        relationships: sources.relationships,
        relationship_renewals: sources.relationship_renewals,
        data_requests: sources.data_requests,
        data_erasure_evidence,
        preference_profile: sources.preference_profile,
        preference_disclosures: sources.preference_disclosures,
        invocation_receipt_scope: Some("authenticated_user_account".to_string()),
        invocation_receipts,
        merchant_identity_claims,
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
                "data_request_count": export.payload.data_requests.len(),
                "data_erasure_evidence_count": export.payload.data_erasure_evidence.len(),
                "preference_profile_included": export.payload.preference_profile.is_some(),
                "preference_disclosure_count": export.payload.preference_disclosures.len()
                ,"invocation_receipt_count": export.payload.invocation_receipts.len()
                ,"merchant_identity_claim_count": export.payload.merchant_identity_claims.len()
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

pub(crate) fn verify_external_export(
    export: ConsumerPortabilityExport,
) -> Result<ConsumerPortabilityExport> {
    let source_project_id = export.source_project_id.trim();
    if source_project_id.is_empty() || source_project_id.chars().count() > 120 {
        bail!("消费者可携带数据包来源项目 ID 长度必须为 1 到 120 个字符");
    }
    if export.source_project_id != source_project_id
        || export.source_project_id.chars().any(char::is_control)
    {
        bail!("消费者可携带数据包来源项目 ID 格式无效");
    }
    if export.id.trim().is_empty()
        || export.id.chars().count() > 120
        || export.id != export.id.trim()
        || export.id.chars().any(char::is_control)
    {
        bail!("消费者可携带数据包 ID 长度必须为 1 到 120 个字符");
    }
    normalize_idempotency_key(&export.idempotency_key)?;
    chrono::DateTime::parse_from_rfc3339(&export.created_at)
        .context("消费者可携带数据包创建时间无效")?;
    verify_export_contents(export)
}

pub(crate) fn canonical_export_json(export: &ConsumerPortabilityExport) -> Result<String> {
    let package_json = serde_json::to_string(export).context("序列化消费者可携带数据包失败")?;
    if package_json.len() > MAX_PORTABILITY_PAYLOAD_BYTES + 64 * 1024 {
        bail!("消费者可携带数据包信封超过允许上限");
    }
    Ok(package_json)
}

fn verify_export(
    export: ConsumerPortabilityExport,
    expected_project_id: &str,
) -> Result<ConsumerPortabilityExport> {
    let export = verify_external_export(export)?;
    if export.source_project_id != expected_project_id.trim() {
        bail!("消费者可携带数据包来源项目不一致");
    }
    Ok(export)
}

fn verify_export_contents(export: ConsumerPortabilityExport) -> Result<ConsumerPortabilityExport> {
    let supported_version = matches!(
        (export.schema.as_str(), export.payload.schema.as_str()),
        (
            CONSUMER_PORTABILITY_EXPORT_SCHEMA,
            CONSUMER_PORTABILITY_PAYLOAD_SCHEMA
        ) | (
            CONSUMER_PORTABILITY_EXPORT_SCHEMA_V4,
            CONSUMER_PORTABILITY_PAYLOAD_SCHEMA_V4
        ) | (
            crate::open_commerce_portability_model::CONSUMER_PORTABILITY_EXPORT_SCHEMA_V3,
            crate::open_commerce_portability_model::CONSUMER_PORTABILITY_PAYLOAD_SCHEMA_V3
        ) | (
            crate::open_commerce_portability_model::CONSUMER_PORTABILITY_EXPORT_SCHEMA_V2,
            crate::open_commerce_portability_model::CONSUMER_PORTABILITY_PAYLOAD_SCHEMA_V2
        ) | (
            crate::open_commerce_portability_model::CONSUMER_PORTABILITY_EXPORT_SCHEMA_V1,
            crate::open_commerce_portability_model::CONSUMER_PORTABILITY_PAYLOAD_SCHEMA_V1
        )
    );
    if !supported_version {
        bail!("消费者可携带数据负载版本不受支持");
    }
    let is_v5 = export.schema == CONSUMER_PORTABILITY_EXPORT_SCHEMA;
    let is_v4 = export.schema == CONSUMER_PORTABILITY_EXPORT_SCHEMA_V4;
    let has_invocation_receipts = is_v5
        || is_v4
        || export.schema
            == crate::open_commerce_portability_model::CONSUMER_PORTABILITY_EXPORT_SCHEMA_V3;
    if has_invocation_receipts {
        if export.payload.invocation_receipt_scope.as_deref() != Some("authenticated_user_account")
        {
            bail!("消费者可携带数据包的调用凭证范围无效");
        }
        for receipt in &export.payload.invocation_receipts {
            verify_portable_receipt(receipt)?;
        }
    } else if export.payload.invocation_receipt_scope.is_some()
        || !export.payload.invocation_receipts.is_empty()
    {
        bail!("旧版消费者可携带数据包不能包含 V3 调用凭证字段");
    }
    if is_v5 || is_v4 {
        verify_merchant_identity_claims(&export.payload)?;
    } else if !export.payload.merchant_identity_claims.is_empty() {
        bail!("旧版消费者可携带数据包不能包含 V4 商户身份声明");
    }
    if is_v5 {
        verify_data_erasure_evidence(&export.payload)?;
    } else if !export.payload.data_erasure_evidence.is_empty() {
        bail!("旧版消费者可携带数据包不能包含 V5 删除证明");
    }
    if export.source_project_id != export.source_project_id.trim()
        || export.payload.source_project_id != export.source_project_id
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
    let canonical_payload_json = canonical_payload_json(&export.payload)?;
    if canonical_payload_json != export.payload_json {
        bail!("消费者可携带数据包规范负载不一致");
    }
    let actual_digest = digest_payload(export.payload_json.as_bytes());
    if actual_digest != export.payload_sha256 {
        bail!("消费者可携带数据包完整性校验失败");
    }
    Ok(export)
}

fn canonical_payload_json(payload: &ConsumerPortabilityPayload) -> Result<String> {
    let payload_json = serde_json::to_string(payload).context("序列化消费者可携带数据失败")?;
    if payload_json.len() > MAX_PORTABILITY_PAYLOAD_BYTES {
        bail!("消费者可携带数据包超过 5 MiB 上限");
    }
    Ok(payload_json)
}

fn verify_portable_receipt(receipt: &ConsumerPortableInvocationReceipt) -> Result<()> {
    if receipt.schema != CONSUMER_RECEIPT_SCHEMA {
        bail!("消费者可携带数据包包含不受支持的调用凭证版本");
    }
    let payload: crate::open_commerce_consumer_receipt_model::ConsumerInvocationReceiptPayload =
        serde_json::from_str(&receipt.payload_json).context("解析消费者调用凭证失败")?;
    if payload.schema != CONSUMER_RECEIPT_PAYLOAD_SCHEMA {
        bail!("消费者可携带数据包包含不受支持的调用凭证版本");
    }
    if payload.funds_moved
        || payload.settlement_status != SETTLEMENT_RECORDED_NOT_CHARGED
        || payload.request_shape.contains_raw_values
    {
        bail!("消费者可携带数据包包含越界调用凭证");
    }
    if receipt.payload_sha256.len() != 64
        || !receipt
            .payload_sha256
            .chars()
            .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase())
    {
        bail!("消费者调用凭证摘要格式无效");
    }
    let canonical_payload_json =
        serde_json::to_string(&payload).context("序列化消费者调用凭证失败")?;
    if canonical_payload_json != receipt.payload_json {
        bail!("消费者调用凭证规范负载不一致");
    }
    if digest_payload(receipt.payload_json.as_bytes()) != receipt.payload_sha256 {
        bail!("消费者调用凭证完整性校验失败");
    }
    Ok(())
}

fn portable_merchant_identity_claims(
    store: &Store,
    relationships: &[crate::open_commerce_relationship_model::OpenCommerceConsumerRelationship],
) -> Result<Vec<ConsumerPortableMerchantIdentityClaim>> {
    let merchant_ids = relationships
        .iter()
        .map(|relationship| relationship.merchant_id.clone())
        .collect::<BTreeSet<_>>();
    let mut claims = Vec::new();
    for merchant_id in merchant_ids {
        let mut key_ids = store
            .list_active_open_commerce_merchant_identity_keys(&merchant_id)?
            .into_iter()
            .map(|key| key.key_id)
            .collect::<Vec<_>>();
        key_ids.sort();
        key_ids.dedup();
        if !key_ids.is_empty() {
            claims.push(ConsumerPortableMerchantIdentityClaim {
                source_merchant_id: merchant_id,
                key_ids,
                authority: "merchant_private_key_possession".to_string(),
            });
        }
    }
    Ok(claims)
}

fn portable_data_erasure_evidence(
    evidence: crate::open_commerce_data_erasure_evidence_model::OpenCommerceDataErasureEvidence,
) -> ConsumerPortableDataErasureEvidence {
    ConsumerPortableDataErasureEvidence {
        id: evidence.id,
        data_request_id: evidence.data_request_id,
        merchant_id: evidence.merchant_id,
        evidence_kind: evidence.evidence_kind,
        external_system: evidence.external_system,
        reference_id: evidence.reference_id,
        receipt_sha256: evidence.receipt_sha256,
        summary: evidence.summary,
        source_authority: evidence.source_authority.to_string(),
        platform_verified: evidence.platform_verified,
        created_at: evidence.created_at,
    }
}

fn verify_data_erasure_evidence(payload: &ConsumerPortabilityPayload) -> Result<()> {
    if payload.data_erasure_evidence.len() > 5_000 {
        bail!("消费者可携带数据包的删除证明超过 5000 条上限");
    }
    let requests = payload
        .data_requests
        .iter()
        .map(|request| {
            (
                request.id.as_str(),
                (request.merchant_id.as_str(), request.status.as_str()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut evidence_ids = BTreeSet::new();
    for evidence in &payload.data_erasure_evidence {
        if !evidence_ids.insert(evidence.id.as_str())
            || !valid_portable_text(&evidence.id, 120)
            || requests.get(evidence.data_request_id.as_str()).copied()
                != Some((evidence.merchant_id.as_str(), "completed"))
            || !matches!(
                evidence.evidence_kind.as_str(),
                ERASURE_EVIDENCE_KIND_EXTERNAL_RECEIPT | ERASURE_EVIDENCE_KIND_MERCHANT_ATTESTATION
            )
            || !valid_portable_text(&evidence.external_system, 80)
            || !valid_portable_text(&evidence.reference_id, 160)
            || !valid_portable_text(&evidence.summary, 500)
            || evidence.source_authority != ERASURE_EVIDENCE_SOURCE_AUTHORITY
            || evidence.platform_verified
            || !valid_lower_sha256(&evidence.receipt_sha256)
            || chrono::DateTime::parse_from_rfc3339(&evidence.created_at).is_err()
        {
            bail!("消费者可携带数据包包含无效删除证明");
        }
    }
    Ok(())
}

fn valid_portable_text(value: &str, max_len: usize) -> bool {
    value == value.trim()
        && !value.is_empty()
        && value.chars().count() <= max_len
        && !value.chars().any(char::is_control)
}

fn valid_lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn verify_merchant_identity_claims(payload: &ConsumerPortabilityPayload) -> Result<()> {
    let relationship_merchants = payload
        .relationships
        .iter()
        .map(|relationship| relationship.merchant_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut claimed_merchants = BTreeSet::new();
    for claim in &payload.merchant_identity_claims {
        if !relationship_merchants.contains(claim.source_merchant_id.as_str())
            || !claimed_merchants.insert(claim.source_merchant_id.as_str())
            || claim.authority != "merchant_private_key_possession"
            || claim.key_ids.is_empty()
            || claim.key_ids.len() > 3
        {
            bail!("消费者可携带数据包的商户身份声明无效");
        }
        let mut unique_keys = BTreeSet::new();
        for key_id in &claim.key_ids {
            if key_id.len() != 64
                || !key_id
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
                || !unique_keys.insert(key_id.as_str())
            {
                bail!("消费者可携带数据包的商户身份指纹无效");
            }
        }
    }
    Ok(())
}

fn digest_payload(payload: &[u8]) -> String {
    hex::encode(Sha256::digest(payload))
}

use anyhow::{anyhow, bail, Result};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{
    open_commerce_portability_import_model::{
        ConsumerPortabilityImport, ConsumerPortabilityImportSummary,
        CreateConsumerPortabilityImportRequest, CONSUMER_PORTABILITY_IMPORT_MERGE_STATUS,
        CONSUMER_PORTABILITY_IMPORT_SCHEMA, CONSUMER_PORTABILITY_IMPORT_TRUSTED_STATUS,
        CONSUMER_PORTABILITY_IMPORT_TRUST_STATUS,
    },
    open_commerce_portability_service, open_commerce_portability_trust_service,
    open_commerce_service::OpenCommerceActor,
    store::Store,
};

pub(crate) fn create_import(
    store: &Store,
    destination_project_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: CreateConsumerPortabilityImportRequest,
) -> Result<ConsumerPortabilityImport> {
    ensure_consumer_project_actor(actor)?;
    let source_operator = open_commerce_portability_trust_service::normalize_source_operator(
        &request.source_operator,
    )?;
    let package = open_commerce_portability_service::verify_external_export(request.package)?;
    let package_json = open_commerce_portability_service::canonical_export_json(&package)?;
    let envelope_sha256 = hex::encode(Sha256::digest(package_json.as_bytes()));
    let verified_signature = request
        .signature
        .map(|signature| {
            open_commerce_portability_trust_service::verify_package_signature(
                store,
                destination_project_id,
                actor.user_id,
                &source_operator,
                &package,
                signature,
            )
        })
        .transpose()?;
    let (import_record, created, trust_upgraded) = store.save_consumer_portability_import(
        destination_project_id,
        actor.user_id,
        &source_operator,
        &package,
        &package_json,
        &envelope_sha256,
        verified_signature.as_ref(),
    )?;
    let import_record = verify_import(import_record, destination_project_id)?;
    if created {
        store.record_open_commerce_audit(
            destination_project_id,
            actor.user_id,
            Some(actor.app_id),
            "consumer_portability.import_created",
            "consumer_portability_import",
            &import_record.id,
            &json!({
                "source_operator": import_record.source_operator,
                "source_project_id": import_record.source_project_id,
                "source_package_id": import_record.source_package_id,
                "source_package_schema": import_record.source_package_schema,
                "envelope_sha256": import_record.envelope_sha256,
                "payload_sha256": import_record.payload_sha256,
                "trust_status": import_record.trust_status,
                "merge_status": import_record.merge_status,
            }),
        )?;
    } else if trust_upgraded {
        store.record_open_commerce_audit(
            destination_project_id,
            actor.user_id,
            Some(actor.app_id),
            "consumer_portability.import_trust_upgraded",
            "consumer_portability_import",
            &import_record.id,
            &json!({
                "source_operator": import_record.source_operator,
                "signer_key_id": import_record.signature.as_ref().map(|value| &value.key_id),
                "signature_verified_at": import_record.signature_verified_at,
            }),
        )?;
    }
    Ok(import_record)
}

pub(crate) fn list_imports(
    store: &Store,
    destination_project_id: &str,
    actor: &OpenCommerceActor<'_>,
    limit: usize,
) -> Result<Vec<ConsumerPortabilityImportSummary>> {
    ensure_consumer_project_actor(actor)?;
    store
        .list_consumer_portability_imports(destination_project_id, actor.user_id, limit)?
        .into_iter()
        .map(|value| verify_import(value, destination_project_id).map(|item| item.summary()))
        .collect()
}

pub(crate) fn get_import(
    store: &Store,
    destination_project_id: &str,
    import_id: &str,
    actor: &OpenCommerceActor<'_>,
) -> Result<ConsumerPortabilityImport> {
    ensure_consumer_project_actor(actor)?;
    let import_id = normalize_import_id(import_id)?;
    let import_record = store
        .consumer_portability_import(destination_project_id, actor.user_id, &import_id)?
        .ok_or_else(|| anyhow!("消费者外部数据包导入记录不存在"))?;
    verify_import(import_record, destination_project_id)
}

pub(crate) fn delete_import(
    store: &Store,
    destination_project_id: &str,
    import_id: &str,
    actor: &OpenCommerceActor<'_>,
) -> Result<ConsumerPortabilityImportSummary> {
    ensure_consumer_project_actor(actor)?;
    let import_id = normalize_import_id(import_id)?;
    let import_record = store
        .delete_consumer_portability_import(destination_project_id, actor.user_id, &import_id)?
        .ok_or_else(|| anyhow!("消费者外部数据包导入记录不存在"))?;
    let import_record = verify_import(import_record, destination_project_id)?;
    store.record_open_commerce_audit(
        destination_project_id,
        actor.user_id,
        Some(actor.app_id),
        "consumer_portability.import_deleted",
        "consumer_portability_import",
        &import_record.id,
        &json!({
            "envelope_sha256": import_record.envelope_sha256,
            "payload_sha256": import_record.payload_sha256,
        }),
    )?;
    Ok(import_record.summary())
}

fn verify_import(
    import_record: ConsumerPortabilityImport,
    expected_project_id: &str,
) -> Result<ConsumerPortabilityImport> {
    if import_record.schema != CONSUMER_PORTABILITY_IMPORT_SCHEMA
        || !matches!(
            import_record.trust_status.as_str(),
            CONSUMER_PORTABILITY_IMPORT_TRUST_STATUS | CONSUMER_PORTABILITY_IMPORT_TRUSTED_STATUS
        )
        || import_record.merge_status != CONSUMER_PORTABILITY_IMPORT_MERGE_STATUS
        || import_record.destination_project_id != expected_project_id.trim()
    {
        bail!("消费者外部数据包导入记录边界无效");
    }
    open_commerce_portability_trust_service::normalize_source_operator(
        &import_record.source_operator,
    )?;
    let package =
        open_commerce_portability_service::verify_external_export(import_record.package.clone())?;
    let package_json = open_commerce_portability_service::canonical_export_json(&package)?;
    if package_json != import_record.package_json
        || package.source_project_id != import_record.source_project_id
        || package.id != import_record.source_package_id
        || package.schema != import_record.source_package_schema
        || package.payload_sha256 != import_record.payload_sha256
    {
        bail!("消费者外部数据包导入记录内容不一致");
    }
    let envelope_sha256 = hex::encode(Sha256::digest(package_json.as_bytes()));
    if envelope_sha256 != import_record.envelope_sha256 {
        bail!("消费者外部数据包信封完整性校验失败");
    }
    match import_record.trust_status.as_str() {
        CONSUMER_PORTABILITY_IMPORT_TRUST_STATUS => {
            if import_record.signature.is_some()
                || import_record.signature_verified_at.is_some()
                || import_record.signer_key_record_id.is_some()
            {
                bail!("未信任导入记录不能包含签名证明");
            }
        }
        CONSUMER_PORTABILITY_IMPORT_TRUSTED_STATUS => {
            if import_record.signature.is_none()
                || import_record.signature_verified_at.is_none()
                || import_record.signer_key_record_id.is_none()
            {
                bail!("可信导入记录缺少签名证明");
            }
        }
        _ => unreachable!(),
    }
    Ok(import_record)
}

fn ensure_consumer_project_actor(actor: &OpenCommerceActor<'_>) -> Result<()> {
    if actor.project_role.is_none() {
        bail!("当前调用方不属于消费者项目");
    }
    Ok(())
}

fn normalize_import_id(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 120 {
        bail!("消费者外部数据包导入记录 ID 长度必须为 1 到 120 个字符");
    }
    Ok(value.to_string())
}

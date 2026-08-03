use anyhow::{bail, Result};
use serde_json::json;

use crate::{
    open_commerce_data_erasure_evidence_model::{
        CreateDataErasureEvidenceRequest, OpenCommerceDataErasureEvidence,
        OpenCommerceDataErasureEvidenceList, ERASURE_EVIDENCE_KIND_EXTERNAL_RECEIPT,
        ERASURE_EVIDENCE_KIND_MERCHANT_ATTESTATION,
    },
    open_commerce_service::OpenCommerceActor,
    project_auth::can_edit,
    store::Store,
};

const LIST_SCHEMA: &str = "open_commerce.consumer_data_erasure_evidence_list.v1";
const BOUNDARY: [&str; 3] = [
    "证明由商户项目编辑者提交，平台未连接外部系统核验删除结果",
    "receipt_sha256 只绑定商户持有的回执内容，不代表平台持有或验证原始回执",
    "证明账本为追加式记录，不会恢复已撤销的消费者关系凭证",
];

pub(crate) fn create_merchant_evidence(
    store: &Store,
    merchant_project_id: &str,
    merchant_id: &str,
    request_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: CreateDataErasureEvidenceRequest,
) -> Result<OpenCommerceDataErasureEvidence> {
    let role = actor
        .project_role
        .ok_or_else(|| anyhow::anyhow!("当前调用方不属于商户项目"))?;
    if !can_edit(role) {
        bail!("只有商户项目编辑者可以提交外部删除证明");
    }
    if !request.merchant_confirmed_unverified {
        bail!("提交前必须确认该证明由商户提供且未经平台核验");
    }
    let merchant_id = require_text(merchant_id, "merchant_id", 120)?;
    let request_id = require_text(request_id, "request_id", 120)?;
    let evidence_kind = require_evidence_kind(&request.evidence_kind)?;
    let external_system = require_text(&request.external_system, "外部系统", 80)?;
    let reference_id = require_text(&request.reference_id, "外部回执编号", 160)?;
    let receipt_sha256 = require_sha256(&request.receipt_sha256)?;
    let summary = require_text(&request.summary, "证明摘要", 500)?;
    let (evidence, created) = store.create_open_commerce_data_erasure_evidence(
        merchant_project_id,
        &merchant_id,
        &request_id,
        actor.user_id,
        &evidence_kind,
        &external_system,
        &reference_id,
        &receipt_sha256,
        &summary,
    )?;
    if created {
        store.record_open_commerce_audit(
            merchant_project_id,
            actor.user_id,
            Some(actor.app_id),
            "consumer_data_erasure.evidence_attached",
            "consumer_data_erasure_evidence",
            &evidence.id,
            &json!({
                "data_request_id": evidence.data_request_id,
                "merchant_id": evidence.merchant_id,
                "evidence_kind": evidence.evidence_kind,
                "external_system": evidence.external_system,
                "reference_id": evidence.reference_id,
                "receipt_sha256": evidence.receipt_sha256,
                "source_authority": evidence.source_authority,
                "platform_verified": false
            }),
        )?;
    }
    Ok(evidence)
}

pub(crate) fn list_consumer_evidence(
    store: &Store,
    consumer_project_id: &str,
    actor: &OpenCommerceActor<'_>,
    limit: usize,
) -> Result<OpenCommerceDataErasureEvidenceList> {
    ensure_project_member(actor, "消费者数据请求项目")?;
    Ok(evidence_list(
        store.list_open_commerce_consumer_data_erasure_evidence(
            consumer_project_id,
            actor.user_id,
            limit,
        )?,
    ))
}

pub(crate) fn list_merchant_evidence(
    store: &Store,
    merchant_project_id: &str,
    merchant_id: &str,
    actor: &OpenCommerceActor<'_>,
    limit: usize,
) -> Result<OpenCommerceDataErasureEvidenceList> {
    ensure_project_member(actor, "商户项目")?;
    Ok(evidence_list(
        store.list_open_commerce_merchant_data_erasure_evidence(
            merchant_project_id,
            merchant_id,
            limit,
        )?,
    ))
}

fn evidence_list(
    evidence: Vec<OpenCommerceDataErasureEvidence>,
) -> OpenCommerceDataErasureEvidenceList {
    OpenCommerceDataErasureEvidenceList {
        schema: LIST_SCHEMA,
        evidence,
        boundary: BOUNDARY.to_vec(),
    }
}

fn ensure_project_member(actor: &OpenCommerceActor<'_>, project_label: &str) -> Result<()> {
    if actor.project_role.is_none() {
        bail!("当前调用方不属于{project_label}");
    }
    Ok(())
}

fn require_evidence_kind(value: &str) -> Result<String> {
    let value = value.trim();
    if !matches!(
        value,
        ERASURE_EVIDENCE_KIND_EXTERNAL_RECEIPT | ERASURE_EVIDENCE_KIND_MERCHANT_ATTESTATION
    ) {
        bail!("删除证明类型无效");
    }
    Ok(value.to_string())
}

fn require_text(value: &str, label: &str, max_len: usize) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max_len || value.chars().any(char::is_control) {
        bail!("{label}长度或格式无效");
    }
    Ok(value.to_string())
}

fn require_sha256(value: &str) -> Result<String> {
    let value = value.trim();
    if value.len() != 64
        || !value
            .chars()
            .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character))
    {
        bail!("回执 SHA-256 必须为 64 位小写十六进制字符串");
    }
    Ok(value.to_string())
}

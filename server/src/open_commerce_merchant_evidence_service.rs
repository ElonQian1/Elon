use anyhow::{anyhow, Result};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    open_commerce_merchant_evidence_model::{
        validate_optional_business_receipt, MerchantBusinessEvidenceDetail,
        MerchantBusinessEvidenceList, MerchantBusinessEvidenceSummary, MerchantEvidenceErpBinding,
        MerchantTerminalInvocationRecord,
    },
    open_commerce_model::HANDLER_MERCHANT_RUNTIME,
    store::Store,
};

const SUMMARY_SCHEMA: &str = "open_commerce.merchant_business_evidence.v1";
const LIST_SCHEMA: &str = "open_commerce.merchant_business_evidence_list.v1";
const DETAIL_SCHEMA: &str = "open_commerce.merchant_business_evidence_detail.v1";
const BOUNDARY: [&str; 4] = [
    "平台只证明能力调用、结果摘要和计量记录存在",
    "结构化业务回执是商户运行时声明，不是平台独立核验的订单事实",
    "funds_moved 固定为 false，不代表支付、分账、履约或退款完成",
    "ERP 关联只标识项目实例，不会自动写入商户订单、库存、财务或 CRM",
];

pub(crate) fn list_evidence(
    store: &Store,
    project_id: &str,
    merchant_id: &str,
    limit: usize,
) -> Result<MerchantBusinessEvidenceList> {
    store.open_commerce_merchant_for_project(project_id, merchant_id)?;
    let erp_binding = erp_binding(store, project_id)?;
    let evidence = store
        .list_open_commerce_merchant_terminal_invocations(project_id, merchant_id, limit)?
        .into_iter()
        .map(|record| summary(store, record, erp_binding.clone()))
        .collect::<Result<Vec<_>>>()?;
    Ok(MerchantBusinessEvidenceList {
        schema: LIST_SCHEMA,
        project_id: project_id.trim().to_string(),
        merchant_id: merchant_id.trim().to_string(),
        erp_binding,
        evidence,
        boundary: BOUNDARY.to_vec(),
    })
}

pub(crate) fn get_evidence(
    store: &Store,
    project_id: &str,
    merchant_id: &str,
    invocation_id: &str,
) -> Result<MerchantBusinessEvidenceDetail> {
    store.open_commerce_merchant_for_project(project_id, merchant_id)?;
    let record = store
        .open_commerce_merchant_terminal_invocation(project_id, merchant_id, invocation_id)?
        .ok_or_else(|| anyhow!("商户业务证据不存在"))?;
    let result = record.invocation.result.clone();
    Ok(MerchantBusinessEvidenceDetail {
        schema: DETAIL_SCHEMA,
        evidence: summary(store, record, erp_binding(store, project_id)?)?,
        result,
        boundary: BOUNDARY.to_vec(),
    })
}

fn summary(
    store: &Store,
    record: MerchantTerminalInvocationRecord,
    erp_binding: Option<MerchantEvidenceErpBinding>,
) -> Result<MerchantBusinessEvidenceSummary> {
    let invocation = record.invocation;
    let capability = store.open_commerce_capability(&invocation.capability_id)?;
    let completed_at = invocation
        .completed_at
        .clone()
        .ok_or_else(|| anyhow!("终态调用缺少完成时间"))?;
    let result_sha256 = invocation.result.as_ref().map(result_digest).transpose()?;
    let source_authority = if capability.handler_type == HANDLER_MERCHANT_RUNTIME {
        "merchant_runtime_asserted"
    } else {
        "platform_handler_result"
    };
    let (receipt_state, business_receipt) = receipt_projection(
        &capability.handler_type,
        invocation.status.as_str(),
        invocation.result.as_ref(),
    );
    Ok(MerchantBusinessEvidenceSummary {
        schema: SUMMARY_SCHEMA,
        sequence: record.sequence,
        invocation_id: invocation.id,
        merchant_id: invocation.merchant_id,
        erp_binding,
        capability_key: invocation.capability_key,
        capability_kind: capability.kind,
        requester_app_id: invocation.requester_app_id,
        status: invocation.status,
        source_authority,
        receipt_state,
        business_receipt,
        result_available: invocation.result.is_some(),
        result_sha256,
        error_code: invocation.error_code,
        amount_micros: invocation.amount_micros,
        currency: invocation.currency,
        settlement_status: invocation.settlement_status,
        funds_moved: false,
        created_at: invocation.created_at,
        completed_at,
    })
}

fn receipt_projection(
    handler_type: &str,
    status: &str,
    result: Option<&Value>,
) -> (
    &'static str,
    Option<crate::open_commerce_merchant_evidence_model::MerchantBusinessReceipt>,
) {
    if status != "succeeded" {
        return ("not_available", None);
    }
    if handler_type != HANDLER_MERCHANT_RUNTIME {
        return ("not_applicable", None);
    }
    match result.map(validate_optional_business_receipt) {
        Some(Ok(Some(receipt))) => ("valid", Some(receipt)),
        Some(Ok(None)) => ("digest_only", None),
        Some(Err(_)) => ("invalid_legacy", None),
        None => ("not_available", None),
    }
}

fn erp_binding(store: &Store, project_id: &str) -> Result<Option<MerchantEvidenceErpBinding>> {
    Ok(store
        .erp_instance_for_project(project_id)?
        .map(|instance| MerchantEvidenceErpBinding {
            instance_id: instance.id,
            instance_key: instance.instance_key,
            configuration_revision: instance.configuration_revision,
        }))
}

fn result_digest(result: &Value) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(result)?)))
}

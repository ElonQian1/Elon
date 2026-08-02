use anyhow::{bail, Result};
use serde_json::json;

use crate::{
    open_commerce_business_handoff_model::{
        normalize_handoff_error_code, normalize_handoff_queue_state, normalize_handoff_status,
        normalize_sha256, normalize_target_reference, OpenCommerceBusinessHandoffQueue,
        OpenCommerceBusinessHandoffQueueItem, OpenCommerceBusinessHandoffReceipt,
        OpenCommerceBusinessHandoffReceiptList, RecordBusinessHandoffReceiptRequest,
        BUSINESS_HANDOFF_LIST_SCHEMA, BUSINESS_HANDOFF_QUEUE_ITEM_SCHEMA,
        BUSINESS_HANDOFF_QUEUE_SCHEMA,
    },
    open_commerce_merchant_evidence_service,
    open_commerce_service::OpenCommerceActor,
    project_auth::can_edit,
    store::{RecordOpenCommerceBusinessHandoffReceipt, Store},
};

const BOUNDARY: [&str; 4] = [
    "衔接回执是项目编辑者或其应用的声明，不是平台对外部 ERP/CRM 的独立核验",
    "applied 只允许绑定有效标准业务回执，且不会复制商户订单、客户、库存或财务表",
    "目标记录号只保存 SHA-256，不在平台保留外部系统的原始记录号",
    "funds_moved 固定为 false，不代表支付、分账、履约或退款完成",
];

const QUEUE_BOUNDARY: [&str; 4] = [
    "待衔接队列由终态业务证据与最新衔接回执实时派生，不是另一套订单状态",
    "pending 表示尚无衔接回执；retry_required 表示最新回执为 rejected",
    "applied 或 ignored 的证据会自动移出队列，新的 rejected 回执会重新进入队列",
    "队列不自动调用外部 ERP/CRM，不代表支付、履约、退款或资金移动",
];

pub(crate) fn record_receipt(
    store: &Store,
    project_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: RecordBusinessHandoffReceiptRequest,
) -> Result<OpenCommerceBusinessHandoffReceipt> {
    require_editor(actor.project_role)?;
    if !request.confirmed_by_user {
        bail!("必须由当前用户明确确认真实 ERP/CRM 衔接结果");
    }

    let evidence = open_commerce_merchant_evidence_service::get_evidence(
        store,
        project_id,
        &request.merchant_id,
        &request.invocation_id,
    )?;
    let expected_digest = evidence
        .evidence
        .result_sha256
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("该业务证据没有可绑定的结果摘要"))?;
    let supplied_digest = normalize_sha256(&request.evidence_result_sha256, "业务证据摘要")?;
    if supplied_digest != expected_digest {
        bail!("业务证据摘要已变化或与当前调用结果不匹配");
    }

    validate_outcome(
        &request,
        evidence.evidence.status.as_str(),
        evidence.evidence.receipt_state,
    )?;
    let (receipt, created) = store.record_open_commerce_business_handoff_receipt(
        RecordOpenCommerceBusinessHandoffReceipt {
            project_id,
            actor_user_id: actor.user_id,
            actor_app_id: actor.app_id,
            request,
        },
    )?;
    if created {
        store.record_open_commerce_audit(
            project_id,
            actor.user_id,
            Some(actor.app_id),
            "business_handoff.recorded",
            "business_handoff_receipt",
            &receipt.id,
            &json!({
                "merchant_id":receipt.merchant_id,
                "invocation_id":receipt.invocation_id,
                "integration_id":receipt.integration_id,
                "receipt_key":receipt.receipt_key,
                "status":receipt.status,
                "target_domain":receipt.target_domain,
                "evidence_result_sha256":receipt.evidence_result_sha256,
                "target_reference_sha256":receipt.target_reference_sha256,
                "error_code":receipt.error_code,
                "assertion_authority":receipt.assertion_authority,
                "funds_moved":false
            }),
        )?;
    }
    Ok(receipt)
}

pub(crate) fn list_receipts(
    store: &Store,
    project_id: &str,
    merchant_id: &str,
    limit: usize,
) -> Result<OpenCommerceBusinessHandoffReceiptList> {
    Ok(OpenCommerceBusinessHandoffReceiptList {
        schema: BUSINESS_HANDOFF_LIST_SCHEMA,
        project_id: project_id.trim().to_string(),
        merchant_id: merchant_id.trim().to_string(),
        receipts: store.list_open_commerce_business_handoff_receipts(
            project_id,
            merchant_id,
            limit,
        )?,
        boundary: BOUNDARY.to_vec(),
    })
}

pub(crate) fn list_queue(
    store: &Store,
    project_id: &str,
    merchant_id: &str,
    state: Option<&str>,
    limit: usize,
) -> Result<OpenCommerceBusinessHandoffQueue> {
    let state_filter = normalize_handoff_queue_state(state)?;
    let limit = limit.clamp(1, 200);
    let mut records = store.list_open_commerce_business_handoff_queue_records(
        project_id,
        merchant_id,
        state_filter.as_deref(),
        limit.saturating_add(1),
    )?;
    let has_more = records.len() > limit;
    records.truncate(limit);

    let mut items = Vec::with_capacity(records.len());
    for record in records {
        let invocation_id = record.invocation.id.clone();
        let evidence = open_commerce_merchant_evidence_service::get_evidence(
            store,
            project_id,
            merchant_id,
            &invocation_id,
        )?
        .evidence;
        let latest_receipt = store.latest_open_commerce_business_handoff_receipt(
            project_id,
            merchant_id,
            &invocation_id,
        )?;
        let queue_state = match latest_receipt
            .as_ref()
            .map(|receipt| receipt.status.as_str())
        {
            None => "pending",
            Some("rejected") => "retry_required",
            Some("applied" | "ignored") => continue,
            Some(_) => continue,
        };
        items.push(OpenCommerceBusinessHandoffQueueItem {
            schema: BUSINESS_HANDOFF_QUEUE_ITEM_SCHEMA,
            queue_state,
            can_apply: evidence.status == "succeeded" && evidence.receipt_state == "valid",
            evidence,
            latest_receipt,
        });
    }
    let returned_pending_count = items
        .iter()
        .filter(|item| item.queue_state == "pending")
        .count();
    let returned_retry_required_count = items.len() - returned_pending_count;

    Ok(OpenCommerceBusinessHandoffQueue {
        schema: BUSINESS_HANDOFF_QUEUE_SCHEMA,
        project_id: project_id.trim().to_string(),
        merchant_id: merchant_id.trim().to_string(),
        state_filter,
        items,
        returned_pending_count,
        returned_retry_required_count,
        has_more,
        boundary: QUEUE_BOUNDARY.to_vec(),
    })
}

fn validate_outcome(
    request: &RecordBusinessHandoffReceiptRequest,
    invocation_status: &str,
    receipt_state: &str,
) -> Result<()> {
    let status = normalize_handoff_status(&request.status)?;
    let target_reference = normalize_target_reference(request.target_reference.as_deref())?;
    let error_code = normalize_handoff_error_code(request.error_code.as_deref())?;
    match status.as_str() {
        "applied" => {
            if invocation_status != "succeeded" || receipt_state != "valid" {
                bail!("只有带有效标准业务回执的成功调用才能声明为已应用");
            }
            if target_reference.is_none() {
                bail!("声明已应用时必须提供外部 ERP/CRM 目标记录号");
            }
            if error_code.is_some() {
                bail!("声明已应用时不能同时提供错误代码");
            }
        }
        "ignored" | "rejected" => {
            if target_reference.is_some() {
                bail!("忽略或拒绝时不能声明外部目标记录号");
            }
            if error_code.is_none() {
                bail!("忽略或拒绝时必须提供可审计的结果代码");
            }
        }
        _ => unreachable!("status was normalized"),
    }
    Ok(())
}

fn require_editor(role: Option<&str>) -> Result<()> {
    if !role.is_some_and(can_edit) {
        bail!("只有项目编辑者可以记录 ERP/CRM 衔接结果");
    }
    Ok(())
}

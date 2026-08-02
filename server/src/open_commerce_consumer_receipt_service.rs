use anyhow::{anyhow, bail, Context, Result};
use sha2::{Digest, Sha256};

use crate::{
    open_commerce_consumer_receipt_model::{
        ConsumerInvocationReceipt, ConsumerInvocationReceiptList, ConsumerInvocationReceiptPayload,
        ConsumerInvocationReceiptSummary, ConsumerInvocationRequestShape,
        CONSUMER_RECEIPT_PAYLOAD_SCHEMA, CONSUMER_RECEIPT_SCHEMA,
    },
    open_commerce_model::{OpenCommerceInvocation, SETTLEMENT_RECORDED_NOT_CHARGED},
    store::Store,
};

const MAX_RECEIPT_BYTES: usize = 5 * 1024 * 1024;

pub(crate) fn list_receipts(
    store: &Store,
    requester_user_id: &str,
    limit: usize,
) -> Result<ConsumerInvocationReceiptList> {
    ensure_user_id(requester_user_id)?;
    let receipts = store
        .list_user_open_commerce_terminal_invocations(requester_user_id, limit)?
        .into_iter()
        .map(summary)
        .collect::<Result<Vec<_>>>()?;
    Ok(ConsumerInvocationReceiptList {
        schema: "open_commerce.consumer_invocation_receipts.v1",
        scope: "authenticated_user_account",
        receipts,
    })
}

pub(crate) fn get_receipt(
    store: &Store,
    requester_user_id: &str,
    invocation_id: &str,
) -> Result<ConsumerInvocationReceipt> {
    ensure_user_id(requester_user_id)?;
    let invocation_id = invocation_id.trim();
    if invocation_id.is_empty() || invocation_id.chars().count() > 120 {
        bail!("消费者调用凭证 ID 长度必须为 1 到 120 个字符");
    }
    let invocation = store
        .user_open_commerce_terminal_invocation(requester_user_id, invocation_id)?
        .ok_or_else(|| anyhow!("消费者调用凭证不存在"))?;
    receipt_from_invocation(invocation)
}

fn summary(invocation: OpenCommerceInvocation) -> Result<ConsumerInvocationReceiptSummary> {
    ensure_unfunded_settlement(&invocation)?;
    let completed_at = invocation
        .completed_at
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("终态调用缺少完成时间，无法生成消费者凭证摘要"))?;
    Ok(ConsumerInvocationReceiptSummary {
        invocation_id: invocation.id,
        merchant_id: invocation.merchant_id,
        capability_key: invocation.capability_key,
        requester_app_id: invocation.requester_app_id,
        status: invocation.status,
        result_available: invocation.result.is_some(),
        error_code: invocation.error_code,
        amount_micros: invocation.amount_micros,
        currency: invocation.currency,
        settlement_status: invocation.settlement_status,
        funds_moved: false,
        created_at: invocation.created_at,
        completed_at,
    })
}

pub(crate) fn receipt_from_invocation(
    invocation: OpenCommerceInvocation,
) -> Result<ConsumerInvocationReceipt> {
    ensure_unfunded_settlement(&invocation)?;
    let completed_at = invocation
        .completed_at
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("终态调用缺少完成时间，无法生成消费者凭证"))?;
    let payload = ConsumerInvocationReceiptPayload {
        schema: CONSUMER_RECEIPT_PAYLOAD_SCHEMA.to_string(),
        invocation_id: invocation.id,
        merchant_id: invocation.merchant_id,
        capability_key: invocation.capability_key,
        requester_app_id: invocation.requester_app_id,
        request_shape: safe_request_shape(&invocation.request_shape)?,
        status: invocation.status,
        result: invocation.result,
        error_code: invocation.error_code,
        units: invocation.units,
        unit_price_micros: invocation.unit_price_micros,
        amount_micros: invocation.amount_micros,
        currency: invocation.currency,
        settlement_status: invocation.settlement_status,
        funds_moved: false,
        created_at: invocation.created_at,
        completed_at,
    };
    let payload_json = serde_json::to_string(&payload).context("序列化消费者调用凭证失败")?;
    if payload_json.len() > MAX_RECEIPT_BYTES {
        bail!("消费者调用凭证超过 V1 的 5 MiB 上限");
    }
    Ok(ConsumerInvocationReceipt {
        schema: CONSUMER_RECEIPT_SCHEMA.to_string(),
        payload_sha256: hex::encode(Sha256::digest(payload_json.as_bytes())),
        payload_json,
        payload,
    })
}

fn safe_request_shape(value: &serde_json::Value) -> Result<ConsumerInvocationRequestShape> {
    let object = value
        .as_object()
        .ok_or_else(|| anyhow!("调用字段摘要格式无效"))?;
    if object.get("contains_raw_values") != Some(&serde_json::Value::Bool(false)) {
        bail!("调用字段摘要不能包含原始值");
    }
    let input_bytes = object
        .get("input_bytes")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| anyhow!("调用字段摘要缺少 input_bytes"))?;
    let fields = object
        .get("input_fields")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow!("调用字段摘要缺少 input_fields"))?;
    if fields.len() > 200 {
        bail!("调用字段摘要超过 200 个字段");
    }
    let input_fields = fields
        .iter()
        .map(|field| {
            field
                .as_str()
                .filter(|field| !field.is_empty() && field.chars().count() <= 128)
                .map(str::to_string)
                .ok_or_else(|| anyhow!("调用字段摘要包含无效字段名"))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(ConsumerInvocationRequestShape {
        input_fields,
        input_bytes,
        contains_raw_values: false,
    })
}

fn ensure_unfunded_settlement(invocation: &OpenCommerceInvocation) -> Result<()> {
    if invocation.settlement_status != SETTLEMENT_RECORDED_NOT_CHARGED {
        bail!("当前结算状态不能生成未扣真实资金的 V1 消费者凭证");
    }
    Ok(())
}

fn ensure_user_id(value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("消费者身份不能为空");
    }
    Ok(())
}

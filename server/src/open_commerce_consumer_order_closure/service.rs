use anyhow::{anyhow, bail, Result};

use crate::{
    open_commerce_consumer_receipt_service,
    open_commerce_merchant_evidence_model::validate_optional_business_receipt, store::Store,
};

use super::model::{
    ConsumerOrderClosure, ConsumerOrderErpHandoff, ConsumerOrderInvocation,
    ConsumerOrderPlatformMeter, CONSUMER_ORDER_CLOSURE_SCHEMA,
};

pub(crate) fn get_order_closure(
    store: &Store,
    requester_user_id: &str,
    invocation_id: &str,
) -> Result<ConsumerOrderClosure> {
    let requester_user_id = requester_user_id.trim();
    let invocation_id = invocation_id.trim();
    if requester_user_id.is_empty() {
        bail!("消费者身份不能为空");
    }
    if invocation_id.is_empty() || invocation_id.chars().count() > 120 {
        bail!("消费者订单闭环 ID 长度必须为 1 到 120 个字符");
    }

    let invocation = store
        .user_open_commerce_terminal_invocation(requester_user_id, invocation_id)?
        .ok_or_else(|| anyhow!("消费者订单闭环不存在"))?;
    if invocation.status != "succeeded" {
        bail!("消费者订单闭环不存在");
    }
    let project_id = invocation.project_id.clone();
    let merchant_id = invocation.merchant_id.clone();
    let consumer_receipt =
        open_commerce_consumer_receipt_service::receipt_from_invocation(invocation)?;
    let payload = consumer_receipt.payload;
    let result = payload
        .result
        .clone()
        .ok_or_else(|| anyhow!("消费者订单闭环不存在"))?;
    let merchant_order = validate_optional_business_receipt(&result)?
        .filter(|receipt| receipt.entity_type == "order")
        .ok_or_else(|| anyhow!("消费者订单闭环不存在"))?;

    let latest_handoff = store.latest_open_commerce_business_handoff_receipt(
        &project_id,
        &merchant_id,
        invocation_id,
    )?;
    let (closure_status, erp_handoff) = match latest_handoff {
        None => ("merchant_confirmed_erp_pending", None),
        Some(receipt) => {
            if receipt.funds_moved {
                bail!("ERP 衔接回执超出 V1 零资金边界");
            }
            let closure_status = match receipt.status.as_str() {
                "applied" => "erp_recorded",
                "rejected" => "erp_retry_required",
                "ignored" => "erp_ignored",
                _ => bail!("ERP 衔接回执状态不受支持"),
            };
            (
                closure_status,
                Some(ConsumerOrderErpHandoff {
                    status: receipt.status,
                    target_domain: receipt.target_domain,
                    target_reference_sha256: receipt.target_reference_sha256,
                    error_code: receipt.error_code,
                    assertion_authority: receipt.assertion_authority,
                    completed_at: receipt.completed_at,
                    funds_moved: false,
                }),
            )
        }
    };

    Ok(ConsumerOrderClosure {
        schema: CONSUMER_ORDER_CLOSURE_SCHEMA,
        scope: "authenticated_consumer_account",
        invocation: ConsumerOrderInvocation {
            invocation_id: payload.invocation_id,
            merchant_id: payload.merchant_id,
            capability_key: payload.capability_key,
            requester_app_id: payload.requester_app_id,
            status: payload.status,
            error_code: payload.error_code,
            created_at: payload.created_at,
            completed_at: payload.completed_at,
        },
        merchant_order,
        merchant_statement_authority: "merchant_invocation_result_asserted",
        result,
        platform_meter: ConsumerOrderPlatformMeter {
            units: payload.units,
            unit_price_micros: payload.unit_price_micros,
            amount_micros: payload.amount_micros,
            currency: payload.currency,
            settlement_status: payload.settlement_status,
            funds_moved: false,
        },
        erp_handoff,
        closure_status,
        funds_moved: false,
        boundary: vec![
            "订单金额来自商户标准业务回执；平台调用计量使用独立的 micros 字段。",
            "ERP 状态来自最新衔接回执，仅证明目标系统声明的处理结果。",
            "该视图不证明真实支付、配送、履约、退款或链上结算。",
        ],
    })
}

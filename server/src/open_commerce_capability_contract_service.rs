//! Invocation-time enforcement helpers for open-commerce capability contracts.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::{
    open_commerce_model::{OpenCommerceCapability, OpenCommerceInvocation, OpenCommerceMerchant},
    open_commerce_service::OpenCommerceActor,
    store::Store,
};

pub(crate) fn validate_replayed_output(
    store: &Store,
    actor: &OpenCommerceActor<'_>,
    merchant: &OpenCommerceMerchant,
    capability: &OpenCommerceCapability,
    requester_app_id: &str,
    invocation: &OpenCommerceInvocation,
) -> Result<()> {
    if invocation.status != "succeeded" {
        return Ok(());
    }
    let result = invocation
        .result
        .as_ref()
        .ok_or_else(|| anyhow!("成功调用缺少结果，无法验证当前能力契约"))?;
    if let Err(error) =
        crate::open_commerce_capability_schema::validate_output(&capability.output_schema, result)
    {
        store.record_open_commerce_audit(
            &merchant.project_id,
            actor.user_id,
            Some(requester_app_id),
            "invocation.replay_contract_rejected",
            "invocation",
            &invocation.id,
            &json!({
                "merchant_id": merchant.id,
                "capability_key": capability.capability_key,
                "contract_path": error.path,
                "contract_code": error.code
            }),
        )?;
        return Err(anyhow::Error::new(error));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn record_failed_invocation(
    store: &Store,
    actor: &OpenCommerceActor<'_>,
    merchant: &OpenCommerceMerchant,
    capability: &OpenCommerceCapability,
    requester_app_id: &str,
    invocation_id: &str,
    error_code: &str,
    contract_issue: Option<(&str, &str)>,
) -> Result<()> {
    let failed = store.finish_open_commerce_invocation_failure(invocation_id, error_code)?;
    let mut metadata = json!({
        "merchant_id": merchant.id,
        "capability_key": capability.capability_key,
        "error_code": error_code
    });
    if let Some((path, code)) = contract_issue {
        metadata["contract_path"] = Value::String(path.to_string());
        metadata["contract_code"] = Value::String(code.to_string());
    }
    store.record_open_commerce_audit(
        &merchant.project_id,
        actor.user_id,
        Some(requester_app_id),
        "invocation.failed",
        "invocation",
        &failed.id,
        &metadata,
    )?;
    Ok(())
}

use anyhow::Result;
use serde_json::json;

use crate::{
    open_commerce_grant_budget_model::{
        OpenCommerceGrantBudgetDecision, OpenCommerceGrantBudgetExceeded,
    },
    open_commerce_model::{OpenCommerceCapability, OpenCommerceInvocation, OpenCommerceMerchant},
    store::Store,
};

pub(crate) fn enforce_invocation(
    store: &Store,
    merchant: &OpenCommerceMerchant,
    capability: &OpenCommerceCapability,
    requester_user_id: &str,
    requester_app_id: &str,
    invocation: &OpenCommerceInvocation,
) -> Result<Option<OpenCommerceGrantBudgetDecision>> {
    match store.reserve_open_commerce_grant_budget(invocation) {
        Ok(decision) => Ok(decision),
        Err(error) => {
            let exceeded = error.is::<OpenCommerceGrantBudgetExceeded>();
            let error_code = if exceeded {
                "grant_budget_exceeded"
            } else {
                "grant_budget_rejected"
            };
            let action = if exceeded {
                "invocation.grant_budget_exceeded"
            } else {
                "invocation.grant_budget_rejected"
            };
            let failed =
                store.finish_open_commerce_invocation_failure(&invocation.id, error_code)?;
            store.record_open_commerce_audit(
                &merchant.project_id,
                requester_user_id,
                Some(requester_app_id),
                action,
                "invocation",
                &failed.id,
                &json!({
                    "merchant_id": merchant.id,
                    "capability_key": capability.capability_key,
                    "grant_id": invocation.grant_id,
                    "error_code": error_code
                }),
            )?;
            Err(error)
        }
    }
}

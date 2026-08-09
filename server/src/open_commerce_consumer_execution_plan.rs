//! Read-only readiness planning between consumer discovery and capability invocation.

use anyhow::{bail, Result};
use serde::Serialize;
use serde_json::Value;

use crate::{
    open_commerce_app_block_service, open_commerce_capability_schema,
    open_commerce_directory_model::{
        OpenCommerceDirectoryCapability, OpenCommerceDirectoryMerchant,
    },
    open_commerce_directory_service, open_commerce_grant_readiness,
    open_commerce_model::{
        normalize_app_id, normalize_capability_key, validate_json_object, ACCESS_AUTHORIZED,
        ACCESS_PUBLIC,
    },
    store::Store,
};

#[derive(Debug, Serialize)]
pub(crate) struct ConsumerCapabilityExecutionPlan {
    pub schema: &'static str,
    pub side_effects_created: bool,
    pub requester_app_id: String,
    pub app_identity_kind: &'static str,
    pub merchant: OpenCommerceDirectoryMerchant,
    pub capability: OpenCommerceDirectoryCapability,
    pub input_valid: bool,
    pub readiness: &'static str,
    pub grant_id: Option<String>,
    pub grant_budget_status: Option<&'static str>,
    pub authorization_request_id: Option<String>,
    pub next_steps: Vec<ConsumerCapabilityExecutionStep>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ConsumerCapabilityExecutionStep {
    pub order: usize,
    pub key: &'static str,
    pub label: &'static str,
    pub mcp_tool: Option<&'static str>,
    pub requires_explicit_user_confirmation: bool,
}

pub(crate) fn plan(
    store: &Store,
    user_id: &str,
    requester_app_id: &str,
    uses_default_mcp_identity: bool,
    merchant_id: &str,
    capability_key: &str,
    input: &Value,
) -> Result<ConsumerCapabilityExecutionPlan> {
    let requester_app_id = normalize_app_id(requester_app_id)?;
    let capability_key = normalize_capability_key(capability_key)?;
    let input = validate_json_object(input, "拟调用输入")?;
    let detail = open_commerce_directory_service::discover_merchant(store, merchant_id)?;
    let capability = detail
        .capabilities
        .iter()
        .find(|capability| capability.capability_key == capability_key)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("公开目录中不存在该能力"))?;
    open_commerce_capability_schema::validate_input(&capability.input_schema, &input)
        .map_err(anyhow::Error::new)?;

    if !uses_default_mcp_identity {
        store.ensure_open_commerce_developer_app_owned_by_user(&requester_app_id, user_id)?;
    }
    open_commerce_app_block_service::ensure_app_allowed(
        store,
        &detail.merchant.id,
        &requester_app_id,
        false,
    )?;

    let (readiness, grant_id, grant_budget_status, authorization_request_id) =
        match capability.access_level.as_str() {
            ACCESS_PUBLIC => (ready_status(&capability.kind)?, None, None, None),
            ACCESS_AUTHORIZED if uses_default_mcp_identity => {
                ("app_registration_required", None, None, None)
            }
            ACCESS_AUTHORIZED => {
                let grants = store.list_active_open_commerce_grant_records_for_app_capability(
                    &detail.merchant.id,
                    &requester_app_id,
                    &capability.capability_key,
                )?;
                if let Some((grant, budget_readiness)) = open_commerce_grant_readiness::select_best(
                    &grants,
                    capability.unit_price_micros,
                    &capability.currency,
                ) {
                    if budget_readiness.is_available() {
                        (
                            ready_status(&capability.kind)?,
                            Some(grant.id.clone()),
                            Some(budget_readiness.key()),
                            None,
                        )
                    } else {
                        let request_id = store.pending_authorization_for_app_capability(
                            &detail.merchant.id,
                            &requester_app_id,
                            &capability.capability_key,
                        )?;
                        if request_id.is_some() {
                            (
                                "authorization_pending",
                                Some(grant.id.clone()),
                                Some(budget_readiness.key()),
                                request_id,
                            )
                        } else {
                            (
                                "grant_refresh_required",
                                Some(grant.id.clone()),
                                Some(budget_readiness.key()),
                                None,
                            )
                        }
                    }
                } else {
                    let request_id = store.pending_authorization_for_app_capability(
                        &detail.merchant.id,
                        &requester_app_id,
                        &capability.capability_key,
                    )?;
                    if request_id.is_some() {
                        ("authorization_pending", None, None, request_id)
                    } else {
                        ("authorization_request_required", None, None, None)
                    }
                }
            }
            _ => bail!("公开目录能力访问级别无效"),
        };

    Ok(ConsumerCapabilityExecutionPlan {
        schema: "open_commerce.consumer_capability_execution_plan.v1",
        side_effects_created: false,
        requester_app_id,
        app_identity_kind: if uses_default_mcp_identity {
            "mcp_default_system"
        } else {
            "registered_developer_app"
        },
        merchant: detail.merchant,
        capability,
        input_valid: true,
        readiness,
        grant_id,
        grant_budget_status,
        authorization_request_id,
        next_steps: next_steps(readiness),
    })
}

fn ready_status(kind: &str) -> Result<&'static str> {
    match kind {
        "query" => Ok("invoke_ready"),
        "action" => Ok("action_confirmation_required"),
        _ => bail!("公开目录能力类型无效"),
    }
}

fn next_steps(readiness: &str) -> Vec<ConsumerCapabilityExecutionStep> {
    let steps = match readiness {
        "invoke_ready" => vec![(
            "invoke",
            "使用幂等键调用能力",
            Some("open_commerce_invoke"),
            false,
        )],
        "action_confirmation_required" => vec![
            (
                "prepare_action_confirmation",
                "准备输入绑定的短时动作确认",
                Some("open_commerce_prepare_action_confirmation"),
                false,
            ),
            (
                "obtain_explicit_user_confirmation",
                "向当前用户展示动作并取得明确同意",
                None,
                true,
            ),
            (
                "confirm_action_confirmation",
                "确认用户已同意当前动作",
                Some("open_commerce_confirm_action_confirmation"),
                true,
            ),
            (
                "invoke",
                "携带一次性确认和幂等键调用能力",
                Some("open_commerce_invoke"),
                false,
            ),
        ],
        "app_registration_required" => vec![(
            "register_developer_app",
            "在开发者门户注册独立 App 并以该 App 身份重新规划",
            None,
            true,
        )],
        "authorization_request_required" => vec![(
            "request_authorization",
            "经用户明确同意后向商户提交单能力授权申请",
            Some("open_commerce_request_consumer_authorization"),
            true,
        )],
        "grant_refresh_required" => vec![(
            "request_grant_refresh",
            "经用户明确同意后向商户申请新的期限或预算",
            Some("open_commerce_request_consumer_authorization"),
            true,
        )],
        "authorization_pending" => vec![(
            "wait_for_merchant_decision",
            "等待商户批准或拒绝现有授权申请",
            None,
            false,
        )],
        _ => Vec::new(),
    };
    steps
        .into_iter()
        .enumerate()
        .map(
            |(index, (key, label, mcp_tool, requires_explicit_user_confirmation))| {
                ConsumerCapabilityExecutionStep {
                    order: index + 1,
                    key,
                    label,
                    mcp_tool,
                    requires_explicit_user_confirmation,
                }
            },
        )
        .collect()
}

#[cfg(test)]
#[path = "open_commerce_consumer_mcp_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "open_commerce_consumer_execution_plan_tests.rs"]
mod tests;

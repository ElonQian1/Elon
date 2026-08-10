//! Consumer-facing MCP discovery backed by the same policy service as the PC sandbox.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    open_commerce_consumer, open_commerce_consumer_authorization_mcp,
    open_commerce_consumer_execution_plan, open_commerce_consumer_model::ConsumerDiscoveryRequest,
    open_commerce_developer_model::CreateAuthorizationRequest, open_commerce_grant_readiness,
    open_commerce_model::normalize_capability_key, store::Store,
};

const DISCOVER_FOR_CONSUMER: &str = "open_commerce_discover_for_consumer";
const PLAN_CONSUMER_CAPABILITY: &str = "open_commerce_plan_consumer_capability";
const REQUEST_CONSUMER_AUTHORIZATION: &str = "open_commerce_request_consumer_authorization";
const REQUEST_AUTHORIZATION_PHRASE: &str = "REQUEST_AUTHORIZATION";

#[derive(Debug, Deserialize)]
struct PlanArguments {
    merchant_id: String,
    capability_key: String,
    #[serde(default = "empty_object")]
    input: Value,
}

#[derive(Debug, Deserialize)]
struct RequestAuthorizationArguments {
    merchant_id: String,
    capability_key: String,
    purpose: String,
    confirmation_phrase: String,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        json!({
            "name": DISCOVER_FOR_CONSUMER,
            "description": "按消费者明确提供的偏好和硬约束发现已发布商户能力。返回透明非付费排序、候选范围、来源声明和授权状态；只读，不申请授权、不调用能力、不下单。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type":"string","maxLength":200},
                    "capability_key": {"type":"string","maxLength":96},
                    "ranking_policy": {
                        "type":"string",
                        "enum":[
                            "transparent_preference_match.v1",
                            "lowest_unit_price.v1",
                            "public_access_first.v1",
                            "recently_updated.v1",
                            "merchant_name.v1"
                        ]
                    },
                    "include_ranking_receipt": {"type":"boolean","default":false},
                    "require_current_declaration": {"type":"boolean","default":false},
                    "require_internal_sync_receipt": {"type":"boolean","default":false},
                    "source_provider_key": {"type":"string","maxLength":64},
                    "source_data_domain": {"type":"string","maxLength":64},
                    "max_source_age_seconds": {"type":"integer","minimum":1,"maximum":31536000},
                    "price_currency": {"type":"string","minLength":3,"maxLength":3},
                    "capability_kind": {"type":"string","enum":["query","action"]},
                    "access_level": {"type":"string","enum":["public","authorized"]},
                    "require_city_match": {"type":"boolean","default":false},
                    "require_category_match": {"type":"boolean","default":false},
                    "require_all_tags_match": {"type":"boolean","default":false},
                    "preferences": {
                        "type":"object",
                        "properties": {
                            "categories": {
                                "type":"array","maxItems":20,
                                "items":{"type":"string","maxLength":80}
                            },
                            "tags": {
                                "type":"array","maxItems":40,
                                "items":{"type":"string","maxLength":80}
                            },
                            "city": {"type":"string","maxLength":120},
                            "max_unit_price_micros": {
                                "type":"integer","minimum":0,"maximum":1000000000000000i64
                            },
                            "prefer_public": {"type":"boolean","default":false}
                        },
                        "additionalProperties":false,
                        "default":{}
                    },
                    "limit": {"type":"integer","minimum":1,"maximum":50,"default":10}
                },
                "additionalProperties":false
            },
            "annotations": {
                "readOnlyHint":true,
                "destructiveHint":false,
                "idempotentHint":true,
                "openWorldHint":true
            }
        }),
        json!({
            "name":PLAN_CONSUMER_CAPABILITY,
            "description":"只读校验一个已发现能力及拟调用输入，并返回调用、授权申请或动作确认的下一步。不会创建授权、确认、调用、订单、计量或结算记录。",
            "inputSchema":{
                "type":"object",
                "required":["merchant_id","capability_key"],
                "properties":{
                    "merchant_id":{"type":"string","minLength":1,"maxLength":120},
                    "capability_key":{"type":"string","minLength":3,"maxLength":96},
                    "input":{"type":"object","default":{}}
                },
                "additionalProperties":false
            },
            "annotations":{
                "readOnlyHint":true,
                "destructiveHint":false,
                "idempotentHint":true,
                "openWorldHint":true
            }
        }),
        json!({
            "name":REQUEST_CONSUMER_AUTHORIZATION,
            "description":"仅在当前用户明确同意后，以 MCP 入口固定的已注册开发者 App 身份，向商户申请一个 authorized 能力。商户仍独立决定批准、期限和预算；工具不会自行批准、调用或下单。",
            "inputSchema":{
                "type":"object",
                "required":["merchant_id","capability_key","purpose","confirmation_phrase"],
                "properties":{
                    "merchant_id":{"type":"string","minLength":1,"maxLength":120},
                    "capability_key":{"type":"string","minLength":3,"maxLength":96},
                    "purpose":{"type":"string","minLength":3,"maxLength":200},
                    "confirmation_phrase":{"const":"REQUEST_AUTHORIZATION"}
                },
                "additionalProperties":false
            },
            "annotations":{
                "readOnlyHint":false,
                "destructiveHint":false,
                "idempotentHint":true,
                "openWorldHint":true
            }
        }),
    ]
}

pub(crate) fn call_if_handled(
    store: &Store,
    project_id: &str,
    user_id: &str,
    app_id: &str,
    uses_default_mcp_identity: bool,
    name: &str,
    arguments: Value,
) -> Result<Option<Value>> {
    if !matches!(
        name,
        DISCOVER_FOR_CONSUMER | PLAN_CONSUMER_CAPABILITY | REQUEST_CONSUMER_AUTHORIZATION
    ) {
        return Ok(None);
    }
    if !uses_default_mcp_identity {
        ensure_app_identity_in_project(store, project_id, user_id, app_id)?;
    }
    match name {
        DISCOVER_FOR_CONSUMER => {
            let mut request: ConsumerDiscoveryRequest = serde_json::from_value(arguments)
                .with_context(|| format!("{DISCOVER_FOR_CONSUMER} 参数无效"))?;
            request.requester_app_id = if uses_default_mcp_identity {
                "pc-web".to_string()
            } else {
                app_id.to_string()
            };
            Ok(Some(serde_json::to_value(
                open_commerce_consumer::discover(store, user_id, request)?,
            )?))
        }
        PLAN_CONSUMER_CAPABILITY => {
            let input: PlanArguments = serde_json::from_value(arguments)
                .with_context(|| format!("{PLAN_CONSUMER_CAPABILITY} 参数无效"))?;
            Ok(Some(serde_json::to_value(
                open_commerce_consumer_execution_plan::plan(
                    store,
                    user_id,
                    app_id,
                    uses_default_mcp_identity,
                    &input.merchant_id,
                    &input.capability_key,
                    &input.input,
                )?,
            )?))
        }
        REQUEST_CONSUMER_AUTHORIZATION => {
            if uses_default_mcp_identity {
                bail!("申请授权前必须通过 x-elon-app-id 使用本人已注册的开发者 App 身份");
            }
            let input: RequestAuthorizationArguments = serde_json::from_value(arguments)
                .with_context(|| format!("{REQUEST_CONSUMER_AUTHORIZATION} 参数无效"))?;
            if input.confirmation_phrase != REQUEST_AUTHORIZATION_PHRASE {
                bail!("授权申请确认短语无效");
            }
            store.ensure_open_commerce_developer_app_owned_by_user(app_id, user_id)?;
            let capability_key = normalize_capability_key(&input.capability_key)?;
            let capability =
                store.open_commerce_capability_by_key(&input.merchant_id, &capability_key)?;
            let grants = store.list_active_open_commerce_grant_records_for_app_capability(
                &input.merchant_id,
                app_id,
                &capability_key,
            )?;
            if open_commerce_grant_readiness::select_best(
                &grants,
                capability.unit_price_micros,
                &capability.currency,
            )
            .is_some_and(|(_, readiness)| readiness.is_available())
            {
                bail!("当前 App 已拥有该能力的有效授权，无需重复申请");
            }
            let pending_before = store.pending_authorization_for_app_capability(
                &input.merchant_id,
                app_id,
                &capability_key,
            )?;
            let authorization = open_commerce_consumer::create_authorization_request(
                store,
                user_id,
                CreateAuthorizationRequest {
                    merchant_id: input.merchant_id,
                    requester_app_id: app_id.to_string(),
                    scopes: vec![capability_key],
                    purpose: input.purpose,
                },
            )?;
            if pending_before.as_deref() != Some(authorization.id.as_str()) {
                store.record_open_commerce_audit(
                    &authorization.merchant_project_id,
                    user_id,
                    Some(&authorization.requester_app_id),
                    "authorization.requested",
                    "authorization_request",
                    &authorization.id,
                    &json!({
                        "merchant_id": authorization.merchant_id,
                        "requester_app_id": authorization.requester_app_id,
                        "scopes": authorization.scopes,
                        "consumer_user_confirmed": true,
                        "source": "mcp"
                    }),
                )?;
            }
            Ok(Some(
                open_commerce_consumer_authorization_mcp::authorization_request_projection(
                    &authorization,
                ),
            ))
        }
        _ => Ok(None),
    }
}

fn ensure_app_identity_in_project(
    store: &Store,
    project_id: &str,
    user_id: &str,
    app_id: &str,
) -> Result<()> {
    let app = store.ensure_open_commerce_developer_app_owned_by_user(app_id, user_id)?;
    if app.project_id != project_id {
        bail!("当前 MCP App 不属于当前项目");
    }
    Ok(())
}

fn empty_object() -> Value {
    json!({})
}

#[cfg(test)]
#[path = "open_commerce_consumer_authorization_request_mcp_tests.rs"]
mod authorization_tests;
#[cfg(test)]
#[path = "open_commerce_consumer_discovery_mcp_tests.rs"]
mod discovery_tests;
#[cfg(test)]
#[path = "open_commerce_consumer_mcp_test_support.rs"]
mod test_support;

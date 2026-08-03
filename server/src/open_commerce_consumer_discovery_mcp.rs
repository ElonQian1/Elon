//! Consumer-facing MCP discovery backed by the same policy service as the PC sandbox.

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    open_commerce_consumer, open_commerce_consumer_execution_plan,
    open_commerce_consumer_model::ConsumerDiscoveryRequest, store::Store,
};

const DISCOVER_FOR_CONSUMER: &str = "open_commerce_discover_for_consumer";
const PLAN_CONSUMER_CAPABILITY: &str = "open_commerce_plan_consumer_capability";

#[derive(Debug, Deserialize)]
struct PlanArguments {
    merchant_id: String,
    capability_key: String,
    #[serde(default = "empty_object")]
    input: Value,
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
                                "type":"integer","minimum":0,"maximum":1000000000000000
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
    ]
}

pub(crate) fn call_if_handled(
    store: &Store,
    user_id: &str,
    app_id: &str,
    uses_default_mcp_identity: bool,
    name: &str,
    arguments: Value,
) -> Result<Option<Value>> {
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
        _ => Ok(None),
    }
}

fn empty_object() -> Value {
    json!({})
}

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    compute_federation_price_snapshot_model::PublishMyComputePriceSnapshotRequest,
    compute_federation_price_snapshot_service, store::Store,
};

const PUBLISH_TOOL: &str = "compute_publish_my_price_snapshot";
const GET_TOOL: &str = "compute_get_my_price_snapshot";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PublishArguments {
    provider_id: String,
    pool_id: String,
    offer_id: String,
    request: PublishMyComputePriceSnapshotRequest,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GetArguments {
    provider_id: String,
    pool_id: String,
    offer_id: String,
    snapshot_id: String,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        tool(
            PUBLISH_TOOL,
            "基于当前用户的 active Offer 发布服务端规范化 fallback_curve 价格快照。必须显式确认；可进入报价候选，但不会预留容量、冻结余额或自动成交。",
            publish_schema(),
            false,
        ),
        tool(
            GET_TOOL,
            "读取当前用户 Offer 下的一份不可变价格快照并重新审计历史绑定。",
            get_schema(),
            true,
        ),
    ]
}

pub(crate) fn call_if_handled(
    store: &Store,
    user_id: &str,
    name: &str,
    arguments: Value,
) -> Result<Option<Value>> {
    match name {
        PUBLISH_TOOL => {
            let input: PublishArguments = decode(arguments, name)?;
            Ok(Some(serde_json::to_value(
                compute_federation_price_snapshot_service::publish_for_user(
                    store,
                    user_id,
                    &input.provider_id,
                    &input.pool_id,
                    &input.offer_id,
                    input.request,
                )?,
            )?))
        }
        GET_TOOL => {
            let input: GetArguments = decode(arguments, name)?;
            Ok(Some(serde_json::to_value(
                compute_federation_price_snapshot_service::get_for_user(
                    store,
                    user_id,
                    &input.provider_id,
                    &input.pool_id,
                    &input.offer_id,
                    &input.snapshot_id,
                )?,
            )?))
        }
        _ => Ok(None),
    }
}

fn publish_schema() -> Value {
    json!({
        "type":"object",
        "required":["provider_id","pool_id","offer_id","request"],
        "properties":{
            "provider_id":bounded_string(160),
            "pool_id":bounded_string(160),
            "offer_id":bounded_string(200),
            "request":{
                "type":"object",
                "required":[
                    "expected_offer_version","expected_offer_digest","delivery_window_id",
                    "consumer_max_amount_micros","provider_max_amount_micros","ttl_seconds",
                    "rounding_mode","idempotency_key","confirm_publish"
                ],
                "properties":{
                    "expected_offer_version":{"type":"integer","minimum":1},
                    "expected_offer_digest":bounded_string(256),
                    "delivery_window_id":bounded_string(160),
                    "consumer_max_amount_micros":{"type":"integer","minimum":0},
                    "provider_max_amount_micros":{"type":"integer","minimum":0},
                    "ttl_seconds":{"type":"integer","minimum":30,"maximum":3600},
                    "rounding_mode":{"type":"string","enum":["half_up","half_even","floor","ceil"]},
                    "idempotency_key":bounded_string(160),
                    "confirm_publish":{"type":"boolean","const":true}
                },
                "additionalProperties":false
            }
        },
        "additionalProperties":false
    })
}

fn get_schema() -> Value {
    json!({
        "type":"object",
        "required":["provider_id","pool_id","offer_id","snapshot_id"],
        "properties":{
            "provider_id":bounded_string(160),
            "pool_id":bounded_string(160),
            "offer_id":bounded_string(200),
            "snapshot_id":bounded_string(200)
        },
        "additionalProperties":false
    })
}

fn bounded_string(max_length: usize) -> Value {
    json!({"type":"string","minLength":1,"maxLength":max_length})
}

fn decode<T: for<'de> Deserialize<'de>>(arguments: Value, name: &str) -> Result<T> {
    serde_json::from_value(arguments).with_context(|| format!("{name} 参数无效"))
}

fn tool(name: &str, description: &str, input_schema: Value, read_only: bool) -> Value {
    json!({
        "name":name,
        "description":description,
        "inputSchema":input_schema,
        "annotations":{
            "readOnlyHint":read_only,
            "destructiveHint":false,
            "idempotentHint":true,
            "openWorldHint":false
        }
    })
}

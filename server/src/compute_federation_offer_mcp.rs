use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    compute_federation_offer_draft_model::CreateMyComputeOfferDraftRequest,
    compute_federation_offer_service, store::Store,
};

const CREATE_DRAFT_TOOL: &str = "compute_create_my_offer_draft";
const GET_OFFER_TOOL: &str = "compute_get_my_offer";
const LIST_OFFERS_TOOL: &str = "compute_list_my_offers";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateArguments {
    provider_id: String,
    pool_id: String,
    request: CreateMyComputeOfferDraftRequest,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OfferArguments {
    provider_id: String,
    pool_id: String,
    offer_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListArguments {
    provider_id: String,
    pool_id: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        tool(
            CREATE_DRAFT_TOOL,
            "为当前用户拥有且已激活的 Provider/CapacityPool 创建服务端规范化 draft Offer。必须显式确认；不会发布 active Offer、生成 Price Snapshot、预留容量或移动资金。",
            create_schema(),
            false,
        ),
        tool(
            GET_OFFER_TOOL,
            "读取当前用户 Provider/CapacityPool 下的一份 Offer，并审计当前投影和不可变版本。不会修改市场状态。",
            offer_schema(),
            true,
        ),
        tool(
            LIST_OFFERS_TOOL,
            "列出当前用户 Provider/CapacityPool 下的 Offer。不会修改市场状态。",
            list_schema(),
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
        CREATE_DRAFT_TOOL => {
            let input: CreateArguments = decode(arguments, name)?;
            Ok(Some(serde_json::to_value(
                compute_federation_offer_service::create_draft_for_user(
                    store,
                    user_id,
                    &input.provider_id,
                    &input.pool_id,
                    input.request,
                )?,
            )?))
        }
        GET_OFFER_TOOL => {
            let input: OfferArguments = decode(arguments, name)?;
            Ok(Some(serde_json::to_value(
                compute_federation_offer_service::get_for_user(
                    store,
                    user_id,
                    &input.provider_id,
                    &input.pool_id,
                    &input.offer_id,
                )?,
            )?))
        }
        LIST_OFFERS_TOOL => {
            let input: ListArguments = decode(arguments, name)?;
            Ok(Some(json!({
                "offers":compute_federation_offer_service::list_for_user(
                    store,
                    user_id,
                    &input.provider_id,
                    &input.pool_id,
                    input.limit,
                )?
            })))
        }
        _ => Ok(None),
    }
}

fn create_schema() -> Value {
    json!({
        "type":"object",
        "required":["provider_id","pool_id","request"],
        "properties":{
            "provider_id":bounded_string(160),
            "pool_id":bounded_string(160),
            "request":{
                "type":"object",
                "required":[
                    "idempotency_key","sku","runtime","resource_profile","capacity",
                    "execution_limits","authorization","price_terms","valid_from",
                    "valid_until","confirm_create"
                ],
                "properties":{
                    "idempotency_key":bounded_string(160),
                    "sku":sku_schema(),
                    "model":{"anyOf":[model_schema(),{"type":"null"}]},
                    "runtime":runtime_schema(),
                    "resource_profile":resource_schema(),
                    "capacity":{"type":"array","minItems":1,"maxItems":256,"items":capacity_schema()},
                    "execution_limits":execution_limits_schema(),
                    "authorization":authorization_schema(),
                    "price_terms":price_terms_schema(),
                    "valid_from":{"type":"string","format":"date-time"},
                    "valid_until":{"type":"string","format":"date-time"},
                    "confirm_create":{"type":"boolean","const":true}
                },
                "additionalProperties":false
            }
        },
        "additionalProperties":false
    })
}

fn sku_schema() -> Value {
    json!({
        "type":"object",
        "required":["sku_id","task_kind","context_or_shape_bucket","verification_tier","sla_tier","delivery_window_class"],
        "properties":{
            "sku_id":bounded_string(160),
            "task_kind":bounded_string(80),
            "context_or_shape_bucket":bounded_string(160),
            "verification_tier":bounded_string(80),
            "sla_tier":bounded_string(80),
            "delivery_window_class":bounded_string(80)
        },
        "additionalProperties":false
    })
}

fn model_schema() -> Value {
    json!({
        "type":"object",
        "required":["model_id","model_family","model_digest","adapter_digests"],
        "properties":{
            "model_id":bounded_string(200),
            "model_family":bounded_string(160),
            "model_digest":bounded_string(256),
            "tokenizer_digest":{"type":["string","null"],"maxLength":256},
            "adapter_digests":{"type":"array","maxItems":64,"items":bounded_string(256),"uniqueItems":true}
        },
        "additionalProperties":false
    })
}

fn runtime_schema() -> Value {
    json!({
        "type":"object",
        "required":["runtime_family","runtime_version","precision","runner_digest"],
        "properties":{
            "runtime_family":bounded_string(160),
            "runtime_version":bounded_string(160),
            "precision":bounded_string(80),
            "runner_digest":bounded_string(256),
            "plugin_id":{"type":["string","null"],"maxLength":160},
            "plugin_version":{"type":["string","null"],"maxLength":160},
            "plugin_digest":{"type":["string","null"],"maxLength":256}
        },
        "additionalProperties":false
    })
}

fn resource_schema() -> Value {
    json!({
        "type":"object",
        "required":["accelerator_kind","accelerator_count","vram_bytes","ram_bytes"],
        "properties":{
            "accelerator_kind":bounded_string(80),
            "accelerator_count":{"type":"integer","minimum":1},
            "vram_bytes":{"type":"integer","minimum":1},
            "ram_bytes":{"type":"integer","minimum":1}
        },
        "additionalProperties":false
    })
}

fn capacity_schema() -> Value {
    json!({
        "type":"object",
        "required":["bucket_id","total_units","reservable_units"],
        "properties":{
            "bucket_id":bounded_string(160),
            "total_units":{"type":"integer","minimum":1},
            "reservable_units":{"type":"integer","minimum":0}
        },
        "additionalProperties":false
    })
}

fn execution_limits_schema() -> Value {
    json!({
        "type":"object",
        "required":["max_concurrent_attempts","max_attempt_runtime_seconds"],
        "properties":{
            "max_concurrent_attempts":{"type":"integer","minimum":1},
            "max_attempt_runtime_seconds":{"type":"integer","minimum":1}
        },
        "additionalProperties":false
    })
}

fn authorization_schema() -> Value {
    json!({
        "type":"object",
        "required":["public","allowed_account_ids","allowed_project_ids","allowed_data_classes"],
        "properties":{
            "public":{"type":"boolean"},
            "allowed_account_ids":{"type":"array","maxItems":256,"items":bounded_string(160),"uniqueItems":true},
            "allowed_project_ids":{"type":"array","maxItems":256,"items":bounded_string(160),"uniqueItems":true},
            "allowed_data_classes":{"type":"array","maxItems":16,"items":{"type":"string","enum":["public","low_sensitivity","restricted"]},"uniqueItems":true}
        },
        "additionalProperties":false
    })
}

fn price_terms_schema() -> Value {
    json!({
        "type":"object",
        "required":["pricing_mode","currency","components","fee_rules"],
        "properties":{
            "pricing_mode":{"type":"string","enum":["spot","index_locked","capacity_forward","capacity_future"]},
            "currency":{"type":"string","pattern":"^[A-Z]{3}$"},
            "curve_id":{"type":["string","null"],"maxLength":160},
            "curve_version":{"type":["integer","null"],"minimum":1},
            "instrument_id":{"type":["string","null"],"maxLength":160},
            "components":{"type":"array","minItems":1,"maxItems":64,"items":price_component_schema()},
            "fee_rules":{"type":"array","maxItems":64,"items":fee_rule_schema()}
        },
        "additionalProperties":false
    })
}

fn price_component_schema() -> Value {
    json!({
        "type":"object",
        "required":["meter","unit_size","consumer_unit_price_micros","provider_unit_price_micros","max_units"],
        "properties":{
            "meter":bounded_string(80),
            "unit_size":{"type":"integer","minimum":1},
            "consumer_unit_price_micros":{"type":"integer","minimum":0},
            "provider_unit_price_micros":{"type":"integer","minimum":0},
            "max_units":{"type":"integer","minimum":1}
        },
        "additionalProperties":false
    })
}

fn fee_rule_schema() -> Value {
    json!({
        "type":"object",
        "required":["fee_kind","charged_to","fixed_amount_micros","rate_basis_points"],
        "properties":{
            "fee_kind":bounded_string(80),
            "charged_to":bounded_string(80),
            "fixed_amount_micros":{"type":"integer","minimum":0},
            "rate_basis_points":{"type":"integer","minimum":0,"maximum":10000},
            "maximum_amount_micros":{"type":["integer","null"],"minimum":0}
        },
        "additionalProperties":false
    })
}

fn offer_schema() -> Value {
    json!({
        "type":"object",
        "required":["provider_id","pool_id","offer_id"],
        "properties":{
            "provider_id":bounded_string(160),
            "pool_id":bounded_string(160),
            "offer_id":bounded_string(200)
        },
        "additionalProperties":false
    })
}

fn list_schema() -> Value {
    json!({
        "type":"object",
        "required":["provider_id","pool_id"],
        "properties":{
            "provider_id":bounded_string(160),
            "pool_id":bounded_string(160),
            "limit":{"type":"integer","minimum":1,"maximum":100,"default":20}
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

fn default_limit() -> usize {
    20
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

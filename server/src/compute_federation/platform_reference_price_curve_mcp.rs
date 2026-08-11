//! Administrator MCP surface for governed platform fallback price curves.

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::store::Store;

use super::{
    management_mcp_support as support,
    platform_reference_price_curve_service::{
        self as service, ApplyPlatformReferencePriceCurveBody,
        ReviewPlatformReferencePriceCurveBody, SubmitPlatformReferencePriceCurveBody,
    },
};

const LIST: &str = "compute_admin_list_platform_reference_price_curves";
const GET: &str = "compute_admin_get_platform_reference_price_curve";
const PREFLIGHT: &str = "compute_admin_preflight_platform_reference_price_curve";
const SUBMIT: &str = "compute_admin_submit_platform_reference_price_curve";
const REVIEW: &str = "compute_admin_review_platform_reference_price_curve";
const APPLY: &str = "compute_admin_apply_platform_reference_price_curve";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EntityArguments {
    batch_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListArguments {
    status: Option<String>,
    #[serde(default = "support::default_limit")]
    limit: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewArguments {
    batch_id: String,
    request: ReviewPlatformReferencePriceCurveBody,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ApplyArguments {
    batch_id: String,
    request: ApplyPlatformReferencePriceCurveBody,
}

pub(crate) fn admin_definitions() -> Vec<Value> {
    vec![
        support::tool(LIST, "平台管理员列出受治理的平台参考回退价格曲线批次。", support::list_schema(), true, false),
        support::tool(GET, "平台管理员读取一个参考回退价格曲线批次及治理回执。", entity_schema(), true, false),
        support::tool(PREFLIGHT, "检查当前管理员对参考价格曲线批次可执行的下一步操作，不改变市场或批次状态。", entity_schema(), true, false),
        support::tool(SUBMIT, "提交与不可变 Offer 精确绑定的平台参考回退曲线。该来源没有市场样本、成交、容量或资金权限；必须显式确认。", submit_schema(), false, false),
        support::tool(REVIEW, "由不同平台管理员独立复核参考价格曲线批次；必须显式确认。", review_schema(), false, false),
        support::tool(APPLY, "应用已批准的参考回退价格曲线，使其可被后续价格快照显式绑定；不会创建订单、预留容量或冻结资金。必须显式确认。", apply_schema(), false, false),
    ]
}

pub(crate) fn call_admin_if_handled(
    store: &Store,
    user_id: &str,
    platform_role: &str,
    name: &str,
    arguments: Value,
) -> Result<Option<Value>> {
    let value = match name {
        LIST => {
            support::ensure_platform_admin(platform_role)?;
            let input: ListArguments = support::decode(arguments, name)?;
            json!({"reference_curve_batches":service::list_for_admin(
                store, input.status.as_deref(), input.limit
            )?})
        }
        GET => {
            support::ensure_platform_admin(platform_role)?;
            let input: EntityArguments = support::decode(arguments, name)?;
            serde_json::to_value(service::get_for_admin(store, &input.batch_id)?)?
        }
        PREFLIGHT => {
            support::ensure_platform_admin(platform_role)?;
            let input: EntityArguments = support::decode(arguments, name)?;
            serde_json::to_value(service::preflight_for_admin(
                store,
                user_id,
                &input.batch_id,
            )?)?
        }
        SUBMIT => {
            support::ensure_platform_admin(platform_role)?;
            serde_json::to_value(service::submit_for_admin(
                store,
                user_id,
                support::decode::<SubmitPlatformReferencePriceCurveBody>(arguments, name)?,
            )?)?
        }
        REVIEW => {
            support::ensure_platform_admin(platform_role)?;
            let input: ReviewArguments = support::decode(arguments, name)?;
            serde_json::to_value(service::review_for_admin(
                store,
                user_id,
                &input.batch_id,
                input.request,
            )?)?
        }
        APPLY => {
            support::ensure_platform_admin(platform_role)?;
            let input: ApplyArguments = support::decode(arguments, name)?;
            serde_json::to_value(service::apply_for_admin(
                store,
                user_id,
                &input.batch_id,
                input.request,
            )?)?
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn entity_schema() -> Value {
    support::entity_schema("batch_id", 200)
}

fn submit_schema() -> Value {
    json!({
        "type":"object",
        "required":[
            "idempotency_key","curve_id","curve_version","valid_from","valid_until",
            "quote_ttl_seconds","entries","confirm_submission"
        ],
        "properties":{
            "idempotency_key":support::bounded_string(200),
            "curve_id":support::bounded_string(160),
            "curve_version":{"type":"integer","minimum":1},
            "valid_from":support::bounded_string(64),
            "valid_until":support::bounded_string(64),
            "quote_ttl_seconds":{"type":"integer","minimum":30,"maximum":3600},
            "entries":{"type":"array","minItems":1,"maxItems":1000,"items":entry_schema()},
            "submission_note":{"type":"string","maxLength":2000,"default":""},
            "confirm_submission":{"type":"boolean","const":true}
        },
        "additionalProperties":false
    })
}

fn entry_schema() -> Value {
    json!({
        "type":"object",
        "required":[
            "entry_key","provider_id","offer_id","offer_version","offer_digest","sku_id",
            "sku_digest","delivery_window_id","delivery_window_digest","pricing_mode","currency",
            "offer_curve_id","offer_curve_version","instrument_id","components","fee_rules",
            "consumer_max_amount_micros","provider_max_amount_micros"
        ],
        "properties":{
            "entry_key":support::bounded_string(300),
            "provider_id":support::bounded_string(160),
            "offer_id":support::bounded_string(200),
            "offer_version":{"type":"integer","minimum":1},
            "offer_digest":support::bounded_string(256),
            "sku_id":support::bounded_string(200),
            "sku_digest":support::bounded_string(256),
            "delivery_window_id":support::bounded_string(200),
            "delivery_window_digest":support::bounded_string(256),
            "pricing_mode":support::bounded_string(100),
            "currency":{"type":"string","const":"CNY"},
            "offer_curve_id":nullable_string(160),
            "offer_curve_version":{"type":["integer","null"],"minimum":1},
            "instrument_id":nullable_string(200),
            "components":{"type":"array","minItems":1,"maxItems":100,"items":component_schema()},
            "fee_rules":{"type":"array","maxItems":0,"items":fee_rule_schema()},
            "consumer_max_amount_micros":{"type":"integer","minimum":0},
            "provider_max_amount_micros":{"type":"integer","minimum":0}
        },
        "additionalProperties":false
    })
}

fn component_schema() -> Value {
    json!({
        "type":"object",
        "required":["meter","unit_size","consumer_unit_price_micros","provider_unit_price_micros","max_units"],
        "properties":{
            "meter":support::bounded_string(100),
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
        "required":["fee_kind","charged_to","fixed_amount_micros","rate_basis_points","maximum_amount_micros"],
        "properties":{
            "fee_kind":support::bounded_string(100),
            "charged_to":support::bounded_string(100),
            "fixed_amount_micros":{"type":"integer","minimum":0},
            "rate_basis_points":{"type":"integer","minimum":0},
            "maximum_amount_micros":{"type":["integer","null"],"minimum":0}
        },
        "additionalProperties":false
    })
}

fn review_schema() -> Value {
    wrapped_request_schema(json!({
        "type":"object",
        "required":[
            "idempotency_key","expected_batch_digest","expected_batch_material_digest",
            "decision","confirm_review"
        ],
        "properties":{
            "idempotency_key":support::bounded_string(200),
            "expected_batch_digest":support::bounded_string(256),
            "expected_batch_material_digest":support::bounded_string(256),
            "decision":{"type":"string","enum":["approved","changes_requested","rejected"]},
            "review_note":{"type":["string","null"],"maxLength":2000},
            "confirm_review":{"type":"boolean","const":true}
        },
        "additionalProperties":false
    }))
}

fn apply_schema() -> Value {
    wrapped_request_schema(json!({
        "type":"object",
        "required":[
            "idempotency_key","expected_batch_digest","expected_batch_material_digest",
            "expected_review_id","expected_review_digest","confirm_application"
        ],
        "properties":{
            "idempotency_key":support::bounded_string(200),
            "expected_batch_digest":support::bounded_string(256),
            "expected_batch_material_digest":support::bounded_string(256),
            "expected_review_id":support::bounded_string(200),
            "expected_review_digest":support::bounded_string(256),
            "apply_note":{"type":"string","maxLength":2000,"default":""},
            "confirm_application":{"type":"boolean","const":true}
        },
        "additionalProperties":false
    }))
}

fn wrapped_request_schema(request: Value) -> Value {
    json!({
        "type":"object",
        "required":["batch_id","request"],
        "properties":{"batch_id":support::bounded_string(200),"request":request},
        "additionalProperties":false
    })
}

fn nullable_string(max_length: usize) -> Value {
    json!({"type":["string","null"],"maxLength":max_length})
}

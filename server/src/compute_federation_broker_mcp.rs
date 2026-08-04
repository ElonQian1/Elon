use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    compute_federation_broker_service::{self, FinishMyComputeRequest, ReserveMyComputeRequest},
    store::{ComputeBrokerFinishAction, Store},
};

const RESERVE_TOOL: &str = "compute_reserve_my_job";
const RELEASE_TOOL: &str = "compute_release_my_reservation";
const EXPIRE_TOOL: &str = "compute_expire_my_reservation";

#[derive(Debug, Deserialize)]
struct ReserveArguments {
    #[serde(flatten)]
    request: ReserveMyComputeRequest,
    confirm_financial_action: bool,
}

#[derive(Debug, Deserialize)]
struct FinishArguments {
    reservation_id: String,
    #[serde(flatten)]
    request: FinishMyComputeRequest,
    #[serde(default)]
    confirm_cancellation: bool,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        tool(
            RESERVE_TOOL,
            "使用当前登录用户的平台人民币余额，为当前项目中已报价的算力 Job 原子冻结预算和容量。会产生真实账户副作用；仅在用户明确确认本次冻结后调用。",
            json!({
                "type":"object",
                "required":["reservation_id","idempotency_key","job_id","expected_job_revision","expected_job_digest","reserved_capacity","expires_at","confirm_financial_action"],
                "properties":{
                    "reservation_id":{"type":"string","minLength":1,"maxLength":160},
                    "idempotency_key":{"type":"string","minLength":1,"maxLength":200},
                    "job_id":{"type":"string","minLength":1,"maxLength":160},
                    "expected_job_revision":{"type":"integer","minimum":1},
                    "expected_job_digest":{"type":"string","minLength":1,"maxLength":200},
                    "reserved_capacity":{"type":"array","minItems":1,"items":{"type":"object","required":["meter","quantity"],"properties":{"meter":{"type":"string","minLength":1,"maxLength":80},"quantity":{"type":"integer","minimum":1}},"additionalProperties":false}},
                    "expires_at":{"type":"string","format":"date-time"},
                    "confirm_financial_action":{"type":"boolean","const":true}
                },
                "additionalProperties":false
            }),
        ),
        tool(
            RELEASE_TOOL,
            "取消当前项目中尚未启动 Attempt 的本人算力 Reservation，并在同一事务退款、归还 held 容量和终结 Job。仅在用户明确确认取消后调用。",
            finish_schema(true),
        ),
        tool(
            EXPIRE_TOOL,
            "在到期时间已经到达后，幂等终结当前项目中尚未启动 Attempt 的本人算力 Reservation，并退款和归还 held 容量。不能提前到期。",
            finish_schema(false),
        ),
    ]
}

pub(crate) fn call_if_handled(
    store: &Store,
    project_id: &str,
    user_id: &str,
    name: &str,
    arguments: Value,
) -> Result<Option<Value>> {
    match name {
        RESERVE_TOOL => {
            let input: ReserveArguments = decode(arguments, name)?;
            if !input.confirm_financial_action {
                bail!("必须由用户明确确认本次算力余额冻结");
            }
            Ok(Some(serde_json::to_value(
                compute_federation_broker_service::reserve_for_user(
                    store,
                    user_id,
                    Some(project_id),
                    input.request,
                )?,
            )?))
        }
        RELEASE_TOOL | EXPIRE_TOOL => {
            let input: FinishArguments = decode(arguments, name)?;
            if name == RELEASE_TOOL && !input.confirm_cancellation {
                bail!("必须由用户明确确认取消算力 Reservation");
            }
            let action = if name == RELEASE_TOOL {
                ComputeBrokerFinishAction::Release
            } else {
                ComputeBrokerFinishAction::Expire
            };
            Ok(Some(serde_json::to_value(
                compute_federation_broker_service::finish_for_user(
                    store,
                    user_id,
                    Some(project_id),
                    input.reservation_id,
                    action,
                    input.request,
                )?,
            )?))
        }
        _ => Ok(None),
    }
}

fn finish_schema(require_confirmation: bool) -> Value {
    let confirmation = if require_confirmation {
        json!({"type":"boolean","const":true})
    } else {
        json!({"type":"boolean","default":false})
    };
    let required = if require_confirmation {
        json!([
            "reservation_id",
            "idempotency_key",
            "expected_reservation_revision",
            "expected_reservation_digest",
            "confirm_cancellation"
        ])
    } else {
        json!([
            "reservation_id",
            "idempotency_key",
            "expected_reservation_revision",
            "expected_reservation_digest"
        ])
    };
    json!({
        "type":"object",
        "required":required,
        "properties":{
            "reservation_id":{"type":"string","minLength":1,"maxLength":160},
            "idempotency_key":{"type":"string","minLength":1,"maxLength":200},
            "expected_reservation_revision":{"type":"integer","minimum":1},
            "expected_reservation_digest":{"type":"string","minLength":1,"maxLength":200},
            "confirm_cancellation":confirmation
        },
        "additionalProperties":false
    })
}

fn decode<T: for<'de> Deserialize<'de>>(arguments: Value, name: &str) -> Result<T> {
    serde_json::from_value(arguments).with_context(|| format!("{name} 参数无效"))
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name":name,
        "description":description,
        "inputSchema":input_schema,
        "annotations":{
            "readOnlyHint":false,
            "destructiveHint":true,
            "idempotentHint":true,
            "openWorldHint":false
        }
    })
}

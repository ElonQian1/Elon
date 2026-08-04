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
const GET_JOB_TOOL: &str = "compute_get_my_job";
const LIST_JOBS_TOOL: &str = "compute_list_my_jobs";
const GET_RESERVATION_TOOL: &str = "compute_get_my_reservation";
const LIST_RESERVATIONS_TOOL: &str = "compute_list_my_reservations";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct JobArguments {
    job_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReservationArguments {
    reservation_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListArguments {
    #[serde(default = "default_limit")]
    limit: usize,
}

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
            GET_JOB_TOOL,
            "读取当前登录用户在当前项目中的一份算力 Job，并返回最新 revision、digest、状态、报价和预算合同。不会修改数据。",
            id_schema("job_id"),
            true,
        ),
        tool(
            LIST_JOBS_TOOL,
            "列出当前登录用户在当前项目中的最新算力 Job。用于在预留前取得当前 revision 与 digest；不会修改数据。",
            list_schema(),
            true,
        ),
        tool(
            GET_RESERVATION_TOOL,
            "读取当前登录用户在当前项目中的一份算力 Reservation，并返回最新 revision、digest、状态和容量绑定。不会修改数据。",
            id_schema("reservation_id"),
            true,
        ),
        tool(
            LIST_RESERVATIONS_TOOL,
            "列出当前登录用户在当前项目中的最新算力 Reservation。用于在释放或到期前取得当前 revision 与 digest；不会修改数据。",
            list_schema(),
            true,
        ),
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
            false,
        ),
        tool(
            RELEASE_TOOL,
            "取消当前项目中尚未启动 Attempt 的本人算力 Reservation，并在同一事务退款、归还 held 容量和终结 Job。仅在用户明确确认取消后调用。",
            finish_schema(true),
            false,
        ),
        tool(
            EXPIRE_TOOL,
            "在到期时间已经到达后，幂等终结当前项目中尚未启动 Attempt 的本人算力 Reservation，并退款和归还 held 容量。不能提前到期。",
            finish_schema(false),
            false,
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
        GET_JOB_TOOL => {
            let input: JobArguments = decode(arguments, name)?;
            Ok(Some(serde_json::to_value(
                compute_federation_broker_service::get_job_for_user(
                    store,
                    user_id,
                    Some(project_id),
                    &input.job_id,
                )?,
            )?))
        }
        LIST_JOBS_TOOL => {
            let input: ListArguments = decode(arguments, name)?;
            Ok(Some(json!({
                "jobs":compute_federation_broker_service::list_jobs_for_user(
                    store,
                    user_id,
                    Some(project_id),
                    input.limit,
                )?
            })))
        }
        GET_RESERVATION_TOOL => {
            let input: ReservationArguments = decode(arguments, name)?;
            Ok(Some(serde_json::to_value(
                compute_federation_broker_service::get_reservation_for_user(
                    store,
                    user_id,
                    Some(project_id),
                    &input.reservation_id,
                )?,
            )?))
        }
        LIST_RESERVATIONS_TOOL => {
            let input: ListArguments = decode(arguments, name)?;
            Ok(Some(json!({
                "reservations":compute_federation_broker_service::list_reservations_for_user(
                    store,
                    user_id,
                    Some(project_id),
                    input.limit,
                )?
            })))
        }
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

fn id_schema(field: &str) -> Value {
    let mut properties = serde_json::Map::new();
    properties.insert(
        field.to_string(),
        json!({"type":"string","minLength":1,"maxLength":160}),
    );
    json!({
        "type":"object",
        "required":[field],
        "properties":properties,
        "additionalProperties":false
    })
}

fn list_schema() -> Value {
    json!({
        "type":"object",
        "properties":{"limit":{"type":"integer","minimum":1,"maximum":100,"default":20}},
        "additionalProperties":false
    })
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
            "destructiveHint":!read_only,
            "idempotentHint":true,
            "openWorldHint":false
        }
    })
}

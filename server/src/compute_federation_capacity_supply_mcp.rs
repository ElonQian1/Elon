use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    compute_federation_capacity_supply_service::{
        self, AddMyComputeCapacitySupplyLineRequest, AddMyComputeCapacitySupplyRequest,
        WithdrawMyComputeCapacitySupplyLineRequest, WithdrawMyComputeCapacitySupplyRequest,
    },
    store::Store,
};

const ADD_SUPPLY_TOOL: &str = "compute_add_my_capacity_supply";
const WITHDRAW_SUPPLY_TOOL: &str = "compute_withdraw_my_capacity_supply";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AddSupplyArguments {
    provider_id: String,
    pool_id: String,
    idempotency_key: String,
    lines: Vec<AddMyComputeCapacitySupplyLineRequest>,
    confirm_supply: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WithdrawSupplyArguments {
    provider_id: String,
    pool_id: String,
    idempotency_key: String,
    lines: Vec<WithdrawMyComputeCapacitySupplyLineRequest>,
    confirm_withdrawal: bool,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        tool(
            ADD_SUPPLY_TOOL,
            "向本人当前 CapacityPool 的同一交付窗口原子追加 self-declared 供给。必须显式确认；该操作不激活 Provider/Pool，也不代表容量已验证或可交易。",
            add_supply_schema(),
        ),
        tool(
            WITHDRAW_SUPPLY_TOOL,
            "从本人当前 CapacityPool 的同一交付窗口原子撤回尚在 available 的 self-declared 供给。必须显式确认，不能撤回 held 或 active 容量。",
            withdraw_supply_schema(),
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
        ADD_SUPPLY_TOOL => {
            let input: AddSupplyArguments =
                serde_json::from_value(arguments).with_context(|| format!("{name} 参数无效"))?;
            Ok(Some(serde_json::to_value(
                compute_federation_capacity_supply_service::add_for_user(
                    store,
                    user_id,
                    &input.provider_id,
                    &input.pool_id,
                    AddMyComputeCapacitySupplyRequest {
                        idempotency_key: input.idempotency_key,
                        lines: input.lines,
                        confirm_supply: input.confirm_supply,
                    },
                )?,
            )?))
        }
        WITHDRAW_SUPPLY_TOOL => {
            let input: WithdrawSupplyArguments =
                serde_json::from_value(arguments).with_context(|| format!("{name} 参数无效"))?;
            Ok(Some(serde_json::to_value(
                compute_federation_capacity_supply_service::withdraw_for_user(
                    store,
                    user_id,
                    &input.provider_id,
                    &input.pool_id,
                    WithdrawMyComputeCapacitySupplyRequest {
                        idempotency_key: input.idempotency_key,
                        lines: input.lines,
                        confirm_withdrawal: input.confirm_withdrawal,
                    },
                )?,
            )?))
        }
        _ => Ok(None),
    }
}

fn add_supply_schema() -> Value {
    json!({
        "type":"object",
        "required":[
            "provider_id","pool_id","idempotency_key","lines","confirm_supply"
        ],
        "properties":{
            "provider_id":{"type":"string","minLength":1,"maxLength":160},
            "pool_id":{"type":"string","minLength":1,"maxLength":160},
            "idempotency_key":{"type":"string","minLength":1,"maxLength":160},
            "lines":{
                "type":"array",
                "minItems":1,
                "maxItems":64,
                "items":{
                    "type":"object",
                    "required":["bucket_id","quantity_units"],
                    "properties":{
                        "bucket_id":{"type":"string","minLength":1,"maxLength":160},
                        "quantity_units":{"type":"integer","minimum":1}
                    },
                    "additionalProperties":false
                }
            },
            "confirm_supply":{"type":"boolean","const":true}
        },
        "additionalProperties":false
    })
}

fn withdraw_supply_schema() -> Value {
    json!({
        "type":"object",
        "required":[
            "provider_id","pool_id","idempotency_key","lines","confirm_withdrawal"
        ],
        "properties":{
            "provider_id":{"type":"string","minLength":1,"maxLength":160},
            "pool_id":{"type":"string","minLength":1,"maxLength":160},
            "idempotency_key":{"type":"string","minLength":1,"maxLength":160},
            "lines":{
                "type":"array",
                "minItems":1,
                "maxItems":64,
                "items":{
                    "type":"object",
                    "required":["bucket_id","quantity_units"],
                    "properties":{
                        "bucket_id":{"type":"string","minLength":1,"maxLength":160},
                        "quantity_units":{"type":"integer","minimum":1}
                    },
                    "additionalProperties":false
                }
            },
            "confirm_withdrawal":{"type":"boolean","const":true}
        },
        "additionalProperties":false
    })
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

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::store::Store;

use super::service;

const GET_ORDER_CLOSURE: &str = "open_commerce_get_my_order_closure";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Arguments {
    invocation_id: String,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![json!({
        "name":GET_ORDER_CLOSURE,
        "description":"读取当前登录消费者本人一笔订单的商户回执、最新 ERP 衔接结果与零资金计量边界。不会返回项目、授权、接入器凭据、Claim、租约密钥或原始 ERP 记录号。",
        "inputSchema":{
            "type":"object",
            "required":["invocation_id"],
            "properties":{"invocation_id":{"type":"string","minLength":1,"maxLength":120}},
            "additionalProperties":false
        },
        "annotations":{
            "readOnlyHint":true,
            "destructiveHint":false,
            "idempotentHint":true,
            "openWorldHint":false
        }
    })]
}

pub(crate) fn call_if_handled(
    store: &Store,
    user_id: &str,
    name: &str,
    arguments: Value,
) -> Result<Option<Value>> {
    if name != GET_ORDER_CLOSURE {
        return Ok(None);
    }
    let input: Arguments = serde_json::from_value(arguments)
        .with_context(|| format!("{GET_ORDER_CLOSURE} 参数无效"))?;
    Ok(Some(serde_json::to_value(service::get_order_closure(
        store,
        user_id,
        &input.invocation_id,
    )?)?))
}

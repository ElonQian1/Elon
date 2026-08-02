use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{open_commerce_consumer_receipt_service, store::Store};

const LIST_RECEIPTS: &str = "open_commerce_list_my_invocation_receipts";
const GET_RECEIPT: &str = "open_commerce_get_my_invocation_receipt";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListArguments {
    #[serde(default = "default_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptArguments {
    invocation_id: String,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        tool(
            LIST_RECEIPTS,
            "读取当前登录用户在全部商户项目产生的终态调用凭证摘要。不会返回原始请求、内部用户 ID、授权 ID、请求哈希或其他用户记录。",
            json!({
                "type":"object",
                "properties":{"limit":{"type":"integer","minimum":1,"maximum":200,"default":50}},
                "additionalProperties":false
            }),
        ),
        tool(
            GET_RECEIPT,
            "读取当前登录用户拥有的一份终态调用凭证详情及 SHA-256 摘要。结果可能包含商户当时返回给该用户的数据；摘要只用于完整性复核，不代表支付或链上存证。",
            json!({
                "type":"object",
                "required":["invocation_id"],
                "properties":{"invocation_id":{"type":"string","minLength":1,"maxLength":120}},
                "additionalProperties":false
            }),
        ),
    ]
}

pub(crate) fn call_if_handled(
    store: &Store,
    user_id: &str,
    name: &str,
    arguments: Value,
) -> Result<Option<Value>> {
    let value = match name {
        LIST_RECEIPTS => {
            let input: ListArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_consumer_receipt_service::list_receipts(
                store,
                user_id,
                input.limit,
            )?)?
        }
        GET_RECEIPT => {
            let input: ReceiptArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_consumer_receipt_service::get_receipt(
                store,
                user_id,
                &input.invocation_id,
            )?)?
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn decode<T: serde::de::DeserializeOwned>(arguments: Value, name: &str) -> Result<T> {
    serde_json::from_value(arguments).with_context(|| format!("{name} 参数无效"))
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name":name,
        "description":description,
        "inputSchema":input_schema,
        "annotations":{
            "readOnlyHint":true,
            "destructiveHint":false,
            "idempotentHint":true,
            "openWorldHint":false
        }
    })
}

fn default_limit() -> usize {
    50
}

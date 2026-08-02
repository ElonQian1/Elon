use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{open_commerce_merchant_evidence_service, store::Store};

const LIST_EVIDENCE: &str = "open_commerce_list_merchant_business_evidence";
const GET_EVIDENCE: &str = "open_commerce_get_merchant_business_evidence";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListArguments {
    merchant_id: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DetailArguments {
    merchant_id: String,
    invocation_id: String,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        tool(
            LIST_EVIDENCE,
            "读取当前项目指定商户的终态能力调用证据、结果摘要和可选标准业务回执。ERP 关联不等于自动入库，回执也不证明支付或履约完成。",
            json!({
                "type":"object",
                "required":["merchant_id"],
                "properties":{
                    "merchant_id":{"type":"string","minLength":1,"maxLength":120},
                    "limit":{"type":"integer","minimum":1,"maximum":200,"default":50}
                },
                "additionalProperties":false
            }),
        ),
        tool(
            GET_EVIDENCE,
            "读取当前项目一条商户能力调用证据和商户当时返回的结果。仅用于后续 ERP/CRM 适配，不应把调用成功冒充为真实资金、履约或退款证明。",
            json!({
                "type":"object",
                "required":["merchant_id","invocation_id"],
                "properties":{
                    "merchant_id":{"type":"string","minLength":1,"maxLength":120},
                    "invocation_id":{"type":"string","minLength":1,"maxLength":120}
                },
                "additionalProperties":false
            }),
        ),
    ]
}

pub(crate) fn call_if_handled(
    store: &Store,
    project_id: &str,
    name: &str,
    arguments: Value,
) -> Result<Option<Value>> {
    let value = match name {
        LIST_EVIDENCE => {
            let input: ListArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_merchant_evidence_service::list_evidence(
                store,
                project_id,
                &input.merchant_id,
                input.limit,
            )?)?
        }
        GET_EVIDENCE => {
            let input: DetailArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_merchant_evidence_service::get_evidence(
                store,
                project_id,
                &input.merchant_id,
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

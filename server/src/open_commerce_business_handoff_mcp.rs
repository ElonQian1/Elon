use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    open_commerce_business_handoff_model::RecordBusinessHandoffReceiptRequest,
    open_commerce_business_handoff_service, open_commerce_service::OpenCommerceActor, store::Store,
};

const LIST_RECEIPTS: &str = "open_commerce_list_business_handoff_receipts";
const LIST_QUEUE: &str = "open_commerce_list_business_handoff_queue";
const RECORD_RECEIPT: &str = "open_commerce_record_business_handoff_receipt";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListArguments {
    merchant_id: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct QueueArguments {
    merchant_id: String,
    #[serde(default)]
    state: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        tool(
            LIST_RECEIPTS,
            "读取当前项目指定商户的 ERP/CRM 业务衔接回执。回执是项目编辑者声明，不是支付、履约或外部系统独立证明。",
            json!({
                "type":"object",
                "required":["merchant_id"],
                "properties":{
                    "merchant_id":{"type":"string","minLength":1,"maxLength":120},
                    "limit":{"type":"integer","minimum":1,"maximum":200,"default":50}
                },
                "additionalProperties":false
            }),
            true,
        ),
        tool(
            LIST_QUEUE,
            "读取当前商户尚未完成 ERP/CRM 衔接的业务证据。pending 尚无回执，retry_required 表示最新处理失败；成功或忽略后自动移出。该工具只读，不会调用外部系统或移动资金。",
            json!({
                "type":"object",
                "required":["merchant_id"],
                "properties":{
                    "merchant_id":{"type":"string","minLength":1,"maxLength":120},
                    "state":{"type":"string","enum":["pending","retry_required"]},
                    "limit":{"type":"integer","minimum":1,"maximum":200,"default":50}
                },
                "additionalProperties":false
            }),
            true,
        ),
        tool(
            RECORD_RECEIPT,
            "仅在用户已确认真实 ERP/CRM 处理结果后，幂等记录业务证据的衔接回执。applied 必须绑定有效标准业务回执和外部目标记录号；平台只保存目标记录号摘要，不移动资金。",
            json!({
                "type":"object",
                "required":[
                    "merchant_id","invocation_id","integration_id","receipt_key",
                    "status","target_domain","evidence_result_sha256",
                    "confirmed_by_user","completed_at"
                ],
                "properties":{
                    "merchant_id":{"type":"string","minLength":1,"maxLength":120},
                    "invocation_id":{"type":"string","minLength":1,"maxLength":120},
                    "integration_id":{"type":"string","minLength":1,"maxLength":120},
                    "receipt_key":{"type":"string","minLength":3,"maxLength":128},
                    "status":{"type":"string","enum":["applied","ignored","rejected"]},
                    "target_domain":{"type":"string","enum":["erp","crm"]},
                    "evidence_result_sha256":{
                        "type":"string","pattern":"^[A-Fa-f0-9]{64}$"
                    },
                    "target_reference":{"type":"string","minLength":1,"maxLength":160},
                    "error_code":{"type":"string","minLength":2,"maxLength":96},
                    "confirmed_by_user":{"const":true},
                    "completed_at":{"type":"string","format":"date-time"}
                },
                "additionalProperties":false
            }),
            false,
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn call_if_handled(
    store: &Store,
    project_id: &str,
    user_id: &str,
    project_role: &str,
    app_id: &str,
    name: &str,
    arguments: Value,
) -> Result<Option<Value>> {
    let value = match name {
        LIST_RECEIPTS => {
            let input: ListArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_business_handoff_service::list_receipts(
                store,
                project_id,
                &input.merchant_id,
                input.limit,
            )?)?
        }
        LIST_QUEUE => {
            let input: QueueArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_business_handoff_service::list_queue(
                store,
                project_id,
                &input.merchant_id,
                input.state.as_deref(),
                input.limit,
            )?)?
        }
        RECORD_RECEIPT => {
            let request: RecordBusinessHandoffReceiptRequest = decode(arguments, name)?;
            serde_json::to_value(open_commerce_business_handoff_service::record_receipt(
                store,
                project_id,
                &OpenCommerceActor {
                    user_id,
                    app_id,
                    project_role: Some(project_role),
                },
                request,
            )?)?
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn decode<T: serde::de::DeserializeOwned>(arguments: Value, name: &str) -> Result<T> {
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

fn default_limit() -> usize {
    50
}

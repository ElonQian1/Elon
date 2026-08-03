//! Consumer-owned authorization request status without cross-member identity leakage.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    open_commerce_client_lifecycle_service, open_commerce_service::OpenCommerceActor, store::Store,
};

const LIST_MY_AUTHORIZATION_REQUESTS: &str =
    "open_commerce_list_my_consumer_authorization_requests";
const CANCEL_MY_AUTHORIZATION_REQUEST: &str =
    "open_commerce_cancel_my_consumer_authorization_request";
const CANCEL_CONFIRMATION_PHRASE: &str = "CANCEL_AUTHORIZATION_REQUEST";

#[derive(Debug, Deserialize)]
struct ListArguments {
    #[serde(default)]
    status: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize)]
struct CancelArguments {
    request_id: String,
    confirmation_phrase: String,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        json!({
            "name":LIST_MY_AUTHORIZATION_REQUESTS,
            "description":"列出当前用户通过当前项目开发者 App 发出的授权申请，可按状态筛选。不会返回其他项目成员、商户项目或审核人内部身份。",
            "inputSchema":{
                "type":"object",
                "properties":{
                    "status":{"type":"string","enum":["pending","approved","rejected","canceled"]},
                    "limit":{"type":"integer","minimum":1,"maximum":100,"default":50}
                },
                "additionalProperties":false
            },
            "annotations":{
                "readOnlyHint":true,
                "destructiveHint":false,
                "idempotentHint":true,
                "openWorldHint":false
            }
        }),
        json!({
            "name":CANCEL_MY_AUTHORIZATION_REQUEST,
            "description":"仅在当前用户明确同意后，撤回当前项目中由本人 App 发出且仍在等待的授权申请。不会撤销已批准 Grant，也不会取消其他成员的申请。",
            "inputSchema":{
                "type":"object",
                "required":["request_id","confirmation_phrase"],
                "properties":{
                    "request_id":{"type":"string","minLength":1,"maxLength":120},
                    "confirmation_phrase":{"const":"CANCEL_AUTHORIZATION_REQUEST"}
                },
                "additionalProperties":false
            },
            "annotations":{
                "readOnlyHint":false,
                "destructiveHint":true,
                "idempotentHint":true,
                "openWorldHint":false
            }
        }),
    ]
}

pub(crate) fn call_if_handled(
    store: &Store,
    project_id: &str,
    user_id: &str,
    project_role: &str,
    app_id: &str,
    name: &str,
    arguments: Value,
) -> Result<Option<Value>> {
    match name {
        LIST_MY_AUTHORIZATION_REQUESTS => {
            let input: ListArguments = serde_json::from_value(arguments)
                .with_context(|| format!("{LIST_MY_AUTHORIZATION_REQUESTS} 参数无效"))?;
            let requests = store
                .list_user_project_open_commerce_authorization_requests(
                    project_id,
                    user_id,
                    input.status.as_deref(),
                    input.limit,
                )?
                .into_iter()
                .map(|request| {
                    json!({
                        "request_id":request.id,
                        "merchant_id":request.merchant_id,
                        "requester_app_id":request.requester_app_id,
                        "scopes":request.scopes,
                        "purpose":request.purpose,
                        "status":request.status,
                        "decision_reason":request.decision_reason,
                        "grant_id":request.grant_id,
                        "grant_expires_at":request.grant_expires_at,
                        "grant_max_invocations":request.grant_max_invocations,
                        "grant_max_amount_micros":request.grant_max_amount_micros,
                        "grant_budget_currency":request.grant_budget_currency,
                        "created_at":request.created_at,
                        "updated_at":request.updated_at
                    })
                })
                .collect::<Vec<_>>();
            Ok(Some(json!({
                "schema":"open_commerce.consumer_authorization_request_list.v1",
                "project_id":project_id,
                "status_filter":input.status,
                "count":requests.len(),
                "requests":requests
            })))
        }
        CANCEL_MY_AUTHORIZATION_REQUEST => {
            let input: CancelArguments = serde_json::from_value(arguments)
                .with_context(|| format!("{CANCEL_MY_AUTHORIZATION_REQUEST} 参数无效"))?;
            if input.confirmation_phrase != CANCEL_CONFIRMATION_PHRASE {
                bail!("撤回授权申请确认短语无效");
            }
            let current = store.open_commerce_authorization_request(&input.request_id)?;
            if current.requester_user_id != user_id {
                bail!("当前用户无权撤回该授权申请");
            }
            let request = open_commerce_client_lifecycle_service::cancel_outbound_request(
                store,
                project_id,
                &input.request_id,
                &OpenCommerceActor {
                    user_id,
                    app_id,
                    project_role: Some(project_role),
                },
            )?;
            Ok(Some(serde_json::to_value(request)?))
        }
        _ => Ok(None),
    }
}

fn default_limit() -> usize {
    50
}

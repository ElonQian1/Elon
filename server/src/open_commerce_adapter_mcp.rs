use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    open_commerce_adapter_claim_model::ResumeAdapterHandoffClaimRequest,
    open_commerce_adapter_claim_service, open_commerce_adapter_service,
    open_commerce_service::OpenCommerceActor, store::Store,
};

const LIST: &str = "open_commerce_list_adapter_credentials";
const ROTATE: &str = "open_commerce_rotate_adapter_credential";
const REVOKE: &str = "open_commerce_revoke_adapter_credential";
const LIST_CLAIMS: &str = "open_commerce_list_adapter_handoff_claims";
const RESUME_CLAIM: &str = "open_commerce_resume_adapter_handoff_claim";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct IntegrationArguments {
    integration_id: String,
    confirmed_by_user: bool,
    expires_in_days: i64,
    allow_task_claims: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CredentialArguments {
    credential_id: String,
    confirmed_by_user: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListClaimsArguments {
    #[serde(default = "default_claim_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResumeClaimArguments {
    claim_id: String,
    confirmed_by_user: bool,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        tool(
            LIST,
            "列出当前项目的适配器机器凭据元数据，不返回明文 Token。",
            json!({"type":"object","properties":{},"additionalProperties":false}),
            true,
        ),
        tool(
            ROTATE,
            "为指定数据接入签发或轮换机器凭据。默认只能写业务衔接回执；只有 allow_task_claims=true 才显式增加单任务领取权。明文 Token 只返回一次，调用前必须取得用户明确同意。",
            json!({
                "type":"object",
                "required":["integration_id","confirmed_by_user","expires_in_days","allow_task_claims"],
                "properties":{
                    "integration_id":{"type":"string","minLength":1,"maxLength":120},
                    "confirmed_by_user":{"const":true},
                    "expires_in_days":{"type":"integer","minimum":1,"maximum":366},
                    "allow_task_claims":{"type":"boolean"}
                },
                "additionalProperties":false
            }),
            false,
        ),
        tool(
            REVOKE,
            "撤销适配器机器凭据，使原 Token 立即失效。调用前必须取得用户明确同意。",
            json!({
                "type":"object",
                "required":["credential_id","confirmed_by_user"],
                "properties":{
                    "credential_id":{"type":"string","minLength":1,"maxLength":120},
                    "confirmed_by_user":{"const":true}
                },
                "additionalProperties":false
            }),
            false,
        ),
        tool(
            LIST_CLAIMS,
            "列出当前项目最近的接入器任务租约、退避和暂停状态，不返回机器 Token 或租约密钥。",
            json!({
                "type":"object",
                "properties":{"limit":{"type":"integer","minimum":1,"maximum":200}},
                "additionalProperties":false
            }),
            true,
        ),
        tool(
            RESUME_CLAIM,
            "把第 6 次拒绝后暂停的当前接入器任务重新排队。只允许项目编辑者，并且必须先取得用户明确确认。",
            json!({
                "type":"object",
                "required":["claim_id","confirmed_by_user"],
                "properties":{
                    "claim_id":{"type":"string","minLength":1,"maxLength":160},
                    "confirmed_by_user":{"const":true}
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
    let actor = OpenCommerceActor {
        user_id,
        app_id,
        project_role: Some(project_role),
    };
    let value = match name {
        LIST => serde_json::to_value(open_commerce_adapter_service::list_credentials(
            store, project_id,
        )?)?,
        ROTATE => {
            let input: IntegrationArguments = decode(arguments, name)?;
            require_confirmation(input.confirmed_by_user)?;
            serde_json::to_value(open_commerce_adapter_service::rotate_credential(
                store,
                project_id,
                &input.integration_id,
                input.expires_in_days,
                input.allow_task_claims,
                &actor,
            )?)?
        }
        REVOKE => {
            let input: CredentialArguments = decode(arguments, name)?;
            require_confirmation(input.confirmed_by_user)?;
            serde_json::to_value(open_commerce_adapter_service::revoke_credential(
                store,
                project_id,
                &input.credential_id,
                &actor,
            )?)?
        }
        LIST_CLAIMS => {
            let input: ListClaimsArguments = decode(arguments, name)?;
            serde_json::to_value(open_commerce_adapter_claim_service::list_claims(
                store,
                project_id,
                input.limit,
            )?)?
        }
        RESUME_CLAIM => {
            let input: ResumeClaimArguments = decode(arguments, name)?;
            require_confirmation(input.confirmed_by_user)?;
            serde_json::to_value(open_commerce_adapter_claim_service::resume_retry(
                store,
                project_id,
                &input.claim_id,
                ResumeAdapterHandoffClaimRequest {
                    confirmed_by_user: true,
                },
                &actor,
            )?)?
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn require_confirmation(confirmed_by_user: bool) -> Result<()> {
    if !confirmed_by_user {
        bail!("管理适配器机器凭据前必须取得用户明确确认");
    }
    Ok(())
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
            "idempotentHint":false,
            "openWorldHint":false
        }
    })
}

fn default_claim_limit() -> usize {
    50
}

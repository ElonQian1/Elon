use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    open_commerce_adapter_service, open_commerce_service::OpenCommerceActor, store::Store,
};

const LIST: &str = "open_commerce_list_adapter_credentials";
const ROTATE: &str = "open_commerce_rotate_adapter_credential";
const REVOKE: &str = "open_commerce_revoke_adapter_credential";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct IntegrationArguments {
    integration_id: String,
    confirmed_by_user: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CredentialArguments {
    credential_id: String,
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
            "为指定数据接入签发或轮换仅可写业务衔接回执的机器凭据。明文 Token 只返回一次，调用前必须取得用户明确同意。",
            json!({
                "type":"object",
                "required":["integration_id","confirmed_by_user"],
                "properties":{
                    "integration_id":{"type":"string","minLength":1,"maxLength":120},
                    "confirmed_by_user":{"const":true}
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

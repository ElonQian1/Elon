use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    compute_federation_provider_service::{self, CreateMyComputeProviderRequest},
    store::Store,
};

const CREATE_PROVIDER_TOOL: &str = "compute_create_my_provider";
const GET_PROVIDER_TOOL: &str = "compute_get_my_provider";
const LIST_PROVIDERS_TOOL: &str = "compute_list_my_providers";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderArguments {
    provider_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListArguments {
    #[serde(default = "default_limit")]
    limit: usize,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        tool(
            CREATE_PROVIDER_TOOL,
            "为当前登录用户登记一个 self_declared、registering 的 user_node 或 managed_cluster 算力 Provider。不能提交路由、凭据、适配器或验证证据。",
            create_provider_schema(),
            false,
        ),
        tool(
            GET_PROVIDER_TOOL,
            "读取当前登录用户拥有的一份算力 Provider 脱敏视图。不会返回路由地址、凭据引用、适配器配置或结算账户。",
            provider_id_schema(),
            true,
        ),
        tool(
            LIST_PROVIDERS_TOOL,
            "列出当前登录用户拥有的算力 Provider 脱敏视图。不会修改数据。",
            list_schema(),
            true,
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
        CREATE_PROVIDER_TOOL => {
            let input: CreateMyComputeProviderRequest = decode(arguments, name)?;
            Ok(Some(serde_json::to_value(
                compute_federation_provider_service::create_for_user(store, user_id, input)?,
            )?))
        }
        GET_PROVIDER_TOOL => {
            let input: ProviderArguments = decode(arguments, name)?;
            Ok(Some(serde_json::to_value(
                compute_federation_provider_service::get_for_user(
                    store,
                    user_id,
                    &input.provider_id,
                )?,
            )?))
        }
        LIST_PROVIDERS_TOOL => {
            let input: ListArguments = decode(arguments, name)?;
            Ok(Some(json!({
                "providers":compute_federation_provider_service::list_for_user(
                    store,
                    user_id,
                    input.limit,
                )?
            })))
        }
        _ => Ok(None),
    }
}

fn create_provider_schema() -> Value {
    json!({
        "type":"object",
        "required":[
            "provider_id","provider_kind","display_name","task_kinds",
            "accelerator_kinds","regions","allowed_data_classes",
            "supports_streaming","supports_checkpointing"
        ],
        "properties":{
            "provider_id":{"type":"string","minLength":1,"maxLength":160},
            "provider_kind":{"type":"string","enum":["user_node","managed_cluster"]},
            "display_name":{"type":"string","minLength":1,"maxLength":160},
            "home_region":{"type":["string","null"],"maxLength":80},
            "task_kinds":{"type":"array","minItems":1,"items":{"type":"string","minLength":1,"maxLength":80},"uniqueItems":true},
            "accelerator_kinds":{"type":"array","minItems":1,"items":{"type":"string","minLength":1,"maxLength":80},"uniqueItems":true},
            "regions":{"type":"array","items":{"type":"string","minLength":1,"maxLength":80},"uniqueItems":true},
            "allowed_data_classes":{"type":"array","items":{"type":"string","enum":["public","low_sensitivity","restricted"]},"uniqueItems":true},
            "supports_streaming":{"type":"boolean"},
            "supports_checkpointing":{"type":"boolean"},
            "declared_hardware_digest":{"type":["string","null"],"maxLength":256}
        },
        "additionalProperties":false
    })
}

fn provider_id_schema() -> Value {
    json!({
        "type":"object",
        "required":["provider_id"],
        "properties":{"provider_id":{"type":"string","minLength":1,"maxLength":160}},
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

fn decode<T: for<'de> Deserialize<'de>>(arguments: Value, name: &str) -> Result<T> {
    serde_json::from_value(arguments).with_context(|| format!("{name} 参数无效"))
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
            "destructiveHint":false,
            "idempotentHint":true,
            "openWorldHint":false
        }
    })
}

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    compute_federation_capacity_pool_service::{self, CreateMyComputeCapacityPoolRequest},
    store::Store,
};

const CREATE_POOL_TOOL: &str = "compute_create_my_capacity_pool";
const GET_POOL_TOOL: &str = "compute_get_my_capacity_pool";
const LIST_POOLS_TOOL: &str = "compute_list_my_capacity_pools";
const AUDIT_POOL_TOOL: &str = "compute_audit_my_capacity_pool";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreatePoolArguments {
    provider_id: String,
    pool_id: String,
    resource_scope_key: String,
    region_or_data_zone: String,
    resource_profile: Value,
    meter_policies: Vec<
        crate::compute_federation_capacity_pool_service::CreateMyComputeCapacityMeterPolicyRequest,
    >,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PoolArguments {
    provider_id: String,
    pool_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListArguments {
    provider_id: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        tool(
            CREATE_POOL_TOOL,
            "为当前登录用户拥有的 Provider 登记 registering 容量池。服务端生成全部摘要，不激活、不发行容量，也不创建 Offer。",
            create_pool_schema(),
            false,
        ),
        tool(
            GET_POOL_TOOL,
            "读取本人 Provider 的一份容量池脱敏视图，不返回资源范围密钥或原始资源档案。",
            pool_schema(),
            true,
        ),
        tool(
            LIST_POOLS_TOOL,
            "列出本人 Provider 的容量池脱敏视图。",
            list_schema(),
            true,
        ),
        tool(
            AUDIT_POOL_TOOL,
            "按不可变账本重新计算本人当前 CapacityPool epoch 的余额，返回健康状态、差异和审计计数；不修改账本或 Pool 状态。",
            pool_schema(),
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
        CREATE_POOL_TOOL => {
            let input: CreatePoolArguments = decode(arguments, name)?;
            Ok(Some(serde_json::to_value(
                compute_federation_capacity_pool_service::create_for_user(
                    store,
                    user_id,
                    &input.provider_id,
                    CreateMyComputeCapacityPoolRequest {
                        pool_id: input.pool_id,
                        resource_scope_key: input.resource_scope_key,
                        region_or_data_zone: input.region_or_data_zone,
                        resource_profile: input.resource_profile,
                        meter_policies: input.meter_policies,
                    },
                )?,
            )?))
        }
        GET_POOL_TOOL => {
            let input: PoolArguments = decode(arguments, name)?;
            Ok(Some(serde_json::to_value(
                compute_federation_capacity_pool_service::get_for_user(
                    store,
                    user_id,
                    &input.provider_id,
                    &input.pool_id,
                )?,
            )?))
        }
        LIST_POOLS_TOOL => {
            let input: ListArguments = decode(arguments, name)?;
            Ok(Some(json!({
                "capacity_pools":compute_federation_capacity_pool_service::list_for_user(
                    store,
                    user_id,
                    &input.provider_id,
                    input.limit,
                )?
            })))
        }
        AUDIT_POOL_TOOL => {
            let input: PoolArguments = decode(arguments, name)?;
            Ok(Some(serde_json::to_value(
                compute_federation_capacity_pool_service::audit_for_user(
                    store,
                    user_id,
                    &input.provider_id,
                    &input.pool_id,
                )?,
            )?))
        }
        _ => Ok(None),
    }
}

fn create_pool_schema() -> Value {
    json!({
        "type":"object",
        "required":[
            "provider_id","pool_id","resource_scope_key","region_or_data_zone",
            "resource_profile","meter_policies"
        ],
        "properties":{
            "provider_id":{"type":"string","minLength":1,"maxLength":160},
            "pool_id":{"type":"string","minLength":1,"maxLength":160},
            "resource_scope_key":{"type":"string","minLength":1,"maxLength":256},
            "region_or_data_zone":{"type":"string","minLength":1,"maxLength":80},
            "resource_profile":{"type":"object"},
            "meter_policies":{
                "type":"array",
                "minItems":1,
                "maxItems":64,
                "items":{
                    "type":"object",
                    "required":["meter","meter_mode","quantum_units"],
                    "properties":{
                        "meter":{"type":"string","minLength":1,"maxLength":80},
                        "meter_mode":{"type":"string","enum":["consumable","reusable"]},
                        "quantum_units":{"type":"integer","minimum":1}
                    },
                    "additionalProperties":false
                }
            }
        },
        "additionalProperties":false
    })
}

fn pool_schema() -> Value {
    json!({
        "type":"object",
        "required":["provider_id","pool_id"],
        "properties":{
            "provider_id":{"type":"string","minLength":1,"maxLength":160},
            "pool_id":{"type":"string","minLength":1,"maxLength":160}
        },
        "additionalProperties":false
    })
}

fn list_schema() -> Value {
    json!({
        "type":"object",
        "required":["provider_id"],
        "properties":{
            "provider_id":{"type":"string","minLength":1,"maxLength":160},
            "limit":{"type":"integer","minimum":1,"maximum":100,"default":20}
        },
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

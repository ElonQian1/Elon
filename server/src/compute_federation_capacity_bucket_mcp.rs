use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    compute_federation_capacity_bucket_service::{self, CreateMyComputeCapacityBucketRequest},
    store::Store,
};

const CREATE_BUCKET_TOOL: &str = "compute_create_my_capacity_bucket";
const GET_BUCKET_TOOL: &str = "compute_get_my_capacity_bucket";
const LIST_BUCKETS_TOOL: &str = "compute_list_my_capacity_buckets";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateBucketArguments {
    provider_id: String,
    pool_id: String,
    bucket_id: String,
    window_id: String,
    starts_at_utc: String,
    ends_at_utc: String,
    meter: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BucketArguments {
    provider_id: String,
    pool_id: String,
    bucket_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListArguments {
    provider_id: String,
    pool_id: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        tool(
            CREATE_BUCKET_TOOL,
            "在本人 Provider 的当前 Pool 版本下创建 open、零余额 CapacityBucket。服务端生成窗口和 Bucket 摘要，不发行容量。",
            create_bucket_schema(),
            false,
        ),
        tool(
            GET_BUCKET_TOOL,
            "读取本人 Pool 下的一份 CapacityBucket 与当前账本余额。",
            bucket_schema(),
            true,
        ),
        tool(
            LIST_BUCKETS_TOOL,
            "列出本人 Pool 当前 epoch 的 CapacityBucket 与余额。",
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
        CREATE_BUCKET_TOOL => {
            let input: CreateBucketArguments = decode(arguments, name)?;
            Ok(Some(serde_json::to_value(
                compute_federation_capacity_bucket_service::create_for_user(
                    store,
                    user_id,
                    &input.provider_id,
                    &input.pool_id,
                    CreateMyComputeCapacityBucketRequest {
                        bucket_id: input.bucket_id,
                        window_id: input.window_id,
                        starts_at_utc: input.starts_at_utc,
                        ends_at_utc: input.ends_at_utc,
                        meter: input.meter,
                    },
                )?,
            )?))
        }
        GET_BUCKET_TOOL => {
            let input: BucketArguments = decode(arguments, name)?;
            Ok(Some(serde_json::to_value(
                compute_federation_capacity_bucket_service::get_for_user(
                    store,
                    user_id,
                    &input.provider_id,
                    &input.pool_id,
                    &input.bucket_id,
                )?,
            )?))
        }
        LIST_BUCKETS_TOOL => {
            let input: ListArguments = decode(arguments, name)?;
            Ok(Some(json!({
                "capacity_buckets":compute_federation_capacity_bucket_service::list_for_user(
                    store,
                    user_id,
                    &input.provider_id,
                    &input.pool_id,
                    input.limit,
                )?
            })))
        }
        _ => Ok(None),
    }
}

fn create_bucket_schema() -> Value {
    json!({
        "type":"object",
        "required":[
            "provider_id","pool_id","bucket_id","window_id",
            "starts_at_utc","ends_at_utc","meter"
        ],
        "properties":{
            "provider_id":{"type":"string","minLength":1,"maxLength":160},
            "pool_id":{"type":"string","minLength":1,"maxLength":160},
            "bucket_id":{"type":"string","minLength":1,"maxLength":160},
            "window_id":{"type":"string","minLength":1,"maxLength":160},
            "starts_at_utc":{"type":"string","format":"date-time"},
            "ends_at_utc":{"type":"string","format":"date-time"},
            "meter":{"type":"string","minLength":1,"maxLength":80}
        },
        "additionalProperties":false
    })
}

fn bucket_schema() -> Value {
    json!({
        "type":"object",
        "required":["provider_id","pool_id","bucket_id"],
        "properties":{
            "provider_id":{"type":"string","minLength":1,"maxLength":160},
            "pool_id":{"type":"string","minLength":1,"maxLength":160},
            "bucket_id":{"type":"string","minLength":1,"maxLength":160}
        },
        "additionalProperties":false
    })
}

fn list_schema() -> Value {
    json!({
        "type":"object",
        "required":["provider_id","pool_id"],
        "properties":{
            "provider_id":{"type":"string","minLength":1,"maxLength":160},
            "pool_id":{"type":"string","minLength":1,"maxLength":160},
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

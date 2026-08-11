use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

pub(super) fn ensure_platform_admin(platform_role: &str) -> Result<()> {
    if !matches!(platform_role, "admin" | "owner") {
        bail!("只有平台管理员可以调用该分布式算力管理工具");
    }
    Ok(())
}

pub(super) fn decode<T: for<'de> Deserialize<'de>>(arguments: Value, name: &str) -> Result<T> {
    serde_json::from_value(arguments).with_context(|| format!("{name} 参数无效"))
}

pub(super) fn tool(
    name: &str,
    description: &str,
    input_schema: Value,
    read_only: bool,
    destructive: bool,
) -> Value {
    json!({
        "name":name,
        "description":description,
        "inputSchema":input_schema,
        "annotations":{
            "readOnlyHint":read_only,
            "destructiveHint":destructive,
            "idempotentHint":true,
            "openWorldHint":false
        }
    })
}

pub(super) fn bounded_string(max_length: usize) -> Value {
    json!({"type":"string","minLength":1,"maxLength":max_length})
}

pub(super) fn optional_status() -> Value {
    json!({"type":"string","minLength":1,"maxLength":64})
}

pub(super) fn entity_schema(entity_key: &str, max_length: usize) -> Value {
    let mut properties = serde_json::Map::new();
    properties.insert(entity_key.to_string(), bounded_string(max_length));
    json!({
        "type":"object",
        "required":[entity_key],
        "properties":Value::Object(properties),
        "additionalProperties":false
    })
}

pub(super) fn list_schema() -> Value {
    json!({
        "type":"object",
        "properties":{
            "status":optional_status(),
            "limit":{"type":"integer","minimum":1,"maximum":1000,"default":50}
        },
        "additionalProperties":false
    })
}

pub(super) fn default_limit() -> usize {
    50
}

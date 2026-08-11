//! Platform-administrator MCP surface for activation governance.

mod plan;
mod recovery;
mod request;

use anyhow::Result;
use serde_json::{json, Value};

use crate::store::Store;

use super::management_mcp_support as support;

pub(crate) fn admin_definitions() -> Vec<Value> {
    let mut tools = request::definitions();
    tools.extend(plan::definitions());
    tools.extend(recovery::definitions());
    tools
}

pub(crate) fn call_admin_if_handled(
    store: &Store,
    user_id: &str,
    platform_role: &str,
    name: &str,
    arguments: Value,
) -> Result<Option<Value>> {
    if let Some(value) =
        request::call_if_handled(store, user_id, platform_role, name, arguments.clone())?
    {
        return Ok(Some(value));
    }
    if let Some(value) =
        plan::call_if_handled(store, user_id, platform_role, name, arguments.clone())?
    {
        return Ok(Some(value));
    }
    recovery::call_if_handled(store, user_id, platform_role, name, arguments)
}

fn request_schema() -> Value {
    support::entity_schema("request_id", 160)
}

fn wrapped_schema(body: Value) -> Value {
    json!({
        "type":"object",
        "required":["request_id","request"],
        "properties":{
            "request_id":support::bounded_string(160),
            "request":body
        },
        "additionalProperties":false
    })
}

fn digest_schema() -> Value {
    json!({"type":"string","pattern":"^[0-9a-f]{64}$"})
}

fn endpoint_schema() -> Value {
    json!({
        "type":"object",
        "required":["endpoint_id","transport"],
        "properties":{
            "endpoint_id":support::bounded_string(160),
            "transport":support::bounded_string(80),
            "address_hint":{"type":["string","null"],"maxLength":1000},
            "gateway_id":{"type":["string","null"],"maxLength":160},
            "credential_ref":{"type":["string","null"],"maxLength":160}
        },
        "additionalProperties":false
    })
}

fn adapter_schema() -> Value {
    json!({
        "type":"object",
        "required":["adapter_id","adapter_version","config_revision","config_digest"],
        "properties":{
            "adapter_id":support::bounded_string(160),
            "adapter_version":support::bounded_string(80),
            "config_revision":{"type":"integer","minimum":1},
            "config_digest":digest_schema()
        },
        "additionalProperties":false
    })
}

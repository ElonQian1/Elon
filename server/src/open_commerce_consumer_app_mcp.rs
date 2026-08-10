//! Read-only MCP view of the caller's own developer application identities.

use anyhow::{bail, Result};
use serde_json::{json, Value};

use crate::store::Store;

const LIST_MY_CONSUMER_APPS: &str = "open_commerce_list_my_consumer_apps";

pub(crate) fn definitions() -> Vec<Value> {
    vec![json!({
        "name":LIST_MY_CONSUMER_APPS,
        "description":"列出当前项目中属于当前用户的开发者 App，供消费者 AI 选择 x-elon-app-id。只返回非秘密状态摘要，不返回测试 Token、Token 提示或生产凭据。",
        "inputSchema":{"type":"object","properties":{},"additionalProperties":false},
        "annotations":{
            "readOnlyHint":true,
            "destructiveHint":false,
            "idempotentHint":true,
            "openWorldHint":false
        }
    })]
}

pub(crate) fn call_if_handled(
    store: &Store,
    project_id: &str,
    user_id: &str,
    current_app_id: &str,
    uses_default_mcp_identity: bool,
    name: &str,
    arguments: Value,
) -> Result<Option<Value>> {
    if name != LIST_MY_CONSUMER_APPS {
        return Ok(None);
    }
    ensure_empty(&arguments)?;
    let apps = store
        .list_project_open_commerce_developer_apps(project_id)?
        .into_iter()
        .filter(|app| app.owner_user_id == user_id)
        .collect::<Vec<_>>();
    if !uses_default_mcp_identity && !apps.iter().any(|app| app.app_id == current_app_id) {
        bail!("当前 MCP App 不属于当前用户和项目，或该 App 已不存在");
    }
    let apps = apps
        .into_iter()
        .map(|app| {
            json!({
                "record_id":app.id,
                "app_id":app.app_id,
                "display_name":app.display_name,
                "status":app.status,
                "environment":app.environment,
                "manifest_status":app.manifest_status,
                "requested_scopes":app.requested_scopes,
                "updated_at":app.updated_at,
                "is_current_mcp_identity":!uses_default_mcp_identity && app.app_id == current_app_id,
                "can_use_for_sandbox_mcp":app.status == "active" && app.environment == "sandbox"
            })
        })
        .collect::<Vec<_>>();
    Ok(Some(json!({
        "schema":"open_commerce.consumer_app_directory.v1",
        "project_id":project_id,
        "current_mcp_identity":{
            "app_id":current_app_id,
            "kind":if uses_default_mcp_identity {"default_system"} else {"registered_app"}
        },
        "test_tokens_included":false,
        "production_credentials_included":false,
        "apps":apps
    })))
}

fn ensure_empty(arguments: &Value) -> Result<()> {
    if !arguments.as_object().is_some_and(serde_json::Map::is_empty) {
        bail!("{LIST_MY_CONSUMER_APPS} 不接受参数");
    }
    Ok(())
}

#[cfg(test)]
#[path = "open_commerce_consumer_app_mcp_tests.rs"]
mod tests;

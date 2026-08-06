//! Single-tool feature workflow profile for ordinary Codex tasks.

use anyhow::{bail, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;

use crate::{
    node_agent_project_docs_mcp::McpRequest,
    project_document_response::{compact_text, project_tool_response},
};

pub(crate) const PROFILE: &str = "feature";
pub(crate) const TOOL_NAME: &str = "project_feature_workflow";

#[derive(Debug, Deserialize)]
struct WorkflowArguments {
    action: String,
    #[serde(default)]
    payload: Value,
}

pub(crate) fn handles(profile: Option<&str>) -> bool {
    profile == Some(PROFILE)
}

pub(crate) fn handle_request(workspace: &Path, request: &McpRequest) -> Result<Value> {
    match request.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion":"2025-03-26",
            "capabilities":{"tools":{"listChanged":false}},
            "serverInfo":{"name":"yilong-project-features","version":"1.0.0"},
            "instructions":"Use this single tool only to create or advance an explicit project feature. Call action=describe first when field requirements are unknown. Requirement and source bodies stay in native files; this workflow stores bounded Git metadata and evidence identities only. Current source/tests outrank registry metadata."
        })),
        "tools/list" => Ok(json!({"tools":[definition()]})),
        "tools/call" => call_tool(workspace, request.params.clone()),
        "ping" => Ok(json!({})),
        _ => bail!("项目功能 MCP 不支持 method: {}", request.method),
    }
}

fn definition() -> Value {
    json!({
        "name":TOOL_NAME,
        "description":"按需处理 Git 功能需求生命周期。无关任务不要调用；字段不明确先用 describe，只在需要时加载详细契约。",
        "inputSchema":{
            "type":"object",
            "required":["action"],
            "properties":{
                "action":{"type":"string","enum":["describe","register","list","update","rebind_requirement","plan","claim","release_claim","transition","record_evidence","check_drift","history"]},
                "payload":{"type":"object","description":"所选动作参数；describe 的 payload 为空对象。"}
            }
        }
    })
}

fn call_tool(workspace: &Path, params: Value) -> Result<Value> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("tools/call 缺少 name"))?;
    if name != TOOL_NAME {
        bail!("单工具功能 profile 不支持：{name}");
    }
    let input: WorkflowArguments = serde_json::from_value(
        params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({})),
    )?;
    let value = if input.action == "describe" {
        json!({
            "schema":"elon.project_feature_workflow_description.v1",
            "operations":crate::node_agent_project_feature_tools::definitions(),
            "instruction":"Choose one operation and pass its inputSchema fields as payload to project_feature_workflow."
        })
    } else {
        let tool_name = operation_tool_name(&input.action)?;
        let payload = if input.payload.is_null() {
            json!({})
        } else {
            input.payload
        };
        crate::node_agent_project_feature_tools::try_call(workspace, tool_name, payload)?
            .ok_or_else(|| anyhow::anyhow!("功能动作没有处理器：{}", input.action))?
    };
    let value = project_tool_response(TOOL_NAME, &json!({}), value)?;
    Ok(json!({
        "content":[{"type":"text","text":compact_text(TOOL_NAME, &value)?}],
        "structuredContent":value,
        "isError":false,
    }))
}

fn operation_tool_name(action: &str) -> Result<&'static str> {
    match action.trim() {
        "register" => Ok("project_features_register"),
        "list" => Ok("project_features_list"),
        "update" => Ok("project_features_update"),
        "rebind_requirement" => Ok("project_features_rebind_requirement"),
        "plan" => Ok("project_features_plan"),
        "claim" => Ok("project_features_claim"),
        "release_claim" => Ok("project_features_release_claim"),
        "transition" => Ok("project_features_transition"),
        "record_evidence" => Ok("project_features_record_evidence"),
        "check_drift" => Ok("project_features_check_drift"),
        "history" => Ok("project_features_history"),
        _ => bail!("未知功能动作：{action}；不确定字段时先调用 describe"),
    }
}

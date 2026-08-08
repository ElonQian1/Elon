//! Project-bound MCP profile for allowlisted Win semantic control and diagnostics.

use anyhow::{bail, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::HashSet, path::Path};

use crate::{node_agent_project_docs_mcp::McpRequest, NodeRuntime};

pub(crate) const PROFILE: &str = "win_control";

#[derive(Debug, Deserialize)]
struct TimelineArguments {
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    sources: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ActionArguments {
    kind: String,
    #[serde(default)]
    route: Option<String>,
    #[serde(default)]
    trace_id: Option<String>,
}

pub(crate) fn handles(profile: Option<&str>) -> bool {
    profile == Some(PROFILE)
}

pub(crate) fn handle_request(
    runtime: &NodeRuntime,
    workspace: &Path,
    request: &McpRequest,
) -> Result<Value> {
    match request.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion":"2025-03-26",
            "capabilities":{"tools":{"listChanged":false}},
            "serverInfo":{"name":"yilong-win-control","version":"1.0.0"},
            "instructions":"Use semantic allowlisted actions only. Read status/timeline before changing the Win client. Never request arbitrary JavaScript, Tauri commands, URLs, cookies, request bodies, prompts, or secrets. A queued action is not successful until a Tauri receipt says succeeded."
        })),
        "tools/list" => Ok(json!({"tools": definitions()})),
        "tools/call" => call_tool(runtime, workspace, request.params.clone()),
        "ping" => Ok(json!({})),
        _ => bail!("Win 控制 MCP 不支持 method: {}", request.method),
    }
}

fn definitions() -> Vec<Value> {
    vec![
        json!({
            "name":"win_control_status",
            "description":"读取 Win/Tauri 在线状态、动作白名单、日志来源和安全边界。",
            "inputSchema":{"type":"object","additionalProperties":false}
        }),
        json!({
            "name":"win_control_timeline",
            "description":"读取项目绑定的脱敏统一时间线；CLI 正文仍在授权 task journal。",
            "inputSchema":{
                "type":"object","additionalProperties":false,
                "properties":{
                    "limit":{"type":"integer","minimum":1,"maximum":300,"default":120},
                    "sources":{"type":"array","maxItems":6,"items":{"type":"string","enum":["frontend","rust","cli","network","tauri","control"]}}
                }
            }
        }),
        json!({
            "name":"win_control_action",
            "description":"排队白名单 Win 语义动作。queued 不等于成功，随后必须读 timeline/状态确认 Tauri 回执。",
            "inputSchema":{
                "type":"object","required":["kind"],"additionalProperties":false,
                "properties":{
                    "kind":{"type":"string","enum":["show_window","focus_window","navigate","reload_page","open_devtools","close_devtools","capture_state"]},
                    "route":{"type":"string","maxLength":180,"description":"仅 navigate 使用的已登记相对路径，不含 URL/query/hash。"},
                    "trace_id":{"type":"string","maxLength":160}
                }
            }
        }),
    ]
}

fn call_tool(runtime: &NodeRuntime, workspace: &Path, params: Value) -> Result<Value> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("tools/call 缺少 name"))?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let value = match name {
        "win_control_status" => json!({
            "schema":"elon.win_codex_control_status.v1",
            "project_root":workspace.to_string_lossy(),
            "capabilities":runtime.win_codex_control.capabilities(),
        }),
        "win_control_timeline" => {
            let input: TimelineArguments = serde_json::from_value(arguments)?;
            let sources = input
                .sources
                .into_iter()
                .filter(|source| {
                    matches!(
                        source.as_str(),
                        "frontend" | "rust" | "cli" | "network" | "tauri" | "control"
                    )
                })
                .collect::<HashSet<_>>();
            crate::node_agent_win_codex_control::timeline_payload(
                runtime,
                Some(workspace),
                0,
                input.limit.unwrap_or(120).clamp(1, 300),
                &sources,
            )
        }
        "win_control_action" => {
            let input: ActionArguments = serde_json::from_value(arguments)?;
            let action = runtime
                .win_codex_control
                .enqueue_action(
                    input.trace_id.as_deref().unwrap_or("codex_mcp"),
                    &input.kind,
                    input.route.as_deref(),
                    "codex_mcp",
                )
                .map_err(anyhow::Error::msg)?;
            json!({
                "schema":"elon.win_codex_action.v1",
                "action":action,
                "completion_rule":"queued is not success; wait for a succeeded Tauri receipt",
            })
        }
        _ => bail!("Win 控制 profile 不支持工具：{name}"),
    };
    Ok(tool_result(name, value))
}

fn tool_result(name: &str, value: Value) -> Value {
    let text = serde_json::to_string(&value).unwrap_or_else(|_| format!("{name} returned data"));
    json!({
        "content":[{"type":"text","text":text}],
        "structuredContent":value,
        "isError":false,
    })
}

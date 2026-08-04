//! Minimal write-only MCP profile for explicit post-task project-memory receipts.

use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::path::Path;

use crate::node_agent_project_docs_mcp::McpRequest;

pub(crate) const PROFILE: &str = "receipt";

pub(crate) fn handles(profile: Option<&str>) -> bool {
    profile == Some(PROFILE)
}

pub(crate) fn handle_request(
    workspace: &Path,
    request: &McpRequest,
    session_id: Option<&str>,
) -> Result<Value> {
    match request.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": "2025-03-26",
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "yilong-project-receipt", "version": "1.0.0" },
            "instructions": "Use the single receipt tool only after an explicit coding task when native reads established a reusable, evidence-backed navigation fact. Skip duplicates, task-local trivia, guesses, and conflicts. Submit paths/locators only; never source bodies, prompts, chats, commands, tool output, or Codex private memories. Receipts remain local pending-review candidates until the existing review/apply flow promotes them."
        })),
        "tools/list" => Ok(json!({
            "tools": [crate::node_agent_project_docs_mcp_native_context_tools::receipt_definition()]
        })),
        "tools/call" => crate::node_agent_project_docs_mcp_native_context_tools::call_receipt_tool(
            workspace,
            request.params.clone(),
            session_id,
        ),
        "ping" => Ok(json!({})),
        _ => bail!("项目理解回执 MCP 不支持 method: {}", request.method),
    }
}

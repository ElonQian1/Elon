//! Minimal read-only MCP profile for ordinary coding tasks.
//!
//! Codex already owns exact workspace search and file reads. This profile only
//! returns a bounded, revision-aware navigation plan so normal tasks do not pay
//! for the complete project-document governance tool catalog.

use anyhow::{bail, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;

use crate::{
    node_agent_project_docs_mcp::McpRequest,
    project_document_knowledge_graph_service::plan_context,
    project_document_response::{compact_text, project_tool_response},
};

pub(crate) const PROFILE: &str = "context";
pub(crate) const TOOL_NAME: &str = "project_context_plan";

const SERVER_INSTRUCTIONS: &str = "Use project_context_plan only to orient an unfamiliar, cross-file, architecture, or current-status task before broad searching. It returns paths and evidence, never source bodies. Open every selected path with Codex native file tools before editing. Current workspace files/tests decide implementation truth; authoritative current docs/ADRs decide accepted direction. Report conflicts and re-plan when git_head or catalog_revision changes. Skip this tool for a precise single-file task.";

#[derive(Debug, Deserialize)]
struct ContextPlanArguments {
    query: String,
    #[serde(default = "default_token_budget")]
    max_tokens: u64,
    #[serde(default = "default_document_limit")]
    max_documents: usize,
}

pub(crate) fn handles(profile: Option<&str>) -> bool {
    profile == Some(PROFILE)
}

pub(crate) fn handle_request(workspace: &Path, request: &McpRequest) -> Result<Value> {
    match request.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": "2025-03-26",
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "yilong-project-context", "version": "1.0.0" },
            "instructions": SERVER_INSTRUCTIONS,
        })),
        "tools/list" => Ok(json!({ "tools": [definition()] })),
        "tools/call" => call_tool(workspace, request.params.clone()),
        "ping" => Ok(json!({})),
        _ => bail!("轻量项目上下文 MCP 不支持 method: {}", request.method),
    }
}

pub(crate) fn definition() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "为陌生项目、跨文件、架构或当前状态任务返回少量带 Git/revision、权威性、图谱入口和实现引用的阅读导航。零正文、只读；随后必须用代理原生文件搜索/读取核对真实工作区。精确单文件任务不要调用。",
        "inputSchema": {
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 500,
                    "description": "当前用户任务或要理解的问题。"
                },
                "max_tokens": {
                    "type": "integer",
                    "minimum": 200,
                    "maximum": 2400,
                    "default": 1200,
                    "description": "相关文档的规划预算；不返回正文。"
                },
                "max_documents": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 8,
                    "default": 6
                }
            }
        }
    })
}

pub(crate) fn try_call(workspace: &Path, name: &str, arguments: Value) -> Result<Option<Value>> {
    if name != TOOL_NAME {
        return Ok(None);
    }
    let input: ContextPlanArguments = serde_json::from_value(arguments)?;
    Ok(Some(build_plan(workspace, input)?))
}

fn call_tool(workspace: &Path, params: Value) -> Result<Value> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("tools/call 缺少 name"))?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let value = try_call(workspace, name, arguments.clone())?
        .ok_or_else(|| anyhow::anyhow!("轻量项目上下文 MCP 只开放 {TOOL_NAME}"))?;
    let value = project_tool_response(name, &arguments, value)?;
    Ok(json!({
        "content": [{ "type": "text", "text": compact_text(name, &value)? }],
        "structuredContent": value,
        "isError": false,
    }))
}

fn build_plan(workspace: &Path, input: ContextPlanArguments) -> Result<Value> {
    let query = input.query.trim();
    if query.is_empty() {
        bail!("project_context_plan.query 不能为空");
    }
    if query.chars().count() > 500 {
        bail!("project_context_plan.query 最多 500 个字符");
    }

    let mut plan = plan_context(
        workspace,
        query,
        None,
        input.max_tokens.clamp(200, 2_400),
        input.max_documents.clamp(1, 8),
        1_600,
    )?;
    let git_head = crate::node_agent_update_checkpoint::git_output(
        workspace,
        &["rev-parse", "--verify", "HEAD^{commit}"],
    );
    let branch =
        crate::node_agent_update_checkpoint::git_output(workspace, &["branch", "--show-current"]);
    let git_clean =
        crate::node_agent_update_checkpoint::git_output(workspace, &["status", "--porcelain"])
            .map(|status| status.trim().is_empty());
    let catalog_revision = plan.get("catalog_revision").cloned().unwrap_or(Value::Null);

    plan["contract"] = json!({
        "schema": "elon.project_context_plan.v1",
        "read_only": true,
        "source_bodies_returned": 0,
        "native_search_replaced": false,
    });
    plan["workspace_revision"] = json!({
        "git_head": git_head,
        "git_branch": branch,
        "git_clean": git_clean,
        "catalog_revision": catalog_revision,
        "replan_when_revision_changes": true,
    });
    plan["source_policy"] = json!({
        "implementation_truth": ["current_workspace_files", "tests", "build_or_runtime_evidence"],
        "accepted_direction": ["binding_project_rules", "authoritative_current_status", "accepted_decisions"],
        "navigation_only": ["knowledge_graph", "repo_map", "symbol_index", "generated_summaries"],
        "default_excluded": ["drafts", "discussions", "historical_reports", "archives", "traces"],
        "conflict_rule": "Report documentation/implementation drift; do not silently let summaries override current files or let code erase an accepted decision.",
    });
    plan["native_tool_handoff"] = json!({
        "next": "Open only the selected paths/sections and implementation_refs with Codex native search/read tools.",
        "before_edit": "Verify exact current source and tests at workspace_revision.git_head; treat this plan as navigation, not copied source truth.",
        "skip_expansion_when": "The first selected source resolves the question or the task already names an exact file/symbol.",
    });
    Ok(plan)
}

fn default_token_budget() -> u64 {
    1_200
}

fn default_document_limit() -> usize {
    6
}

#[cfg(test)]
mod tests {
    use super::{definition, handle_request, handles, PROFILE, TOOL_NAME};
    use crate::node_agent_project_docs_mcp::McpRequest;
    use serde_json::json;
    use std::path::Path;

    #[test]
    fn context_profile_exposes_one_bounded_read_only_tool() {
        let definition = definition();
        assert_eq!(definition["name"], TOOL_NAME);
        assert_eq!(
            definition["inputSchema"]["properties"]["max_tokens"]["maximum"],
            2400
        );
        assert_eq!(
            definition["inputSchema"]["properties"]["max_documents"]["maximum"],
            8
        );
        assert!(handles(Some(PROFILE)));
        assert!(!handles(None));
    }

    #[test]
    fn context_profile_lists_only_the_navigation_tool() {
        let request: McpRequest = serde_json::from_value(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {}
        }))
        .unwrap();
        let response = handle_request(Path::new("."), &request).unwrap();
        assert_eq!(response["tools"].as_array().unwrap().len(), 1);
        assert_eq!(response["tools"][0]["name"], TOOL_NAME);
    }
}

//! Minimal read-only MCP profile for ordinary coding tasks.
//!
//! Codex already owns exact workspace search and file reads. This profile only
//! returns a bounded, revision-aware navigation plan so normal tasks do not pay
//! for the complete project-document governance tool catalog.

use anyhow::{bail, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{path::Path, time::Instant};

use crate::{
    node_agent_project_context_cache::{
        inspect_workspace, lookup, request_cache_key, stable_plan_id, store, WorkspaceRevision,
    },
    node_agent_project_context_projection::{
        enforce_response_projection, not_modified_response, project_navigation_plan,
        source_conflict_summary,
    },
    node_agent_project_docs_mcp::McpRequest,
    project_document_knowledge_graph_service::plan_context,
    project_document_response::{compact_text, project_tool_response},
};

pub(crate) const PROFILE: &str = "context";
pub(crate) const TOOL_NAME: &str = "project_context_plan";

const SERVER_INSTRUCTIONS: &str = "Use project_context_plan only for unfamiliar, cross-file, architecture, or current-status work. It returns bounded metadata, never source bodies. Reuse plan_receipt.plan_id as previous_plan_id; an unchanged revision then returns only a small receipt. Clean HEAD or a complete bounded dirty-content fingerprint controls cache reuse; incomplete dirty states fail closed. Open selected paths with Codex native tools before editing. Current files/tests decide implementation truth; binding current docs/ADRs decide accepted direction. Skip precise single-file tasks.";
const MIN_RESPONSE_TOKENS: u64 = 800;
const MAX_RESPONSE_TOKENS: u64 = 2_000;

#[derive(Debug, Deserialize)]
struct ContextPlanArguments {
    query: String,
    #[serde(default = "default_token_budget")]
    max_tokens: u64,
    #[serde(default = "default_document_limit")]
    max_documents: usize,
    #[serde(default = "default_response_token_budget")]
    max_response_tokens: u64,
    #[serde(default)]
    previous_plan_id: Option<String>,
    #[serde(default)]
    force_refresh: bool,
}

pub(crate) fn handles(profile: Option<&str>) -> bool {
    profile == Some(PROFILE)
}

pub(crate) fn handle_request(workspace: &Path, request: &McpRequest) -> Result<Value> {
    match request.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": "2025-03-26",
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "yilong-project-context", "version": "1.2.0" },
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
        "description": "为陌生项目、跨文件、架构或当前状态任务返回严格限额的 Git/revision、权威性、图谱入口和实现引用。零正文、只读；重复调用传 previous_plan_id 可只取未变化回执。精确单文件任务不要调用。",
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
                },
                "max_response_tokens": {
                    "type": "integer",
                    "minimum": MIN_RESPONSE_TOKENS,
                    "maximum": MAX_RESPONSE_TOKENS,
                    "default": 1200,
                    "description": "本工具结构化响应的近似硬预算，不是后续文件阅读预算。"
                },
                "previous_plan_id": {
                    "type": "string",
                    "maxLength": 96,
                    "description": "复用上一响应 plan_receipt.plan_id；revision 未变时只返回小回执。"
                },
                "force_refresh": {
                    "type": "boolean",
                    "default": false,
                    "description": "仅怀疑索引异常时绕过短缓存。"
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
    let started = Instant::now();
    let query = input.query.trim();
    if query.is_empty() {
        bail!("project_context_plan.query 不能为空");
    }
    if query.chars().count() > 500 {
        bail!("project_context_plan.query 最多 500 个字符");
    }
    let previous_plan_id = input.previous_plan_id.as_deref().map(str::trim);
    if previous_plan_id.is_some_and(|value| value.len() > 96) {
        bail!("previous_plan_id 最多 96 个字符");
    }
    let max_tokens = input.max_tokens.clamp(200, 2_400);
    let max_documents = input.max_documents.clamp(1, 8);
    let max_response_tokens = input
        .max_response_tokens
        .clamp(MIN_RESPONSE_TOKENS, MAX_RESPONSE_TOKENS);
    let revision = inspect_workspace(workspace);
    let cache_key = request_cache_key(
        workspace,
        &revision,
        query,
        max_tokens,
        max_documents,
        max_response_tokens,
    );

    let cached = (!input.force_refresh)
        .then(|| cache_key.as_deref().and_then(lookup))
        .flatten();
    let (mut plan, cache_status, cache_age_ms) = if let Some(hit) = cached {
        (hit.plan, "hit", Some(hit.age_ms))
    } else {
        let plan = build_fresh_plan(
            workspace,
            query,
            max_tokens,
            max_documents,
            max_response_tokens,
            &revision,
            cache_key.is_some(),
        )?;
        if let Some(key) = cache_key.clone() {
            store(key, plan.clone());
        }
        let status = if cache_key.is_none() {
            if revision.git_clean == Some(false) {
                "bypass_incomplete_dirty_fingerprint"
            } else {
                "bypass_unknown_revision"
            }
        } else if input.force_refresh {
            "refreshed"
        } else {
            "miss"
        };
        (plan, status, None)
    };
    let plan_id = plan
        .pointer("/plan_receipt/plan_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !input.force_refresh && previous_plan_id == Some(plan_id) {
        return Ok(not_modified_response(
            &plan,
            cache_status,
            cache_age_ms,
            max_response_tokens,
            elapsed_ms(started),
        ));
    }
    let estimated_full_plan_tokens = plan
        .pointer("/plan_receipt/estimated_full_plan_tokens")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    plan["cache"] = json!({
        "status": cache_status,
        "age_ms": cache_age_ms,
        "ttl_seconds": 300,
        "revision_mode": revision.fingerprint_status,
        "fingerprinted_files": revision.fingerprint_file_count,
        "fingerprinted_bytes": revision.fingerprint_total_bytes,
        "bypass_reason": revision.cache_bypass_reason,
        "dirty_requires_complete_fingerprint": true,
    });
    plan["performance_receipt"] = json!({
        "planning_ms": elapsed_ms(started),
        "cache_reused": cache_status == "hit",
        "estimated_full_plan_tokens": estimated_full_plan_tokens,
        "estimated_tokens_avoided": 0,
    });
    Ok(plan)
}

fn build_fresh_plan(
    workspace: &Path,
    query: &str,
    max_tokens: u64,
    max_documents: usize,
    max_response_tokens: u64,
    revision: &WorkspaceRevision,
    cacheable: bool,
) -> Result<Value> {
    let raw = plan_context(workspace, query, None, max_tokens, max_documents, 1_600)?;
    let mut plan = project_navigation_plan(&raw);
    let catalog_revision = plan.get("catalog_revision").cloned().unwrap_or(Value::Null);
    plan["contract"] = json!({
        "schema": "elon.project_context_plan.v3",
        "read_only": true,
        "source_bodies_returned": 0,
        "native_search_replaced": false,
        "max_response_tokens": max_response_tokens,
    });
    plan["workspace_revision"] = json!({
        "git_head": revision.git_head,
        "git_branch": revision.git_branch,
        "git_clean": revision.git_clean,
        "worktree_fingerprint": revision.worktree_fingerprint,
        "fingerprint_status": revision.fingerprint_status,
        "fingerprinted_files": revision.fingerprint_file_count,
        "fingerprinted_bytes": revision.fingerprint_total_bytes,
        "cache_bypass_reason": revision.cache_bypass_reason,
        "catalog_revision": catalog_revision,
        "replan_when_revision_changes": true,
    });
    plan["source_policy"] = json!({
        "precedence": [
            {"rank":1,"role":"implementation_truth","sources":["current_files","tests","runtime_evidence"]},
            {"rank":2,"role":"accepted_direction","sources":["binding_rules","current_status","accepted_decisions"]},
            {"rank":3,"role":"navigation_only","sources":["knowledge_graph","indexes","generated_summaries"]}
        ],
        "default_excluded": ["drafts","discussions","history","archives","traces"],
        "conflict_action": "Report drift and verify both current source and binding direction; never silently choose an index or summary."
    });
    plan["source_conflict_summary"] = source_conflict_summary(&plan, revision);
    plan["native_tool_handoff"] = json!({
        "next": "Open only selected paths and implementation_refs with Codex native search/read.",
        "before_edit": "Verify exact current source/tests; this response is navigation metadata.",
        "stop": "Stop expanding once one verified source resolves the task."
    });
    plan = enforce_response_projection(plan, max_response_tokens);
    let receipt_material = json!({
        "schema": "elon.project_context_receipt.v2",
        "query": plan.get("query"),
        "workspace_revision": plan.get("workspace_revision"),
        "catalog_revision": plan.get("catalog_revision"),
        "matched_nodes": plan.get("matched_nodes"),
        "mandatory_rules": plan.get("mandatory_rules"),
        "relevant_documents": plan.get("relevant_documents"),
        "selected_paths": plan.get("selected_paths"),
    });
    let estimated_full_plan_tokens = serde_json::to_vec(&plan)?.len().div_ceil(4);
    plan["plan_receipt"] = json!({
        "plan_id": stable_plan_id(&receipt_material),
        "schema": "elon.project_context_receipt.v2",
        "cacheable": cacheable,
        "revision_bound": true,
        "reuse_parameter": "previous_plan_id",
        "estimated_full_plan_tokens": estimated_full_plan_tokens,
    });
    Ok(plan)
}

fn default_token_budget() -> u64 {
    1_200
}

fn default_document_limit() -> usize {
    6
}

fn default_response_token_budget() -> u64 {
    1_200
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
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
        assert_eq!(
            definition["inputSchema"]["properties"]["max_response_tokens"]["maximum"],
            2000
        );
        assert!(definition["inputSchema"]["properties"]["previous_plan_id"].is_object());
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

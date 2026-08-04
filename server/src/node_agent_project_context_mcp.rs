//! Minimal read-only MCP profile for ordinary coding tasks.
//!
//! Codex already owns exact workspace search and file reads. This profile only
//! returns a bounded, revision-aware navigation plan so normal tasks do not pay
//! for the complete project-document governance tool catalog.

mod session;

use anyhow::{bail, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{fs, path::Path, time::Instant};

use crate::{
    node_agent_project_context_cache::{
        inspect_workspace, lookup, request_cache_key, stable_plan_id, store, WorkspaceRevision,
    },
    node_agent_project_context_projection::{
        enforce_response_projection, project_navigation_plan, source_conflict_summary,
    },
    node_agent_project_docs_mcp::McpRequest,
    project_document_file_operation_model::normalize_document_path,
    project_document_governance::{parse_manifest, SECTION_CONFIG_PATH},
    project_document_knowledge_graph_service::plan_context_scoped,
    project_document_native_context_projection::{relevant_memories, MemoryRetrievalScope},
    project_document_response::{compact_text, project_tool_response},
};

pub(crate) const PROFILE: &str = "context";
pub(crate) const TOOL_NAME: &str = "project_context_plan";

const SERVER_INSTRUCTIONS: &str = "Use project_context_plan only for unfamiliar, cross-file, architecture, or current-status work. It returns bounded metadata, never source bodies. The short-lived MCP session remembers delivered plan/source hashes and automatically returns a small receipt or source delta; previous_plan_id remains an explicit cross-client fallback. Clean HEAD or a complete bounded dirty-content fingerprint controls cache reuse; incomplete dirty states fail closed. Open only added or changed paths with native tools. Current files/tests decide implementation truth; binding current docs/ADRs decide accepted direction. Skip precise single-file tasks.";
const MIN_RESPONSE_TOKENS: u64 = 800;
const MAX_RESPONSE_TOKENS: u64 = 2_000;

#[derive(Debug, Deserialize)]
struct ContextPlanArguments {
    query: String,
    #[serde(default)]
    task_paths: Vec<String>,
    #[serde(default)]
    scope_id: String,
    #[serde(default)]
    release: String,
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

pub(crate) fn handle_request(
    workspace: &Path,
    request: &McpRequest,
    receipt_path: Option<&Path>,
) -> Result<Value> {
    match request.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": "2025-03-26",
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "yilong-project-context", "version": "1.4.0" },
            "instructions": SERVER_INSTRUCTIONS,
        })),
        "tools/list" => Ok(json!({ "tools": [definition()] })),
        "tools/call" => call_tool(workspace, request.params.clone(), receipt_path),
        "ping" => Ok(json!({})),
        _ => bail!("轻量项目上下文 MCP 不支持 method: {}", request.method),
    }
}

pub(crate) fn definition() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "为陌生项目、跨文件、架构或当前状态任务返回严格限额的 Git/revision、权威性、图谱入口、实现引用和最多 3 条证据 hash 有效的已审核导航记忆。零正文、只读；同一短期会话自动复用 plan/source hash，只返回未变化或增量回执。精确单文件任务不要调用。",
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
                "task_paths": {
                    "type": "array",
                    "maxItems": 16,
                    "items": {"type":"string","maxLength":500},
                    "description": "可选的工作区相对任务路径；用于严格排除其他模块的 path-scoped 记忆。"
                },
                "scope_id": {
                    "type": "string",
                    "maxLength": 80,
                    "description": "可选 knowledge federation scope_id；大型仓库只规划该分片。"
                },
                "release": {
                    "type": "string",
                    "maxLength": 120,
                    "description": "可选 release/channel 标识；只选择显式匹配的 release-scoped 记忆。"
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
                    "description": "显式复用上一响应 plan_receipt.plan_id；同一 MCP 会话也会自动记忆，跨客户端恢复时可传。"
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

pub(crate) fn try_call(
    workspace: &Path,
    name: &str,
    arguments: Value,
    receipt_path: Option<&Path>,
) -> Result<Option<Value>> {
    if name != TOOL_NAME {
        return Ok(None);
    }
    let input: ContextPlanArguments = serde_json::from_value(arguments)?;
    Ok(Some(build_plan(workspace, input, receipt_path)?))
}

fn call_tool(workspace: &Path, params: Value, receipt_path: Option<&Path>) -> Result<Value> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("tools/call 缺少 name"))?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let value = try_call(workspace, name, arguments.clone(), receipt_path)?
        .ok_or_else(|| anyhow::anyhow!("轻量项目上下文 MCP 只开放 {TOOL_NAME}"))?;
    let value = project_tool_response(name, &arguments, value)?;
    let mut response = json!({
        "content": [{ "type": "text", "text": compact_text(name, &value)? }],
        "structuredContent": value,
        "isError": false,
    });
    session::attach_tool_result_size(&mut response)?;
    Ok(response)
}

fn build_plan(
    workspace: &Path,
    input: ContextPlanArguments,
    receipt_path: Option<&Path>,
) -> Result<Value> {
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
    let task_paths = normalize_task_paths(input.task_paths)?;
    let scope_id = bounded_scope_value(&input.scope_id, 80, "scope_id")?;
    let release = bounded_scope_value(&input.release, 120, "release")?;
    let revision = inspect_workspace(workspace);
    let retrieval_scope = MemoryRetrievalScope {
        task_paths,
        scope_id,
        git_branch: revision.git_branch.clone().unwrap_or_default(),
        release,
        worktree_clean: revision.git_clean,
    };
    let task_scope_key = serde_json::to_string(&retrieval_scope)?;
    let cache_key = request_cache_key(
        workspace,
        &revision,
        query,
        max_tokens,
        max_documents,
        max_response_tokens,
        &task_scope_key,
    );

    let cached = (!input.force_refresh)
        .then(|| cache_key.as_deref().and_then(lookup))
        .flatten();
    let (plan, cache_status, cache_age_ms) = if let Some(hit) = cached {
        (hit.plan, "hit", Some(hit.age_ms))
    } else {
        let plan = build_fresh_plan(
            workspace,
            query,
            max_tokens,
            max_documents,
            max_response_tokens,
            &revision,
            &retrieval_scope,
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
    session::finish(
        plan,
        session::DeliveryInput {
            receipt_path,
            query,
            task_scope_key: &task_scope_key,
            max_tokens,
            max_documents,
            max_response_tokens,
            previous_plan_id,
            force_refresh: input.force_refresh,
            cache_status,
            cache_age_ms,
            planning_ms: elapsed_ms(started),
            revision: &revision,
        },
    )
}

fn build_fresh_plan(
    workspace: &Path,
    query: &str,
    max_tokens: u64,
    max_documents: usize,
    max_response_tokens: u64,
    revision: &WorkspaceRevision,
    retrieval_scope: &MemoryRetrievalScope,
    cacheable: bool,
) -> Result<Value> {
    let scope_id =
        (!retrieval_scope.scope_id.is_empty()).then_some(retrieval_scope.scope_id.as_str());
    let raw = plan_context_scoped(
        workspace,
        query,
        None,
        scope_id,
        max_tokens,
        max_documents,
        1_600,
    )?;
    let mut plan = project_navigation_plan(&raw);
    plan["verified_project_memory"] = portable_memory_projection(workspace, query, retrieval_scope);
    let catalog_revision = plan.get("catalog_revision").cloned().unwrap_or(Value::Null);
    plan["contract"] = json!({
        "schema": "elon.project_context_plan.v6",
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
    plan["task_scope"] = serde_json::to_value(retrieval_scope)?;
    plan["source_policy"] = json!({
        "precedence": [
            {"rank":1,"role":"implementation_truth","sources":["current_files","tests","runtime_evidence"]},
            {"rank":2,"role":"accepted_direction","sources":["binding_rules","current_status","accepted_decisions"]},
            {"rank":3,"role":"navigation_only","sources":["verified_project_memory","knowledge_graph","indexes","generated_summaries"]}
        ],
        "default_excluded": ["drafts","discussions","history","archives","traces"],
        "conflict_action": "Report drift and verify both current source and binding direction; never silently choose an index or summary."
    });
    plan["source_conflict_summary"] = source_conflict_summary(&plan, revision);
    plan["native_tool_handoff"] = json!({
        "next": "Open only selected paths and implementation_refs with Codex native search/read.",
        "before_edit": "Verify exact current source/tests; this response is navigation metadata.",
        "stop": "Stop expanding once one verified source resolves the task.",
        "memory_rule": "Reusable memory may narrow where to look, but never replaces current native reads before an edit.",
        "durable_handoff": "Submit stable verified navigation facts only in a later full-governance session; never copy tool output or source bodies."
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
        "verified_project_memory": plan.get("verified_project_memory"),
        "selected_paths": plan.get("selected_paths"),
        "task_scope": plan.get("task_scope"),
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

fn portable_memory_projection(
    workspace: &Path,
    query: &str,
    retrieval_scope: &MemoryRetrievalScope,
) -> Value {
    let path = workspace.join(SECTION_CONFIG_PATH);
    let Some(content) = fs::read_to_string(path).ok() else {
        return relevant_memories(workspace, query, retrieval_scope, &[], 3);
    };
    let Ok(manifest) = parse_manifest(Some(&content)) else {
        return json!({
            "schema":"elon.project_context_memory.v2",
            "status":"manifest_invalid",
            "selected":[],
            "selected_count":0,
            "source_bodies_returned":0,
            "action":"Run full project document governance analysis before trusting portable memory."
        });
    };
    relevant_memories(
        workspace,
        query,
        retrieval_scope,
        &manifest.context_memories,
        3,
    )
}

fn normalize_task_paths(paths: Vec<String>) -> Result<Vec<String>> {
    if paths.len() > 16 {
        bail!("project_context_plan.task_paths 最多 16 条");
    }
    let mut paths = paths
        .into_iter()
        .map(|path| normalize_document_path(&path))
        .collect::<Result<Vec<_>>>()?;
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn bounded_scope_value(value: &str, limit: usize, field: &str) -> Result<String> {
    let value = value.trim();
    if value.chars().count() > limit || value.chars().any(char::is_control) {
        bail!("project_context_plan.{field} 最多 {limit} 个字符且不能包含控制字符");
    }
    Ok(value.to_string())
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
    use crate::{
        node_agent_project_docs_mcp::{
            descriptor_for_project_context, descriptor_for_project_receipt, test_transport_routes,
            McpRequest,
        },
        node_agent_project_docs_mcp_native_context_tools::RECEIPT_TOOL,
    };
    use serde_json::json;
    use std::{fs, path::Path};

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
        let response = handle_request(Path::new("."), &request, None).unwrap();
        assert_eq!(response["tools"].as_array().unwrap().len(), 1);
        assert_eq!(response["tools"][0]["name"], TOOL_NAME);
    }

    #[tokio::test]
    async fn streamable_transport_keeps_minimal_profiles_fixed_and_separate() {
        let root = std::env::temp_dir().join(format!(
            "elon_project_memory_profile_{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(root.join(".git")).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            axum::serve(listener, test_transport_routes())
                .await
                .unwrap();
        });
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let context = descriptor_for_project_context(root.to_str().unwrap(), port).unwrap();
        let receipt = descriptor_for_project_receipt(root.to_str().unwrap(), port).unwrap();

        for (descriptor, expected_tool) in [(&context, TOOL_NAME), (&receipt, RECEIPT_TOOL)] {
            let response = client
                .post(descriptor["url"].as_str().unwrap())
                .json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}))
                .send()
                .await
                .unwrap();
            assert!(response.status().is_success());
            let body: serde_json::Value = response.json().await.unwrap();
            let tools = body["result"]["tools"].as_array().unwrap();
            assert_eq!(tools.len(), 1);
            assert_eq!(tools[0]["name"], expected_tool);
        }

        let switched_url = context["url"]
            .as_str()
            .unwrap()
            .replace("profile=context", "profile=receipt");
        let switched = client
            .post(switched_url)
            .json(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}))
            .send()
            .await
            .unwrap();
        assert_eq!(switched.status(), reqwest::StatusCode::UNAUTHORIZED);

        server.abort();
        for descriptor in [&context, &receipt] {
            let session_id = descriptor["sessionId"].as_str().unwrap();
            fs::remove_dir_all(
                std::env::temp_dir()
                    .join("elon-project-docs-mcp")
                    .join(session_id),
            )
            .unwrap();
        }
        fs::remove_dir_all(root).unwrap();
    }
}

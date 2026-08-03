//! Strict metadata-only projection for the lightweight project context MCP.

use serde_json::{json, Value};
use std::collections::HashSet;

use crate::node_agent_project_context_cache::WorkspaceRevision;

pub(crate) fn project_navigation_plan(raw: &Value) -> Value {
    json!({
        "catalog_revision": raw.get("catalog_revision"),
        "identity": compact_identity(raw.get("identity").unwrap_or(&Value::Null)),
        "query": raw.get("query"),
        "matched_nodes": raw.get("matched_nodes").and_then(Value::as_array)
            .into_iter().flatten().take(6).map(compact_node).collect::<Vec<_>>(),
        "mandatory_rules": raw.get("mandatory_rules").and_then(Value::as_array)
            .into_iter().flatten().take(3).map(compact_rule).collect::<Vec<_>>(),
        "relevant_documents": raw.get("relevant_documents").and_then(Value::as_array)
            .into_iter().flatten().take(8).map(compact_relevant_document).collect::<Vec<_>>(),
        "budget": raw.get("budget"),
    })
}

fn compact_identity(identity: &Value) -> Value {
    json!({
        "canonical_workspace": identity.get("canonical_workspace"),
        "manifest_revision": identity.get("manifest_revision"),
        "knowledge_map_revision": identity.get("knowledge_map_revision"),
        "source": identity.get("source"),
    })
}

fn compact_node(node: &Value) -> Value {
    json!({
        "id": node.get("id"),
        "view": node.get("view"),
        "label": node.get("label"),
        "status": node.get("status"),
        "score": node.get("score"),
        "entrypoint": node.get("entrypoint"),
        "document_paths": bounded_strings(node.get("document_paths"), 4),
        "implementation_refs": bounded_strings(node.get("implementation_refs"), 4),
    })
}

fn compact_rule(rule: &Value) -> Value {
    json!({
        "reason": rule.get("reason"),
        "document": compact_document(rule.get("document").unwrap_or(&Value::Null)),
    })
}

fn compact_relevant_document(candidate: &Value) -> Value {
    json!({
        "score": candidate.get("score"),
        "reason": candidate.get("reason"),
        "read_plan": candidate.get("read_plan"),
        "document": compact_document(candidate.get("document").unwrap_or(&Value::Null)),
    })
}

fn compact_document(document: &Value) -> Value {
    json!({
        "path": document.get("path"),
        "title": document.get("title"),
        "role": document.get("role"),
        "lifecycle": document.get("lifecycle"),
        "authority": document.get("authority"),
        "default_retrieval": document.get("default_retrieval"),
        "ambiguous": document.get("ambiguous"),
        "token_estimate": document.get("token_estimate"),
        "content_hash": document.get("content_hash"),
        "headings": bounded_strings(document.get("headings"), 4),
        "section": document.get("section"),
        "version_status": document.get("version_status"),
    })
}

fn bounded_strings(value: Option<&Value>, limit: usize) -> Vec<Value> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| item.is_string())
        .take(limit)
        .cloned()
        .collect()
}

pub(crate) fn source_conflict_summary(plan: &Value, revision: &WorkspaceRevision) -> Value {
    let mut warnings = Vec::new();
    let mut seen = HashSet::new();
    for collection in ["mandatory_rules", "relevant_documents"] {
        for item in plan
            .get(collection)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(document) = item.get("document") else {
                continue;
            };
            let path = document.get("path").and_then(Value::as_str).unwrap_or("");
            if let Some((code, action)) = document_warning(document) {
                let identity = format!("{path}:{code}");
                if seen.insert(identity) && warnings.len() < 6 {
                    warnings.push(json!({"path":path,"code":code,"action":action}));
                }
            }
        }
    }
    if revision.git_clean == Some(false) {
        let (code, action) = if revision.worktree_fingerprint.is_some() {
            (
                "dirty_workspace_fingerprinted",
                "Cache is bound to changed-file content hashes; still verify local edits with native tools.",
            )
        } else {
            (
                "dirty_workspace_uncacheable",
                "Fingerprint is incomplete, so cache reuse is disabled; verify local edits with native tools.",
            )
        };
        warnings.push(json!({
            "path": Value::Null,
            "code": code,
            "action": action,
        }));
    }
    let warning_count = warnings.len();
    let requires_native_resolution = !warnings.is_empty();
    json!({
        "metadata_warning_count": warning_count,
        "metadata_warnings": warnings,
        "semantic_content_compared": false,
        "requires_native_resolution": requires_native_resolution,
    })
}

fn document_warning(document: &Value) -> Option<(&'static str, &'static str)> {
    if document.get("ambiguous").and_then(Value::as_bool) == Some(true) {
        return Some((
            "ambiguous_metadata",
            "Verify authority and current status before use.",
        ));
    }
    let lifecycle = document
        .get("lifecycle")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let version_status = document
        .get("version_status")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if matches!(
        lifecycle,
        "draft" | "deprecated" | "superseded" | "archived" | "source_material"
    ) || matches!(
        version_status,
        "draft" | "deprecated" | "superseded" | "archived"
    ) {
        return Some((
            "non_current_source",
            "Use only as history or evidence; do not treat as current fact.",
        ));
    }
    let authority = document
        .get("authority")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if matches!(
        authority,
        "proposal" | "non_authoritative" | "none" | "unknown"
    ) {
        return Some((
            "non_authoritative_source",
            "Use for navigation only and verify a current authority/source.",
        ));
    }
    None
}

pub(crate) fn enforce_response_projection(mut plan: Value, max_response_tokens: u64) -> Value {
    let target_bytes = max_response_tokens.saturating_mul(4).saturating_sub(900) as usize;
    let mut removed = 0usize;
    while serialized_len(&plan) > target_bytes {
        if pop_array(&mut plan, "relevant_documents", 1)
            || pop_array(&mut plan, "matched_nodes", 2)
            || pop_nested_array(&mut plan, "source_conflict_summary", "metadata_warnings", 1)
            || pop_array(&mut plan, "mandatory_rules", 1)
        {
            removed += 1;
            continue;
        }
        plan = compact_budget_fallback(&plan);
        break;
    }
    plan["response_projection"] = json!({
        "max_estimated_tokens": max_response_tokens,
        "items_omitted": removed,
        "truncated": removed > 0 || plan.get("selected_paths").is_some(),
    });
    plan
}

fn compact_budget_fallback(plan: &Value) -> Value {
    let selected_paths = plan
        .get("relevant_documents")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.pointer("/document/path").cloned())
        .take(4)
        .collect::<Vec<_>>();
    let entrypoints = plan
        .get("matched_nodes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("entrypoint").cloned())
        .filter(|item| item.as_str().is_some_and(|value| !value.is_empty()))
        .take(4)
        .collect::<Vec<_>>();
    json!({
        "status": "bounded_summary",
        "catalog_revision": plan.get("catalog_revision"),
        "query": plan.get("query"),
        "contract": plan.get("contract"),
        "workspace_revision": plan.get("workspace_revision"),
        "source_policy": plan.get("source_policy"),
        "selected_paths": selected_paths,
        "matched_entrypoints": entrypoints,
        "source_conflict_summary": {
            "metadata_warning_count": plan.pointer("/source_conflict_summary/metadata_warning_count"),
            "semantic_content_compared": false,
            "details_omitted": true,
        },
        "native_tool_handoff": plan.get("native_tool_handoff"),
    })
}

fn serialized_len(value: &Value) -> usize {
    serde_json::to_vec(value).map_or(usize::MAX, |encoded| encoded.len())
}

fn pop_array(value: &mut Value, key: &str, minimum: usize) -> bool {
    let Some(items) = value.get_mut(key).and_then(Value::as_array_mut) else {
        return false;
    };
    if items.len() <= minimum {
        return false;
    }
    items.pop();
    true
}

fn pop_nested_array(value: &mut Value, parent: &str, key: &str, minimum: usize) -> bool {
    let Some(items) = value
        .get_mut(parent)
        .and_then(|parent| parent.get_mut(key))
        .and_then(Value::as_array_mut)
    else {
        return false;
    };
    if items.len() <= minimum {
        return false;
    }
    items.pop();
    true
}

pub(crate) fn not_modified_response(
    plan: &Value,
    cache_status: &str,
    cache_age_ms: Option<u64>,
    max_response_tokens: u64,
    planning_ms: u64,
) -> Value {
    let estimated_full_plan_tokens = plan
        .pointer("/plan_receipt/estimated_full_plan_tokens")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    let mut response = json!({
        "status": "not_modified",
        "contract": plan.get("contract"),
        "plan_receipt": plan.get("plan_receipt"),
        "workspace_revision": plan.get("workspace_revision"),
        "source_conflict_summary": {
            "metadata_warning_count": plan.pointer("/source_conflict_summary/metadata_warning_count"),
            "requires_native_resolution": plan.pointer("/source_conflict_summary/requires_native_resolution"),
        },
        "cache": {
            "status": cache_status,
            "age_ms": cache_age_ms,
            "ttl_seconds": 300,
        },
        "response_projection": {"max_estimated_tokens":max_response_tokens,"delta_only":true},
        "instruction": "Reuse the prior plan; do not repeat project-wide search. Re-plan only when Git/catalog revision or task scope changes."
    });
    for _ in 0..4 {
        let estimated_delta_tokens =
            serde_json::to_vec(&response).map_or(0, |encoded| encoded.len().div_ceil(4)) as u64;
        let performance = json!({
            "planning_ms": planning_ms,
            "cache_reused": cache_status == "hit",
            "estimated_full_plan_tokens": estimated_full_plan_tokens,
            "estimated_delta_tokens_before_response_budget": estimated_delta_tokens,
            "estimated_tokens_avoided": estimated_full_plan_tokens.saturating_sub(estimated_delta_tokens),
        });
        if response.get("performance_receipt") == Some(&performance) {
            break;
        }
        response["performance_receipt"] = performance;
    }
    response
}

#[cfg(test)]
mod tests {
    use super::{enforce_response_projection, not_modified_response, project_navigation_plan};
    use serde_json::json;

    #[test]
    fn projection_drops_unbounded_document_metadata() {
        let raw = json!({
            "relevant_documents":[{"document":{
                "path":"AI_CURRENT.md","title":"Current","reason":"long reason",
                "headings":["a","b","c","d","e"],"content_hash":"abc"
            }}]
        });
        let projected = project_navigation_plan(&raw);
        let document = &projected["relevant_documents"][0]["document"];
        assert!(document.get("reason").is_none());
        assert_eq!(document["headings"].as_array().unwrap().len(), 4);
    }

    #[test]
    fn response_projection_omits_items_to_fit_budget() {
        let large = json!({
            "relevant_documents": (0..50).map(|index| json!({"document":{"path":format!("docs/{index}.md"),"title":"x".repeat(200)}})).collect::<Vec<_>>(),
            "matched_nodes": [], "mandatory_rules": []
        });
        let bounded = enforce_response_projection(large, 800);
        assert!(serde_json::to_vec(&bounded).unwrap().len() < 3_200);
    }

    #[test]
    fn unchanged_receipt_reports_estimated_token_savings() {
        let plan = json!({
            "plan_receipt":{"plan_id":"ctx_1","estimated_full_plan_tokens":1200},
            "workspace_revision":{"git_head":"abc"},
            "contract":{"source_bodies_returned":0},
        });
        let receipt = not_modified_response(&plan, "hit", Some(10), 1200, 5);
        assert_eq!(receipt["performance_receipt"]["planning_ms"], 5);
        assert!(
            receipt["performance_receipt"]["estimated_tokens_avoided"]
                .as_u64()
                .unwrap()
                > 0
        );
    }
}

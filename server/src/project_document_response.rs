//! Bounded MCP projections and honest serialized response budgets.

use anyhow::{bail, Result};
use serde_json::{json, Map, Value};

const DEFAULT_PAGE_LIMIT: usize = 80;
const MAX_PAGE_LIMIT: usize = 200;

#[derive(Debug, Clone)]
pub(crate) struct ProjectionRequest {
    pub projection: String,
    pub detail: String,
    pub topic: String,
    pub offset: usize,
    pub limit: usize,
}

impl ProjectionRequest {
    pub(crate) fn from_arguments(arguments: &Value) -> Result<Self> {
        let projection = text(arguments, "projection", "page");
        if !matches!(projection.as_str(), "summary" | "page" | "detail" | "full") {
            bail!("projection 只支持 summary、page、detail 或 full");
        }
        let explicit_offset = number(arguments, "offset", 0);
        let offset = arguments
            .get("cursor")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(decode_cursor)
            .transpose()?
            .unwrap_or(explicit_offset);
        Ok(Self {
            projection,
            detail: text(arguments, "detail", ""),
            topic: text(arguments, "topic", ""),
            offset,
            limit: number(arguments, "limit", DEFAULT_PAGE_LIMIT).clamp(1, MAX_PAGE_LIMIT),
        })
    }

    pub(crate) fn is_full(&self) -> bool {
        self.projection == "full" || self.detail == "all"
    }
}

pub(crate) fn project_tool_response(
    tool: &str,
    arguments: &Value,
    mut value: Value,
) -> Result<Value> {
    let mut request = ProjectionRequest::from_arguments(arguments)?;
    if tool == "project_docs_get_health" && arguments.get("projection").is_none() {
        request.projection = "summary".to_string();
    }
    value = match tool {
        "project_docs_analyze" => project_analysis(value, &request),
        "project_docs_get_map" => project_map(value, &request),
        "project_docs_plan_context" => project_context(value, &request),
        "project_docs_get_health" => project_health(value, &request),
        "project_docs_get_issues" => project_issues(value, &request),
        "project_docs_get_suggestions" => project_suggestions(value, &request),
        "project_discussions_get_suggestions" => discussion_suggestions(value, &request),
        "project_docs_save_suggestions"
        | "project_docs_apply_suggestions"
        | "project_docs_apply_file_operations"
        | "project_discussions_save_proposal"
        | "project_discussions_apply" => project_mutation(value, &request),
        _ => value,
    };
    attach_response_budget(&mut value)?;
    Ok(value)
}

fn project_issues(mut value: Value, request: &ProjectionRequest) -> Value {
    if request.projection == "summary" {
        value["issues"] = json!([]);
        if let Some(page) = value.get_mut("pagination") {
            page["returned"] = json!(0);
        }
        value["returned"] = json!(0);
    }
    value["projection"] = json!({"mode":request.projection,"topic":request.topic});
    value
}

pub(crate) fn compact_text(tool: &str, value: &Value) -> Result<String> {
    let mut summary = Map::new();
    summary.insert("tool".to_string(), json!(tool));
    for key in [
        "status",
        "catalog_revision",
        "manifest_revision",
        "suggestions_revision",
        "graph_revision",
        "counts",
        "promoted_documents",
        "pagination",
        "response_budget",
        "git_baseline_commit",
        "git_result_commit",
        "git_document_transaction_complete",
    ] {
        if let Some(item) = value.get(key) {
            summary.insert(key.to_string(), item.clone());
        }
    }
    summary.insert(
        "instruction".to_string(),
        json!("Use structuredContent for the requested bounded projection; request detail/full explicitly before expanding a collection."),
    );
    Ok(serde_json::to_string(&Value::Object(summary))?)
}

pub(crate) fn pagination(offset: usize, limit: usize, total: usize, returned: usize) -> Value {
    let next = offset.saturating_add(returned);
    json!({
        "offset": offset,
        "limit": limit,
        "returned": returned,
        "total_matching": total,
        "has_more": next < total,
        "next_offset": (next < total).then_some(next),
        "next_cursor": (next < total).then(|| encode_cursor(next)),
    })
}

fn project_analysis(mut value: Value, request: &ProjectionRequest) -> Value {
    if request.is_full() {
        return value;
    }
    let include_documents = request.projection != "summary";
    if !include_documents {
        value["documents"] = json!([]);
        if let Some(page) = value.get_mut("pagination") {
            page["returned"] = json!(0);
        }
    }
    if request.detail != "health" {
        value["document_health"] = health_summary(&value["document_health"]);
    }
    if request.detail != "manifest" {
        value["manifest"] = manifest_summary(&value["manifest"]);
    }
    if request.detail != "suggestions" {
        value["suggestions"] = suggestion_summary(&value["suggestions"]);
    }
    let collections_expanded = [
        include_documents.then_some("documents"),
        (request.detail == "health").then_some("document_health"),
        (request.detail == "manifest").then_some("manifest"),
        (request.detail == "suggestions").then_some("suggestions"),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    value["projection"] = json!({
        "mode": request.projection,
        "detail": request.detail,
        "topic": request.topic,
        "collections_expanded": collections_expanded,
    });
    value
}

fn project_map(mut value: Value, request: &ProjectionRequest) -> Value {
    if request.is_full() {
        return value;
    }
    if request.projection == "summary" {
        value.as_object_mut().map(|object| {
            object.remove("nodes");
            object.remove("edges");
        });
        if let Some(diagnostics) = value.get_mut("diagnostics") {
            diagnostics["findings"] = json!([]);
        }
        value["projection"] = json!({"mode":"summary"});
        return value;
    }
    let Some(nodes) = value.get("nodes").and_then(Value::as_array).cloned() else {
        value["projection"] = json!({"mode":request.projection});
        return value;
    };
    let total = nodes.len();
    let page = nodes
        .into_iter()
        .skip(request.offset)
        .take(request.limit)
        .collect::<Vec<_>>();
    let visible = page
        .iter()
        .filter_map(|node| node.get("id").and_then(Value::as_str))
        .collect::<std::collections::HashSet<_>>();
    let edges = value
        .get("edges")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|edge| {
            edge.get("source")
                .and_then(Value::as_str)
                .is_some_and(|id| visible.contains(id))
                && edge
                    .get("target")
                    .and_then(Value::as_str)
                    .is_some_and(|id| visible.contains(id))
        })
        .cloned()
        .collect::<Vec<_>>();
    value["nodes"] = json!(page);
    value["edges"] = json!(edges);
    value["pagination"] = pagination(
        request.offset,
        request.limit,
        total,
        value["nodes"].as_array().map_or(0, Vec::len),
    );
    value["projection"] = json!({"mode":request.projection,"detail":request.detail});
    value
}

fn project_context(mut value: Value, request: &ProjectionRequest) -> Value {
    if request.is_full() {
        return value;
    }
    if request.projection == "summary" {
        value["mandatory_rules"] = json!([]);
        value["relevant_documents"] = json!([]);
        value["matched_nodes"] = json!([]);
    } else if request.offset > 0 || value["relevant_documents"].as_array().is_some() {
        page_array(&mut value, "relevant_documents", request);
    }
    value["projection"] = json!({"mode":request.projection,"detail":request.detail});
    value
}

fn project_health(value: Value, request: &ProjectionRequest) -> Value {
    if request.is_full() || request.detail == "all" {
        return value;
    }
    if request.detail == "issues" {
        let issues = value
            .pointer("/governance_workflow/issues")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let total = value
            .pointer("/governance_workflow/total_issues")
            .and_then(Value::as_u64)
            .map(|item| item as usize)
            .unwrap_or(issues.len());
        let already_paged = value
            .pointer("/issues_page/offset")
            .and_then(Value::as_u64)
            .is_some_and(|offset| offset as usize == request.offset);
        let page = if already_paged {
            issues
        } else {
            issues
                .into_iter()
                .skip(request.offset)
                .take(request.limit)
                .collect::<Vec<_>>()
        };
        let returned = page.len();
        return json!({
            "source": value["source"], "identity": value["identity"], "overall": value["overall"],
            "quality_summary": value.pointer("/quality/summary"),
            "workflow_summary": value.pointer("/governance_workflow/summary"),
            "score_explanation": value.pointer("/governance_workflow/score_explanation"),
            "issues": page,
            "pagination": pagination(request.offset, request.limit, total, returned),
            "projection": {"mode":request.projection,"detail":"issues","topic":request.topic},
        });
    }
    let mut summary = health_summary(&value);
    summary["projection"] = json!({"mode":request.projection,"detail":request.detail});
    summary
}

fn project_suggestions(mut value: Value, request: &ProjectionRequest) -> Value {
    if request.is_full() {
        return value;
    }
    let Some(suggestions) = value.get("suggestions").cloned() else {
        return value;
    };
    if suggestions.is_null() {
        return value;
    }
    if request.projection == "summary" {
        value["suggestions"] = suggestion_summary(&suggestions);
        return value;
    }
    let detail = if request.detail.is_empty() {
        "assignments"
    } else {
        request.detail.as_str()
    };
    let mut projected = suggestion_summary(&suggestions);
    let key = match detail {
        "sections" => "proposed_sections",
        "operations" => "file_operations",
        "section_operations" => "section_operations",
        "conflicts" => "conflicts",
        "graph" => "proposed_knowledge_graph",
        _ => "assignments",
    };
    if key == "proposed_knowledge_graph" {
        projected[key] = suggestions[key].clone();
    } else {
        let collection = suggestions[key].as_array().cloned().unwrap_or_default();
        let page = collection
            .iter()
            .skip(request.offset)
            .take(request.limit)
            .cloned()
            .collect::<Vec<_>>();
        projected[key] = json!(page);
        projected["pagination"] = pagination(
            request.offset,
            request.limit,
            collection.len(),
            projected[key].as_array().map_or(0, Vec::len),
        );
    }
    projected["projection"] = json!({"mode":request.projection,"detail":detail});
    value["suggestions"] = projected;
    value
}

fn project_mutation(mut value: Value, request: &ProjectionRequest) -> Value {
    if request.is_full() {
        return value;
    }
    if value.get("manifest").is_some() {
        value["manifest"] = manifest_summary(&value["manifest"]);
    }
    if value.get("suggestions").is_some() {
        value["suggestions"] = suggestion_summary(&value["suggestions"]);
    }
    value["projection"] =
        json!({"mode":"summary","reason":"mutation responses default to receipts and revisions"});
    value
}

fn discussion_suggestions(mut value: Value, request: &ProjectionRequest) -> Value {
    if request.is_full() {
        return value;
    }
    let summarized = {
        let suggestions = &value["suggestions"];
        if suggestions.is_null() {
            Value::Null
        } else {
            json!({
                "version": suggestions["version"],
                "status": suggestions["status"],
                "summary": suggestions["summary"],
                "documents_read": suggestions["documents_read"],
                "estimated_tokens_used": suggestions["estimated_tokens_used"],
                "counts": value["counts"],
            })
        }
    };
    value["suggestions"] = summarized;
    value["projection"] =
        json!({"mode":"summary","reason":"discussion suggestions default to receipts and counts"});
    value
}

fn health_summary(value: &Value) -> Value {
    json!({
        "version": value["version"], "source": value["source"], "identity": value["identity"],
        "overall": value["overall"], "architecture": value["architecture"],
        "quality": {"summary": value.pointer("/quality/summary"), "total_issues": value.pointer("/quality/total_issues"), "returned_issues": 0, "issues": []},
        "governance_workflow": {
            "summary": value.pointer("/governance_workflow/summary"),
            "score_explanation": value.pointer("/governance_workflow/score_explanation"),
            "total_issues": value.pointer("/governance_workflow/total_issues"), "returned_issues": 0,
        },
        "maintenance": value["maintenance"],
        "federation": {
            "enabled": value.pointer("/federation/enabled"), "source": value.pointer("/federation/source"),
            "root_id": value.pointer("/federation/root_id"), "node_count": value.pointer("/federation/node_count"),
            "aggregated_score": value.pointer("/federation/aggregated_score"),
            "unhealthy_nodes": value.pointer("/federation/unhealthy_nodes"), "max_depth": value.pointer("/federation/max_depth"),
            "nodes_returned": 0,
        },
    })
}

fn manifest_summary(value: &Value) -> Value {
    json!({
        "version": value["version"], "profile": value["profile"], "home": value["home"],
        "counts": {
            "sections": array_len(value, "sections"), "assignments": object_len(value, "assignments"),
            "secondary_assignments": object_len(value, "secondary_assignments"),
            "governance_facets": object_len(value, "governance_facets"),
            "document_metadata": object_len(value, "document_metadata"),
            "knowledge_graph_nodes": value.pointer("/knowledge_graph/nodes").and_then(Value::as_array).map_or(0, Vec::len),
            "knowledge_graph_edges": value.pointer("/knowledge_graph/edges").and_then(Value::as_array).map_or(0, Vec::len),
        }
    })
}

fn suggestion_summary(value: &Value) -> Value {
    if value.is_null() {
        return Value::Null;
    }
    json!({
        "version": value["version"], "status": value["status"], "summary": value["summary"],
        "documents_read": value["documents_read"], "estimated_tokens_used": value["estimated_tokens_used"],
        "counts": {
            "proposed_sections": array_len(value, "proposed_sections"), "assignments": array_len(value, "assignments"),
            "section_operations": array_len(value, "section_operations"), "file_operations": array_len(value, "file_operations"),
            "conflicts": array_len(value, "conflicts"), "document_metadata": object_len(value, "document_metadata"),
            "knowledge_graph_nodes": value.pointer("/proposed_knowledge_graph/nodes").and_then(Value::as_array).map_or(0, Vec::len),
        }
    })
}

fn page_array(value: &mut Value, key: &str, request: &ProjectionRequest) {
    let collection = value[key].as_array().cloned().unwrap_or_default();
    let page = collection
        .iter()
        .skip(request.offset)
        .take(request.limit)
        .cloned()
        .collect::<Vec<_>>();
    value[key] = json!(page);
    value["pagination"] = pagination(
        request.offset,
        request.limit,
        collection.len(),
        value[key].as_array().map_or(0, Vec::len),
    );
}

fn attach_response_budget(value: &mut Value) -> Result<()> {
    for _ in 0..8 {
        let bytes = serde_json::to_vec(value)?.len();
        let estimated_tokens = bytes.div_ceil(4);
        let budget = json!({
            "serialized_bytes": bytes,
            "estimated_tokens": estimated_tokens,
            "token_estimator": "utf8_bytes_div_4_ceil",
            "encoding": "utf-8",
        });
        if value.get("response_budget") == Some(&budget) {
            break;
        }
        value["response_budget"] = budget;
    }
    Ok(())
}

fn encode_cursor(offset: usize) -> String {
    format!("offset:{offset}")
}

fn decode_cursor(value: &str) -> Result<usize> {
    value
        .trim()
        .strip_prefix("offset:")
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| anyhow::anyhow!("cursor 无效；请使用响应返回的 next_cursor"))
}

fn text(value: &Value, key: &str, default: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or(default)
        .trim()
        .to_ascii_lowercase()
}

fn number(value: &Value, key: &str, default: usize) -> usize {
    value
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(default)
}

fn array_len(value: &Value, key: &str) -> usize {
    value.get(key).and_then(Value::as_array).map_or(0, Vec::len)
}

fn object_len(value: &Value, key: &str) -> usize {
    value
        .get(key)
        .and_then(Value::as_object)
        .map_or(0, Map::len)
}

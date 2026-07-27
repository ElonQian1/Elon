//! MCP tools for deterministic discussion-graph review and safe repair preparation.

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;

use crate::{
    project_discussion_graph_review::{
        prepare_safe_discussion_repair, review_discussion_graph, DiscussionReviewIssue,
    },
    project_document_response::{pagination, ProjectionRequest},
};

#[derive(Debug, Deserialize)]
struct ReviewArguments {
    #[serde(default)]
    severity: String,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        tool(
            "project_discussions_review_graph",
            "以确定性规则审查讨论图的来源、权威性、重复节点、未解决异议、实现证据、失效文档和演化链；不读取正文、不调用模型。",
            json!({"type":"object","properties":{
                "severity":{"type":"string","enum":["","error","warning","advice"],"default":""},
                "offset":{"type":"integer","minimum":0,"default":0},
                "limit":{"type":"integer","minimum":1,"maximum":200,"default":80},
                "cursor":{"type":"string","pattern":"^offset:[0-9]+$"},
                "projection":{"type":"string","enum":["summary","page","detail","full"],"default":"page"}
            }}),
        ),
        tool(
            "project_discussions_prepare_safe_repair",
            "只为可无歧义修正的结构问题生成 repair proposal，不直接修改图；语义取舍仍交给 AI 按来源处理。",
            json!({"type":"object","properties":{}}),
        ),
    ]
}

pub(crate) fn try_call(workspace: &Path, name: &str, arguments: Value) -> Result<Option<Value>> {
    let value = match name {
        "project_discussions_review_graph" => {
            let projection = ProjectionRequest::from_arguments(&arguments)?;
            let input: ReviewArguments = decode(arguments)?;
            review(workspace, input, projection)?
        }
        "project_discussions_prepare_safe_repair" => prepare_safe_discussion_repair(workspace)?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn review(
    workspace: &Path,
    input: ReviewArguments,
    projection: ProjectionRequest,
) -> Result<Value> {
    let report = review_discussion_graph(workspace)?;
    let severity = input.severity.trim();
    let mut issues = report
        .issues
        .into_iter()
        .filter(|issue| severity.is_empty() || issue.severity == severity)
        .collect::<Vec<_>>();
    let total = issues.len();
    let page = if projection.projection == "summary" {
        Vec::new()
    } else {
        issues
            .drain(..)
            .skip(projection.offset)
            .take(projection.limit)
            .collect::<Vec<DiscussionReviewIssue>>()
    };
    let returned = page.len();
    Ok(json!({
        "graph_revision": report.graph_revision,
        "health_score": report.health_score,
        "severity_counts": report.severity_counts,
        "safe_repair_count": report.safe_repair_count,
        "issues": page,
        "pagination": pagination(projection.offset, projection.limit, total, returned),
        "next_action": if report.safe_repair_count > 0 {
            "调用 project_discussions_prepare_safe_repair；其余问题由 AI 按来源生成 proposal。"
        } else if total > 0 {
            "由 AI 按 issue 建议只读取命中来源，生成修正 proposal。"
        } else {
            "当前确定性检查未发现问题。"
        },
        "budget": report.budget,
    }))
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({"name":name,"description":description,"inputSchema":input_schema})
}

fn decode<T: for<'de> Deserialize<'de>>(arguments: Value) -> Result<T> {
    serde_json::from_value(arguments).map_err(Into::into)
}

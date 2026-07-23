//! Optional quality and managed-vault tools kept outside the core MCP router.

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;

use crate::{
    project_document_governance_service::analyze_workspace,
    project_document_index::ProjectDocumentIndex,
    project_document_issue_workflow::{health_trend, update_issue, IssueWorkflowUpdate},
    project_document_maintenance::list_governed_issues_page,
    project_document_response::{pagination, ProjectionRequest},
    project_document_versioning::{
        document_version_diff, list_document_versions, restore_document_version,
    },
};

#[derive(Debug, Deserialize)]
struct IssueArguments {
    #[serde(default)]
    issue_types: Vec<String>,
    #[serde(default)]
    statuses: Vec<String>,
    #[serde(default)]
    severities: Vec<String>,
    #[serde(default)]
    owner: String,
    #[serde(default)]
    offset: usize,
    #[serde(default = "default_issue_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize)]
struct VersionArguments {
    #[serde(default = "default_version_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize)]
struct RestoreArguments {
    commit: String,
}

#[derive(Debug, Deserialize)]
struct DiffArguments {
    commit: String,
    #[serde(default)]
    path: String,
}

#[derive(Debug, Deserialize)]
struct UpdateIssueArguments {
    fingerprint: String,
    status: String,
    #[serde(default)]
    owner: String,
    #[serde(default)]
    due_at: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    snoozed_until: String,
}

pub(crate) fn issue_definition() -> Value {
    tool(
        "project_docs_get_issues",
        "分页读取服务端确定性质量问题及证据，不读取全部正文。支持按类型、状态、严重度和负责人筛选。",
        json!({"type":"object","properties":{
            "issue_types":{"type":"array","maxItems":12,"items":{"type":"string"}},
            "statuses":{"type":"array","maxItems":5,"items":{"type":"string","enum":["open","assigned","snoozed","ignored","resolved"]}},
            "severities":{"type":"array","maxItems":3,"items":{"type":"string","enum":["error","warning","info"]}},
            "owner":{"type":"string","maxLength":80},
            "offset":{"type":"integer","minimum":0,"default":0},
            "limit":{"type":"integer","minimum":1,"maximum":200,"default":50},
            "cursor":{"type":"string","pattern":"^offset:[0-9]+$"},
            "projection":{"type":"string","enum":["summary","page"],"default":"page"}
        }}),
    )
}

pub(crate) fn history_definitions() -> Vec<Value> {
    vec![
        tool(
            "project_docs_get_health",
            "读取可解释的文档健康总分、分类计数和分页问题证据。summary 默认不展开集合；detail=issues 分页返回失效链接、孤立、owner/review、过期、重复和实现漂移。",
            json!({"type":"object","properties":{
                "issue_types":{"type":"array","maxItems":12,"items":{"type":"string"}},
                "statuses":{"type":"array","maxItems":5,"items":{"type":"string"}},
                "severities":{"type":"array","maxItems":3,"items":{"type":"string"}},
                "owner":{"type":"string","maxLength":80},
                "topic":{"type":"string","maxLength":120},
                "offset":{"type":"integer","minimum":0,"default":0},
                "limit":{"type":"integer","minimum":1,"maximum":200,"default":50},
                "cursor":{"type":"string","pattern":"^offset:[0-9]+$"},
                "projection":{"type":"string","enum":["summary","page","detail","full"],"default":"summary"},
                "detail":{"type":"string","enum":["","issues","all"],"default":""}
            }}),
        ),
        tool(
            "project_docs_update_issue",
            "更新质量问题的负责人、期限和处理状态。忽略或延期必须提供原因，延期必须提供恢复日期。",
            json!({"type":"object","required":["fingerprint","status"],"properties":{
                "fingerprint":{"type":"string","minLength":32,"maxLength":128},
                "status":{"type":"string","enum":["open","assigned","snoozed","ignored","resolved"]},
                "owner":{"type":"string","maxLength":80},"due_at":{"type":"string","maxLength":10},
                "reason":{"type":"string","maxLength":500},"snoozed_until":{"type":"string","maxLength":10}
            }}),
        ),
        tool(
            "project_docs_get_health_history",
            "读取最近的文档健康分、问题数和可执行问题数趋势，不读取正文。",
            json!({"type":"object","properties":{"limit":{"type":"integer","minimum":1,"maximum":365,"default":30}}}),
        ),
        tool(
            "project_docs_get_history",
            "读取普通 Git 项目或平台托管知识库的文档版本历史，并标出可安全回滚的仅文档提交。",
            json!({"type":"object","properties":{"limit":{"type":"integer","minimum":1,"maximum":100,"default":20}}}),
        ),
        tool(
            "project_docs_get_version_diff",
            "读取指定文档提交的有界差异；可选 path 只查看一篇 Markdown。",
            json!({"type":"object","required":["commit"],"properties":{"commit":{"type":"string","minLength":7,"maxLength":64},"path":{"type":"string","maxLength":500}}}),
        ),
        tool(
            "project_docs_restore_version",
            "恢复托管知识库快照，或安全反向应用普通项目中的仅文档提交；始终创建新的可追溯提交。",
            json!({"type":"object","required":["commit"],"properties":{"commit":{"type":"string","minLength":7,"maxLength":64}}}),
        ),
    ]
}

pub(crate) fn try_call(workspace: &Path, name: &str, arguments: Value) -> Result<Option<Value>> {
    let projection = ProjectionRequest::from_arguments(&arguments)?;
    let value = match name {
        "project_docs_get_issues" => {
            let input: IssueArguments = decode(arguments)?;
            let (issues, total) = list_governed_issues_page(
                workspace,
                &input.issue_types,
                &input.statuses,
                &input.severities,
                &input.owner,
                projection.offset.max(input.offset),
                projection.limit.min(input.limit.max(1)),
            )?;
            let returned = issues.len();
            json!({"issues": issues, "pagination":pagination(projection.offset.max(input.offset), projection.limit.min(input.limit.max(1)), total, returned), "returned": returned, "total_matching":total})
        }
        "project_docs_get_health" => {
            let input: IssueArguments = decode(arguments)?;
            let (issues, total) = list_governed_issues_page(
                workspace,
                &input.issue_types,
                &input.statuses,
                &input.severities,
                &input.owner,
                projection.offset,
                projection.limit,
            )?;
            let mut health = analyze_workspace(workspace, 0, 1, false)?["document_health"].clone();
            health["governance_workflow"]["issues"] = json!(issues);
            health["governance_workflow"]["total_issues"] = json!(total);
            health["issues_page"] = pagination(
                projection.offset,
                projection.limit,
                total,
                health
                    .pointer("/governance_workflow/issues")
                    .and_then(Value::as_array)
                    .map_or(0, Vec::len),
            );
            health
        }
        "project_docs_update_issue" => {
            let input: UpdateIssueArguments = decode(arguments)?;
            let index = ProjectDocumentIndex::open(workspace)?;
            json!({"workflow": update_issue(&index, IssueWorkflowUpdate {
                fingerprint: input.fingerprint, status: input.status, owner: input.owner,
                due_at: input.due_at, reason: input.reason, snoozed_until: input.snoozed_until,
            })?})
        }
        "project_docs_get_health_history" => {
            let input: VersionArguments = decode(arguments)?;
            let index = ProjectDocumentIndex::open(workspace)?;
            json!({"trend": health_trend(&index, input.limit)?})
        }
        "project_docs_get_history" => {
            let input: VersionArguments = decode(arguments)?;
            json!({"versions": list_document_versions(workspace, input.limit)?})
        }
        "project_docs_get_version_diff" => {
            let input: DiffArguments = decode(arguments)?;
            document_version_diff(
                workspace,
                &input.commit,
                (!input.path.trim().is_empty()).then_some(input.path.as_str()),
            )?
        }
        "project_docs_restore_version" => {
            let input: RestoreArguments = decode(arguments)?;
            restore_document_version(workspace, &input.commit)?
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({"name":name,"description":description,"inputSchema":input_schema})
}

fn decode<T: for<'de> Deserialize<'de>>(arguments: Value) -> Result<T> {
    Ok(serde_json::from_value(arguments)?)
}

fn default_issue_limit() -> usize {
    50
}

fn default_version_limit() -> usize {
    20
}

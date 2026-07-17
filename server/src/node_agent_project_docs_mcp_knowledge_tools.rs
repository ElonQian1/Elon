//! Optional quality and managed-vault tools kept outside the core MCP router.

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;

use crate::{
    project_document_maintenance::list_issues,
    project_document_vault::{list_versions, restore_version},
};

#[derive(Debug, Deserialize)]
struct IssueArguments {
    #[serde(default)]
    issue_types: Vec<String>,
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

pub(crate) fn issue_definition() -> Value {
    tool(
        "project_docs_get_issues",
        "分页读取服务端确定性质量问题及证据，不读取全部正文。可筛选 broken_link、orphan_document、missing_owner、missing_review_date、overdue_review、implementation_conflict 等类型。",
        json!({"type":"object","properties":{
            "issue_types":{"type":"array","maxItems":12,"items":{"type":"string"}},
            "offset":{"type":"integer","minimum":0,"default":0},
            "limit":{"type":"integer","minimum":1,"maximum":200,"default":50}
        }}),
    )
}

pub(crate) fn history_definitions() -> [Value; 2] {
    [
        tool(
            "project_docs_get_history",
            "读取平台托管知识库的自动 Git 版本历史；普通项目会拒绝此调用。",
            json!({"type":"object","properties":{"limit":{"type":"integer","minimum":1,"maximum":100,"default":20}}}),
        ),
        tool(
            "project_docs_restore_version",
            "把平台托管知识库恢复到指定历史提交，并创建一个新的可追溯恢复提交。",
            json!({"type":"object","required":["commit"],"properties":{"commit":{"type":"string","minLength":7,"maxLength":64}}}),
        ),
    ]
}

pub(crate) fn try_call(workspace: &Path, name: &str, arguments: Value) -> Result<Option<Value>> {
    let value = match name {
        "project_docs_get_issues" => {
            let input: IssueArguments = decode(arguments)?;
            let issues = list_issues(workspace, &input.issue_types, input.offset, input.limit)?;
            let returned = issues.len();
            json!({"issues": issues, "offset": input.offset, "returned": returned})
        }
        "project_docs_get_history" => {
            let input: VersionArguments = decode(arguments)?;
            json!({"versions": list_versions(workspace, input.limit)?})
        }
        "project_docs_restore_version" => {
            let input: RestoreArguments = decode(arguments)?;
            json!({"commit": restore_version(workspace, &input.commit)?, "restored_from": input.commit})
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

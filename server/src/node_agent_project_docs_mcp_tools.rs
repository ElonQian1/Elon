use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;

use crate::{
    project_document_file_operations::{
        apply_reviewed_file_operations, ApplyFileOperationsRequest,
    },
    project_document_governance::DocumentOrganizationSuggestions,
    project_document_governance_service::{
        analyze_workspace, apply_saved_suggestions, default_page_size, default_read_chars,
        get_suggestions, read_documents, save_suggestions,
    },
    project_document_observability::{get_status, record_tool_failure, record_tool_success},
};

#[derive(Debug, Deserialize)]
struct AnalyzeArguments {
    #[serde(default)]
    offset: usize,
    #[serde(default = "default_page_size")]
    limit: usize,
    #[serde(default)]
    ambiguous_only: bool,
}

#[derive(Debug, Deserialize)]
struct ReadArguments {
    paths: Vec<String>,
    #[serde(default = "default_read_chars")]
    max_chars_per_document: usize,
    #[serde(default)]
    expected_catalog_revision: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SaveSuggestionsArguments {
    suggestions: DocumentOrganizationSuggestions,
    expected_catalog_revision: String,
    #[serde(default)]
    expected_suggestions_revision: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApplySuggestionsArguments {
    reviewed: bool,
    expected_catalog_revision: String,
    #[serde(default)]
    expected_manifest_revision: Option<String>,
    #[serde(default)]
    expected_suggestions_revision: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApplyFileOperationsArguments {
    reviewed: bool,
    operation_ids: Vec<String>,
    #[serde(default)]
    allow_rename: bool,
    #[serde(default)]
    allow_move: bool,
    expected_catalog_revision: String,
    #[serde(default)]
    expected_manifest_revision: Option<String>,
    #[serde(default)]
    expected_suggestions_revision: Option<String>,
}

pub(crate) fn tool_definitions() -> Vec<Value> {
    vec![
        tool(
            "project_docs_analyze",
            "零模型 token 扫描当前 Git 项目的文档路径、标题、哈希、标题层级、生命周期与权威性，并分页返回紧凑目录、虚拟分区和现有 AI 建议。文档治理任务第一步调用；不会读取正文或修改文件。",
            json!({
                "type":"object",
                "properties":{
                    "offset":{"type":"integer","minimum":0,"default":0},
                    "limit":{"type":"integer","minimum":1,"maximum":200,"default":80},
                    "ambiguous_only":{"type":"boolean","default":false}
                }
            }),
        ),
        tool(
            "project_docs_get_status",
            "读取当前项目最近一次文档整理的逐阶段状态、revision、低 token 用量、失败代码和恢复建议；不读取 Markdown，不修改项目文件。",
            json!({"type":"object","properties":{}}),
        ),
        tool(
            "project_docs_read",
            "只按需读取 analyze 目录中确有必要的 Markdown；最多 12 份、总计 48000 字符。优先读取 ambiguous 或与任务直接相关的文档，不得用它全量扫描仓库。",
            json!({
                "type":"object",
                "required":["paths"],
                "properties":{
                    "paths":{"type":"array","minItems":1,"maxItems":12,"items":{"type":"string"}},
                    "max_chars_per_document":{"type":"integer","minimum":1,"maximum":24000,"default":6000},
                    "expected_catalog_revision":{"type":"string","description":"建议传 analyze 返回的 catalog_revision，防止基于过期目录阅读。"}
                }
            }),
        ),
        tool(
            "project_docs_get_suggestions",
            "读取结构化 AI 整理建议及 revision；不读取 Markdown，不修改文件。",
            json!({"type":"object","properties":{}}),
        ),
        tool(
            "project_docs_save_suggestions",
            "保存 AI 根据紧凑目录形成的结构化建议。只能写 .elon/document-organization-suggestions.json，不能移动、删除、改写 Markdown，也不能直接更新分区配置。",
            json!({
                "type":"object",
                "required":["suggestions","expected_catalog_revision"],
                "properties":{
                    "expected_catalog_revision":{"type":"string","minLength":1},
                    "expected_suggestions_revision":{"type":"string"},
                    "suggestions":suggestions_schema()
                }
            }),
        ),
        tool(
            "project_docs_apply_suggestions",
            "仅在用户或审核流程明确确认后，把 ready 建议应用为虚拟分区配置并把建议标记 applied。必须 reviewed=true 且 revisions 一致；不会移动或改写 Markdown。",
            json!({
                "type":"object",
                "required":["reviewed","expected_catalog_revision"],
                "properties":{
                    "reviewed":{"type":"boolean","const":true},
                    "expected_catalog_revision":{"type":"string","minLength":1},
                    "expected_manifest_revision":{"type":"string"},
                    "expected_suggestions_revision":{"type":"string"}
                }
            }),
        ),
        tool(
            "project_docs_apply_file_operations",
            "仅在用户逐项审核后，对选中的 Markdown 执行重命名或移动。必须 reviewed=true，并分别授予 rename/move 权限；校验目录、建议和源文件 revision；禁止覆盖、删除、越界、改写正文或自动提交 Git。",
            json!({
                "type":"object",
                "required":["reviewed","operation_ids","expected_catalog_revision"],
                "properties":{
                    "reviewed":{"type":"boolean","const":true},
                    "operation_ids":{"type":"array","minItems":1,"maxItems":100,"items":{"type":"string"}},
                    "allow_rename":{"type":"boolean","default":false},
                    "allow_move":{"type":"boolean","default":false},
                    "expected_catalog_revision":{"type":"string","minLength":1},
                    "expected_manifest_revision":{"type":"string"},
                    "expected_suggestions_revision":{"type":"string"}
                }
            }),
        ),
    ]
}

pub(crate) fn call_tool(workspace: &Path, params: Value) -> Result<Value> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("tools/call 缺少 name"))?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let result = (|| -> Result<Value> {
        match name {
            "project_docs_get_status" => {
                ensure_empty_object(&arguments, name)?;
                get_status(workspace, None)
            }
            "project_docs_analyze" => {
                let input: AnalyzeArguments = decode(arguments, name)?;
                analyze_workspace(workspace, input.offset, input.limit, input.ambiguous_only)
            }
            "project_docs_read" => {
                let input: ReadArguments = decode(arguments, name)?;
                read_documents(
                    workspace,
                    &input.paths,
                    input.max_chars_per_document,
                    input.expected_catalog_revision.as_deref(),
                )
            }
            "project_docs_get_suggestions" => {
                ensure_empty_object(&arguments, name)?;
                get_suggestions(workspace)
            }
            "project_docs_save_suggestions" => {
                let input: SaveSuggestionsArguments = decode(arguments, name)?;
                save_suggestions(
                    workspace,
                    input.suggestions,
                    &input.expected_catalog_revision,
                    input.expected_suggestions_revision.as_deref(),
                )
            }
            "project_docs_apply_suggestions" => {
                let input: ApplySuggestionsArguments = decode(arguments, name)?;
                apply_saved_suggestions(
                    workspace,
                    input.reviewed,
                    &input.expected_catalog_revision,
                    input.expected_manifest_revision.as_deref(),
                    input.expected_suggestions_revision.as_deref(),
                )
            }
            "project_docs_apply_file_operations" => {
                let input: ApplyFileOperationsArguments = decode(arguments, name)?;
                apply_reviewed_file_operations(
                    workspace,
                    ApplyFileOperationsRequest {
                        reviewed: input.reviewed,
                        operation_ids: &input.operation_ids,
                        allow_rename: input.allow_rename,
                        allow_move: input.allow_move,
                        expected_catalog_revision: &input.expected_catalog_revision,
                        expected_manifest_revision: input.expected_manifest_revision.as_deref(),
                        expected_suggestions_revision: input
                            .expected_suggestions_revision
                            .as_deref(),
                    },
                )
            }
            _ => Err(anyhow::anyhow!("未知项目文档 MCP 工具：{name}")),
        }
    })();
    let value = match result {
        Ok(value) => {
            record_tool_success(workspace, name, &value);
            value
        }
        Err(error) => {
            record_tool_failure(workspace, name, &error);
            return Err(error);
        }
    };
    Ok(json!({
        "content": [{ "type": "text", "text": serde_json::to_string_pretty(&value)? }],
        "structuredContent": value,
        "isError": false,
    }))
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema,
    })
}

fn decode<T: for<'de> Deserialize<'de>>(arguments: Value, name: &str) -> Result<T> {
    serde_json::from_value(arguments).with_context(|| format!("{name} 参数无效"))
}

fn ensure_empty_object(arguments: &Value, name: &str) -> Result<()> {
    if arguments.as_object().is_some_and(|value| value.is_empty()) {
        Ok(())
    } else {
        bail!("{name} 不接受参数")
    }
}

fn suggestions_schema() -> Value {
    json!({
        "type":"object",
        "required":["version","status","summary","proposed_sections","assignments","conflicts","move_suggestions","documents_read","estimated_tokens_used"],
        "properties":{
            "version":{"type":"integer","const":1},
            "status":{"type":"string","const":"ready"},
            "summary":{"type":"string","maxLength":4000},
            "proposed_sections":{
                "type":"array","maxItems":8,
                "items":{
                    "type":"object","required":["id","label","detail","color"],
                    "properties":{
                        "id":{"type":"string","maxLength":48},
                        "label":{"type":"string","maxLength":40},
                        "detail":{"type":"string","maxLength":120},
                        "color":{"type":"string","pattern":"^#[0-9A-Fa-f]{6}$"}
                    }
                }
            },
            "assignments":{
                "type":"array","maxItems":500,
                "items":{
                    "type":"object","required":["path","section_id","reason"],
                    "properties":{
                        "path":{"type":"string"},
                        "section_id":{"type":"string","maxLength":64},
                        "reason":{"type":"string","maxLength":500}
                    }
                }
            },
            "conflicts":{"type":"array","maxItems":100,"items":{"type":"string","maxLength":1000}},
            "move_suggestions":{"type":"array","maxItems":100,"items":{"type":"string","maxLength":1000}},
            "file_operations":{
                "type":"array","maxItems":100,
                "items":{
                    "type":"object",
                    "required":["id","kind","source_path","target_path","source_revision","reason"],
                    "properties":{
                        "id":{"type":"string","pattern":"^[A-Za-z0-9._-]+$","maxLength":80},
                        "kind":{"type":"string","enum":["rename","move"]},
                        "source_path":{"type":"string"},
                        "target_path":{"type":"string"},
                        "source_revision":{"type":"string","description":"必须等于 analyze 返回的 content_hash"},
                        "reason":{"type":"string","maxLength":500},
                        "status":{"type":"string","const":"proposed","default":"proposed"}
                    }
                }
            },
            "documents_read":{"type":"integer","minimum":0},
            "estimated_tokens_used":{"type":"integer","minimum":0}
        }
    })
}

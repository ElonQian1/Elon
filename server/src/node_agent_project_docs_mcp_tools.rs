use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;

use crate::{
    project_document_authorization::DocumentAutomationMode,
    project_document_file_operations::{apply_file_operations, ApplyFileOperationsRequest},
    project_document_governance::DocumentOrganizationSuggestions,
    project_document_governance_service::{
        analyze_workspace_scoped_query, apply_saved_suggestions, default_page_size,
        default_read_chars, get_suggestions, read_documents, save_suggestions,
    },
    project_document_observability::{get_status, record_tool_failure, record_tool_success},
    project_document_response::{compact_text, project_tool_response, ProjectionRequest},
};

#[derive(Debug, Deserialize)]
struct AnalyzeArguments {
    #[serde(default)]
    offset: usize,
    #[serde(default = "default_page_size")]
    limit: usize,
    #[serde(default)]
    ambiguous_only: bool,
    #[serde(default)]
    scope_id: Option<String>,
    #[serde(default)]
    topic: Option<String>,
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
    #[serde(default)]
    authorization_mode: DocumentAutomationMode,
    expected_catalog_revision: String,
    #[serde(default)]
    expected_suggestions_revision: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApplySuggestionsArguments {
    #[serde(default)]
    authorization_mode: DocumentAutomationMode,
    #[serde(default)]
    reviewed: bool,
    expected_catalog_revision: String,
    #[serde(default)]
    expected_manifest_revision: Option<String>,
    #[serde(default)]
    expected_suggestions_revision: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApplyFileOperationsArguments {
    #[serde(default)]
    authorization_mode: DocumentAutomationMode,
    #[serde(default)]
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
    #[serde(default)]
    git_baseline_commit: Option<String>,
}

pub(crate) fn tool_definitions() -> Vec<Value> {
    let mut definitions = vec![
        tool(
            "project_docs_analyze",
            "零模型 token 扫描当前 Git 项目的文档路径、标题、哈希、标题层级、生命周期与权威性，并返回项目类型推断、知识架构完整度、缺失基础文档、紧凑目录、虚拟分区和现有 AI 建议。文档治理任务第一步调用；不会读取正文或修改文件。",
            json!({
                "type":"object",
                "properties":{
                    "offset":{"type":"integer","minimum":0,"default":0},
                    "limit":{"type":"integer","minimum":1,"maximum":200,"default":80},
                    "ambiguous_only":{"type":"boolean","default":false}
                    ,"scope_id":{"type":"string","description":"可选联邦知识节点 id；大型项目只返回该节点目录。"},
                    "topic":{"type":"string","description":"按路径、标题、权威状态或主题筛选。"},
                    "cursor":{"type":"string","pattern":"^offset:[0-9]+$"},
                    "projection":{"type":"string","enum":["summary","page","detail","full"],"default":"page"},
                    "detail":{"type":"string","description":"detail/full 时指定 document_health、manifest 或 suggestions。"}
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
            json!({"type":"object","properties":{
                "offset":{"type":"integer","minimum":0,"default":0},
                "limit":{"type":"integer","minimum":1,"maximum":200,"default":80},
                "cursor":{"type":"string","pattern":"^offset:[0-9]+$"},
                "projection":{"type":"string","enum":["summary","page","detail","full"],"default":"page"}
            }}),
        ),
        tool(
            "project_docs_save_suggestions",
            "保存 AI 根据紧凑目录形成的结构化建议，包括项目类型、层级主题、知识首页、文档关系和安全路径操作。只能写 .elon/document-organization-suggestions.json，不能移动、删除、改写 Markdown，也不能直接更新分区配置。",
            json!({
                "type":"object",
                "required":["suggestions","expected_catalog_revision"],
                "properties":{
                    "expected_catalog_revision":{"type":"string","minLength":1},
                    "authorization_mode":{"type":"string","enum":["git_backed_full","trusted_reversible","review_all","suggestions_only"],"default":"git_backed_full"},
                    "expected_suggestions_revision":{"type":"string"},
                    "suggestions":suggestions_schema()
                }
            }),
        ),
        tool(
            "project_docs_apply_suggestions",
            "把 ready 建议应用为虚拟分区配置并标记 applied。authorization_mode 默认 git_backed_full：先创建仅文档 Git 基线；无实体操作时立即提交整理结果，有实体操作时把基线 SHA 交给 apply_file_operations 完成整理后提交。review_all 需要 reviewed=true；suggestions_only 禁止应用。",
            json!({
                "type":"object",
                "required":["expected_catalog_revision"],
                "properties":{
                    "authorization_mode":{"type":"string","enum":["git_backed_full","trusted_reversible","review_all","suggestions_only"],"default":"git_backed_full"},
                    "reviewed":{"type":"boolean","default":false},
                    "expected_catalog_revision":{"type":"string","minLength":1},
                    "expected_manifest_revision":{"type":"string"},
                    "expected_suggestions_revision":{"type":"string"}
                }
            }),
        ),
        tool(
            "project_docs_apply_file_operations",
            "对选中的 Markdown 执行重命名或移动。authorization_mode 默认 git_backed_full：确认整理前 Git 基线后自动执行，并创建整理后仅文档提交；review_all 需要 reviewed=true 和对应权限；suggestions_only 禁止应用。始终校验 revision，禁止覆盖、越界、修改代码或自动 push。",
            json!({
                "type":"object",
                "required":["operation_ids","expected_catalog_revision"],
                "properties":{
                    "authorization_mode":{"type":"string","enum":["git_backed_full","trusted_reversible","review_all","suggestions_only"],"default":"git_backed_full"},
                    "reviewed":{"type":"boolean","default":false},
                    "operation_ids":{"type":"array","minItems":1,"maxItems":100,"items":{"type":"string"}},
                    "allow_rename":{"type":"boolean","default":false},
                    "allow_move":{"type":"boolean","default":false},
                    "expected_catalog_revision":{"type":"string","minLength":1},
                    "expected_manifest_revision":{"type":"string"},
                    "expected_suggestions_revision":{"type":"string"}
                    ,"git_baseline_commit":{"type":"string","description":"git_backed_full 下优先传 apply_suggestions 返回的整理前提交 SHA。"}
                }
            }),
        ),
    ];
    definitions.insert(
        1,
        crate::node_agent_project_docs_mcp_knowledge_tools::issue_definition(),
    );
    definitions.splice(
        2..2,
        crate::node_agent_project_docs_mcp_graph_tools::definitions(),
    );
    definitions.splice(
        7..7,
        crate::node_agent_project_docs_mcp_discussion_tools::definitions(),
    );
    definitions.extend(crate::node_agent_project_docs_mcp_knowledge_tools::history_definitions());
    definitions.extend(crate::node_agent_project_docs_mcp_review_tools::definitions());
    definitions
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
    let projection_arguments = arguments.clone();
    let result = (|| -> Result<Value> {
        match name {
            "project_docs_get_status" => {
                ensure_empty_object(&arguments, name)?;
                get_status(workspace, None)
            }
            "project_docs_analyze" => {
                let projection = ProjectionRequest::from_arguments(&arguments)?;
                let input: AnalyzeArguments = decode(arguments, name)?;
                analyze_workspace_scoped_query(
                    workspace,
                    projection.offset.max(input.offset),
                    projection.limit.min(input.limit.max(1)),
                    input.ambiguous_only,
                    input.scope_id.as_deref(),
                    input.topic.as_deref(),
                )
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
            "project_docs_get_suggestions" => get_suggestions(workspace),
            "project_docs_save_suggestions" => {
                let input: SaveSuggestionsArguments = decode(arguments, name)?;
                save_suggestions(
                    workspace,
                    input.suggestions,
                    input.authorization_mode,
                    &input.expected_catalog_revision,
                    input.expected_suggestions_revision.as_deref(),
                )
            }
            "project_docs_apply_suggestions" => {
                let input: ApplySuggestionsArguments = decode(arguments, name)?;
                apply_saved_suggestions(
                    workspace,
                    input.authorization_mode,
                    input.reviewed,
                    &input.expected_catalog_revision,
                    input.expected_manifest_revision.as_deref(),
                    input.expected_suggestions_revision.as_deref(),
                )
            }
            "project_docs_apply_file_operations" => {
                let input: ApplyFileOperationsArguments = decode(arguments, name)?;
                apply_file_operations(
                    workspace,
                    ApplyFileOperationsRequest {
                        authorization_mode: input.authorization_mode,
                        reviewed: input.reviewed,
                        operation_ids: &input.operation_ids,
                        allow_rename: input.allow_rename,
                        allow_move: input.allow_move,
                        expected_catalog_revision: &input.expected_catalog_revision,
                        expected_manifest_revision: input.expected_manifest_revision.as_deref(),
                        expected_suggestions_revision: input
                            .expected_suggestions_revision
                            .as_deref(),
                        git_baseline_commit: input.git_baseline_commit.as_deref(),
                    },
                )
            }
            _ => crate::node_agent_project_docs_mcp_graph_tools::try_call(
                workspace,
                name,
                arguments.clone(),
            )?
            .or(
                crate::node_agent_project_docs_mcp_discussion_tools::try_call(
                    workspace,
                    name,
                    arguments.clone(),
                )?,
            )
            .or(crate::node_agent_project_docs_mcp_review_tools::try_call(
                workspace,
                name,
                arguments.clone(),
            )?)
            .or(
                crate::node_agent_project_docs_mcp_knowledge_tools::try_call(
                    workspace, name, arguments,
                )?,
            )
            .ok_or_else(|| anyhow::anyhow!("未知项目文档 MCP 工具：{name}")),
        }
    })();
    let value = match result {
        Ok(value) => {
            let value = project_tool_response(name, &projection_arguments, value)?;
            record_tool_success(workspace, name, &value);
            value
        }
        Err(error) => {
            record_tool_failure(workspace, name, &error);
            return Err(error);
        }
    };
    Ok(json!({
        "content": [{ "type": "text", "text": compact_text(name, &value)? }],
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
            "proposed_profile":{"type":"string","enum":["auto","software-platform","software-api","product","research","operations","personal-knowledge"],"default":"auto"},
            "proposed_home":knowledge_home_schema(),
            "proposed_sections":{
                "type":"array","maxItems":256,
                "items":{
                    "type":"object","required":["id","label","detail","color"],
                    "properties":{
                        "id":{"type":"string","maxLength":48},
                        "label":{"type":"string","maxLength":40},
                        "detail":{"type":"string","maxLength":120},
                        "color":{"type":"string","pattern":"^#[0-9A-Fa-f]{6}$"},
                        "parent_id":{"type":"string","maxLength":48,"default":""},
                        "order":{"type":"integer","minimum":0,"maximum":9999,"default":0},
                        "icon":{"type":"string","maxLength":32,"default":""},
                        "entrypoint":{"type":"string","default":""}
                    }
                }
            },
            "assignments":{
                "type":"array","maxItems":20000,
                "items":{
                    "type":"object","required":["path","section_id","reason"],
                    "properties":{
                        "path":{"type":"string"},
                        "section_id":{"type":"string","maxLength":64},
                        "reason":{"type":"string","maxLength":500},
                        "secondary":{"type":"boolean","default":false,"description":"true 表示副主题；false 表示唯一主主题"}
                    }
                }
            },
            "section_operations":{"type":"array","maxItems":256,"items":{"type":"object","required":["id","kind","section_id","reason","impact"],"properties":{
                "id":{"type":"string","maxLength":80},
                "kind":{"type":"string","enum":["create","rename","move","merge","delete"]},
                "section_id":{"type":"string","maxLength":48},
                "target_section_id":{"type":"string","maxLength":48},
                "parent_id":{"type":"string","maxLength":48},
                "label":{"type":"string","maxLength":40},
                "reason":{"type":"string","maxLength":500},
                "impact":{"type":"string","maxLength":500}
            }}},
            "conflicts":{"type":"array","maxItems":100,"items":{"type":"string","maxLength":1000}},
            "move_suggestions":{"type":"array","maxItems":100,"items":{"type":"string","maxLength":1000}},
            "architecture_findings":{"type":"array","maxItems":100,"items":{"type":"string","maxLength":1000}},
            "missing_document_types":{"type":"array","maxItems":100,"items":{"type":"string","maxLength":120}},
            "document_metadata":{
                "type":"object","maxProperties":20000,
                "additionalProperties":knowledge_metadata_schema()
            },
            "governance_facets":{
                "type":"object","maxProperties":20000,
                "additionalProperties":{
                    "type":"object","properties":{
                        "retrieval":{"type":"string","enum":["required","on_demand","excluded"]},
                        "lifecycle":{"type":"string","enum":["active","accepted","source_material","draft","deprecated","superseded","archived","unclassified"]},
                        "authority":{"type":"string","enum":["binding","authoritative","guidance","evidence","proposal","non_authoritative","none","unknown"]},
                        "document_type":{"type":"string","maxLength":64}
                    }
                }
            },
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
            "proposed_knowledge_graph":crate::node_agent_project_docs_mcp_graph_tools::proposal_schema(),
            "documents_read":{"type":"integer","minimum":0},
            "estimated_tokens_used":{"type":"integer","minimum":0}
        }
    })
}

fn knowledge_home_schema() -> Value {
    json!({
        "type":"object",
        "properties":{
            "title":{"type":"string","maxLength":80},
            "summary":{"type":"string","maxLength":1000},
            "entrypoint":{"type":"string"},
            "start_here":{"type":"array","maxItems":12,"items":{"type":"string"}}
        }
    })
}

fn knowledge_metadata_schema() -> Value {
    json!({
        "type":"object",
        "properties":{
            "doc_type":{"type":"string","maxLength":64},
            "id":{"type":"string","maxLength":120},
            "audience":{"type":"array","maxItems":12,"items":{"type":"string","maxLength":80}},
            "owner":{"type":"string","maxLength":80},
            "owners":{"type":"array","maxItems":12,"items":{"type":"string","maxLength":80}},
            "reviewed_at":{"type":"string","pattern":"^[0-9]{4}-[0-9]{2}-[0-9]{2}$"},
            "review_interval_days":{"type":"integer","minimum":1,"maximum":3650,"default":180},
            "implementation_refs":{"type":"array","maxItems":32,"items":{"type":"string","maxLength":500}},
            "version":{"type":"string","maxLength":40},
            "version_status":{"type":"string","enum":["current","draft","deprecated","superseded","archived"]},
            "related":{"type":"array","maxItems":24,"items":{"type":"string"}},
            "supersedes":{"type":"array","maxItems":24,"items":{"type":"string"}},
            "relations":{"type":"array","maxItems":48,"items":{"type":"object","required":["relation","target"],"properties":{
                "relation":{"type":"string","enum":["related","supports","depends_on","implements","evidence_for","supersedes","replaced_by","see_also"]},
                "target":{"type":"string"}
            }}}
        }
    })
}

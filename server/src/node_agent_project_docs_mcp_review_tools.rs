use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;

use crate::{
    project_document_modularity_review::review_document_modularity,
    project_document_retrieval_acceptance::{test_document_retrieval, RetrievalAcceptanceCase},
    project_document_section_reader::{read_document_sections, SectionReadRequest},
};

#[derive(Debug, Deserialize)]
struct ReadSectionsArguments {
    sections: Vec<SectionReadRequest>,
    #[serde(default = "default_section_chars")]
    max_chars_per_section: usize,
    #[serde(default)]
    expected_catalog_revision: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ReviewModularityArguments {
    #[serde(default)]
    paths: Vec<String>,
    #[serde(default = "default_max_lines")]
    max_lines: usize,
    #[serde(default = "default_max_bytes")]
    max_bytes: u64,
    #[serde(default = "default_max_headings")]
    max_headings: usize,
}

#[derive(Debug, Deserialize)]
struct TestRetrievalArguments {
    #[serde(default)]
    cases: Option<Vec<RetrievalAcceptanceCase>>,
    #[serde(default = "default_retrieval_tokens")]
    max_tokens: u64,
    #[serde(default = "default_retrieval_documents")]
    max_documents: usize,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        tool(
            "project_docs_read_sections",
            "按 Markdown 标题或稳定 section_id 精确读取一个或多个章节，返回行号、父标题链和受限正文；适合大文档的局部上下文，不需要读取整个文件。",
            json!({
                "type":"object",
                "required":["sections"],
                "properties":{
                    "sections":{
                        "type":"array","minItems":1,"maxItems":12,
                        "items":{
                            "type":"object","required":["path","heading"],
                            "properties":{
                                "path":{"type":"string","minLength":1},
                                "heading":{"type":"string","minLength":1,"description":"标题文本或稳定 section_id。"},
                                "include_children":{"type":"boolean","default":true}
                            }
                        }
                    },
                    "max_chars_per_section":{"type":"integer","minimum":1,"maximum":24000,"default":6000},
                    "expected_catalog_revision":{"type":"string"}
                }
            }),
        ),
        tool(
            "project_docs_review_modularity",
            "零模型 token 审查 Markdown 是否过大、标题过多或混合职责；区分应保留的历史讨论来源与应拆分的当前权威文档，只给出安全拆分候选，不自动改写正文。",
            json!({
                "type":"object",
                "properties":{
                    "paths":{"type":"array","maxItems":100,"items":{"type":"string"}},
                    "max_lines":{"type":"integer","minimum":100,"maximum":10000,"default":800},
                    "max_bytes":{"type":"integer","minimum":8000,"maximum":2000000,"default":50000},
                    "max_headings":{"type":"integer","minimum":8,"maximum":500,"default":40}
                }
            }),
        ),
        tool(
            "project_docs_test_retrieval",
            "运行可重复的 AI 文档检索验收用例，检查任务查询是否命中期望文档、避开禁止文档及满足首位要求；默认读取 .elon/document-retrieval-cases.json，不读取 Markdown 正文。",
            json!({
                "type":"object",
                "properties":{
                    "cases":{
                        "type":"array","minItems":1,"maxItems":20,
                        "items":{
                            "type":"object","required":["id","query","expected_paths"],
                            "properties":{
                                "id":{"type":"string","minLength":1},
                                "query":{"type":"string","minLength":1},
                                "node_id":{"type":"string"},
                                "expected_paths":{"type":"array","minItems":1,"items":{"type":"string"}},
                                "forbidden_paths":{"type":"array","items":{"type":"string"}},
                                "require_first":{"type":"string"}
                            }
                        }
                    },
                    "max_tokens":{"type":"integer","minimum":200,"maximum":12000,"default":3000},
                    "max_documents":{"type":"integer","minimum":1,"maximum":24,"default":8}
                }
            }),
        ),
    ]
}

pub(crate) fn try_call(workspace: &Path, name: &str, arguments: Value) -> Result<Option<Value>> {
    let value = match name {
        "project_docs_read_sections" => {
            let input: ReadSectionsArguments = decode(arguments, name)?;
            read_document_sections(
                workspace,
                &input.sections,
                input.max_chars_per_section,
                input.expected_catalog_revision.as_deref(),
            )?
        }
        "project_docs_review_modularity" => {
            let input: ReviewModularityArguments = decode(arguments, name)?;
            review_document_modularity(
                workspace,
                &input.paths,
                input.max_lines,
                input.max_bytes,
                input.max_headings,
            )?
        }
        "project_docs_test_retrieval" => {
            let input: TestRetrievalArguments = decode(arguments, name)?;
            test_document_retrieval(
                workspace,
                input.cases,
                input.max_tokens,
                input.max_documents,
            )?
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({"name":name,"description":description,"inputSchema":input_schema})
}

fn decode<T: for<'de> Deserialize<'de>>(arguments: Value, name: &str) -> Result<T> {
    serde_json::from_value(arguments).with_context(|| format!("{name} 参数无效"))
}

fn default_section_chars() -> usize {
    6_000
}

fn default_max_lines() -> usize {
    800
}

fn default_max_bytes() -> u64 {
    50_000
}

fn default_max_headings() -> usize {
    40
}

fn default_retrieval_tokens() -> u64 {
    3_000
}

fn default_retrieval_documents() -> usize {
    8
}

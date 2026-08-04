//! Full-governance MCP tools for native-agent project understanding handoff.

use anyhow::{bail, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;

use crate::project_document_native_context::{
    list_candidates, record_candidate, ProjectContextEvidence, ProjectContextMemory,
};

const RECORD_TOOL: &str = "project_docs_record_native_context_candidate";
const LIST_TOOL: &str = "project_docs_list_native_context_candidates";

#[derive(Debug, Deserialize)]
struct RecordArguments {
    #[serde(default)]
    candidate_id: String,
    summary: String,
    #[serde(default)]
    topics: Vec<String>,
    evidence: Vec<ProjectContextEvidence>,
    #[serde(default = "default_producer")]
    producer: String,
}

#[derive(Debug, Deserialize)]
struct ListArguments {
    #[serde(default = "default_status")]
    status: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        tool(
            RECORD_TOOL,
            "把 Codex Desktop/CLI 已用原生工具核对出的项目理解保存为本机待审核候选。只接收短摘要、topic、工作区相对路径、定位符和当前 SHA-256；不接收或保存源码正文、命令输出、聊天、prompt 或 Codex 私有 memories，也不修改 Git 工作区。",
            json!({
                "type":"object",
                "required":["summary","topics","evidence"],
                "properties":{
                    "candidate_id":{"type":"string","maxLength":80,"description":"可选稳定 id；省略时根据摘要与证据派生。"},
                    "summary":{"type":"string","minLength":12,"maxLength":800,"description":"仅陈述已由 evidence 支持的导航结论；不得粘贴源码。"},
                    "topics":{"type":"array","minItems":1,"maxItems":8,"items":{"type":"string","maxLength":48}},
                    "evidence":{"type":"array","minItems":1,"maxItems":8,"items":evidence_schema()},
                    "producer":{"type":"string","maxLength":40,"default":"codex_native_tools"}
                }
            }),
        ),
        tool(
            LIST_TOOL,
            "列出外部本机索引中的原生工具理解候选，供项目文档工作台审核并入 suggestions.proposed_context_memories。候选本身不是项目真源；跨 PC 生效必须经过既有 revision、authorization 和 apply 流程。",
            json!({
                "type":"object",
                "properties":{
                    "status":{"type":"string","enum":["pending","reviewed","rejected","applied","all"],"default":"pending"},
                    "limit":{"type":"integer","minimum":1,"maximum":20,"default":10}
                }
            }),
        ),
    ]
}

pub(crate) fn try_call(workspace: &Path, name: &str, arguments: Value) -> Result<Option<Value>> {
    match name {
        RECORD_TOOL => {
            let input: RecordArguments = serde_json::from_value(arguments)?;
            let candidate = record_candidate(
                workspace,
                ProjectContextMemory {
                    candidate_id: input.candidate_id,
                    summary: input.summary,
                    topics: input.topics,
                    evidence: input.evidence,
                    reviewed_at: String::new(),
                },
                &input.producer,
            )?;
            Ok(Some(json!({
                "status":"pending_review",
                "candidate":candidate,
                "storage":"external_project_document_index",
                "repository_changed":false,
                "source_bodies_stored":0,
                "next":"Review in the project document workspace or list with project_docs_list_native_context_candidates, then use the existing suggestions/apply flow."
            })))
        }
        LIST_TOOL => {
            let input: ListArguments = serde_json::from_value(arguments)?;
            if input.limit == 0 || input.limit > 20 {
                bail!("native context candidate limit 必须在 1..=20");
            }
            let candidates = list_candidates(workspace, &input.status, input.limit)?;
            Ok(Some(json!({
                "status":input.status,
                "candidate_count":candidates.len(),
                "candidates":candidates,
                "authority":"candidate_only",
                "repository_changed":false,
                "source_bodies_returned":0
            })))
        }
        _ => Ok(None),
    }
}

pub(crate) fn memory_schema() -> Value {
    json!({
        "type":"array",
        "maxItems":64,
        "items":{
            "type":"object",
            "required":["summary","topics","evidence","reviewed_at"],
            "properties":{
                "candidate_id":{"type":"string","maxLength":80},
                "summary":{"type":"string","minLength":12,"maxLength":800},
                "topics":{"type":"array","minItems":1,"maxItems":8,"items":{"type":"string","maxLength":48}},
                "evidence":{"type":"array","minItems":1,"maxItems":8,"items":evidence_schema()},
                "reviewed_at":{"type":"string","minLength":1,"maxLength":40,"description":"审核时间或审核 revision；进入共享 manifest 前必填。"}
            }
        }
    })
}

fn evidence_schema() -> Value {
    json!({
        "type":"object",
        "required":["path","content_hash"],
        "properties":{
            "path":{"type":"string","description":"工作区内规范相对路径。"},
            "content_hash":{"type":"string","pattern":"^[0-9A-Fa-f]{64}$"},
            "locator":{"type":"string","maxLength":120,"description":"可选 symbol、heading 或行附近定位；不是正文。"},
            "evidence_kind":{"type":"string","enum":["source","test","document","configuration"],"default":"source"}
        }
    })
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({"name":name,"description":description,"inputSchema":input_schema})
}

fn default_producer() -> String {
    "codex_native_tools".to_string()
}

fn default_status() -> String {
    "pending".to_string()
}

fn default_limit() -> usize {
    10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_never_accept_source_bodies() {
        let definitions = definitions();
        assert_eq!(definitions.len(), 2);
        let encoded = serde_json::to_string(&definitions).unwrap();
        assert!(!encoded.contains("source_body"));
        assert!(!encoded.contains("tool_output"));
    }
}

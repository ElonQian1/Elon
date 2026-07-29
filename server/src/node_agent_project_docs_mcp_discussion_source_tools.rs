//! MCP tools for incremental, low-token reading of imported conversation sources.

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;

#[derive(Debug, Deserialize)]
struct SourcePathArguments {
    path: String,
}

#[derive(Debug, Deserialize)]
struct SourceChunkArguments {
    path: String,
    chunk_id: String,
    #[serde(default)]
    expected_source_revision: Option<String>,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        tool(
            "project_discussions_get_source_manifest",
            "扫描一份已导入聊天的稳定消息锚点与 chunk 清单，只返回 chunk id、行号、哈希和大小，不返回正文、不消耗模型 token。首次编译或中断续编前调用。",
            json!({"type":"object","required":["path"],"properties":{
                "path":{"type":"string","pattern":"^docs/inbox/conversations/.+\\.md$"}
            }}),
        ),
        tool(
            "project_discussions_read_source_chunk",
            "按 manifest 的稳定 chunk id 读取一段聊天正文。一次只返回一个 chunk，并提供下一个 chunk id；节点 source_refs 应使用返回的 source_id 与 turn 锚点。",
            json!({"type":"object","required":["path","chunk_id"],"properties":{
                "path":{"type":"string","pattern":"^docs/inbox/conversations/.+\\.md$"},
                "chunk_id":{"type":"string","pattern":"^chunk-[0-9]{4}-[0-9a-f]{10}$"},
                "expected_source_revision":{"type":"string","maxLength":128}
            }}),
        ),
    ]
}

pub(crate) fn try_call(workspace: &Path, name: &str, arguments: Value) -> Result<Option<Value>> {
    let value = match name {
        "project_discussions_get_source_manifest" => {
            let input: SourcePathArguments = serde_json::from_value(arguments)?;
            crate::project_discussion_source_chunks::source_manifest(workspace, &input.path)?
        }
        "project_discussions_read_source_chunk" => {
            let input: SourceChunkArguments = serde_json::from_value(arguments)?;
            crate::project_discussion_source_chunks::read_source_chunk(
                workspace,
                &input.path,
                &input.chunk_id,
                input.expected_source_revision.as_deref(),
            )?
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({"name":name,"description":description,"inputSchema":input_schema})
}

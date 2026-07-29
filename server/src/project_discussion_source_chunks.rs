//! Stable, resumable chunks for long conversation sources.

use anyhow::{bail, Result};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;

use crate::project_document_files::read_project_document_file;

const SOURCE_PREFIX: &str = "docs/inbox/conversations/";
const TARGET_CHARS: usize = 12_000;
const MAX_CHUNKS: usize = 512;

#[derive(Debug, Clone, Serialize)]
struct SourceChunk {
    id: String,
    index: usize,
    start_line: usize,
    end_line: usize,
    char_count: usize,
    first_anchor: String,
    last_anchor: String,
    digest: String,
    #[serde(skip_serializing)]
    content: String,
}

pub(crate) fn source_manifest(workspace: &Path, path: &str) -> Result<Value> {
    let source = load_source(workspace, path)?;
    let chunks = build_chunks(&source.body)?;
    Ok(json!({
        "path": source.path,
        "source_id": source.source_id,
        "source_revision": source.revision,
        "source_format": source.format,
        "message_count": source.message_count,
        "chunk_count": chunks.len(),
        "chunks": chunks,
        "next_action": "按 chunk 顺序调用 project_discussions_read_source_chunk；每个 chunk 只读一次，并把已处理 chunk id 写入 proposal source.processed_chunk_ids。",
        "budget": {
            "classification_model_tokens": 0,
            "source_bodies_scanned_locally": 1,
            "source_body_chars_returned": 0,
            "metadata_only": true
        }
    }))
}

pub(crate) fn read_source_chunk(
    workspace: &Path,
    path: &str,
    chunk_id: &str,
    expected_revision: Option<&str>,
) -> Result<Value> {
    let source = load_source(workspace, path)?;
    if expected_revision
        .filter(|expected| !expected.trim().is_empty())
        .is_some_and(|expected| expected != source.revision)
    {
        bail!("聊天来源 revision 已变化，请重新获取 chunk manifest");
    }
    let chunks = build_chunks(&source.body)?;
    let position = chunks
        .iter()
        .position(|chunk| chunk.id == chunk_id.trim())
        .ok_or_else(|| anyhow::anyhow!("聊天 chunk 不存在：{}", chunk_id.trim()))?;
    let chunk = &chunks[position];
    Ok(json!({
        "path": source.path,
        "source_id": source.source_id,
        "source_revision": source.revision,
        "chunk": {
            "id": chunk.id,
            "index": chunk.index,
            "start_line": chunk.start_line,
            "end_line": chunk.end_line,
            "first_anchor": chunk.first_anchor,
            "last_anchor": chunk.last_anchor,
            "content": chunk.content,
        },
        "next_chunk_id": chunks.get(position + 1).map(|next| next.id.as_str()),
        "remaining_chunks": chunks.len().saturating_sub(position + 1),
        "budget": {
            "classification_model_tokens": 0,
            "chat_bodies_read": 1,
            "source_body_chars_returned": chunk.char_count,
            "estimated_model_tokens": (chunk.char_count + 3) / 4,
            "metadata_only": false
        }
    }))
}

struct LoadedSource {
    path: String,
    source_id: String,
    revision: String,
    format: String,
    message_count: usize,
    body: String,
}

fn load_source(workspace: &Path, path: &str) -> Result<LoadedSource> {
    let path = path.trim().replace('\\', "/");
    if !path.starts_with(SOURCE_PREFIX) || !path.to_ascii_lowercase().ends_with(".md") {
        bail!("讨论来源分块只允许读取 docs/inbox/conversations 下的 Markdown");
    }
    let document = read_project_document_file(workspace, &path)?;
    let (metadata, body) = frontmatter_and_body(&document.content);
    let fallback = format!("{:x}", Sha256::digest(body.as_bytes()));
    let source_id = metadata_value(metadata, "source_id")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("conversation-{}", &fallback[..16]));
    let revision = metadata_value(metadata, "source_revision")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.clone());
    let format = metadata_value(metadata, "source_format").unwrap_or_else(|| "markdown".into());
    let message_count = metadata_value(metadata, "source_message_count")
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| body.lines().filter(|line| is_turn_heading(line)).count());
    Ok(LoadedSource {
        path,
        source_id,
        revision,
        format,
        message_count,
        body: body.trim().to_string(),
    })
}

fn build_chunks(body: &str) -> Result<Vec<SourceChunk>> {
    let blocks = source_blocks(body);
    let mut groups = Vec::<Vec<(usize, String)>>::new();
    let mut current = Vec::new();
    let mut current_chars = 0usize;
    for (line, block) in blocks {
        let chars = block.chars().count();
        if !current.is_empty() && current_chars + chars > TARGET_CHARS {
            groups.push(std::mem::take(&mut current));
            current_chars = 0;
        }
        if chars > TARGET_CHARS {
            if !current.is_empty() {
                groups.push(std::mem::take(&mut current));
                current_chars = 0;
            }
            for piece in split_large_block(line, &block) {
                groups.push(vec![piece]);
            }
            continue;
        }
        current.push((line, block));
        current_chars += chars;
    }
    if !current.is_empty() {
        groups.push(current);
    }
    if groups.len() > MAX_CHUNKS {
        bail!("聊天来源超过 {MAX_CHUNKS} 个 chunk，请先按会话或主题拆分");
    }
    Ok(groups
        .into_iter()
        .enumerate()
        .map(|(index, group)| chunk_from_group(index, group))
        .collect())
}

fn source_blocks(body: &str) -> Vec<(usize, String)> {
    let lines = body.lines().collect::<Vec<_>>();
    let has_turns = lines.iter().any(|line| is_turn_heading(line));
    let mut blocks = Vec::new();
    let mut start = 1usize;
    let mut current = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let boundary = if has_turns {
            is_turn_heading(line)
        } else {
            line.trim().is_empty() && !current.is_empty()
        };
        if boundary && !current.is_empty() {
            blocks.push((start, current.join("\n")));
            current.clear();
            start = index + 1;
        }
        if !(!has_turns && line.trim().is_empty() && current.is_empty()) {
            current.push((*line).to_string());
        }
    }
    if !current.is_empty() {
        blocks.push((start, current.join("\n")));
    }
    blocks
}

fn split_large_block(start_line: usize, block: &str) -> Vec<(usize, String)> {
    let chars = block.chars().collect::<Vec<_>>();
    chars
        .chunks(TARGET_CHARS)
        .enumerate()
        .map(|(index, chunk)| (start_line + index, chunk.iter().collect::<String>()))
        .collect()
}

fn chunk_from_group(index: usize, group: Vec<(usize, String)>) -> SourceChunk {
    let start_line = group.first().map(|item| item.0).unwrap_or(1);
    let content = group
        .iter()
        .map(|item| item.1.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    let end_line = start_line + content.lines().count().saturating_sub(1);
    let anchors = content.lines().filter_map(turn_anchor).collect::<Vec<_>>();
    let digest = format!("{:x}", Sha256::digest(content.as_bytes()));
    SourceChunk {
        id: format!("chunk-{:04}-{}", index + 1, &digest[..10]),
        index,
        start_line,
        end_line,
        char_count: content.chars().count(),
        first_anchor: anchors.first().cloned().unwrap_or_default(),
        last_anchor: anchors.last().cloned().unwrap_or_default(),
        digest,
        content,
    }
}

fn is_turn_heading(line: &str) -> bool {
    line.trim_start().starts_with("## turn-")
}

fn turn_anchor(line: &str) -> Option<String> {
    line.trim()
        .strip_prefix("## ")
        .and_then(|value| value.split_whitespace().next())
        .filter(|value| value.starts_with("turn-"))
        .map(str::to_string)
}

fn frontmatter_and_body(content: &str) -> (&str, &str) {
    if !content.starts_with("---\n") {
        return ("", content);
    }
    let rest = &content[4..];
    if let Some(index) = rest.find("\n---\n") {
        (&rest[..index], &rest[index + 5..])
    } else {
        ("", content)
    }
}

fn metadata_value(metadata: &str, key: &str) -> Option<String> {
    metadata.lines().find_map(|line| {
        let value = line.strip_prefix(&format!("{key}:"))?.trim();
        serde_json::from_str::<String>(value)
            .ok()
            .or_else(|| Some(value.trim_matches('"').to_string()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_align_to_normalized_turns() {
        let body = (1..=20)
            .map(|index| format!("## turn-{index:04} · 用户\n\n{}", "内容".repeat(500)))
            .collect::<Vec<_>>()
            .join("\n\n");
        let chunks = build_chunks(&body).unwrap();
        assert!(chunks.len() > 1);
        assert_eq!(chunks[0].first_anchor, "turn-0001");
        assert!(chunks
            .iter()
            .all(|chunk| chunk.char_count <= TARGET_CHARS * 2));
    }

    #[test]
    fn metadata_parser_accepts_json_quoted_values() {
        let (metadata, body) =
            frontmatter_and_body("---\nsource_id: \"conversation-one\"\n---\n\n正文");
        assert_eq!(
            metadata_value(metadata, "source_id").as_deref(),
            Some("conversation-one")
        );
        assert_eq!(body.trim(), "正文");
    }
}

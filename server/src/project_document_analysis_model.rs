//! Compact catalog projection and federation scope helpers.

use anyhow::{anyhow, Result};
use homecli_proto::ProjectDocumentEntry;
use serde::Serialize;
use serde_json::Value;

use crate::project_document_governance::{effective_section, DocumentSectionManifest};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CompactDocument<'a> {
    path: &'a str,
    title: &'a str,
    size_bytes: u64,
    role: &'a str,
    lifecycle: &'a str,
    authority: &'a str,
    scope: &'a str,
    default_retrieval: bool,
    ambiguous: bool,
    confidence: &'a str,
    reason: &'a str,
    token_estimate: u64,
    content_hash: &'a str,
    headings: &'a [String],
    section: String,
}

pub(crate) fn compact_document<'a>(
    document: &'a ProjectDocumentEntry,
    manifest: &DocumentSectionManifest,
) -> CompactDocument<'a> {
    CompactDocument {
        path: &document.path,
        title: &document.title,
        size_bytes: document.byte_len,
        role: &document.metadata.role,
        lifecycle: &document.metadata.lifecycle,
        authority: &document.metadata.authority,
        scope: &document.metadata.scope,
        default_retrieval: document.metadata.default_retrieval,
        ambiguous: document.metadata.ambiguous,
        confidence: &document.metadata.confidence,
        reason: &document.metadata.reason,
        token_estimate: document.metadata.token_estimate,
        content_hash: &document.metadata.content_hash,
        headings: &document.metadata.headings,
        section: effective_section(document, manifest),
    }
}

pub(crate) fn federation_scope_path(
    analysis: &Value,
    scope_id: Option<&str>,
) -> Result<Option<String>> {
    let Some(scope_id) = scope_id.filter(|value| !value.trim().is_empty()) else {
        return Ok(None);
    };
    analysis
        .pointer("/federation/nodes")
        .and_then(Value::as_array)
        .and_then(|nodes| {
            nodes
                .iter()
                .find(|node| node.get("id").and_then(Value::as_str) == Some(scope_id))
        })
        .and_then(|node| node.get("scope_path"))
        .and_then(Value::as_str)
        .map(|value| Some(value.to_string()))
        .ok_or_else(|| anyhow!("未知知识节点：{scope_id}"))
}

pub(crate) fn document_in_scope(path: &str, scope: &str) -> bool {
    let path = path.replace('\\', "/").to_ascii_lowercase();
    let scope = scope
        .replace('\\', "/")
        .trim_matches('/')
        .to_ascii_lowercase();
    scope.is_empty() || path == scope || path.starts_with(&format!("{scope}/"))
}

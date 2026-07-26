//! Compact catalog projection and federation scope helpers.

use anyhow::{anyhow, Result};
use homecli_proto::ProjectDocumentEntry;
use serde::Serialize;
use serde_json::Value;

use crate::{
    project_document_federation::path_matches_scope,
    project_document_governance::{effective_section, DocumentSectionManifest},
    project_document_governance_facets::{
        effective_facets_with_metadata, DocumentGovernanceFacets,
    },
};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FederationScope {
    pub path: String,
    pub include_globs: Vec<String>,
    pub exclude_globs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CompactDocument<'a> {
    path: &'a str,
    title: &'a str,
    size_bytes: u64,
    role: &'a str,
    lifecycle: String,
    authority: String,
    scope: &'a str,
    default_retrieval: bool,
    ambiguous: bool,
    confidence: &'a str,
    reason: String,
    token_estimate: u64,
    content_hash: &'a str,
    headings: &'a [String],
    section: String,
    primary_topic: String,
    secondary_topics: Vec<String>,
    governance: DocumentGovernanceFacets,
    version: String,
    version_status: String,
}

pub(crate) fn compact_document<'a>(
    document: &'a ProjectDocumentEntry,
    manifest: &DocumentSectionManifest,
) -> CompactDocument<'a> {
    let path = document.path.replace('\\', "/");
    let governance = effective_facets_with_metadata(
        document,
        manifest.governance_facets.get(&path),
        manifest.document_metadata.get(&path),
    );
    let manifest_classified = document.metadata.ambiguous
        && governance.lifecycle != "unclassified"
        && governance.authority != "unknown";
    CompactDocument {
        path: &document.path,
        title: &document.title,
        size_bytes: document.byte_len,
        role: &document.metadata.role,
        lifecycle: governance.lifecycle.clone(),
        authority: governance.authority.clone(),
        scope: &document.metadata.scope,
        default_retrieval: governance.retrieval != "excluded",
        ambiguous: document.metadata.ambiguous && !manifest_classified,
        confidence: &document.metadata.confidence,
        reason: if manifest_classified {
            format!(
                "{}；受控知识清单以 owner、复查日期和治理属性完成分类",
                document.metadata.reason
            )
        } else {
            document.metadata.reason.clone()
        },
        token_estimate: document.metadata.token_estimate,
        content_hash: &document.metadata.content_hash,
        headings: &document.metadata.headings,
        section: effective_section(document, manifest),
        primary_topic: manifest.assignments.get(&path).cloned().unwrap_or_default(),
        secondary_topics: manifest
            .secondary_assignments
            .get(&path)
            .cloned()
            .unwrap_or_default(),
        governance,
        version: manifest
            .document_metadata
            .get(&path)
            .map(|metadata| metadata.version.clone())
            .unwrap_or_default(),
        version_status: manifest
            .document_metadata
            .get(&path)
            .map(|metadata| metadata.version_status.clone())
            .unwrap_or_default(),
    }
}

pub(crate) fn federation_scope(
    analysis: &Value,
    scope_id: Option<&str>,
) -> Result<Option<FederationScope>> {
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
        .map(|node| {
            Some(FederationScope {
                path: node
                    .get("scope_path")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                include_globs: string_array(node.get("include_globs")),
                exclude_globs: string_array(node.get("exclude_globs")),
            })
        })
        .ok_or_else(|| anyhow!("未知知识节点：{scope_id}"))
}

pub(crate) fn document_in_scope(path: &str, scope: &FederationScope) -> bool {
    path_matches_scope(
        path,
        &scope.path,
        &scope.include_globs,
        &scope.exclude_globs,
    )
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

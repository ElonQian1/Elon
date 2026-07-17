//! Reusable parsing and implementation-evidence rules for document quality.

use anyhow::{Context, Result};
use chrono::{NaiveDate, Utc};
use homecli_proto::ProjectDocumentEntry;
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Component, Path},
};

use crate::{
    project_document_governance::{DocumentKnowledgeMetadata, DocumentSectionManifest},
    project_document_index::{file_modified_millis, ProjectDocumentIndex},
    project_document_quality::{make_issue, DocumentQualityIssue},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct DocumentQualityFacts {
    pub(super) local_links: Vec<String>,
    pub(super) anchors: Vec<String>,
    pub(super) external_links: Vec<String>,
}

pub(super) fn load_facts(
    workspace: &Path,
    document: &ProjectDocumentEntry,
    index: &ProjectDocumentIndex,
) -> Result<DocumentQualityFacts> {
    if let Some(value) =
        index.cached_quality_facts(&document.path, &document.metadata.content_hash)?
    {
        return serde_json::from_value(value).context("解析文档质量事实失败");
    }
    let content = fs::read_to_string(workspace.join(&document.path)).unwrap_or_default();
    let facts = DocumentQualityFacts {
        local_links: markdown_links(&content, false),
        external_links: markdown_links(&content, true),
        anchors: document
            .metadata
            .headings
            .iter()
            .map(|heading| markdown_anchor(heading))
            .collect(),
    };
    index.store_quality_facts(
        &document.path,
        &document.metadata.content_hash,
        &serde_json::to_value(&facts)?,
    )?;
    Ok(facts)
}

pub(super) fn resolve_link_target(source_path: &str, raw: &str) -> (String, String) {
    let raw = raw.split('?').next().unwrap_or(raw);
    let mut parts = raw.splitn(2, '#');
    let target = parts.next().unwrap_or_default();
    let anchor = parts.next().map(markdown_anchor).unwrap_or_default();
    if target.is_empty() {
        return (normalize(source_path), anchor);
    }
    let base = Path::new(source_path)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    (normalize_relative_path(&base.join(target)), anchor)
}

pub(super) fn check_implementation_refs(
    workspace: &Path,
    path: &str,
    metadata: &DocumentKnowledgeMetadata,
    cache: &mut HashMap<String, bool>,
    issues: &mut Vec<DocumentQualityIssue>,
) {
    for reference in &metadata.implementation_refs {
        let exists = *cache
            .entry(reference.clone())
            .or_insert_with(|| implementation_reference_exists(workspace, reference));
        if !exists {
            issues.push(make_issue(
                "implementation_conflict",
                "error",
                path,
                format!("文档引用的实现不存在：{reference}"),
                "程序按文件、路由或符号检索均未找到对应实现".to_string(),
                "核对实现路径；需要语义判断时再交给 AI 阅读证据",
                95,
            ));
        } else if implementation_changed_after_review(workspace, reference, &metadata.reviewed_at) {
            issues.push(make_issue(
                "implementation_drift",
                "info",
                path,
                format!("关联实现晚于文档复查时间：{reference}"),
                format!("文档上次复查：{}", metadata.reviewed_at),
                "按需让 AI 对照文档片段和实现证据复核",
                80,
            ));
        }
    }
}

pub(super) fn manifest_entrypoints(manifest: &DocumentSectionManifest) -> HashSet<String> {
    std::iter::once(manifest.home.entrypoint.as_str())
        .chain(manifest.home.start_here.iter().map(String::as_str))
        .chain(
            manifest
                .sections
                .iter()
                .map(|section| section.entrypoint.as_str()),
        )
        .filter(|path| !path.is_empty())
        .map(normalize)
        .collect()
}

pub(super) fn eligible_for_governance(document: &ProjectDocumentEntry) -> bool {
    document.metadata.default_retrieval
        || matches!(
            document.metadata.role.as_str(),
            "policy" | "router" | "architecture" | "spec" | "requirement" | "runbook"
        )
}

pub(super) fn eligible_for_orphan_check(document: &ProjectDocumentEntry) -> bool {
    !matches!(
        document.metadata.lifecycle.as_str(),
        "draft" | "archived" | "superseded"
    ) && !matches!(
        document.metadata.role.as_str(),
        "report" | "status" | "archive" | "discussion" | "note"
    )
}

pub(super) fn review_is_overdue(metadata: &DocumentKnowledgeMetadata) -> bool {
    let Ok(reviewed) = NaiveDate::parse_from_str(&metadata.reviewed_at, "%Y-%m-%d") else {
        return false;
    };
    let interval = metadata.review_interval_days.clamp(1, 3_650) as i64;
    reviewed + chrono::Duration::days(interval) < Utc::now().date_naive()
}

pub(super) fn normalize(path: &str) -> String {
    path.trim().replace('\\', "/")
}

fn markdown_links(content: &str, external: bool) -> Vec<String> {
    let mut links = Vec::new();
    let mut rest = content;
    while let Some(start) = rest.find("](") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find(')') else { break };
        let raw = rest[..end]
            .trim()
            .trim_matches('<')
            .trim_matches('>')
            .split_whitespace()
            .next()
            .unwrap_or_default();
        let is_external = raw.starts_with("http://") || raw.starts_with("https://");
        if !raw.is_empty()
            && is_external == external
            && !raw.starts_with("mailto:")
            && !raw.starts_with("data:")
        {
            links.push(raw.to_string());
        }
        rest = &rest[end + 1..];
    }
    links.sort();
    links.dedup();
    links
}

fn normalize_relative_path(path: &Path) -> String {
    let mut output = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => output.push(value.to_string_lossy().to_string()),
            Component::ParentDir => {
                output.pop();
            }
            Component::CurDir => {}
            _ => return String::new(),
        }
    }
    output.join("/")
}

fn markdown_anchor(value: &str) -> String {
    value
        .trim()
        .trim_start_matches('#')
        .to_ascii_lowercase()
        .chars()
        .filter_map(|ch| {
            if ch.is_alphanumeric() || ('\u{4e00}'..='\u{9fff}').contains(&ch) {
                Some(ch)
            } else if ch.is_whitespace() || ch == '-' {
                Some('-')
            } else {
                None
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn implementation_reference_exists(workspace: &Path, reference: &str) -> bool {
    let (kind, value) = reference.split_once(':').unwrap_or(("file", reference));
    match kind {
        "file" => workspace.join(value.trim_start_matches('/')).exists(),
        "route" | "symbol" => search_source(workspace, value),
        _ => workspace.join(reference).exists(),
    }
}

fn implementation_changed_after_review(workspace: &Path, reference: &str, reviewed: &str) -> bool {
    let Ok(reviewed) = NaiveDate::parse_from_str(reviewed, "%Y-%m-%d") else {
        return false;
    };
    let (kind, value) = reference.split_once(':').unwrap_or(("file", reference));
    if kind != "file" {
        return false;
    }
    let modified_ms = file_modified_millis(&workspace.join(value.trim_start_matches('/')));
    let reviewed_ms: u64 = reviewed
        .and_hms_opt(23, 59, 59)
        .and_then(|value| value.and_utc().timestamp_millis().try_into().ok())
        .unwrap_or_default();
    modified_ms > reviewed_ms
}

fn search_source(workspace: &Path, needle: &str) -> bool {
    let needle = needle.trim();
    if needle.is_empty() {
        return false;
    }
    WalkBuilder::new(workspace)
        .hidden(false)
        .git_ignore(true)
        .build()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
        .filter(|entry| source_extension(entry.path()))
        .take(5_000)
        .any(|entry| {
            fs::metadata(entry.path()).is_ok_and(|metadata| metadata.len() <= 2 * 1024 * 1024)
                && fs::read_to_string(entry.path()).is_ok_and(|content| content.contains(needle))
        })
}

fn source_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| {
            matches!(
                value,
                "rs" | "ts"
                    | "tsx"
                    | "js"
                    | "jsx"
                    | "kt"
                    | "java"
                    | "py"
                    | "go"
                    | "toml"
                    | "json"
                    | "yml"
                    | "yaml"
            )
        })
}

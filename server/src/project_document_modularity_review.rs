//! Deterministic review for oversized or mixed-responsibility Markdown.

use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::{collections::HashSet, fs, path::Path};

use crate::{
    project_docs_scan::{collect_project_documents_with_options, ProjectDocumentScanOptions},
    project_document_governance::{parse_manifest, SECTION_CONFIG_PATH},
    project_document_governance_facets::effective_facets_with_metadata,
};

pub(crate) fn review_document_modularity(
    workspace: &Path,
    requested_paths: &[String],
    max_lines: usize,
    max_bytes: u64,
    max_headings: usize,
) -> Result<Value> {
    let snapshot = collect_project_documents_with_options(
        workspace,
        ProjectDocumentScanOptions {
            seed_missing_defaults: false,
            catalog_only: true,
            include_analysis: false,
        },
    )?;
    let manifest = fs::read_to_string(workspace.join(SECTION_CONFIG_PATH)).ok();
    let manifest = parse_manifest(manifest.as_deref())?;
    let requested = requested_paths
        .iter()
        .map(|path| normalize(path))
        .collect::<HashSet<_>>();
    let known = snapshot
        .documents
        .iter()
        .map(|document| normalize(&document.path))
        .collect::<HashSet<_>>();
    if let Some(path) = requested.iter().find(|path| !known.contains(*path)) {
        bail!("模块化审查路径不在当前文档目录：{path}");
    }
    let max_lines = max_lines.clamp(100, 10_000);
    let max_bytes = max_bytes.clamp(8_000, 2_000_000);
    let max_headings = max_headings.clamp(8, 500);
    let mut findings = Vec::new();
    let mut bodies_scanned = 0usize;
    let mut bytes_scanned = 0u64;
    for document in snapshot
        .documents
        .iter()
        .filter(|document| requested.is_empty() || requested.contains(&normalize(&document.path)))
    {
        let metadata_candidate = document.byte_len > max_bytes
            || document.metadata.headings.len() > max_headings
            || document.metadata.token_estimate > max_bytes.div_ceil(4);
        if !metadata_candidate && requested.is_empty() {
            continue;
        }
        let path = document.path.replace('\\', "/");
        let content = fs::read_to_string(workspace.join(&path))?;
        bodies_scanned += 1;
        bytes_scanned = bytes_scanned.saturating_add(content.len() as u64);
        let line_count = content.lines().count();
        let headings = markdown_headings(&content);
        let exceeds = line_count > max_lines
            || document.byte_len > max_bytes
            || headings.len() > max_headings;
        if !exceeds {
            continue;
        }
        let facets = effective_facets_with_metadata(
            document,
            manifest.governance_facets.get(&path),
            manifest.document_metadata.get(&path),
        );
        let source_material = facets.lifecycle == "source_material"
            || matches!(document.metadata.role.as_str(), "discussion" | "archive");
        let h2 = headings
            .iter()
            .filter(|(level, _)| *level == 2)
            .map(|(_, text)| text.clone())
            .take(12)
            .collect::<Vec<_>>();
        findings.push(json!({
            "path": path,
            "title": document.title,
            "line_count": line_count,
            "byte_len": document.byte_len,
            "heading_count": headings.len(),
            "estimated_tokens": document.metadata.token_estimate,
            "governance": facets,
            "severity": if line_count > max_lines.saturating_mul(2) || document.byte_len > max_bytes.saturating_mul(2) {"warning"} else {"advice"},
            "recommendation": if source_material {
                "retain_source_material_and_compile"
            } else {
                "create_package_index_and_split_by_responsibility"
            },
            "suggested_child_topics": h2,
            "safe_apply": false,
            "reason": if source_material {
                "历史讨论保持低权威原始来源；把已接受结论晋升到正式模块文档，不逐段重写原记录。"
            } else {
                "当前文档已超过长期维护阈值；保留一个短入口，并按职责把正文拆到同目录子文档。"
            },
        }));
    }
    findings.sort_by(|left, right| right["byte_len"].as_u64().cmp(&left["byte_len"].as_u64()));
    Ok(json!({
        "catalog_revision": snapshot.revision,
        "summary": {
            "documents_reviewed": if requested.is_empty() {snapshot.documents.len()} else {requested.len()},
            "bodies_scanned": bodies_scanned,
            "findings": findings.len(),
            "warnings": findings.iter().filter(|finding| finding["severity"] == "warning").count(),
        },
        "thresholds": {
            "max_lines": max_lines,
            "max_bytes": max_bytes,
            "max_headings": max_headings,
        },
        "findings": findings,
        "budget": {
            "classification_model_tokens": 0,
            "document_bodies_scanned": bodies_scanned,
            "bytes_scanned": bytes_scanned,
            "document_content_returned": false,
        },
        "apply_policy": {
            "automatic_content_split": false,
            "reason": "章节边界不等于职责边界；工具只提供确定性证据和拆分候选，正文修改仍需 AI 按权威文档规则执行。"
        },
    }))
}

fn markdown_headings(content: &str) -> Vec<(usize, String)> {
    let mut output = Vec::new();
    let mut fenced = false;
    for line in content.lines() {
        let line = line.trim_start();
        if line.starts_with("```") || line.starts_with("~~~") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        let level = line
            .chars()
            .take_while(|character| *character == '#')
            .count();
        if (1..=6).contains(&level) && line.chars().nth(level).is_some_and(char::is_whitespace) {
            output.push((
                level,
                line[level..]
                    .trim()
                    .trim_end_matches('#')
                    .trim()
                    .to_string(),
            ));
        }
    }
    output
}

fn normalize(value: &str) -> String {
    value.trim().replace('\\', "/").to_ascii_lowercase()
}

#[cfg(test)]
#[path = "project_document_modularity_review_tests.rs"]
mod tests;

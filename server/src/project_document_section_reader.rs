//! Heading-addressable Markdown reads for bounded AI context.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashSet, path::Path};

use crate::{
    project_docs_scan::{collect_project_documents_with_options, ProjectDocumentScanOptions},
    project_document_files::read_project_document_file,
};

const MAX_SECTIONS: usize = 12;
const MAX_SECTION_CHARS: usize = 24_000;
const MAX_TOTAL_CHARS: usize = 48_000;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SectionReadRequest {
    pub path: String,
    pub heading: String,
    #[serde(default = "default_include_children")]
    pub include_children: bool,
}

#[derive(Debug, Clone, Serialize)]
struct MarkdownHeading {
    id: String,
    text: String,
    level: usize,
    line: usize,
    end_line: usize,
    parent_ids: Vec<String>,
}

pub(crate) fn read_document_sections(
    workspace: &Path,
    requests: &[SectionReadRequest],
    max_chars_per_section: usize,
    expected_catalog_revision: Option<&str>,
) -> Result<Value> {
    if requests.is_empty() || requests.len() > MAX_SECTIONS {
        bail!("project_docs_read_sections 一次必须读取 1 到 {MAX_SECTIONS} 个章节");
    }
    let snapshot = collect_project_documents_with_options(
        workspace,
        ProjectDocumentScanOptions {
            seed_missing_defaults: false,
            catalog_only: true,
            include_analysis: false,
        },
    )?;
    if expected_catalog_revision
        .filter(|value| !value.trim().is_empty())
        .is_some_and(|expected| expected != snapshot.revision)
    {
        bail!("文档目录已变化，请重新调用 project_docs_analyze");
    }
    let known_paths = snapshot
        .documents
        .iter()
        .map(|document| normalize(&document.path))
        .collect::<HashSet<_>>();
    let char_limit = max_chars_per_section.clamp(1, MAX_SECTION_CHARS);
    let mut total_chars = 0usize;
    let mut sections = Vec::new();
    for request in requests {
        let path = request.path.trim().replace('\\', "/");
        if !known_paths.contains(&normalize(&path)) {
            bail!("请求读取的路径不在当前文档目录：{path}");
        }
        let file = read_project_document_file(workspace, &path)?;
        let lines = file.content.lines().collect::<Vec<_>>();
        let headings = parse_headings(&lines);
        let selected = select_heading(&headings, &request.heading, &path)?;
        let end_line = if request.include_children {
            selected.end_line
        } else {
            headings
                .iter()
                .find(|heading| heading.line > selected.line)
                .map(|heading| heading.line.saturating_sub(1))
                .unwrap_or(lines.len())
        };
        let raw = lines
            .get(selected.line.saturating_sub(1)..end_line)
            .unwrap_or_default()
            .join("\n");
        let remaining = MAX_TOTAL_CHARS.saturating_sub(total_chars);
        if remaining == 0 {
            break;
        }
        let take = char_limit.min(remaining);
        let content = raw.chars().take(take).collect::<String>();
        let truncated = content.chars().count() < raw.chars().count();
        total_chars += content.chars().count();
        sections.push(json!({
            "path": file.path,
            "revision": file.revision,
            "section_id": selected.id,
            "heading": selected.text,
            "level": selected.level,
            "start_line": selected.line,
            "end_line": end_line,
            "parent_ids": selected.parent_ids,
            "include_children": request.include_children,
            "content": content,
            "truncated": truncated,
        }));
    }
    Ok(json!({
        "catalog_revision": snapshot.revision,
        "sections": sections,
        "sections_read": sections.len(),
        "characters_returned": total_chars,
        "estimated_tokens_returned": (total_chars as u64).div_ceil(4),
        "limits": {
            "max_sections": MAX_SECTIONS,
            "max_chars_per_section": char_limit,
            "max_total_chars": MAX_TOTAL_CHARS,
        },
    }))
}

fn parse_headings(lines: &[&str]) -> Vec<MarkdownHeading> {
    let mut headings = Vec::new();
    let mut stack = Vec::<(usize, String)>::new();
    let mut slug_counts = std::collections::HashMap::<String, usize>::new();
    let mut fenced = false;
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        let level = trimmed
            .chars()
            .take_while(|character| *character == '#')
            .count();
        if !(1..=6).contains(&level) || !trimmed.chars().nth(level).is_some_and(char::is_whitespace)
        {
            continue;
        }
        let text = trimmed[level..]
            .trim()
            .trim_end_matches('#')
            .trim()
            .to_string();
        if text.is_empty() {
            continue;
        }
        let base = heading_slug(&text);
        let count = slug_counts.entry(base.clone()).or_default();
        let id = if *count == 0 {
            base
        } else {
            format!("{base}-{}", *count)
        };
        *count += 1;
        stack.retain(|(parent_level, _)| *parent_level < level);
        let parent_ids = stack.iter().map(|(_, id)| id.clone()).collect::<Vec<_>>();
        stack.push((level, id.clone()));
        headings.push(MarkdownHeading {
            id,
            text,
            level,
            line: index + 1,
            end_line: lines.len(),
            parent_ids,
        });
    }
    for index in 0..headings.len() {
        headings[index].end_line = headings
            .iter()
            .skip(index + 1)
            .find(|candidate| candidate.level <= headings[index].level)
            .map(|candidate| candidate.line.saturating_sub(1))
            .unwrap_or(lines.len());
    }
    headings
}

fn select_heading<'a>(
    headings: &'a [MarkdownHeading],
    selector: &str,
    path: &str,
) -> Result<&'a MarkdownHeading> {
    let selector = selector.trim().trim_start_matches('#');
    let normalized = heading_slug(selector);
    let matches = headings
        .iter()
        .filter(|heading| {
            heading.id.eq_ignore_ascii_case(selector)
                || heading.text.eq_ignore_ascii_case(selector)
                || heading_slug(&heading.text) == normalized
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [heading] => Ok(heading),
        [] => bail!("文档 {path} 中不存在标题或 section_id：{selector}"),
        _ => bail!("文档 {path} 中标题“{selector}”不唯一，请改用 section_id"),
    }
}

fn heading_slug(value: &str) -> String {
    let mut output = String::new();
    let mut pending_dash = false;
    for character in value.trim().to_lowercase().chars() {
        if character.is_alphanumeric() || (!character.is_ascii() && !character.is_whitespace()) {
            if pending_dash && !output.is_empty() {
                output.push('-');
            }
            output.push(character);
            pending_dash = false;
        } else if character.is_whitespace() || character == '-' || character == '_' {
            pending_dash = true;
        }
    }
    output
}

fn normalize(value: &str) -> String {
    value.trim().replace('\\', "/").to_ascii_lowercase()
}

fn default_include_children() -> bool {
    true
}

#[cfg(test)]
#[path = "project_document_section_reader_tests.rs"]
mod tests;

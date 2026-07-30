// server/src/project_docs_scan.rs

//! Shared Markdown discovery for project documentation surfaces.
use anyhow::{anyhow, Result};
use homecli_proto::{ProjectDocumentEntry, ProjectDocumentsSnapshot};
use ignore::WalkBuilder;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    path::{Path as FsPath, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    project_default_docs::{default_project_documents, ensure_default_docs_in_workspace},
    project_document_index::{file_modified_millis, ProjectDocumentIndex},
    project_document_maintenance::enrich_catalog,
    project_document_policy::classify_project_document,
};

const MAX_DOCUMENTS: usize = 48;
const MAX_CATALOG_DOCUMENTS: usize = 20_000;
const MAX_DOC_CHARS: usize = 24_000;
const MAX_TOTAL_CHARS: usize = 220_000;

const ROOT_DOCS: &[&str] = &[
    "AGENTS.md",
    "CODEX.md",
    "CLAUDE.md",
    "GEMINI.md",
    "README.md",
    "README.zh-CN.md",
    "README_CN.md",
    "AI.md",
    "AI_AGENT.md",
    "AI_AGENTS.md",
    "AI_CURRENT.md",
    "AI_PROJECT.md",
    "AI_ARCHITECTURE.md",
    "AI_INDEX.md",
    "AI_RULES.md",
    "AI_TASK_TEMPLATE.md",
];

const DOC_DIRS: &[(&str, usize, usize)] = &[
    (".github/instructions", 100, 4),
    (".github/prompts", 120, 4),
    (".github/agents", 130, 4),
    (".github/skills", 140, 5),
    ("docs", 300, 3),
    ("documentation", 310, 3),
];

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProjectDocumentScanOptions {
    pub seed_missing_defaults: bool,
    pub catalog_only: bool,
    /// Run full quality, maintenance and federation analysis. Metadata-only
    /// consumers such as bounded graph queries deliberately disable this.
    pub include_analysis: bool,
}

impl Default for ProjectDocumentScanOptions {
    fn default() -> Self {
        Self {
            seed_missing_defaults: false,
            catalog_only: false,
            include_analysis: true,
        }
    }
}

#[cfg(test)]
fn collect_project_documents(workspace: &FsPath) -> Result<ProjectDocumentsSnapshot> {
    collect_project_documents_with_options(workspace, ProjectDocumentScanOptions::default())
}

pub(crate) fn collect_project_documents_with_options(
    workspace: &FsPath,
    options: ProjectDocumentScanOptions,
) -> Result<ProjectDocumentsSnapshot> {
    let workspace_path = workspace.to_string_lossy().to_string();
    if !workspace.is_dir() {
        return Ok(build_snapshot(
            workspace_path,
            "platform_default",
            default_project_documents(),
            vec![
                "项目工作区不存在或服务器当前不可读取。".to_string(),
                "已加载平台默认项目文档，项目工作区可用后会优先显示同名仓库文档。".to_string(),
            ],
        ));
    }

    let mut warnings = Vec::new();
    seed_default_documents_if_requested(workspace, options, &mut warnings);
    let candidates = discover_document_candidates(workspace, options.catalog_only, &mut warnings);
    let mut documents = Vec::new();
    let mut total_chars = 0usize;
    let mut index = if options.catalog_only {
        match ProjectDocumentIndex::open(workspace) {
            Ok(index) => Some(index),
            Err(error) => {
                warnings.push(format!("持久文档索引暂不可用，已降级为完整扫描：{error}"));
                None
            }
        }
    } else {
        None
    };
    let mut seen_paths = HashSet::new();

    let document_limit = if options.catalog_only {
        MAX_CATALOG_DOCUMENTS
    } else {
        MAX_DOCUMENTS
    };
    for path in candidates.into_iter().take(document_limit) {
        let relative = relative_path(workspace, &path);
        seen_paths.insert(relative.clone());
        let file_size = fs::metadata(&path)
            .map(|metadata| metadata.len())
            .unwrap_or_default();
        let modified_at_ms = file_modified_millis(&path);
        let cached = index
            .as_ref()
            .and_then(|value| {
                value
                    .cached_document(&relative, file_size, modified_at_ms)
                    .ok()
            })
            .flatten()
            .map(|mut document| {
                document.content.clear();
                document.truncated = false;
                (document, 0)
            });
        match cached.map(Ok).unwrap_or_else(|| {
            read_document_snapshot(workspace, &path, total_chars, options.catalog_only)
        }) {
            Ok((document, used_chars)) => {
                total_chars += used_chars;
                if let Some(index) = index.as_mut() {
                    if let Err(error) = index.observe_document(&document, modified_at_ms) {
                        warnings.push(format!("无法更新 {} 的增量索引：{error}", relative));
                    }
                }
                documents.push(document);
                if !options.catalog_only && total_chars >= MAX_TOTAL_CHARS {
                    warnings.push(format!(
                        "文档频道已达到本次加载上限，{} 之后的文档会按需由 AI 再读取。",
                        relative
                    ));
                    break;
                }
            }
            Err(error) => warnings.push(format!("无法读取 {}：{}", relative, error)),
        }
    }

    // Existing projects must reflect their real workspace. Mixing absent platform
    // templates into a non-empty catalog creates phantom pages that cannot be
    // opened or edited. Empty workspaces still receive a read-only starter view;
    // editable user projects seed the same defaults before scanning.
    if documents.is_empty() {
        append_default_documents(&mut documents, &mut warnings);
    }
    if options.catalog_only {
        for document in &mut documents {
            document.content.clear();
            document.truncated = false;
        }
    }
    if let Some(index) = index.as_mut() {
        if let Err(error) = index.finish_scan(&seen_paths) {
            warnings.push(format!("无法完成文档删除事件对账：{error}"));
        }
    }
    let source = snapshot_source(&documents);
    let mut snapshot = build_snapshot(workspace_path, source, documents, warnings);
    if options.include_analysis {
        if let Some(index) = index.as_ref() {
            match enrich_catalog(workspace, &snapshot.documents, index) {
                Ok(analysis) => snapshot.analysis = analysis,
                Err(error) => snapshot
                    .warnings
                    .push(format!("文档健康分析暂不可用：{error}")),
            }
        }
    }
    Ok(snapshot)
}

fn seed_default_documents_if_requested(
    workspace: &FsPath,
    options: ProjectDocumentScanOptions,
    warnings: &mut Vec<String>,
) {
    if !options.seed_missing_defaults {
        return;
    }
    match ensure_default_docs_in_workspace(workspace) {
        Ok(0) => {}
        Ok(created) => warnings.push(format!(
            "已为项目补齐 {created} 份缺失的默认 AI 指令文档；已有同名文件未被覆盖。"
        )),
        Err(error) => warnings.push(format!(
            "无法自动补齐默认 AI 指令文档，已改为仅展示可读取文档：{error}"
        )),
    }
}

fn discover_document_candidates(
    workspace: &FsPath,
    catalog_only: bool,
    warnings: &mut Vec<String>,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for relative in ROOT_DOCS {
        push_if_markdown(workspace, relative, &mut candidates);
    }
    push_if_markdown(
        workspace,
        ".github/copilot-instructions.md",
        &mut candidates,
    );
    for (dir, priority, depth) in DOC_DIRS {
        collect_markdown_dir(&workspace.join(dir), *priority, *depth, &mut candidates);
    }
    if catalog_only {
        for entry in WalkBuilder::new(workspace)
            .hidden(false)
            .git_ignore(true)
            .filter_entry(|entry| !entry.path().is_dir() || !should_skip_dir(entry.path()))
            .build()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
            .filter(|entry| is_markdown_file(entry.path()))
            .take(MAX_CATALOG_DOCUMENTS + 1)
        {
            let relative = relative_path(workspace, entry.path());
            candidates.push((root_priority(&relative), entry.into_path()));
        }
    }

    candidates.sort_by_key(|(priority, path)| (*priority, relative_path(workspace, path)));
    let mut seen = HashSet::new();
    let mut unique = Vec::new();
    for (_priority, path) in candidates {
        let relative = relative_path(workspace, &path);
        if seen.insert(relative) {
            unique.push(path);
        }
    }
    let limit = if catalog_only {
        MAX_CATALOG_DOCUMENTS
    } else {
        MAX_DOCUMENTS
    };
    if unique.len() > limit {
        warnings.push(format!(
            "发现 {} 份项目文档，本次优先展示前 {} 份。",
            unique.len(),
            limit
        ));
    }
    unique
}

fn append_default_documents(documents: &mut Vec<ProjectDocumentEntry>, warnings: &mut Vec<String>) {
    let existing = documents
        .iter()
        .map(|doc| normalized_doc_path(&doc.path))
        .collect::<HashSet<_>>();
    let missing_defaults = default_project_documents()
        .into_iter()
        .filter(|doc| !existing.contains(&normalized_doc_path(&doc.path)))
        .collect::<Vec<_>>();

    if missing_defaults.is_empty() {
        return;
    }

    if documents.is_empty() {
        warnings.push("当前项目尚未创建自定义文档，已加载平台默认项目文档。".to_string());
    } else {
        warnings
            .push("已补充缺失的平台默认 AI 指令文档；项目内同名 Markdown 会优先显示。".to_string());
    }
    documents.extend(missing_defaults);
}

fn normalized_doc_path(path: &str) -> String {
    path.replace('\\', "/").to_ascii_lowercase()
}

fn push_if_markdown(workspace: &FsPath, relative: &str, candidates: &mut Vec<(usize, PathBuf)>) {
    let path = workspace.join(relative);
    if path.is_file() && is_markdown_file(&path) {
        candidates.push((root_priority(relative), path));
    }
}

fn collect_markdown_dir(
    dir: &FsPath,
    priority: usize,
    max_depth: usize,
    candidates: &mut Vec<(usize, PathBuf)>,
) {
    if max_depth == 0 || !dir.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            if should_skip_dir(&path) {
                continue;
            }
            collect_markdown_dir(&path, priority + 1, max_depth - 1, candidates);
        } else if is_markdown_file(&path) {
            candidates.push((priority, path));
        }
    }
}

fn read_document_snapshot(
    workspace: &FsPath,
    path: &FsPath,
    total_chars: usize,
    catalog_only: bool,
) -> Result<(ProjectDocumentEntry, usize)> {
    let raw = fs::read_to_string(path)?;
    let remaining = if catalog_only {
        usize::MAX
    } else {
        MAX_TOTAL_CHARS.saturating_sub(total_chars)
    };
    if !catalog_only && remaining == 0 {
        return Err(anyhow!("已达到文档频道总加载上限"));
    }
    let limit = MAX_DOC_CHARS.min(remaining);
    let raw_char_count = raw.chars().count();
    let truncated = !catalog_only && raw_char_count > limit;
    let content: String = if catalog_only {
        String::new()
    } else {
        raw.chars().take(limit).collect()
    };
    let used_chars = content.chars().count();
    let path_label = relative_path(workspace, path);
    let metadata = classify_project_document(&path_label, &raw, raw_char_count);
    let title = markdown_title(&raw).unwrap_or_else(|| {
        path.file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("项目文档")
            .to_string()
    });
    Ok((
        ProjectDocumentEntry {
            path: path_label,
            title,
            content,
            truncated,
            byte_len: raw.len() as u64,
            source: "workspace".to_string(),
            metadata,
        },
        used_chars,
    ))
}

pub(crate) fn build_snapshot(
    workspace_path: String,
    source: &str,
    documents: Vec<ProjectDocumentEntry>,
    warnings: Vec<String>,
) -> ProjectDocumentsSnapshot {
    let revision = compute_revision(source, &workspace_path, &documents, &warnings);
    ProjectDocumentsSnapshot {
        workspace_path,
        revision,
        source: source.to_string(),
        generated_at_ms: now_millis(),
        documents,
        warnings,
        analysis: serde_json::Value::Null,
    }
}

fn snapshot_source(documents: &[ProjectDocumentEntry]) -> &'static str {
    let has_workspace = documents.iter().any(|doc| doc.source == "workspace");
    let has_default = documents.iter().any(|doc| doc.source == "platform_default");
    match (has_workspace, has_default) {
        (true, true) => "workspace_with_defaults",
        (true, false) => "workspace",
        (false, true) => "platform_default",
        (false, false) => "empty",
    }
}

fn compute_revision(
    source: &str,
    workspace_path: &str,
    documents: &[ProjectDocumentEntry],
    warnings: &[String],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    hasher.update([0]);
    hasher.update(workspace_path.as_bytes());
    hasher.update([0]);
    for doc in documents {
        hasher.update(doc.path.as_bytes());
        hasher.update([0]);
        hasher.update(doc.title.as_bytes());
        hasher.update([0]);
        hasher.update(doc.source.as_bytes());
        hasher.update([0]);
        hasher.update(doc.byte_len.to_le_bytes());
        hasher.update([doc.truncated as u8]);
        hasher.update(doc.content.as_bytes());
        hasher.update(doc.metadata.content_hash.as_bytes());
        hasher.update(doc.metadata.role.as_bytes());
        hasher.update(doc.metadata.lifecycle.as_bytes());
        hasher.update(doc.metadata.authority.as_bytes());
        hasher.update([doc.metadata.default_retrieval as u8]);
        hasher.update([0]);
    }
    for warning in warnings {
        hasher.update(warning.as_bytes());
        hasher.update([0]);
    }
    format!("{:x}", hasher.finalize())
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

fn markdown_title(content: &str) -> Option<String> {
    content.lines().find_map(|line| {
        let trimmed = line.trim();
        trimmed
            .strip_prefix("# ")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn root_priority(relative: &str) -> usize {
    match relative.replace('\\', "/").as_str() {
        "AGENTS.md" => 0,
        ".github/copilot-instructions.md" => 1,
        "CODEX.md" => 5,
        "CLAUDE.md" => 6,
        "GEMINI.md" => 7,
        "AI_CURRENT.md" => 19,
        "AI_PROJECT.md" => 20,
        "AI_ARCHITECTURE.md" => 21,
        "AI_INDEX.md" => 22,
        "AI_RULES.md" => 23,
        "AI_TASK_TEMPLATE.md" => 24,
        "README.md" => 10,
        "README.zh-CN.md" | "README_CN.md" => 11,
        path if path.starts_with(".github/instructions/") => 100,
        path if path.starts_with(".github/prompts/") => 120,
        path if path.starts_with(".github/agents/") => 130,
        path if path.starts_with(".github/skills/") => 140,
        path if path.starts_with("docs/") => 300,
        _ => 500,
    }
}

fn is_markdown_file(path: &FsPath) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "md" | "markdown" | "mdown"
            )
        })
        .unwrap_or(false)
}

fn should_skip_dir(path: &FsPath) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| {
            matches!(
                name,
                ".git" | ".gradle" | "build" | "node_modules" | "target"
            )
        })
        .unwrap_or(false)
}

fn relative_path(workspace: &FsPath, path: &FsPath) -> String {
    path.strip_prefix(workspace)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
#[path = "project_docs_scan_tests.rs"]
mod tests;

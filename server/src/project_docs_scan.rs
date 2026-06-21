//! Shared Markdown discovery for project documentation surfaces.
use anyhow::{anyhow, Result};
use homecli_proto::{ProjectDocumentEntry, ProjectDocumentsSnapshot};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    path::{Path as FsPath, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::project_default_docs::{default_project_documents, ensure_default_docs_in_workspace};

const MAX_DOCUMENTS: usize = 48;
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
];

const DOC_DIRS: &[(&str, usize, usize)] = &[
    (".github/instructions", 100, 4),
    (".github/prompts", 120, 4),
    (".github/agents", 130, 4),
    (".github/skills", 140, 5),
    ("docs", 300, 3),
];

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ProjectDocumentScanOptions {
    pub seed_missing_defaults: bool,
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
    let candidates = discover_document_candidates(workspace, &mut warnings);
    let mut documents = Vec::new();
    let mut total_chars = 0usize;

    for path in candidates.into_iter().take(MAX_DOCUMENTS) {
        let relative = relative_path(workspace, &path);
        match read_document_snapshot(workspace, &path, total_chars) {
            Ok((document, used_chars)) => {
                total_chars += used_chars;
                documents.push(document);
                if total_chars >= MAX_TOTAL_CHARS {
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

    append_default_documents(&mut documents, &mut warnings);
    let source = snapshot_source(&documents);

    Ok(build_snapshot(workspace_path, source, documents, warnings))
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

fn discover_document_candidates(workspace: &FsPath, warnings: &mut Vec<String>) -> Vec<PathBuf> {
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
        collect_markdown_dir(
            workspace,
            &workspace.join(dir),
            *priority,
            *depth,
            &mut candidates,
        );
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
    if unique.len() > MAX_DOCUMENTS {
        warnings.push(format!(
            "发现 {} 份项目文档，本次优先展示前 {} 份。",
            unique.len(),
            MAX_DOCUMENTS
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
    workspace: &FsPath,
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
            collect_markdown_dir(workspace, &path, priority + 1, max_depth - 1, candidates);
        } else if is_markdown_file(&path) {
            candidates.push((priority, path));
        }
    }
}

fn read_document_snapshot(
    workspace: &FsPath,
    path: &FsPath,
    total_chars: usize,
) -> Result<(ProjectDocumentEntry, usize)> {
    let raw = fs::read_to_string(path)?;
    let remaining = MAX_TOTAL_CHARS.saturating_sub(total_chars);
    if remaining == 0 {
        return Err(anyhow!("已达到文档频道总加载上限"));
    }
    let limit = MAX_DOC_CHARS.min(remaining);
    let truncated = raw.chars().count() > limit;
    let content: String = raw.chars().take(limit).collect();
    let used_chars = content.chars().count();
    let path_label = relative_path(workspace, path);
    let title = markdown_title(&content).unwrap_or_else(|| {
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
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn project_docs_collects_agent_and_instruction_docs_first() {
        let root = std::env::temp_dir().join(format!(
            "elon-project-docs-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join(".github/instructions")).unwrap();
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join("AGENTS.md"), "# Agent Rules\nread me").unwrap();
        fs::write(root.join("CODEX.md"), "# Codex Rules\nfollow me").unwrap();
        fs::write(
            root.join(".github/instructions/git.instructions.md"),
            "# Git Workflow\ncommit and push",
        )
        .unwrap();
        fs::write(root.join("docs/guide.md"), "# User Guide\nhello").unwrap();

        let snapshot = collect_project_documents(&root).unwrap();
        let _ = fs::remove_dir_all(&root);

        let paths = snapshot
            .documents
            .iter()
            .map(|doc| doc.path.as_str())
            .collect::<Vec<_>>();
        assert_eq!(paths[0], "AGENTS.md");
        assert_eq!(paths[1], "CODEX.md");
        assert!(paths.contains(&".github/instructions/git.instructions.md"));
        assert!(paths.contains(&"docs/guide.md"));
        assert!(paths.contains(&".github/copilot-instructions.md"));
        assert!(!snapshot.revision.is_empty());
        assert_eq!(snapshot.source, "workspace_with_defaults");
    }

    #[test]
    fn project_docs_returns_default_docs_for_empty_workspace() {
        let root = std::env::temp_dir().join(format!(
            "elon-project-docs-empty-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();

        let snapshot = collect_project_documents(&root).unwrap();
        let _ = fs::remove_dir_all(&root);

        let paths = snapshot
            .documents
            .iter()
            .map(|doc| doc.path.as_str())
            .collect::<Vec<_>>();
        assert!(paths.contains(&"AGENTS.md"));
        assert!(paths.contains(&"CODEX.md"));
        assert!(paths.contains(&".github/copilot-instructions.md"));
        assert!(paths.contains(&".github/instructions/project-workflow.instructions.md"));
        assert!(paths.contains(&".github/instructions/git-workflow.instructions.md"));
        assert!(paths.contains(&".github/instructions/android.instructions.md"));
        assert!(paths.contains(&".github/instructions/ui.instructions.md"));
        assert!(paths.contains(&".github/instructions/backend.instructions.md"));
        assert!(paths.contains(&"CLAUDE.md"));
        assert!(paths.contains(&"GEMINI.md"));
        assert!(snapshot
            .warnings
            .iter()
            .any(|warning| warning.contains("默认项目文档")));
        assert!(!snapshot.revision.is_empty());
        assert_eq!(snapshot.source, "platform_default");
    }

    #[test]
    fn project_docs_can_seed_missing_default_docs() {
        let root = std::env::temp_dir().join(format!(
            "elon-project-docs-seed-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();

        let snapshot = collect_project_documents_with_options(
            &root,
            ProjectDocumentScanOptions {
                seed_missing_defaults: true,
            },
        )
        .unwrap();
        let agents = fs::read_to_string(root.join("AGENTS.md")).unwrap();
        let manifest = fs::read_to_string(root.join(".elon/default-docs.json")).unwrap();
        let _ = fs::remove_dir_all(&root);

        assert!(agents.contains(".github/copilot-instructions.md"));
        assert!(manifest.contains("copilot-primary-bridged-agents"));
        assert!(snapshot
            .warnings
            .iter()
            .any(|warning| warning.contains("补齐")));
        assert_eq!(snapshot.source, "workspace_with_defaults");
    }

    #[test]
    fn project_docs_revision_changes_when_content_changes() {
        let root = std::env::temp_dir().join(format!(
            "elon-project-docs-revision-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("AGENTS.md"), "# Agent Rules\none").unwrap();
        let first = collect_project_documents(&root).unwrap();
        fs::write(root.join("AGENTS.md"), "# Agent Rules\ntwo").unwrap();
        let second = collect_project_documents(&root).unwrap();
        let _ = fs::remove_dir_all(&root);

        assert_ne!(first.revision, second.revision);
    }
}

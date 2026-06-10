//! Shared Markdown discovery for project documentation surfaces.
use anyhow::{anyhow, Result};
use homecli_proto::{ProjectDocumentEntry, ProjectDocumentsSnapshot};
use std::{
    collections::HashSet,
    fs,
    path::{Path as FsPath, PathBuf},
};

const MAX_DOCUMENTS: usize = 48;
const MAX_DOC_CHARS: usize = 24_000;
const MAX_TOTAL_CHARS: usize = 220_000;

const ROOT_DOCS: &[&str] = &[
    "AGENTS.md",
    "CODEX.md",
    "README.md",
    "README.zh-CN.md",
    "README_CN.md",
    "CLAUDE.md",
    "GEMINI.md",
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

pub(crate) fn collect_project_documents(workspace: &FsPath) -> Result<ProjectDocumentsSnapshot> {
    let workspace_path = workspace.to_string_lossy().to_string();
    if !workspace.is_dir() {
        return Ok(ProjectDocumentsSnapshot {
            workspace_path,
            documents: Vec::new(),
            warnings: vec!["项目工作区不存在或服务器当前不可读取。".to_string()],
        });
    }

    let mut warnings = Vec::new();
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

    if documents.is_empty() {
        warnings
            .push("没有发现 AGENTS/CODEX/README、GitHub 指令或 docs Markdown 文档。".to_string());
    }

    Ok(ProjectDocumentsSnapshot {
        workspace_path,
        documents,
        warnings,
    })
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
        },
        used_chars,
    ))
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
        "CODEX.md" => 5,
        "README.md" => 10,
        ".github/copilot-instructions.md" => 20,
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
    }
}

//! Safe, project-relative Markdown reads and optimistic atomic writes.

use anyhow::{anyhow, bail, Context, Result};
use sha2::{Digest, Sha256};
use std::path::{Component, Path, PathBuf};

const MAX_DOCUMENT_BYTES: usize = 2 * 1024 * 1024;
const PROJECT_DOCUMENT_CONFIG_PATHS: &[&str] = &[
    ".elon/document-sections.json",
    ".elon/document-organization-suggestions.json",
    ".elon/knowledge-federation.json",
    ".elon/discussion-graph.json",
    ".elon/discussion-graph-suggestions.json",
    ".elon/project-features.json",
];

#[derive(Debug)]
pub(crate) struct ProjectDocumentFile {
    pub(crate) path: String,
    pub(crate) content: String,
    pub(crate) revision: String,
    pub(crate) byte_len: u64,
}

#[derive(Debug)]
pub(crate) struct ProjectDocumentWriteError {
    pub(crate) message: String,
    pub(crate) conflict: bool,
}

impl std::fmt::Display for ProjectDocumentWriteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

pub(crate) fn read_project_document_file(
    workspace: &Path,
    document_path: &str,
) -> Result<ProjectDocumentFile> {
    let (path, relative) = resolve_existing_markdown(workspace, document_path)?;
    let bytes = std::fs::read(&path).with_context(|| format!("无法读取项目文档 {}", relative))?;
    if bytes.len() > MAX_DOCUMENT_BYTES {
        bail!("项目文档超过 2 MiB 编辑上限：{}", relative);
    }
    let content = String::from_utf8(bytes).context("项目文档不是 UTF-8 文本")?;
    Ok(ProjectDocumentFile {
        path: relative,
        revision: content_revision(&content),
        byte_len: content.len() as u64,
        content,
    })
}

pub(crate) fn write_project_document_file(
    workspace: &Path,
    document_path: &str,
    content: &str,
    expected_revision: Option<&str>,
) -> std::result::Result<ProjectDocumentFile, ProjectDocumentWriteError> {
    if content.len() > MAX_DOCUMENT_BYTES {
        return Err(write_error("项目文档超过 2 MiB 编辑上限", false));
    }
    let (target, relative) = resolve_markdown_target(workspace, document_path, true)
        .map_err(|error| write_error(error.to_string(), false))?;

    let current_content = if target.is_file() {
        let current = std::fs::read_to_string(&target)
            .map_err(|error| write_error(format!("无法读取当前文档：{error}"), false))?;
        Some(current)
    } else {
        None
    };
    let current_revision = current_content.as_deref().map(content_revision);
    if let Some(expected) = expected_revision.filter(|value| !value.trim().is_empty()) {
        if current_revision.as_deref() != Some(expected) {
            return Err(write_error("文档已被其他会话修改，请刷新后合并更改", true));
        }
    }

    crate::node_agent_atomic_file::write(&target, content.as_bytes())
        .map_err(|error| write_error(error.to_string(), false))?;
    if let Err(error) = crate::project_document_vault::checkpoint_after_write(workspace, &relative)
    {
        let rollback = match current_content {
            Some(previous) => crate::node_agent_atomic_file::write(&target, previous.as_bytes()),
            None => std::fs::remove_file(&target).map_err(Into::into),
        };
        return Err(write_error(
            format!(
                "托管知识库自动保存失败：{error}{}",
                rollback
                    .err()
                    .map(|rollback| format!("；回滚也失败：{rollback}"))
                    .unwrap_or_default()
            ),
            false,
        ));
    }
    Ok(ProjectDocumentFile {
        path: relative,
        content: content.to_string(),
        revision: content_revision(content),
        byte_len: content.len() as u64,
    })
}

pub(crate) fn move_project_document_file(
    workspace: &Path,
    source_path: &str,
    target_path: &str,
    expected_source_revision: &str,
) -> std::result::Result<ProjectDocumentFile, ProjectDocumentWriteError> {
    if !is_markdown_document_path(source_path) || !is_markdown_document_path(target_path) {
        return Err(write_error(
            "实体整理只允许移动或重命名 Markdown 文件",
            false,
        ));
    }
    let current = read_project_document_file(workspace, source_path)
        .map_err(|error| write_error(error.to_string(), false))?;
    if current.revision != expected_source_revision {
        return Err(write_error(
            "源文档已被其他会话修改，请重新分析后再执行实体整理",
            true,
        ));
    }
    let (source, source_relative) = resolve_existing_markdown(workspace, source_path)
        .map_err(|error| write_error(error.to_string(), false))?;
    let (target, target_relative) = resolve_markdown_target(workspace, target_path, true)
        .map_err(|error| write_error(error.to_string(), false))?;
    if source_relative.eq_ignore_ascii_case(&target_relative) {
        return Err(write_error("源路径和目标路径不能相同或仅大小写不同", false));
    }
    if target.exists() {
        return Err(write_error(
            format!("目标文档已存在，禁止覆盖：{target_relative}"),
            true,
        ));
    }
    std::fs::rename(&source, &target).map_err(|error| {
        write_error(
            format!("无法把 {source_relative} 移动到 {target_relative}：{error}"),
            false,
        )
    })?;
    if let Err(error) =
        crate::project_document_vault::checkpoint_after_write(workspace, &target_relative)
    {
        let rollback = std::fs::rename(&target, &source).err();
        return Err(write_error(
            format!(
                "托管知识库自动保存失败：{error}{}",
                rollback
                    .map(|rollback| format!("；文件移动回滚也失败：{rollback}"))
                    .unwrap_or_default()
            ),
            false,
        ));
    }
    Ok(ProjectDocumentFile {
        path: target_relative,
        content: current.content,
        revision: current.revision,
        byte_len: current.byte_len,
    })
}

fn resolve_existing_markdown(workspace: &Path, document_path: &str) -> Result<(PathBuf, String)> {
    let (target, relative) = resolve_markdown_target(workspace, document_path, false)?;
    if !target.is_file() {
        bail!("项目文档不存在：{}", relative);
    }
    let canonical_root = std::fs::canonicalize(workspace)
        .with_context(|| format!("无法解析项目工作区 {}", workspace.display()))?;
    let canonical_target =
        std::fs::canonicalize(&target).with_context(|| format!("无法解析项目文档 {}", relative))?;
    if !canonical_target.starts_with(&canonical_root) {
        bail!("项目文档路径越过工作区边界");
    }
    Ok((canonical_target, relative))
}

fn resolve_markdown_target(
    workspace: &Path,
    document_path: &str,
    create_parent: bool,
) -> Result<(PathBuf, String)> {
    if !workspace.is_dir() {
        bail!("项目工作区不存在：{}", workspace.display());
    }
    let normalized = document_path.trim().replace('\\', "/");
    let relative_path = Path::new(&normalized);
    if normalized.is_empty()
        || relative_path.is_absolute()
        || relative_path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        bail!("项目文档必须使用工作区内的相对路径");
    }
    let extension = relative_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let normalized_lower = normalized.to_ascii_lowercase();
    if !matches!(extension.as_str(), "md" | "markdown" | "mdown")
        && !PROJECT_DOCUMENT_CONFIG_PATHS.contains(&normalized_lower.as_str())
    {
        bail!("项目文档只允许 Markdown，或受控的 .elon 文档分区配置");
    }
    let canonical_root = std::fs::canonicalize(workspace)
        .with_context(|| format!("无法解析项目工作区 {}", workspace.display()))?;
    let target = canonical_root.join(relative_path);
    if std::fs::symlink_metadata(&target)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        bail!("项目文档不能是符号链接");
    }
    let parent = target
        .parent()
        .ok_or_else(|| anyhow!("项目文档缺少父目录"))?;
    let mut existing_ancestor = parent;
    while !existing_ancestor.exists() {
        existing_ancestor = existing_ancestor
            .parent()
            .ok_or_else(|| anyhow!("无法确认项目文档目录边界"))?;
    }
    let canonical_ancestor = std::fs::canonicalize(existing_ancestor)
        .with_context(|| format!("无法解析项目文档目录 {}", existing_ancestor.display()))?;
    if !canonical_ancestor.starts_with(&canonical_root) {
        bail!("项目文档路径越过工作区边界");
    }
    if create_parent {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("无法创建项目文档目录 {}", parent.display()))?;
        let canonical_parent = std::fs::canonicalize(parent)
            .with_context(|| format!("无法解析项目文档目录 {}", parent.display()))?;
        if !canonical_parent.starts_with(&canonical_root) {
            bail!("项目文档路径越过工作区边界");
        }
    }
    Ok((target, normalized))
}

pub(crate) fn content_revision(content: &str) -> String {
    format!("{:x}", Sha256::digest(content.as_bytes()))
}

fn is_markdown_document_path(value: &str) -> bool {
    Path::new(value.trim())
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "md" | "markdown" | "mdown"
            )
        })
        .unwrap_or(false)
}

fn write_error(message: impl Into<String>, conflict: bool) -> ProjectDocumentWriteError {
    ProjectDocumentWriteError {
        message: message.into(),
        conflict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_markdown_and_rejects_stale_revision() {
        let root =
            std::env::temp_dir().join(format!("elon-project-doc-file-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let first =
            write_project_document_file(&root, "docs/inbox/note.md", "# first\n", None).unwrap();
        let second = write_project_document_file(
            &root,
            "docs/inbox/note.md",
            "# second\n",
            Some(&first.revision),
        )
        .unwrap();
        let stale = write_project_document_file(
            &root,
            "docs/inbox/note.md",
            "# stale\n",
            Some(&first.revision),
        )
        .unwrap_err();
        assert!(stale.conflict);
        assert_eq!(
            read_project_document_file(&root, "docs/inbox/note.md")
                .unwrap()
                .revision,
            second.revision
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_paths_outside_workspace() {
        let root = std::env::temp_dir().join(format!(
            "elon-project-doc-boundary-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        assert!(write_project_document_file(&root, "../outside.md", "bad", None).is_err());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn allows_only_controlled_document_json_files() {
        let root =
            std::env::temp_dir().join(format!("elon-project-doc-config-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let config = write_project_document_file(
            &root,
            ".elon/document-sections.json",
            "{\"version\":1}\n",
            None,
        )
        .unwrap();
        assert_eq!(config.path, ".elon/document-sections.json");
        let suggestions = write_project_document_file(
            &root,
            ".elon/document-organization-suggestions.json",
            "{\"status\":\"requested\"}\n",
            None,
        )
        .unwrap();
        assert_eq!(
            suggestions.path,
            ".elon/document-organization-suggestions.json"
        );
        let discussion = write_project_document_file(
            &root,
            ".elon/discussion-graph.json",
            "{\"version\":1}\n",
            None,
        )
        .unwrap();
        assert_eq!(discussion.path, ".elon/discussion-graph.json");
        assert!(write_project_document_file(&root, ".elon/untrusted.json", "{}", None,).is_err());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn missing_config_read_does_not_create_elon_directory() {
        let root = std::env::temp_dir().join(format!(
            "elon-project-doc-read-only-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        assert!(read_project_document_file(&root, ".elon/document-sections.json").is_err());
        assert!(!root.join(".elon").exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn moves_markdown_without_overwriting_or_accepting_stale_content() {
        let root =
            std::env::temp_dir().join(format!("elon-project-doc-move-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("docs/inbox")).unwrap();
        let source =
            write_project_document_file(&root, "docs/inbox/1.md", "# Useful note\n", None).unwrap();
        let moved = move_project_document_file(
            &root,
            "docs/inbox/1.md",
            "docs/current/useful-note.md",
            &source.revision,
        )
        .unwrap();
        assert_eq!(moved.path, "docs/current/useful-note.md");
        assert!(!root.join("docs/inbox/1.md").exists());
        assert!(root.join("docs/current/useful-note.md").is_file());
        assert!(
            move_project_document_file(
                &root,
                "docs/current/useful-note.md",
                "docs/current/other.md",
                "stale",
            )
            .unwrap_err()
            .conflict
        );
        let _ = std::fs::remove_dir_all(root);
    }
}

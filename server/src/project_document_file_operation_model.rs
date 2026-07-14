//! Structured, vendor-neutral schema and validation for proposed Markdown path operations.

use anyhow::{bail, Result};
use homecli_proto::ProjectDocumentEntry;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SuggestedFileOperationKind {
    Rename,
    Move,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SuggestedFileOperationStatus {
    Proposed,
    Applied,
}

impl Default for SuggestedFileOperationStatus {
    fn default() -> Self {
        Self::Proposed
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SuggestedFileOperation {
    pub id: String,
    pub kind: SuggestedFileOperationKind,
    pub source_path: String,
    pub target_path: String,
    pub source_revision: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub status: SuggestedFileOperationStatus,
}

pub(crate) fn normalize_file_operations(
    operations: Vec<SuggestedFileOperation>,
) -> Result<Vec<SuggestedFileOperation>> {
    let mut ids = HashSet::new();
    operations
        .into_iter()
        .map(|operation| {
            let id = normalize_operation_id(&operation.id)?;
            if !ids.insert(id.clone()) {
                bail!("AI 文档文件操作包含重复 id：{id}");
            }
            let source_path = normalize_document_path(&operation.source_path)?;
            let target_path = normalize_document_path(&operation.target_path)?;
            if source_path.eq_ignore_ascii_case(&target_path) {
                bail!("文档文件操作的源路径和目标路径不能相同：{source_path}");
            }
            Ok(SuggestedFileOperation {
                id,
                kind: operation.kind,
                source_path,
                target_path,
                source_revision: truncate_chars(operation.source_revision.trim(), 128),
                reason: truncate_chars(operation.reason.trim(), 500),
                status: operation.status,
            })
        })
        .collect()
}

pub(crate) fn validate_file_operations(
    operations: &[SuggestedFileOperation],
    documents: &[ProjectDocumentEntry],
    known_paths: &HashSet<String>,
) -> Result<()> {
    let known_documents = documents
        .iter()
        .map(|document| {
            (
                document.path.replace('\\', "/").to_ascii_lowercase(),
                document.metadata.content_hash.as_str(),
            )
        })
        .collect::<HashMap<_, _>>();
    for operation in operations {
        if operation.status != SuggestedFileOperationStatus::Proposed {
            bail!("新保存的文件操作必须为 proposed：{}", operation.id);
        }
        let source_key = operation.source_path.to_ascii_lowercase();
        let target_key = operation.target_path.to_ascii_lowercase();
        let Some(content_hash) = known_documents.get(&source_key) else {
            bail!(
                "文件操作引用了目录中不存在的文档：{}",
                operation.source_path
            );
        };
        if known_paths.contains(&target_key) {
            bail!("文件操作目标已存在：{}", operation.target_path);
        }
        if operation.source_revision.is_empty()
            || operation.source_revision.as_str() != *content_hash
        {
            bail!(
                "文件操作必须使用 analyze 返回的最新 source_revision：{}",
                operation.source_path
            );
        }
    }
    Ok(())
}

pub(crate) fn normalize_document_path(value: &str) -> Result<String> {
    let path = value.trim().replace('\\', "/");
    if path.is_empty()
        || path.starts_with('/')
        || path.contains(':')
        || path
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
    {
        bail!("文档归类必须使用工作区内的规范相对路径");
    }
    Ok(path)
}

fn normalize_operation_id(value: &str) -> Result<String> {
    let value = truncate_chars(value.trim(), 80);
    if value.is_empty()
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        bail!("文件操作 id 只能包含字母、数字、点、下划线和连字符");
    }
    Ok(value)
}

fn truncate_chars(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

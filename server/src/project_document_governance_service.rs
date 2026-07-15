//! Local workspace service shared by the project-document MCP tools.

use anyhow::{anyhow, bail, Context, Result};
use homecli_proto::{ProjectDocumentEntry, ProjectDocumentsSnapshot};
use serde::Serialize;
use serde_json::{json, Value};
use std::{collections::HashSet, path::Path};

use crate::{
    project_docs_scan::{collect_project_documents_with_options, ProjectDocumentScanOptions},
    project_document_authorization::{authorize_document_apply, DocumentAutomationMode},
    project_document_files::{
        read_project_document_file, write_project_document_file, ProjectDocumentFile,
    },
    project_document_git_transaction::{commit_document_baseline, commit_document_result},
    project_document_governance::{
        apply_suggestions, effective_section, parse_manifest, parse_suggestions, to_pretty_json,
        validate_ready_suggestions, DocumentOrganizationSuggestions, DocumentSectionManifest,
        SECTION_CONFIG_PATH, SUGGESTIONS_CONFIG_PATH,
    },
};

const DEFAULT_PAGE_SIZE: usize = 80;
const MAX_PAGE_SIZE: usize = 200;
const DEFAULT_READ_CHARS: usize = 6_000;
const MAX_READ_CHARS_PER_DOCUMENT: usize = 24_000;
const MAX_READ_DOCUMENTS: usize = 12;
const MAX_READ_TOTAL_CHARS: usize = 48_000;

#[derive(Debug, Clone)]
pub(crate) struct GovernanceFile<T> {
    pub value: T,
    pub revision: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct CompactDocument<'a> {
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

pub(crate) fn analyze_workspace(
    workspace: &Path,
    offset: usize,
    limit: usize,
    ambiguous_only: bool,
) -> Result<Value> {
    let snapshot = catalog(workspace)?;
    let manifest = load_manifest(workspace)?;
    let suggestions = load_suggestions(workspace)?;
    let limit = limit.clamp(1, MAX_PAGE_SIZE);
    let candidates = snapshot
        .documents
        .iter()
        .filter(|document| !ambiguous_only || document.metadata.ambiguous)
        .collect::<Vec<_>>();
    let documents = candidates
        .iter()
        .skip(offset)
        .take(limit)
        .map(|document| compact_document(document, &manifest.value))
        .collect::<Vec<_>>();
    let full_tokens = snapshot
        .documents
        .iter()
        .map(|document| document.metadata.token_estimate)
        .sum::<u64>();
    let default_tokens = snapshot
        .documents
        .iter()
        .filter(|document| document.metadata.default_retrieval)
        .map(|document| document.metadata.token_estimate)
        .sum::<u64>();
    let ambiguous_documents = snapshot
        .documents
        .iter()
        .filter(|document| document.metadata.ambiguous)
        .count();
    let excluded_by_default = snapshot
        .documents
        .iter()
        .filter(|document| !document.metadata.default_retrieval)
        .count();
    Ok(json!({
        "workspace": snapshot.workspace_path,
        "catalog_revision": snapshot.revision,
        "source": snapshot.source,
        "warnings": snapshot.warnings,
        "pagination": {
            "offset": offset,
            "limit": limit,
            "returned": documents.len(),
            "matching_documents": candidates.len(),
            "next_offset": (offset + documents.len() < candidates.len()).then_some(offset + documents.len()),
        },
        "budget": {
            "classification_model_tokens": 0,
            "estimated_full_read_tokens": full_tokens,
            "estimated_default_retrieval_tokens": default_tokens,
            "estimated_tokens_avoided": full_tokens.saturating_sub(default_tokens),
            "ambiguous_documents": ambiguous_documents,
            "excluded_by_default": excluded_by_default,
        },
        "documents": documents,
        "manifest": manifest.value,
        "manifest_revision": manifest.revision,
        "suggestions": suggestions.value,
        "suggestions_revision": suggestions.revision,
        "permissions": {
            "virtual_organization": {"requires_review": true, "changes_markdown": false},
            "file_operations": {
                "requires_item_review": true,
                "one_shot_permissions": ["rename", "move"],
                "allowed_scope": "workspace_markdown_only",
                "forbidden": ["overwrite", "delete", "edit_content", "outside_workspace", "git_commit", "git_push"]
            }
        },
        "next": "Only read ambiguous or task-relevant paths. Save virtual assignments and optional structured rename/move file_operations, then stop for item-by-item user review.",
    }))
}

pub(crate) fn read_documents(
    workspace: &Path,
    paths: &[String],
    max_chars_per_document: usize,
    expected_catalog_revision: Option<&str>,
) -> Result<Value> {
    if paths.is_empty() || paths.len() > MAX_READ_DOCUMENTS {
        bail!("project_docs_read 一次必须读取 1 到 {MAX_READ_DOCUMENTS} 份文档");
    }
    let snapshot = catalog(workspace)?;
    verify_catalog_revision(&snapshot, expected_catalog_revision)?;
    let known_paths = snapshot
        .documents
        .iter()
        .map(|document| document.path.replace('\\', "/").to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let char_limit = max_chars_per_document.clamp(1, MAX_READ_CHARS_PER_DOCUMENT);
    let mut total_chars = 0usize;
    let mut output = Vec::new();
    for requested_path in paths {
        let normalized = requested_path.trim().replace('\\', "/");
        if !known_paths.contains(&normalized.to_ascii_lowercase()) {
            bail!("请求读取的路径不在当前文档目录：{normalized}");
        }
        let file = read_project_document_file(workspace, &normalized)?;
        let remaining = MAX_READ_TOTAL_CHARS.saturating_sub(total_chars);
        if remaining == 0 {
            break;
        }
        let take = char_limit.min(remaining);
        let content = file.content.chars().take(take).collect::<String>();
        let truncated = content.chars().count() < file.content.chars().count();
        total_chars += content.chars().count();
        output.push(json!({
            "path": file.path,
            "revision": file.revision,
            "byte_len": file.byte_len,
            "content": content,
            "truncated": truncated,
        }));
    }
    Ok(json!({
        "catalog_revision": snapshot.revision,
        "documents": output,
        "documents_read": output.len(),
        "characters_returned": total_chars,
        "estimated_tokens_returned": (total_chars as u64).div_ceil(4),
        "limits": {
            "max_documents": MAX_READ_DOCUMENTS,
            "max_chars_per_document": char_limit,
            "max_total_chars": MAX_READ_TOTAL_CHARS,
        }
    }))
}

pub(crate) fn save_suggestions(
    workspace: &Path,
    suggestions: DocumentOrganizationSuggestions,
    authorization_mode: DocumentAutomationMode,
    expected_catalog_revision: &str,
    expected_suggestions_revision: Option<&str>,
) -> Result<Value> {
    let snapshot = catalog(workspace)?;
    verify_catalog_revision(&snapshot, Some(expected_catalog_revision))?;
    let existing = load_suggestions(workspace)?;
    let suggestions = validate_ready_suggestions(suggestions, &snapshot.documents)?;
    if existing.value.as_ref() == Some(&suggestions) {
        return Ok(json!({
            "status": "ready",
            "already_saved": true,
            "catalog_revision": snapshot.revision,
            "suggestions_revision": existing.revision,
            "suggestions": suggestions,
            "authorization_mode": authorization_mode,
            "requires_user_review": authorization_mode == DocumentAutomationMode::ReviewAll,
            "apply_allowed": authorization_mode != DocumentAutomationMode::SuggestionsOnly,
            "markdown_changed": false,
        }));
    }
    verify_file_revision(
        "AI 整理建议",
        existing.revision.as_deref(),
        expected_suggestions_revision,
    )?;
    let content = to_pretty_json(&suggestions)?;
    let saved = write_project_document_file(
        workspace,
        SUGGESTIONS_CONFIG_PATH,
        &content,
        expected_suggestions_revision,
    )
    .map_err(|error| anyhow!(error.message))?;
    Ok(json!({
        "status": "ready",
        "already_saved": false,
        "catalog_revision": snapshot.revision,
        "suggestions_revision": saved.revision,
        "suggestions": suggestions,
        "authorization_mode": authorization_mode,
        "requires_user_review": authorization_mode == DocumentAutomationMode::ReviewAll,
        "apply_allowed": authorization_mode != DocumentAutomationMode::SuggestionsOnly,
        "markdown_changed": false,
    }))
}

pub(crate) fn get_suggestions(workspace: &Path) -> Result<Value> {
    let suggestions = load_suggestions(workspace)?;
    Ok(json!({
        "suggestions": suggestions.value,
        "suggestions_revision": suggestions.revision,
        "default_authorization_mode": DocumentAutomationMode::TrustedReversible,
        "requires_user_review": false,
    }))
}

pub(crate) fn apply_saved_suggestions(
    workspace: &Path,
    authorization_mode: DocumentAutomationMode,
    reviewed: bool,
    expected_catalog_revision: &str,
    expected_manifest_revision: Option<&str>,
    expected_suggestions_revision: Option<&str>,
) -> Result<Value> {
    let authorization = authorize_document_apply(authorization_mode, reviewed)?;
    let snapshot = catalog(workspace)?;
    verify_catalog_revision(&snapshot, Some(expected_catalog_revision))?;
    let manifest = load_manifest(workspace)?;
    let suggestions = load_suggestions(workspace)?;
    let suggestions_value = suggestions
        .value
        .ok_or_else(|| anyhow!("项目尚未生成 AI 文档整理建议"))?;
    let current_manifest = manifest.value;
    let result = apply_suggestions(
        current_manifest.clone(),
        suggestions_value,
        &snapshot.documents,
    )?;
    if result.already_applied {
        return Ok(json!({
            "status": "applied",
            "already_applied": true,
            "manifest": result.manifest,
            "suggestions": result.suggestions,
            "manifest_revision": manifest.revision,
            "suggestions_revision": suggestions.revision,
            "authorization_mode": authorization.mode,
            "auto_authorized": authorization.auto_authorized,
            "markdown_changed": false,
        }));
    }
    verify_file_revision(
        "AI 整理建议",
        suggestions.revision.as_deref(),
        expected_suggestions_revision,
    )?;
    let manifest_already_applied = result.manifest == current_manifest;
    if !manifest_already_applied {
        verify_file_revision(
            "项目文档分区",
            manifest.revision.as_deref(),
            expected_manifest_revision,
        )?;
    }
    let pending_file_operations = result.suggestions.file_operations.iter().any(|operation| {
        operation.status
            == crate::project_document_governance::SuggestedFileOperationStatus::Proposed
    });
    let git_baseline_commit = if authorization.mode == DocumentAutomationMode::GitBackedFull {
        Some(commit_document_baseline(workspace)?)
    } else {
        None
    };
    let manifest_content = to_pretty_json(&result.manifest)?;
    let manifest_revision = if manifest_already_applied {
        manifest.revision
    } else {
        Some(
            write_project_document_file(
                workspace,
                SECTION_CONFIG_PATH,
                &manifest_content,
                expected_manifest_revision,
            )
            .map_err(|error| anyhow!(error.message))?
            .revision,
        )
    };
    let suggestions_content = to_pretty_json(&result.suggestions)?;
    let suggestions_saved = write_project_document_file(
        workspace,
        SUGGESTIONS_CONFIG_PATH,
        &suggestions_content,
        expected_suggestions_revision,
    )
    .map_err(|error| anyhow!(error.message))?;
    let git_result_commit = if !pending_file_operations {
        git_baseline_commit
            .as_deref()
            .map(|baseline| commit_document_result(workspace, baseline))
            .transpose()?
    } else {
        None
    };
    let git_document_transaction_complete = git_result_commit.is_some();
    Ok(json!({
        "status": "applied",
        "already_applied": false,
        "manifest": result.manifest,
        "suggestions": result.suggestions,
        "manifest_revision": manifest_revision,
        "suggestions_revision": suggestions_saved.revision,
        "manifest_already_applied": manifest_already_applied,
        "applied_assignments": result.applied_assignments,
        "skipped_assignments": result.skipped_assignments,
        "authorization_mode": authorization.mode,
        "auto_authorized": authorization.auto_authorized,
        "git_baseline_commit": git_baseline_commit,
        "git_result_commit": git_result_commit,
        "git_document_transaction_complete": git_document_transaction_complete,
        "markdown_changed": false,
    }))
}

fn catalog(workspace: &Path) -> Result<ProjectDocumentsSnapshot> {
    collect_project_documents_with_options(
        workspace,
        ProjectDocumentScanOptions {
            seed_missing_defaults: false,
            catalog_only: true,
        },
    )
}

fn load_manifest(workspace: &Path) -> Result<GovernanceFile<DocumentSectionManifest>> {
    let file = read_optional_config(workspace, SECTION_CONFIG_PATH)?;
    Ok(GovernanceFile {
        value: parse_manifest(file.as_ref().map(|value| value.content.as_str()))?,
        revision: file.map(|value| value.revision),
    })
}

fn load_suggestions(
    workspace: &Path,
) -> Result<GovernanceFile<Option<DocumentOrganizationSuggestions>>> {
    let file = read_optional_config(workspace, SUGGESTIONS_CONFIG_PATH)?;
    Ok(GovernanceFile {
        value: parse_suggestions(file.as_ref().map(|value| value.content.as_str()))?,
        revision: file.map(|value| value.revision),
    })
}

fn read_optional_config(workspace: &Path, relative: &str) -> Result<Option<ProjectDocumentFile>> {
    if !workspace.join(relative).is_file() {
        return Ok(None);
    }
    read_project_document_file(workspace, relative)
        .map(Some)
        .with_context(|| format!("读取 {relative} 失败"))
}

fn verify_catalog_revision(
    snapshot: &ProjectDocumentsSnapshot,
    expected: Option<&str>,
) -> Result<()> {
    if let Some(expected) = expected.filter(|value| !value.trim().is_empty()) {
        if expected != snapshot.revision {
            bail!("文档目录已变化，请重新调用 project_docs_analyze");
        }
    }
    Ok(())
}

fn verify_file_revision(label: &str, current: Option<&str>, expected: Option<&str>) -> Result<()> {
    match (current, expected.filter(|value| !value.trim().is_empty())) {
        (Some(current), Some(expected)) if current == expected => Ok(()),
        (None, None) => Ok(()),
        (Some(_), None) => bail!("{label}已存在，必须先分析并传入当前 revision"),
        _ => bail!("{label}已被其他会话修改，请重新分析后合并"),
    }
}

fn compact_document<'a>(
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

pub(crate) fn default_page_size() -> usize {
    DEFAULT_PAGE_SIZE
}

pub(crate) fn default_read_chars() -> usize {
    DEFAULT_READ_CHARS
}

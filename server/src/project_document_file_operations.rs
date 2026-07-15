//! Authorized Markdown rename/move operations for local project workspaces.

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use std::{collections::HashSet, path::Path};

use crate::{
    project_docs_scan::{collect_project_documents_with_options, ProjectDocumentScanOptions},
    project_document_authorization::{
        authorize_document_apply, operation_permission_granted, DocumentAutomationMode,
    },
    project_document_files::{
        move_project_document_file, read_project_document_file, write_project_document_file,
        ProjectDocumentFile,
    },
    project_document_git_transaction::{
        commit_document_baseline, commit_document_result, current_document_head,
        verify_document_baseline,
    },
    project_document_governance::{
        parse_manifest, parse_suggestions, to_pretty_json, DocumentSectionManifest,
        SuggestedFileOperation, SuggestedFileOperationKind, SuggestedFileOperationStatus,
        SECTION_CONFIG_PATH, SUGGESTIONS_CONFIG_PATH,
    },
};

pub(crate) struct ApplyFileOperationsRequest<'a> {
    pub authorization_mode: DocumentAutomationMode,
    pub reviewed: bool,
    pub operation_ids: &'a [String],
    pub allow_rename: bool,
    pub allow_move: bool,
    pub expected_catalog_revision: &'a str,
    pub expected_manifest_revision: Option<&'a str>,
    pub expected_suggestions_revision: Option<&'a str>,
    pub git_baseline_commit: Option<&'a str>,
}

pub(crate) fn apply_file_operations(
    workspace: &Path,
    request: ApplyFileOperationsRequest<'_>,
) -> Result<Value> {
    if !workspace.is_dir() || !workspace.join(".git").exists() {
        bail!("实体文档整理只允许在现存 Git 工作区执行");
    }
    let authorization = authorize_document_apply(request.authorization_mode, request.reviewed)?;
    if request.operation_ids.is_empty() || request.operation_ids.len() > 100 {
        bail!("每次必须选择 1 到 100 个实体文档操作");
    }
    let selected_ids = request
        .operation_ids
        .iter()
        .map(|value| value.trim().to_string())
        .collect::<HashSet<_>>();
    if selected_ids.len() != request.operation_ids.len() || selected_ids.contains("") {
        bail!("实体文档操作 id 不能为空或重复");
    }

    let snapshot = catalog(workspace)?;
    if request.expected_catalog_revision.trim().is_empty()
        || request.expected_catalog_revision != snapshot.revision
    {
        bail!("文档目录已变化，请重新分析并审核实体整理建议");
    }
    let manifest_file = read_optional_config(workspace, SECTION_CONFIG_PATH)?;
    let suggestions_file = read_optional_config(workspace, SUGGESTIONS_CONFIG_PATH)?
        .ok_or_else(|| anyhow!("项目尚未生成 AI 文档整理建议"))?;
    verify_file_revision(
        "项目文档分区",
        manifest_file.as_ref().map(|file| file.revision.as_str()),
        request.expected_manifest_revision,
    )?;
    verify_file_revision(
        "AI 整理建议",
        Some(suggestions_file.revision.as_str()),
        request.expected_suggestions_revision,
    )?;

    let mut manifest = parse_manifest(manifest_file.as_ref().map(|file| file.content.as_str()))?;
    let mut suggestions = parse_suggestions(Some(&suggestions_file.content))?
        .ok_or_else(|| anyhow!("项目尚未生成 AI 文档整理建议"))?;
    let known_ids = suggestions
        .file_operations
        .iter()
        .map(|operation| operation.id.as_str())
        .collect::<HashSet<_>>();
    if let Some(unknown) = selected_ids
        .iter()
        .find(|operation_id| !known_ids.contains(operation_id.as_str()))
    {
        bail!("审核请求包含未知文件操作：{unknown}");
    }

    let selected = suggestions
        .file_operations
        .iter()
        .filter(|operation| selected_ids.contains(&operation.id))
        .cloned()
        .collect::<Vec<_>>();
    validate_permissions(
        &selected,
        authorization,
        request.allow_rename,
        request.allow_move,
    )?;
    let pending = selected
        .iter()
        .filter(|operation| operation.status != SuggestedFileOperationStatus::Applied)
        .collect::<Vec<_>>();
    let git_baseline_commit =
        if authorization.mode == DocumentAutomationMode::GitBackedFull && !pending.is_empty() {
            let replaying_partial_result = pending
                .iter()
                .any(|operation| operation_target_matches(workspace, operation));
            let baseline = if let Some(expected) = request.git_baseline_commit {
                let head = current_document_head(workspace)?;
                if head != expected {
                    bail!("整理前 Git 基线已变化，请重新开始文档整理");
                }
                head
            } else if replaying_partial_result {
                current_document_head(workspace)?
            } else {
                commit_document_baseline(workspace)?
            };
            for operation in &pending {
                verify_document_baseline(
                    workspace,
                    &baseline,
                    &operation.source_path,
                    &operation.source_revision,
                )?;
            }
            Some(baseline)
        } else {
            None
        };
    let mut results = Vec::new();
    for operation in &selected {
        let already_applied = operation.status == SuggestedFileOperationStatus::Applied
            || operation_target_matches(workspace, operation);
        if !already_applied {
            move_project_document_file(
                workspace,
                &operation.source_path,
                &operation.target_path,
                &operation.source_revision,
            )
            .map_err(|error| anyhow!(error.message))?;
        }
        remap_manifest_assignment(
            &mut manifest,
            &operation.source_path,
            &operation.target_path,
        );
        for assignment in &mut suggestions.assignments {
            if assignment.path.eq_ignore_ascii_case(&operation.source_path) {
                assignment.path = operation.target_path.clone();
            }
        }
        if let Some(saved) = suggestions
            .file_operations
            .iter_mut()
            .find(|saved| saved.id == operation.id)
        {
            saved.status = SuggestedFileOperationStatus::Applied;
        }
        results.push(json!({
            "id": operation.id,
            "kind": operation.kind,
            "source_path": operation.source_path,
            "target_path": operation.target_path,
            "already_applied": already_applied,
        }));
    }

    let manifest_revision = persist_manifest_if_changed(
        workspace,
        &manifest,
        manifest_file.as_ref(),
        request.expected_manifest_revision,
    )?;
    let suggestions_saved = write_project_document_file(
        workspace,
        SUGGESTIONS_CONFIG_PATH,
        &to_pretty_json(&suggestions)?,
        request.expected_suggestions_revision,
    )
    .map_err(|error| anyhow!(error.message))?;
    let git_result_commit = git_baseline_commit
        .as_deref()
        .map(|baseline| commit_document_result(workspace, baseline))
        .transpose()?;
    let git_document_transaction_complete = git_result_commit.is_some();
    let updated_catalog = catalog(workspace)?;
    Ok(json!({
        "ok": true,
        "status": "file_operations_applied",
        "operations": results,
        "applied_count": results.len(),
        "remaining_count": suggestions.file_operations.iter().filter(|operation| operation.status == SuggestedFileOperationStatus::Proposed).count(),
        "manifest": manifest,
        "suggestions": suggestions,
        "manifest_revision": manifest_revision,
        "suggestions_revision": suggestions_saved.revision,
        "catalog_revision": updated_catalog.revision,
        "authorization_mode": authorization.mode,
        "auto_authorized": authorization.auto_authorized,
        "git_baseline_commit": git_baseline_commit,
        "git_result_commit": git_result_commit,
        "git_document_transaction_complete": git_document_transaction_complete,
        "markdown_changed": true,
        "content_changed": false,
        "files_deleted": false,
        "git_review_required": true,
    }))
}

fn validate_permissions(
    operations: &[SuggestedFileOperation],
    authorization: crate::project_document_authorization::DocumentAuthorization,
    allow_rename: bool,
    allow_move: bool,
) -> Result<()> {
    for operation in operations {
        match operation.kind {
            SuggestedFileOperationKind::Rename
                if !operation_permission_granted(authorization, allow_rename) =>
            {
                bail!("操作 {} 需要本次明确授予 rename 权限", operation.id)
            }
            SuggestedFileOperationKind::Move
                if !operation_permission_granted(authorization, allow_move) =>
            {
                bail!("操作 {} 需要本次明确授予 move 权限", operation.id)
            }
            _ => {}
        }
    }
    Ok(())
}

fn operation_target_matches(workspace: &Path, operation: &SuggestedFileOperation) -> bool {
    if workspace.join(&operation.source_path).is_file() {
        return false;
    }
    read_project_document_file(workspace, &operation.target_path)
        .map(|file| file.revision == operation.source_revision)
        .unwrap_or(false)
}

fn remap_manifest_assignment(
    manifest: &mut DocumentSectionManifest,
    source_path: &str,
    target_path: &str,
) {
    let source_key = manifest
        .assignments
        .keys()
        .find(|path| path.eq_ignore_ascii_case(source_path))
        .cloned();
    if let Some(source_key) = source_key {
        if let Some(section) = manifest.assignments.remove(&source_key) {
            manifest
                .assignments
                .insert(target_path.to_string(), section);
        }
    }
}

fn persist_manifest_if_changed(
    workspace: &Path,
    manifest: &DocumentSectionManifest,
    current: Option<&ProjectDocumentFile>,
    expected_revision: Option<&str>,
) -> Result<Option<String>> {
    let content = to_pretty_json(manifest)?;
    if current.is_some_and(|file| file.content == content)
        || current.is_none() && manifest == &DocumentSectionManifest::default()
    {
        return Ok(current.map(|file| file.revision.clone()));
    }
    Ok(Some(
        write_project_document_file(workspace, SECTION_CONFIG_PATH, &content, expected_revision)
            .map_err(|error| anyhow!(error.message))?
            .revision,
    ))
}

fn catalog(workspace: &Path) -> Result<homecli_proto::ProjectDocumentsSnapshot> {
    collect_project_documents_with_options(
        workspace,
        ProjectDocumentScanOptions {
            seed_missing_defaults: false,
            catalog_only: true,
        },
    )
}

fn read_optional_config(workspace: &Path, relative: &str) -> Result<Option<ProjectDocumentFile>> {
    if !workspace.join(relative).is_file() {
        return Ok(None);
    }
    read_project_document_file(workspace, relative)
        .map(Some)
        .with_context(|| format!("读取 {relative} 失败"))
}

fn verify_file_revision(label: &str, current: Option<&str>, expected: Option<&str>) -> Result<()> {
    match (current, expected.filter(|value| !value.trim().is_empty())) {
        (Some(current), Some(expected)) if current == expected => Ok(()),
        (None, None) => Ok(()),
        (Some(_), None) => bail!("{label}已存在，必须先刷新并传入当前 revision"),
        _ => bail!("{label}已被其他会话修改，请刷新后重新审核"),
    }
}

use super::*;
use std::fs;

use crate::{
    project_document_governance::{
        DocumentOrganizationSuggestions, OrganizationStatus, SuggestedFileOperation,
        SuggestedFileOperationKind, SuggestedFileOperationStatus,
    },
    project_document_governance_service::{
        analyze_workspace, apply_saved_suggestions, save_suggestions,
    },
    project_document_vault::{list_versions, resolve_or_create},
};

#[test]
fn managed_vault_moves_document_and_closes_git_transaction() {
    let vault = resolve_or_create(&format!("test-{}", uuid::Uuid::new_v4().simple())).unwrap();
    let analysis = analyze_workspace(&vault.workspace, 0, 80, false).unwrap();
    let catalog_revision = analysis["catalog_revision"].as_str().unwrap();
    let manifest_revision = analysis["manifest_revision"].as_str();
    let source = analysis["documents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|document| document["path"] == "notes/inbox/README.md")
        .unwrap();
    let source_revision = source["content_hash"].as_str().unwrap().to_string();
    let operation_id = "move-inbox-readme".to_string();
    let suggestions = DocumentOrganizationSuggestions {
        version: 1,
        status: OrganizationStatus::Ready,
        summary: "将收件箱说明移动到知识库指南".to_string(),
        file_operations: vec![SuggestedFileOperation {
            id: operation_id.clone(),
            kind: SuggestedFileOperationKind::Move,
            source_path: "notes/inbox/README.md".to_string(),
            target_path: "notes/guides/inbox.md".to_string(),
            source_revision,
            reason: "把说明文档放入长期指南区域".to_string(),
            status: SuggestedFileOperationStatus::Proposed,
        }],
        ..DocumentOrganizationSuggestions::default()
    };
    let saved = save_suggestions(
        &vault.workspace,
        suggestions,
        DocumentAutomationMode::GitBackedFull,
        catalog_revision,
        None,
    )
    .unwrap();
    let applied = apply_saved_suggestions(
        &vault.workspace,
        DocumentAutomationMode::GitBackedFull,
        false,
        catalog_revision,
        manifest_revision,
        saved["suggestions_revision"].as_str(),
    )
    .unwrap();

    assert_eq!(applied["git_document_transaction_complete"], false);
    let result = apply_file_operations(
        &vault.workspace,
        ApplyFileOperationsRequest {
            authorization_mode: DocumentAutomationMode::GitBackedFull,
            reviewed: false,
            operation_ids: &[operation_id],
            allow_rename: false,
            allow_move: true,
            expected_catalog_revision: catalog_revision,
            expected_manifest_revision: applied["manifest_revision"].as_str(),
            expected_suggestions_revision: applied["suggestions_revision"].as_str(),
            git_baseline_commit: applied["git_baseline_commit"].as_str(),
        },
    )
    .unwrap();

    assert_eq!(result["git_document_transaction_complete"], true);
    assert_eq!(result["applied_count"], 1);
    assert!(!vault.workspace.join("notes/inbox/README.md").exists());
    assert!(vault.workspace.join("notes/guides/inbox.md").is_file());
    assert!(list_versions(&vault.workspace, 20).unwrap().len() >= 5);
    fs::remove_dir_all(vault.workspace).unwrap();
}

use super::*;
use std::fs;

use crate::{
    project_document_authorization::DocumentAutomationMode,
    project_document_governance::{DocumentOrganizationSuggestions, OrganizationStatus},
    project_document_governance_service::{
        analyze_workspace, apply_saved_suggestions, save_suggestions,
    },
};

#[test]
fn managed_vault_checkpoints_and_restores_without_user_git_knowledge() {
    let vault = resolve_or_create(&format!("test-{}", uuid::Uuid::new_v4().simple())).unwrap();
    assert!(vault.created);
    assert!(is_managed_vault(&vault.workspace));
    let initial = current_head(&vault.workspace).unwrap();
    fs::write(vault.workspace.join("README.md"), "# Changed\n").unwrap();
    let changed = checkpoint_after_write(&vault.workspace, "README.md")
        .unwrap()
        .unwrap();
    assert_ne!(initial, changed);
    assert!(list_versions(&vault.workspace, 10).unwrap().len() >= 2);

    let restored = restore_version(&vault.workspace, &initial).unwrap();
    assert_ne!(restored, changed);
    assert!(fs::read_to_string(vault.workspace.join("README.md"))
        .unwrap()
        .contains("我的知识库"));
    fs::remove_dir_all(vault.workspace).unwrap();
}

#[test]
fn managed_vault_applies_ai_structure_without_conflicting_git_heads() {
    let vault = resolve_or_create(&format!("test-{}", uuid::Uuid::new_v4().simple())).unwrap();
    let analysis = analyze_workspace(&vault.workspace, 0, 80, false).unwrap();
    let catalog_revision = analysis["catalog_revision"].as_str().unwrap();
    let manifest_revision = analysis["manifest_revision"].as_str();
    let suggestions = DocumentOrganizationSuggestions {
        version: 1,
        status: OrganizationStatus::Ready,
        summary: "保持个人知识库入口并建立可恢复整理记录".to_string(),
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
    let result = apply_saved_suggestions(
        &vault.workspace,
        DocumentAutomationMode::GitBackedFull,
        false,
        catalog_revision,
        manifest_revision,
        saved["suggestions_revision"].as_str(),
    )
    .unwrap();

    assert_eq!(result["git_document_transaction_complete"], true);
    assert_ne!(result["git_baseline_commit"], result["git_result_commit"]);
    assert!(list_versions(&vault.workspace, 20).unwrap().len() >= 3);
    fs::remove_dir_all(vault.workspace).unwrap();
}

#[test]
fn managed_vault_rolls_back_failed_restore_and_has_no_push_remote() {
    let vault = resolve_or_create(&format!("test-{}", uuid::Uuid::new_v4().simple())).unwrap();
    let initial = current_head(&vault.workspace).unwrap();
    fs::write(vault.workspace.join("README.md"), "# Restore rollback\n").unwrap();
    checkpoint_after_write(&vault.workspace, "README.md").unwrap();
    let before_restore = current_head(&vault.workspace).unwrap();

    let error = restore_version_with(&vault.workspace, &initial, || {
        anyhow::bail!("injected restore failure")
    })
    .unwrap_err();
    assert!(error.to_string().contains("已自动回到恢复前检查点"));
    assert_eq!(current_head(&vault.workspace).unwrap(), before_restore);
    assert!(fs::read_to_string(vault.workspace.join("README.md"))
        .unwrap()
        .contains("Restore rollback"));
    assert!(git_status(&vault.workspace, &["diff", "--quiet"]).unwrap());
    let remotes = git(&vault.workspace, &["remote"]).unwrap();
    assert!(
        remotes.stdout.is_empty(),
        "托管私有笔记不得默认配置 push remote"
    );
    fs::remove_dir_all(vault.workspace).unwrap();
}

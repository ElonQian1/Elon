use std::{fs, path::PathBuf};

use crate::{
    project_document_authorization::DocumentAutomationMode,
    project_document_file_operations::{apply_file_operations, ApplyFileOperationsRequest},
    project_document_files::write_project_document_file,
    project_document_governance::{
        parse_manifest, to_pretty_json, CustomDocumentSection, DocumentKnowledgeHome,
        DocumentKnowledgeMetadata, DocumentOrganizationSuggestions, OrganizationStatus,
        SuggestedAssignment, SuggestedFileOperation, SuggestedFileOperationKind,
        SuggestedFileOperationStatus, SUGGESTIONS_CONFIG_PATH,
    },
    project_document_governance_service::{
        analyze_workspace, apply_saved_suggestions, read_documents, save_suggestions,
    },
};

fn workspace(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "elon_project_docs_governance_{label}_{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::create_dir_all(root.join("docs/archive")).unwrap();
    fs::write(root.join("AGENTS.md"), "# Shared entry\n\nCurrent rules.\n").unwrap();
    fs::write(
        root.join("docs/archive/old.md"),
        "---\nlifecycle: archived\n---\n# Old discussion\n\nIgnore by default.\n",
    )
    .unwrap();
    root
}

fn ready_suggestions() -> DocumentOrganizationSuggestions {
    DocumentOrganizationSuggestions {
        version: 1,
        status: OrganizationStatus::Ready,
        summary: "把旧讨论保持在历史归档。".to_string(),
        proposed_profile: "auto".to_string(),
        proposed_sections: vec![CustomDocumentSection {
            id: "legacy-notes".to_string(),
            label: "旧笔记".to_string(),
            detail: "仅用于历史追溯".to_string(),
            color: "#747984".to_string(),
            ..CustomDocumentSection::default()
        }],
        assignments: vec![SuggestedAssignment {
            path: "docs/archive/old.md".to_string(),
            section_id: "custom:legacy-notes".to_string(),
            reason: "路径和 frontmatter 都表明它是历史资料。".to_string(),
        }],
        conflicts: Vec::new(),
        move_suggestions: Vec::new(),
        file_operations: Vec::new(),
        documents_read: 0,
        estimated_tokens_used: 0,
        ..DocumentOrganizationSuggestions::default()
    }
}

#[test]
fn legacy_assignments_are_split_into_topic_and_governance_facets() {
    let manifest = parse_manifest(Some(
        r##"{
          "version": 1,
          "sections": [{"id":"api","label":"API","detail":"Reference","color":"#123456"}],
          "assignments": {"docs/api.md":"custom:api","docs/draft.md":"drafts"},
          "governance_overrides": {"docs/api.md":"current"}
        }"##,
    ))
    .unwrap();
    assert_eq!(manifest.assignments["docs/api.md"], "custom:api");
    assert_eq!(manifest.governance_overrides["docs/api.md"], "current");
    assert_eq!(manifest.governance_overrides["docs/draft.md"], "drafts");
    assert!(!manifest.assignments.contains_key("docs/draft.md"));
}

#[test]
fn manual_management_metadata_and_audit_are_preserved_safely() {
    let manifest = parse_manifest(Some(
        r##"{
          "version": 1,
          "document_metadata": {
            "docs/api.md": {"order": 2000000, "pinned": true}
          },
          "audit_log": [{
            "id":"menu-1","action":"document.assign_governance",
            "target":"docs/api.md","summary":"标记为当前知识","at":"2026-07-17T00:00:00Z"
          }]
        }"##,
    ))
    .unwrap();
    let metadata = &manifest.document_metadata["docs/api.md"];
    assert!(metadata.pinned);
    assert_eq!(metadata.order, 999_999);
    assert_eq!(manifest.audit_log.len(), 1);
    assert_eq!(manifest.audit_log[0].action, "document.assign_governance");
}

#[test]
fn section_tree_accepts_four_levels_and_rejects_a_fifth() {
    let four_levels = r##"{
      "version": 1,
      "sections": [
        {"id":"l1","label":"一级","parent_id":""},
        {"id":"l2","label":"二级","parent_id":"l1"},
        {"id":"l3","label":"三级","parent_id":"l2"},
        {"id":"l4","label":"四级","parent_id":"l3"}
      ]
    }"##;
    assert!(parse_manifest(Some(four_levels)).is_ok());

    let five_levels = four_levels.replace(
        "{\"id\":\"l4\",\"label\":\"四级\",\"parent_id\":\"l3\"}",
        "{\"id\":\"l4\",\"label\":\"四级\",\"parent_id\":\"l3\"},\n        {\"id\":\"l5\",\"label\":\"五级\",\"parent_id\":\"l4\"}",
    );
    let error = parse_manifest(Some(&five_levels)).unwrap_err();
    assert!(error.to_string().contains("最多支持 4 层"));
}

#[test]
fn trusted_file_operations_are_default_safe_and_preserve_content() {
    let root = workspace("file-operations");
    let original = fs::read_to_string(root.join("docs/archive/old.md")).unwrap();
    let analysis = analyze_workspace(&root, 0, 80, false).unwrap();
    let catalog_revision = analysis["catalog_revision"].as_str().unwrap();
    let source_revision = analysis["documents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|document| document["path"] == "docs/archive/old.md")
        .unwrap()["content_hash"]
        .as_str()
        .unwrap()
        .to_string();
    let mut suggestions = ready_suggestions();
    suggestions.proposed_home = Some(DocumentKnowledgeHome {
        title: "Knowledge home".to_string(),
        summary: "Entry path must follow a safe rename.".to_string(),
        entrypoint: "docs/archive/old.md".to_string(),
        start_here: vec!["docs/archive/old.md".to_string()],
    });
    suggestions.proposed_sections[0].entrypoint = "docs/archive/old.md".to_string();
    suggestions.document_metadata.insert(
        "docs/archive/old.md".to_string(),
        DocumentKnowledgeMetadata {
            doc_type: "archive".to_string(),
            related: vec!["AGENTS.md".to_string()],
            supersedes: vec!["docs/archive/old.md".to_string()],
            ..DocumentKnowledgeMetadata::default()
        },
    );
    suggestions.file_operations = vec![SuggestedFileOperation {
        id: "rename-old-note".to_string(),
        kind: SuggestedFileOperationKind::Rename,
        source_path: "docs/archive/old.md".to_string(),
        target_path: "docs/archive/legacy-discussion.md".to_string(),
        source_revision,
        reason: "名称应表达文档用途".to_string(),
        status: SuggestedFileOperationStatus::Proposed,
    }];
    let saved = save_suggestions(
        &root,
        suggestions,
        DocumentAutomationMode::TrustedReversible,
        catalog_revision,
        None,
    )
    .unwrap();
    let suggestions_revision = saved["suggestions_revision"].as_str().unwrap();
    let organized = apply_saved_suggestions(
        &root,
        DocumentAutomationMode::TrustedReversible,
        false,
        catalog_revision,
        None,
        Some(suggestions_revision),
    )
    .unwrap();
    let manifest_revision = organized["manifest_revision"].as_str().unwrap();
    let suggestions_revision = organized["suggestions_revision"].as_str().unwrap();
    let operation_ids = vec!["rename-old-note".to_string()];
    let denied = apply_file_operations(
        &root,
        ApplyFileOperationsRequest {
            authorization_mode: DocumentAutomationMode::ReviewAll,
            reviewed: true,
            operation_ids: &operation_ids,
            allow_rename: false,
            allow_move: false,
            expected_catalog_revision: catalog_revision,
            expected_manifest_revision: Some(manifest_revision),
            expected_suggestions_revision: Some(suggestions_revision),
            git_baseline_commit: None,
        },
    )
    .unwrap_err();
    assert!(denied.to_string().contains("rename 权限"));
    let applied = apply_file_operations(
        &root,
        ApplyFileOperationsRequest {
            authorization_mode: DocumentAutomationMode::TrustedReversible,
            reviewed: false,
            operation_ids: &operation_ids,
            allow_rename: false,
            allow_move: false,
            expected_catalog_revision: catalog_revision,
            expected_manifest_revision: Some(manifest_revision),
            expected_suggestions_revision: Some(suggestions_revision),
            git_baseline_commit: None,
        },
    )
    .unwrap();
    assert_eq!(applied["applied_count"], 1);
    assert_eq!(applied["content_changed"], false);
    assert_eq!(applied["files_deleted"], false);
    assert_eq!(applied["authorization_mode"], "trusted_reversible");
    assert_eq!(applied["auto_authorized"], true);
    assert!(!root.join("docs/archive/old.md").exists());
    assert_eq!(
        fs::read_to_string(root.join("docs/archive/legacy-discussion.md")).unwrap(),
        original
    );
    assert_eq!(
        applied["suggestions"]["file_operations"][0]["status"],
        "applied"
    );
    assert_eq!(
        applied["manifest"]["home"]["entrypoint"],
        "docs/archive/legacy-discussion.md"
    );
    assert_eq!(
        applied["manifest"]["sections"][0]["entrypoint"],
        "docs/archive/legacy-discussion.md"
    );
    assert!(applied["manifest"]["assignments"]
        .get("docs/archive/legacy-discussion.md")
        .is_some());
    assert!(applied["manifest"]["document_metadata"]
        .get("docs/archive/legacy-discussion.md")
        .is_some());
    assert_eq!(
        applied["manifest"]["document_metadata"]["docs/archive/legacy-discussion.md"]["supersedes"]
            [0],
        "docs/archive/legacy-discussion.md"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn analysis_is_metadata_only_and_paged() {
    let root = workspace("analyze");
    let analysis = analyze_workspace(&root, 0, 1, false).unwrap();
    assert_eq!(analysis["budget"]["classification_model_tokens"], 0);
    assert_eq!(analysis["documents"].as_array().unwrap().len(), 1);
    assert!(analysis["documents"][0].get("content").is_none());
    assert!(analysis["catalog_revision"].as_str().unwrap().len() > 20);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn on_demand_reads_enforce_catalog_and_character_budget() {
    let root = workspace("read");
    let analysis = analyze_workspace(&root, 0, 80, false).unwrap();
    let revision = analysis["catalog_revision"].as_str().unwrap();
    let result = read_documents(&root, &["AGENTS.md".to_string()], 8, Some(revision)).unwrap();
    assert_eq!(result["documents_read"], 1);
    assert_eq!(result["documents"][0]["truncated"], true);
    assert!(read_documents(&root, &["../secret.md".to_string()], 20, Some(revision)).is_err());
    fs::write(root.join("AGENTS.md"), "# Changed after catalog\n").unwrap();
    assert!(read_documents(&root, &["AGENTS.md".to_string()], 20, Some(revision)).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn save_rejects_hallucinated_paths() {
    let root = workspace("hallucination");
    let analysis = analyze_workspace(&root, 0, 80, false).unwrap();
    let revision = analysis["catalog_revision"].as_str().unwrap();
    let mut suggestions = ready_suggestions();
    suggestions.assignments[0].path = "docs/missing.md".to_string();
    let error = save_suggestions(
        &root,
        suggestions,
        DocumentAutomationMode::TrustedReversible,
        revision,
        None,
    )
    .unwrap_err();
    assert!(error.to_string().contains("不存在"));
    assert!(!root
        .join(".elon/document-organization-suggestions.json")
        .exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn review_apply_is_revision_safe_and_idempotent() {
    let root = workspace("apply");
    let original_markdown = fs::read_to_string(root.join("docs/archive/old.md")).unwrap();
    let analysis = analyze_workspace(&root, 0, 80, false).unwrap();
    let catalog_revision = analysis["catalog_revision"].as_str().unwrap();
    let saved = save_suggestions(
        &root,
        ready_suggestions(),
        DocumentAutomationMode::TrustedReversible,
        catalog_revision,
        None,
    )
    .unwrap();
    let suggestion_revision = saved["suggestions_revision"].as_str().unwrap();
    let repeated = save_suggestions(
        &root,
        ready_suggestions(),
        DocumentAutomationMode::TrustedReversible,
        catalog_revision,
        None,
    )
    .unwrap();
    assert_eq!(repeated["already_saved"], true);
    assert!(apply_saved_suggestions(
        &root,
        DocumentAutomationMode::ReviewAll,
        false,
        catalog_revision,
        None,
        Some(suggestion_revision)
    )
    .is_err());
    let applied = apply_saved_suggestions(
        &root,
        DocumentAutomationMode::TrustedReversible,
        false,
        catalog_revision,
        None,
        Some(suggestion_revision),
    )
    .unwrap();
    assert_eq!(applied["status"], "applied");
    assert_eq!(applied["markdown_changed"], false);
    assert_eq!(applied["authorization_mode"], "trusted_reversible");
    assert_eq!(applied["auto_authorized"], true);
    assert_eq!(
        applied["manifest"]["assignments"]["docs/archive/old.md"],
        "custom:legacy-notes"
    );
    let restored_ready = write_project_document_file(
        &root,
        SUGGESTIONS_CONFIG_PATH,
        &to_pretty_json(&ready_suggestions()).unwrap(),
        applied["suggestions_revision"].as_str(),
    )
    .unwrap();
    assert_eq!(restored_ready.revision, suggestion_revision);
    let recovered = apply_saved_suggestions(
        &root,
        DocumentAutomationMode::TrustedReversible,
        true,
        catalog_revision,
        None,
        Some(suggestion_revision),
    )
    .unwrap();
    assert_eq!(recovered["manifest_already_applied"], true);
    let replay = apply_saved_suggestions(
        &root,
        DocumentAutomationMode::TrustedReversible,
        true,
        catalog_revision,
        None,
        Some(suggestion_revision),
    )
    .unwrap();
    assert_eq!(replay["already_applied"], true);
    assert_eq!(
        fs::read_to_string(root.join("docs/archive/old.md")).unwrap(),
        original_markdown
    );
    fs::remove_dir_all(root).unwrap();
}

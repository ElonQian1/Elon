use std::{fs, path::PathBuf};

use crate::{
    project_document_files::write_project_document_file,
    project_document_governance::{
        to_pretty_json, CustomDocumentSection, DocumentOrganizationSuggestions, OrganizationStatus,
        SuggestedAssignment, SUGGESTIONS_CONFIG_PATH,
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
        proposed_sections: vec![CustomDocumentSection {
            id: "legacy-notes".to_string(),
            label: "旧笔记".to_string(),
            detail: "仅用于历史追溯".to_string(),
            color: "#747984".to_string(),
        }],
        assignments: vec![SuggestedAssignment {
            path: "docs/archive/old.md".to_string(),
            section_id: "custom:legacy-notes".to_string(),
            reason: "路径和 frontmatter 都表明它是历史资料。".to_string(),
        }],
        conflicts: Vec::new(),
        move_suggestions: Vec::new(),
        documents_read: 0,
        estimated_tokens_used: 0,
    }
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
    let error = save_suggestions(&root, suggestions, revision, None).unwrap_err();
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
    let saved = save_suggestions(&root, ready_suggestions(), catalog_revision, None).unwrap();
    let suggestion_revision = saved["suggestions_revision"].as_str().unwrap();
    let repeated = save_suggestions(&root, ready_suggestions(), catalog_revision, None).unwrap();
    assert_eq!(repeated["already_saved"], true);
    assert!(apply_saved_suggestions(
        &root,
        false,
        catalog_revision,
        None,
        Some(suggestion_revision)
    )
    .is_err());
    let applied = apply_saved_suggestions(
        &root,
        true,
        catalog_revision,
        None,
        Some(suggestion_revision),
    )
    .unwrap();
    assert_eq!(applied["status"], "applied");
    assert_eq!(applied["markdown_changed"], false);
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
        true,
        catalog_revision,
        None,
        Some(suggestion_revision),
    )
    .unwrap();
    assert_eq!(recovered["manifest_already_applied"], true);
    let replay = apply_saved_suggestions(
        &root,
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

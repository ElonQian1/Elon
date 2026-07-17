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
    assert!(!paths.contains(&".github/copilot-instructions.md"));
    assert!(!snapshot.revision.is_empty());
    assert_eq!(snapshot.source, "workspace");
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
            catalog_only: false,
            include_analysis: true,
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
    assert_eq!(snapshot.source, "workspace");
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

#[test]
fn project_docs_catalog_classifies_without_returning_bodies() {
    let root = std::env::temp_dir().join(format!(
        "elon-project-docs-catalog-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(root.join("docs/archive")).unwrap();
    fs::write(root.join("AGENTS.md"), "# Agent Rules\nrequired").unwrap();
    fs::write(root.join("AI_PROJECT.md"), "# Local Project\ncurrent").unwrap();
    fs::write(root.join("docs/archive/old.md"), "# Old\nlegacy").unwrap();

    let snapshot = collect_project_documents_with_options(
        &root,
        ProjectDocumentScanOptions {
            seed_missing_defaults: false,
            catalog_only: true,
            include_analysis: true,
        },
    )
    .unwrap();
    let unchanged = collect_project_documents_with_options(
        &root,
        ProjectDocumentScanOptions {
            seed_missing_defaults: false,
            catalog_only: true,
            include_analysis: true,
        },
    )
    .unwrap();
    let _ = fs::remove_dir_all(&root);

    assert!(snapshot
        .documents
        .iter()
        .all(|document| document.content.is_empty()));
    assert_eq!(snapshot.analysis["source"], "server");
    assert!(snapshot.analysis["quality"]["summary"]["total_issues"]
        .as_u64()
        .is_some());
    assert_eq!(unchanged.analysis["maintenance"]["changed_documents"], 0);
    let agents = snapshot
        .documents
        .iter()
        .find(|document| document.path == "AGENTS.md")
        .unwrap();
    assert_eq!(agents.metadata.role, "router");
    assert!(agents.metadata.default_retrieval);
    let project_guide = snapshot
        .documents
        .iter()
        .find(|document| document.path == "AI_PROJECT.md")
        .unwrap();
    assert_eq!(project_guide.source, "workspace");
    assert!(!project_guide.metadata.default_retrieval);
    let archived = snapshot
        .documents
        .iter()
        .find(|document| document.path == "docs/archive/old.md")
        .unwrap();
    assert_eq!(archived.metadata.lifecycle, "archived");
    assert!(!archived.metadata.default_retrieval);
}

use super::*;
use homecli_proto::{ProjectDocumentEntry, ProjectDocumentMetadata};
use std::fs;

use crate::{
    project_document_governance::{DocumentKnowledgeHome, DocumentKnowledgeMetadata},
    project_document_index::ProjectDocumentIndex,
};

#[test]
fn quality_report_finds_links_orphans_ownership_and_implementation_conflicts() {
    let root = workspace("all-rules");
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("README.md"),
        "# Home\n\n[Guide](docs/guide.md#missing) [Gone](docs/gone.md)\n\n按需读取 `docs/routed.md`。\n",
    )
    .unwrap();
    fs::write(root.join("docs/guide.md"), "# Guide\n\nUseful.\n").unwrap();
    fs::write(root.join("docs/orphan.md"), "# Orphan\n\nHidden.\n").unwrap();
    fs::write(root.join("docs/routed.md"), "# Routed\n\nRouted.\n").unwrap();
    fs::write(
        root.join("docs/duplicate.md"),
        "# Guide\n\nDuplicate title.\n",
    )
    .unwrap();
    fs::create_dir_all(root.join(".github/agents")).unwrap();
    fs::write(root.join(".github/agents/example.agent.md"), "# Agent\n").unwrap();

    let documents = vec![
        entry("README.md", "Home", "router", true, vec!["Home"]),
        entry("docs/guide.md", "Guide", "spec", false, vec!["Guide"]),
        entry("docs/orphan.md", "Orphan", "spec", false, vec!["Orphan"]),
        entry(
            "docs/duplicate.md",
            "Guide",
            "architecture",
            false,
            vec!["Guide"],
        ),
        entry(
            "docs/routed.md",
            "Routed",
            "domain_policy",
            false,
            vec!["Routed"],
        ),
        entry(
            ".github/agents/example.agent.md",
            "Agent",
            "agent_definition",
            false,
            vec!["Agent"],
        ),
    ];
    let mut manifest = DocumentSectionManifest {
        home: DocumentKnowledgeHome {
            entrypoint: "README.md".to_string(),
            start_here: vec!["README.md".to_string()],
            ..DocumentKnowledgeHome::default()
        },
        ..DocumentSectionManifest::default()
    };
    manifest.document_metadata.insert(
        "README.md".to_string(),
        DocumentKnowledgeMetadata {
            owner: "docs-team".to_string(),
            reviewed_at: "2099-01-01".to_string(),
            review_interval_days: 180,
            implementation_refs: vec!["file:src/missing.rs".to_string()],
            ..DocumentKnowledgeMetadata::default()
        },
    );
    manifest.document_metadata.insert(
        "docs/duplicate.md".to_string(),
        DocumentKnowledgeMetadata {
            doc_type: "architecture".to_string(),
            owner: "architecture-team".to_string(),
            reviewed_at: "2099-01-01".to_string(),
            ..DocumentKnowledgeMetadata::default()
        },
    );
    let index = ProjectDocumentIndex::open(&root).unwrap();
    let report = analyze_document_quality(&root, &documents, &manifest, &index).unwrap();
    let kinds = report
        .issues
        .iter()
        .map(|issue| issue.issue_type.as_str())
        .collect::<Vec<_>>();

    assert!(kinds.contains(&"broken_link"));
    assert!(kinds.contains(&"broken_anchor"));
    assert!(kinds.contains(&"orphan_document"));
    assert!(kinds.contains(&"missing_owner"));
    assert!(kinds.contains(&"missing_review_date"));
    assert!(kinds.contains(&"implementation_conflict"));
    assert!(kinds.contains(&"duplicate_title"));
    assert!(kinds.contains(&"implementation_reference_missing"));
    assert_eq!(report.summary.duplicate_titles, 1);
    assert!(!report.issues.iter().any(|issue| {
        issue.issue_type == "orphan_document"
            && matches!(
                issue.path.as_str(),
                "docs/routed.md" | ".github/agents/example.agent.md"
            )
    }));
    assert_eq!(report.summary.status, "needs_attention");
    drop(index);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn implementation_drift_uses_git_history_and_groups_dirty_evidence_per_document() {
    let root = workspace("git-change-clock");
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("docs/architecture.md"), "# Architecture\n").unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
    fs::write(root.join("src/lib.rs"), "pub fn ready() -> bool { true }\n").unwrap();
    git(&root, &["init"]);
    git(&root, &["config", "user.email", "tests@example.com"]);
    git(&root, &["config", "user.name", "Tests"]);
    git(&root, &["add", "."]);
    let status = crate::git_command_error::git_command()
        .arg("-C")
        .arg(&root)
        .args(["commit", "-m", "old baseline"])
        .env("GIT_AUTHOR_DATE", "2020-01-01T00:00:00Z")
        .env("GIT_COMMITTER_DATE", "2020-01-01T00:00:00Z")
        .status()
        .unwrap();
    assert!(status.success());

    let documents = vec![entry(
        "docs/architecture.md",
        "Architecture",
        "architecture",
        false,
        vec!["Architecture"],
    )];
    let mut manifest = DocumentSectionManifest::default();
    manifest.document_metadata.insert(
        "docs/architecture.md".to_string(),
        DocumentKnowledgeMetadata {
            doc_type: "architecture".to_string(),
            owner: "architecture-team".to_string(),
            reviewed_at: "2026-01-01".to_string(),
            implementation_refs: vec![
                "file:src/main.rs".to_string(),
                "file:src/lib.rs".to_string(),
            ],
            ..DocumentKnowledgeMetadata::default()
        },
    );
    let index = ProjectDocumentIndex::open(&root).unwrap();
    let clean = analyze_document_quality(&root, &documents, &manifest, &index).unwrap();
    assert!(!clean
        .issues
        .iter()
        .any(|issue| issue.issue_type == "implementation_drift"));

    fs::write(
        root.join("src/main.rs"),
        "fn main() { println!(\"changed\"); }\n",
    )
    .unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "pub fn ready() -> bool { false }\n",
    )
    .unwrap();
    let dirty = analyze_document_quality(&root, &documents, &manifest, &index).unwrap();
    let drift = dirty
        .issues
        .iter()
        .filter(|issue| issue.issue_type == "implementation_drift")
        .collect::<Vec<_>>();
    assert_eq!(drift.len(), 1);
    assert!(drift[0].message.contains("2 项"));
    drop(index);
    fs::remove_dir_all(root).unwrap();
}

fn entry(
    path: &str,
    title: &str,
    role: &str,
    default_retrieval: bool,
    headings: Vec<&str>,
) -> ProjectDocumentEntry {
    ProjectDocumentEntry {
        path: path.to_string(),
        title: title.to_string(),
        content: String::new(),
        truncated: false,
        byte_len: 10,
        source: "workspace".to_string(),
        metadata: ProjectDocumentMetadata {
            role: role.to_string(),
            lifecycle: "current".to_string(),
            authority: "project".to_string(),
            default_retrieval,
            content_hash: format!("hash-{path}"),
            headings: headings.into_iter().map(str::to_string).collect(),
            ..ProjectDocumentMetadata::default()
        },
    }
}

fn workspace(label: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "elon_document_quality_{label}_{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn git(root: &std::path::Path, arguments: &[&str]) {
    let status = crate::git_command_error::git_command()
        .arg("-C")
        .arg(root)
        .args(arguments)
        .status()
        .unwrap();
    assert!(status.success(), "git {arguments:?} failed");
}

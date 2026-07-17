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
        "# Home\n\n[Guide](docs/guide.md#missing) [Gone](docs/gone.md)\n",
    )
    .unwrap();
    fs::write(root.join("docs/guide.md"), "# Guide\n\nUseful.\n").unwrap();
    fs::write(root.join("docs/orphan.md"), "# Orphan\n\nHidden.\n").unwrap();

    let documents = vec![
        entry("README.md", "Home", "router", true, vec!["Home"]),
        entry("docs/guide.md", "Guide", "spec", false, vec!["Guide"]),
        entry("docs/orphan.md", "Orphan", "spec", false, vec!["Orphan"]),
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
    assert_eq!(report.summary.status, "needs_attention");
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

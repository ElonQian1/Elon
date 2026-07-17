use super::*;
use std::fs;

use crate::{
    project_document_federation::{KnowledgeFederationHealth, KnowledgeNodeHealth},
    project_document_governance::{DocumentKnowledgeMetadata, DocumentSectionManifest},
};

fn workspace() -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "elon_issue_workflow_{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn federation() -> KnowledgeFederationHealth {
    KnowledgeFederationHealth {
        enabled: true,
        source: "manifest",
        root_id: "root".into(),
        node_count: 2,
        aggregated_score: 90,
        unhealthy_nodes: 0,
        max_depth: 1,
        nodes: vec![
            KnowledgeNodeHealth {
                id: "root".into(),
                label: "Root".into(),
                parent_id: String::new(),
                scope_path: String::new(),
                profile: "software-platform".into(),
                owner: "team".into(),
                include_globs: vec![],
                exclude_globs: vec![],
                document_count: 1,
                direct_children: 1,
                score: 90,
                status: "healthy",
                home_configured: true,
            },
            KnowledgeNodeHealth {
                id: "docs".into(),
                label: "Docs".into(),
                parent_id: "root".into(),
                scope_path: "docs".into(),
                profile: "software-platform".into(),
                owner: "docs-team".into(),
                include_globs: vec!["docs/*.md".into()],
                exclude_globs: vec![],
                document_count: 1,
                direct_children: 0,
                score: 90,
                status: "healthy",
                home_configured: true,
            },
        ],
    }
}

#[test]
fn workflow_requires_reasons_and_persists_context() {
    let root = workspace();
    let index = ProjectDocumentIndex::open(&root).unwrap();
    let mut manifest = DocumentSectionManifest::default();
    manifest
        .assignments
        .insert("docs/api.md".into(), "custom:backend".into());
    manifest
        .secondary_assignments
        .insert("docs/api.md".into(), vec!["custom:reference".into()]);
    manifest.document_metadata.insert(
        "docs/api.md".into(),
        DocumentKnowledgeMetadata {
            owner: "backend-team".into(),
            ..Default::default()
        },
    );
    let issue = json!({
        "fingerprint": "abcdef0123456789abcdef0123456789", "type": "missing_review_date",
        "severity": "warning", "path": "docs/api.md", "message": "missing", "evidence": "none",
        "suggested_action": "review", "confidence": 100
    });
    let report = synchronize(
        &index,
        vec![issue],
        &manifest,
        &federation(),
        (88, 90, 80, 90),
    )
    .unwrap();
    assert_eq!(report["issues"][0]["workflow"]["owner"], "backend-team");
    assert_eq!(report["issues"][0]["context"]["scope_id"], "docs");
    assert_eq!(
        report["issues"][0]["context"]["secondary_topics"][0],
        "custom:reference"
    );

    let invalid = update_issue(
        &index,
        IssueWorkflowUpdate {
            fingerprint: "abcdef0123456789abcdef0123456789".into(),
            status: "ignored".into(),
            ..Default::default()
        },
    );
    assert!(invalid.is_err());
    let updated = update_issue(
        &index,
        IssueWorkflowUpdate {
            fingerprint: "abcdef0123456789abcdef0123456789".into(),
            status: "assigned".into(),
            owner: "alice".into(),
            due_at: "2027-01-02".into(),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(updated.owner, "alice");
    assert_eq!(
        list_filtered(&index, &[], &["assigned".into()], &[], "alice", 0, 20)
            .unwrap()
            .len(),
        1
    );
    assert!(!health_trend(&index, 10).unwrap().is_empty());
    fs::remove_dir_all(root).unwrap();
}

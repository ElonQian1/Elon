use super::*;
use homecli_proto::{ProjectDocumentEntry, ProjectDocumentMetadata};
use std::path::Path;

use crate::project_document_governance_service::analyze_workspace;

fn document(path: &str, title: &str, role: &str, ambiguous: bool) -> ProjectDocumentEntry {
    ProjectDocumentEntry {
        path: path.to_string(),
        title: title.to_string(),
        content: String::new(),
        truncated: false,
        byte_len: 0,
        source: "workspace".to_string(),
        metadata: ProjectDocumentMetadata {
            role: role.to_string(),
            lifecycle: if ambiguous { "unclassified" } else { "active" }.to_string(),
            ambiguous,
            ..ProjectDocumentMetadata::default()
        },
    }
}

#[test]
fn metadata_only_health_separates_topics_from_governance() {
    let documents = vec![
        document("README.md", "Overview", "guide", false),
        document(
            "docs/system-architecture.md",
            "System Architecture",
            "architecture",
            false,
        ),
        document("docs/inbox/idea.md", "Idea", "note", true),
    ];
    let mut manifest = DocumentSectionManifest::default();
    manifest.profile = "software-api".to_string();
    manifest.home.title = "Test API".to_string();
    manifest.home.summary = "Reference".to_string();
    manifest.home.entrypoint = "README.md".to_string();
    manifest
        .assignments
        .insert("README.md".to_string(), "custom:overview".to_string());
    manifest
        .governance_overrides
        .insert("README.md".to_string(), "on-demand".to_string());

    let health = analyze_knowledge_architecture(&documents, &manifest);
    assert_eq!(health.profile, "software-api");
    assert_eq!(health.profile_source, "manifest");
    assert_eq!(health.topic_assigned_documents, 3);
    assert_eq!(health.topic_unassigned_documents, 0);
    assert_eq!(health.ambiguous_documents, 1);
    assert!(health.missing_document_types.contains(&"quickstart"));
}

#[test]
fn platform_profile_is_inferred_without_reading_bodies() {
    let documents = vec![
        document("android/app/README.md", "Android", "guide", false),
        document("pc-frontend/README.md", "PC", "guide", false),
        document("server/src/api.md", "Server API", "guide", false),
        document("docs/node-agent.md", "Node Agent", "guide", false),
    ];

    let health = analyze_knowledge_architecture(&documents, &DocumentSectionManifest::default());
    assert_eq!(health.profile, "software-platform");
    assert_eq!(health.profile_source, "metadata");
    assert!(!health.recommended_sections.is_empty());
}

#[test]
fn repository_self_project_is_scanned_with_zero_model_tokens() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let analysis = analyze_workspace(root, 0, 200, false).unwrap();
    let health = &analysis["knowledge_architecture"];

    assert!(analysis["documents"].as_array().unwrap().len() > 100);
    assert_eq!(analysis["budget"]["classification_model_tokens"], 0);
    assert_eq!(health["profile"], "software-platform");
    assert_eq!(health["home_configured"], true);
    assert!(health["topic_sections"].as_u64().unwrap() >= 12);
    assert_eq!(health["topic_unassigned_documents"], 0);
    assert!(health["score"].as_u64().unwrap() >= 85);
    assert_eq!(analysis["document_health"]["source"], "server");
    assert_eq!(analysis["document_health"]["federation"]["node_count"], 5);
    assert_eq!(
        analysis["document_health"]["maintenance"]["durable_queue"],
        true
    );
    let quality = &analysis["document_health"]["quality"];
    assert_eq!(quality["summary"]["errors"], 0);
    assert_eq!(quality["summary"]["warnings"], 0);
    assert!(quality["summary"]["orphan_documents"].as_u64().unwrap() <= 2);
    assert!(!quality["issues"].as_array().unwrap().iter().any(|issue| {
        let path = issue["path"].as_str().unwrap_or_default();
        path.starts_with(".github/agents/")
            || path.starts_with(".github/prompts/")
            || path.starts_with(".github/skills/")
            || matches!(path, "AI_RULES.md" | "AI_TASK_TEMPLATE.md" | "CODEX.md")
    }));
}

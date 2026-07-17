use super::*;
use homecli_proto::{ProjectDocumentEntry, ProjectDocumentMetadata};
use std::fs;

#[test]
fn explicit_federation_reports_root_and_scoped_module_health() {
    let root = std::env::temp_dir().join(format!(
        "elon_document_federation_{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(root.join(".elon")).unwrap();
    fs::write(
        root.join(FEDERATION_CONFIG_PATH),
        r#"{
          "version": 1,
          "nodes": [
            {"id":"root","label":"Root"},
            {"id":"api","label":"API","parent_id":"root","scope_path":"docs/api"}
          ]
        }"#,
    )
    .unwrap();
    let documents = vec![entry("README.md"), entry("docs/api/README.md")];
    let health =
        analyze_federation(&root, &documents, &DocumentSectionManifest::default()).unwrap();

    assert!(health.enabled);
    assert_eq!(health.node_count, 2);
    assert_eq!(health.max_depth, 1);
    assert_eq!(
        documents_for_node(&documents, &health, Some("api"))
            .unwrap()
            .len(),
        1
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cyclic_nodes_are_rejected() {
    let nodes = vec![
        KnowledgeNode {
            id: "a".to_string(),
            label: "A".to_string(),
            parent_id: "b".to_string(),
            ..KnowledgeNode::default()
        },
        KnowledgeNode {
            id: "b".to_string(),
            label: "B".to_string(),
            parent_id: "a".to_string(),
            ..KnowledgeNode::default()
        },
    ];
    assert!(normalized_nodes(nodes).is_err());
}

fn entry(path: &str) -> ProjectDocumentEntry {
    ProjectDocumentEntry {
        path: path.to_string(),
        title: path.to_string(),
        content: String::new(),
        truncated: false,
        byte_len: 10,
        source: "workspace".to_string(),
        metadata: ProjectDocumentMetadata {
            lifecycle: "current".to_string(),
            content_hash: format!("hash-{path}"),
            ..ProjectDocumentMetadata::default()
        },
    }
}

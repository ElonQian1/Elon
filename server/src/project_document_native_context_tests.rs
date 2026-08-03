use sha2::{Digest, Sha256};
use std::fs;

use crate::{
    project_document_native_context::{
        normalize_memories, ProjectContextEvidence, ProjectContextMemory,
    },
    project_document_native_context_projection::relevant_memories,
};

#[test]
fn memory_normalization_derives_stable_id_without_bodies() {
    let memory = normalize_memories(vec![ProjectContextMemory {
        summary: "The API entrypoint delegates authorization to the policy module.".into(),
        topics: vec!["api".into(), "api".into()],
        evidence: vec![ProjectContextEvidence {
            path: "server/src/main.rs".into(),
            content_hash: "a".repeat(64),
            locator: "routes".into(),
            evidence_kind: "source".into(),
        }],
        reviewed_at: "catalog-revision-1".into(),
        ..Default::default()
    }])
    .unwrap()
    .remove(0);
    assert!(memory.candidate_id.starts_with("native-"));
    assert_eq!(memory.topics, vec!["api"]);
}

#[test]
fn portable_memory_is_returned_only_while_evidence_hash_matches() {
    let root = std::env::temp_dir().join(format!(
        "elon_native_context_{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(root.join("src")).unwrap();
    let source = b"pub fn project_routes() {}\n";
    fs::write(root.join("src/routes.rs"), source).unwrap();
    let memory = normalize_memories(vec![ProjectContextMemory {
        summary: "Project document routes are registered in the routes module.".into(),
        topics: vec!["project routes".into()],
        evidence: vec![ProjectContextEvidence {
            path: "src/routes.rs".into(),
            content_hash: format!("{:x}", Sha256::digest(source)),
            locator: "project_routes".into(),
            evidence_kind: "source".into(),
        }],
        reviewed_at: "catalog-revision-1".into(),
        ..Default::default()
    }])
    .unwrap()
    .remove(0);

    let current = relevant_memories(&root, "project routes", &[memory.clone()], 3);
    assert_eq!(current["selected_count"], 1);
    fs::write(root.join("src/routes.rs"), "pub fn changed_routes() {}\n").unwrap();
    let drifted = relevant_memories(&root, "project routes", &[memory], 3);
    assert_eq!(drifted["selected_count"], 0);
    assert_eq!(drifted["invalidated_count"], 1);
    fs::remove_dir_all(root).unwrap();
}

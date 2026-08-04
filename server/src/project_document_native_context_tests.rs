use sha2::{Digest, Sha256};
use std::fs;

use crate::{
    project_document_native_context::{
        bind_current_evidence_hashes, normalize_memories, ProjectContextEvidence,
        ProjectContextMemory, ProjectContextMemoryReview, ProjectContextMemoryScope,
    },
    project_document_native_context_health::{
        memory_health_report_with_options, MemoryHealthOptions,
    },
    project_document_native_context_projection::{relevant_memories, MemoryRetrievalScope},
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
            git_identity: None,
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
fn old_memory_json_gets_compatible_lifecycle_defaults() {
    let memory: ProjectContextMemory = serde_json::from_value(serde_json::json!({
        "candidate_id":"native-old",
        "summary":"An old shared memory remains readable after lifecycle fields are added.",
        "topics":["compatibility"],
        "evidence":[{"path":"README.md","content_hash":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}],
        "reviewed_at":"catalog:old"
    }))
    .unwrap();
    assert!(memory.owner.is_empty());
    assert_eq!(memory.scope.kind, "repository");
    assert_eq!(memory.review.review_interval_days, 0);
}

#[test]
fn memory_ci_paginates_and_recommends_strict_failure_without_mutation() {
    let root = std::env::temp_dir().join(format!(
        "elon_native_health_{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).unwrap();
    let source = b"portable evidence\n";
    fs::write(root.join("README.md"), source).unwrap();
    let base = ProjectContextMemory {
        summary: "A portable shared memory has bounded evidence and lifecycle metadata.".into(),
        topics: vec!["portable memory".into()],
        evidence: vec![ProjectContextEvidence {
            path: "README.md".into(),
            content_hash: format!("{:x}", Sha256::digest(source)),
            evidence_kind: "document".into(),
            ..Default::default()
        }],
        reviewed_at: "catalog:test".into(),
        owner: "project_maintainers".into(),
        scope: ProjectContextMemoryScope {
            kind: "repository".into(),
            paths: Vec::new(),
            ..Default::default()
        },
        review: ProjectContextMemoryReview {
            reviewed_on: "2099-01-01".into(),
            reviewed_by: "reviewer".into(),
            review_interval_days: 3650,
            expires_at: String::new(),
        },
        ..Default::default()
    };
    let memories = normalize_memories(vec![
        base.clone(),
        ProjectContextMemory {
            summary: "A second portable shared memory exercises deterministic pagination.".into(),
            topics: vec!["pagination".into()],
            ..base
        },
    ])
    .unwrap();
    let report = memory_health_report_with_options(
        &root,
        &memories,
        &MemoryHealthOptions {
            offset: 0,
            limit: 1,
            failure_policy: "strict".into(),
            include_capabilities: false,
        },
    )
    .unwrap();
    assert_eq!(report["pagination"]["returned"], 1);
    assert_eq!(report["pagination"]["next_offset"], 1);
    assert_eq!(report["policy_outcome"]["recommended_exit_code"], 0);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn conflicting_shared_memories_are_not_injected_into_project_context() {
    let root = std::env::temp_dir().join(format!(
        "elon_native_conflict_{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(root.join("src")).unwrap();
    let source = b"pub fn authority() {}\n";
    fs::write(root.join("src/authority.rs"), source).unwrap();
    let evidence = vec![ProjectContextEvidence {
        path: "src/authority.rs".into(),
        content_hash: format!("{:x}", Sha256::digest(source)),
        evidence_kind: "source".into(),
        ..Default::default()
    }];
    let memories = normalize_memories(vec![
        ProjectContextMemory {
            summary: "The authority module accepts decisions through the first policy path.".into(),
            topics: vec!["authority".into()],
            evidence: evidence.clone(),
            reviewed_at: "catalog:first".into(),
            ..Default::default()
        },
        ProjectContextMemory {
            summary: "The authority module accepts decisions through a conflicting second path."
                .into(),
            topics: vec!["authority".into()],
            evidence,
            reviewed_at: "catalog:second".into(),
            ..Default::default()
        },
    ])
    .unwrap();
    let projection = relevant_memories(
        &root,
        "authority",
        &MemoryRetrievalScope::default(),
        &memories,
        3,
    );
    assert_eq!(projection["selected_count"], 0);
    assert_eq!(projection["invalidated_count"], 2);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn candidate_receipt_can_bind_current_hash_from_evidence_path() {
    let root = std::env::temp_dir().join(format!(
        "elon_native_receipt_{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(root.join("src")).unwrap();
    let source = b"pub fn native_receipt() {}\n";
    fs::write(root.join("src/receipt.rs"), source).unwrap();
    let memory = bind_current_evidence_hashes(
        &root,
        ProjectContextMemory {
            summary: "Native receipt evidence hashes are bound by the local server.".into(),
            topics: vec!["native receipt".into()],
            evidence: vec![ProjectContextEvidence {
                path: "src/receipt.rs".into(),
                locator: "native_receipt".into(),
                evidence_kind: "source".into(),
                ..Default::default()
            }],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        memory.evidence[0].content_hash,
        format!("{:x}", Sha256::digest(source))
    );
    fs::remove_dir_all(root).unwrap();
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
            git_identity: None,
        }],
        reviewed_at: "catalog-revision-1".into(),
        ..Default::default()
    }])
    .unwrap()
    .remove(0);

    let current = relevant_memories(
        &root,
        "project routes",
        &MemoryRetrievalScope::default(),
        &[memory.clone()],
        3,
    );
    assert_eq!(current["selected_count"], 1);
    fs::write(root.join("src/routes.rs"), "pub fn changed_routes() {}\n").unwrap();
    let drifted = relevant_memories(
        &root,
        "project routes",
        &MemoryRetrievalScope::default(),
        &[memory],
        3,
    );
    assert_eq!(drifted["selected_count"], 0);
    assert_eq!(drifted["invalidated_count"], 1);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn path_scoped_memory_requires_an_overlapping_task_path() {
    let root = std::env::temp_dir().join(format!(
        "elon_native_scope_{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(root.join("server/src")).unwrap();
    let source = b"pub fn scoped_route() {}\n";
    fs::write(root.join("server/src/scoped.rs"), source).unwrap();
    let memory = normalize_memories(vec![ProjectContextMemory {
        summary: "The scoped server route is owned by the project memory module.".into(),
        topics: vec!["project memory".into()],
        evidence: vec![ProjectContextEvidence {
            path: "server/src/scoped.rs".into(),
            content_hash: format!("{:x}", Sha256::digest(source)),
            ..Default::default()
        }],
        reviewed_at: "catalog:scope".into(),
        scope: ProjectContextMemoryScope {
            kind: "paths".into(),
            paths: vec!["server/src".into()],
            ..Default::default()
        },
        ..Default::default()
    }])
    .unwrap();
    let unrelated = relevant_memories(
        &root,
        "project memory",
        &MemoryRetrievalScope {
            task_paths: vec!["pc-frontend/src".into()],
            ..Default::default()
        },
        &memory,
        3,
    );
    assert_eq!(unrelated["selected_count"], 0);
    assert_eq!(unrelated["scope_filtered_count"], 1);
    let related = relevant_memories(
        &root,
        "project memory",
        &MemoryRetrievalScope {
            task_paths: vec!["server/src/node_agent.rs".into()],
            ..Default::default()
        },
        &memory,
        3,
    );
    assert_eq!(related["selected_count"], 1);
    fs::remove_dir_all(root).unwrap();
}

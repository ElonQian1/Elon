use chrono::{Duration, Utc};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

use crate::{
    git_command_error::git_command,
    project_document_governance::{DocumentSectionManifest, SECTION_CONFIG_PATH},
    project_document_native_context::{
        bind_current_evidence_hashes, normalize_memories, ProjectContextEvidence,
        ProjectContextMemory, ProjectContextMemoryReview, ProjectContextMemoryScope,
    },
    project_document_native_context_health::{
        memory_health_report_with_options, shared_memory_health, MemoryHealthOptions,
    },
    project_document_native_context_projection::{relevant_memories, MemoryRetrievalScope},
};

#[test]
fn memory_ci_matrix_covers_portable_lifecycle_without_repository_writes() {
    let root = workspace();
    for name in [
        "legacy.md",
        "expired.md",
        "overdue.md",
        "relocated.md",
        "conflict.md",
        "healthy.md",
    ] {
        fs::write(root.join("docs").join(name), format!("# {name}\n")).unwrap();
    }
    git(&root, &["init", "--quiet"]);
    git(&root, &["config", "user.name", "Codex"]);
    git(&root, &["config", "user.email", "codex@example.invalid"]);
    git(&root, &["add", "docs"]);
    git(&root, &["commit", "--quiet", "-m", "initial"]);

    let today = Utc::now().date_naive();
    let current_review = review(today, 3650, None);
    let drifted = bound_memory(
        &root,
        "drifted",
        "Relocated memory health evidence must require an explicit reviewed replacement.",
        "docs/relocated.md",
        current_review.clone(),
    );
    let expired = bound_memory(
        &root,
        "expired",
        "Expired memory health evidence must not enter agent context.",
        "docs/expired.md",
        review(
            today - Duration::days(2),
            3650,
            Some(today - Duration::days(1)),
        ),
    );
    let overdue = bound_memory(
        &root,
        "overdue",
        "Overdue memory health evidence remains current but needs review.",
        "docs/overdue.md",
        review(today - Duration::days(31), 30, None),
    );
    let conflict_a = bound_memory(
        &root,
        "conflict-a",
        "Memory health authority is owned by the first reviewed policy.",
        "docs/conflict.md",
        current_review.clone(),
    );
    let conflict_b = bind_current_evidence_hashes(
        &root,
        ProjectContextMemory {
            candidate_id: "conflict-b".into(),
            summary: "Memory health authority is owned by a different reviewed policy.".into(),
            topics: vec!["memory health".into()],
            evidence: vec![ProjectContextEvidence {
                path: "docs/conflict.md".into(),
                evidence_kind: "document".into(),
                ..Default::default()
            }],
            reviewed_at: "catalog:conflict-b".into(),
            owner: "project_maintainers".into(),
            scope: ProjectContextMemoryScope::default(),
            review: current_review.clone(),
        },
    )
    .unwrap();
    let healthy = bound_memory(
        &root,
        "healthy",
        "Healthy memory health evidence remains available to agents.",
        "docs/healthy.md",
        current_review,
    );
    let legacy_bytes = fs::read(root.join("docs/legacy.md")).unwrap();
    let legacy: ProjectContextMemory = serde_json::from_value(json!({
        "candidate_id": "legacy",
        "summary": "Legacy memory health evidence stays readable while governance is completed.",
        "topics": ["memory health"],
        "evidence": [{
            "path": "docs/legacy.md",
            "content_hash": format!("{:x}", Sha256::digest(legacy_bytes)),
            "evidence_kind": "document"
        }],
        "reviewed_at": "catalog:legacy"
    }))
    .unwrap();

    git(
        &root,
        &["mv", "docs/relocated.md", "docs/relocated-current.md"],
    );
    git(&root, &["commit", "--quiet", "-m", "relocate"]);
    let memories = normalize_memories(vec![
        legacy.clone(),
        expired.clone(),
        overdue.clone(),
        drifted.clone(),
        conflict_a.clone(),
        conflict_b.clone(),
        healthy,
    ])
    .unwrap();

    let advisory = report(&root, &memories, "advisory");
    assert_eq!(advisory["checked_count"], 7);
    assert_eq!(advisory["healthy_count"], 1);
    assert_eq!(advisory["current_count"], 3);
    assert_eq!(advisory["drifted_count"], 1);
    assert_eq!(advisory["relocation_suggested_count"], 1);
    assert_eq!(advisory["expired_count"], 1);
    assert_eq!(advisory["review_overdue_count"], 1);
    assert_eq!(advisory["governance_incomplete_count"], 1);
    assert_eq!(advisory["potential_conflict_count"], 2);
    assert_eq!(advisory["policy_outcome"]["status"], "warn");
    assert_eq!(advisory["policy_outcome"]["recommended_exit_code"], 0);
    assert!(advisory["items"].as_array().unwrap().iter().all(|item| {
        item["source_bodies_returned"] == 0
            && item["repair_plan"]
                .as_array()
                .unwrap()
                .iter()
                .all(|action| action["automatic"] == false)
    }));
    let drift_item = item(&advisory, "drifted");
    assert_eq!(drift_item["status"], "relocation_suggested");
    assert_eq!(
        drift_item["relocation_candidates"][0],
        "docs/relocated-current.md"
    );
    assert_eq!(item(&advisory, "legacy")["status"], "governance_incomplete");

    let fail_on_drift = report(&root, &memories, "fail_on_drift");
    assert_eq!(fail_on_drift["policy_outcome"]["status"], "fail");
    assert_eq!(fail_on_drift["policy_outcome"]["recommended_exit_code"], 1);
    let governance_only = vec![
        legacy.clone(),
        overdue,
        conflict_a.clone(),
        conflict_b.clone(),
    ];
    let tolerated = report(&root, &governance_only, "fail_on_drift");
    assert_eq!(tolerated["policy_outcome"]["status"], "warn");
    assert_eq!(tolerated["policy_outcome"]["recommended_exit_code"], 0);
    let strict = report(&root, &governance_only, "strict");
    assert_eq!(strict["policy_outcome"]["status"], "fail");
    assert_eq!(strict["policy_outcome"]["recommended_exit_code"], 1);

    assert_projection_invalidated(&root, &[expired], "memory_expired");
    assert_projection_invalidated(&root, &[drifted], "path_relocation_suggested");
    let conflicts = relevant_memories(
        &root,
        "memory health",
        &MemoryRetrievalScope::default(),
        &[conflict_a, conflict_b],
        3,
    );
    assert_eq!(conflicts["selected_count"], 0);
    assert_eq!(conflicts["invalidated_count"], 2);
    assert_eq!(
        conflicts["invalidated"][0]["reason"],
        "shared_memory_conflict"
    );
    let compatible = relevant_memories(
        &root,
        "memory health",
        &MemoryRetrievalScope::default(),
        &[legacy],
        3,
    );
    assert_eq!(compatible["selected_count"], 1);

    let mut manifest = DocumentSectionManifest::default();
    manifest.context_memories = memories;
    let manifest_path = root.join(SECTION_CONFIG_PATH);
    fs::create_dir_all(manifest_path.parent().unwrap()).unwrap();
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let before = fs::read(&manifest_path).unwrap();
    let status_before = git_text(&root, &["status", "--porcelain=v1"]);
    let page = shared_memory_health(
        &root,
        &MemoryHealthOptions {
            offset: 2,
            limit: 2,
            failure_policy: "strict".into(),
            include_capabilities: false,
        },
    )
    .unwrap();
    assert_eq!(page["pagination"]["returned"], 2);
    assert_eq!(page["pagination"]["total"], 7);
    assert_eq!(page["pagination"]["next_offset"], 4);
    assert_eq!(page["source_bodies_returned"], 0);
    assert!(page.get("capabilities").is_none());
    assert_eq!(fs::read(&manifest_path).unwrap(), before);
    assert_eq!(
        git_text(&root, &["status", "--porcelain=v1"]),
        status_before
    );
    fs::remove_dir_all(root).unwrap();
}

fn report(root: &Path, memories: &[ProjectContextMemory], policy: &str) -> serde_json::Value {
    memory_health_report_with_options(
        root,
        memories,
        &MemoryHealthOptions {
            offset: 0,
            limit: 200,
            failure_policy: policy.into(),
            include_capabilities: false,
        },
    )
    .unwrap()
}

fn item<'a>(report: &'a serde_json::Value, candidate_id: &str) -> &'a serde_json::Value {
    report["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["candidate_id"] == candidate_id)
        .unwrap()
}

fn assert_projection_invalidated(root: &Path, memories: &[ProjectContextMemory], reason: &str) {
    let projection = relevant_memories(
        root,
        "memory health",
        &MemoryRetrievalScope::default(),
        memories,
        3,
    );
    assert_eq!(projection["selected_count"], 0);
    assert_eq!(projection["invalidated_count"], 1);
    assert_eq!(projection["invalidated"][0]["reason"], reason);
}

fn bound_memory(
    root: &Path,
    candidate_id: &str,
    summary: &str,
    path: &str,
    review: ProjectContextMemoryReview,
) -> ProjectContextMemory {
    bind_current_evidence_hashes(
        root,
        ProjectContextMemory {
            candidate_id: candidate_id.into(),
            summary: summary.into(),
            topics: vec!["memory health".into()],
            evidence: vec![ProjectContextEvidence {
                path: path.into(),
                evidence_kind: "document".into(),
                ..Default::default()
            }],
            reviewed_at: format!("catalog:{candidate_id}"),
            owner: "project_maintainers".into(),
            scope: ProjectContextMemoryScope::default(),
            review,
        },
    )
    .unwrap()
}

fn review(
    reviewed_on: chrono::NaiveDate,
    interval: u16,
    expires_at: Option<chrono::NaiveDate>,
) -> ProjectContextMemoryReview {
    ProjectContextMemoryReview {
        reviewed_on: reviewed_on.format("%Y-%m-%d").to_string(),
        reviewed_by: "reviewer".into(),
        review_interval_days: interval,
        expires_at: expires_at
            .map(|date| date.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
    }
}

fn workspace() -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "elon_memory_ci_matrix_{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(root.join("docs")).unwrap();
    root
}

fn git(root: &Path, args: &[&str]) {
    let status = git_command().args(args).current_dir(root).status().unwrap();
    assert!(status.success(), "git command failed: {args:?}");
}

fn git_text(root: &Path, args: &[&str]) -> String {
    let output = git_command().args(args).current_dir(root).output().unwrap();
    assert!(output.status.success(), "git command failed: {args:?}");
    String::from_utf8_lossy(&output.stdout).to_string()
}

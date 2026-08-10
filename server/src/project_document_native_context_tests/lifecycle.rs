use sha2::{Digest, Sha256};
use std::{fs, path::Path};

use crate::{
    project_document_authorization::DocumentAutomationMode,
    project_document_index::state_root,
    project_document_native_context::{ProjectContextEvidence, ProjectContextMemory},
    project_document_native_context_receipt::{
        record_attested_receipt, record_receipt, revise_candidate,
    },
    project_document_native_context_review::{candidate_page, review_candidates},
};

#[test]
fn candidate_receipt_review_and_restore_stay_outside_the_repository() {
    let root = workspace("lifecycle");
    fs::write(root.join("src/first.rs"), "pub fn first_entry() {}\n").unwrap();
    fs::write(root.join("src/second.rs"), "pub fn second_entry() {}\n").unwrap();
    let first = memory(
        "src/first.rs",
        "The first source owns the native candidate lifecycle entrypoint.",
        "candidate lifecycle",
    );
    let second = memory(
        "src/second.rs",
        "The second source proves bounded atomic receipt ingestion.",
        "atomic receipt",
    );

    let invalid_batch = record_receipt(
        &root,
        vec![
            first.clone(),
            memory(
                "src/missing.rs",
                "A missing evidence path must reject the complete receipt batch.",
                "invalid evidence",
            ),
        ],
        "codex_test",
    );
    assert!(invalid_batch.is_err());
    assert_eq!(
        candidate_page(&root, "pending", 0, 10).unwrap()["pagination"]["total"],
        0
    );

    let receipt = record_receipt(&root, vec![first, second], "codex_test").unwrap();
    assert_eq!(receipt["recorded_count"], 2);
    assert_eq!(receipt["effect_receipt"]["source_bodies_stored"], 0);
    assert_eq!(receipt["repository_changed"], false);

    let first_page = candidate_page(&root, "pending", 0, 1).unwrap();
    assert_eq!(first_page["pagination"]["returned"], 1);
    assert_eq!(first_page["pagination"]["total"], 2);
    assert_eq!(first_page["pagination"]["next_offset"], 1);
    assert_eq!(first_page["source_bodies_returned"], 0);
    let candidate_id = first_page["candidates"][0]["candidate_id"]
        .as_str()
        .unwrap()
        .to_string();

    let rejected = review_candidates(
        &root,
        vec![candidate_id.clone()],
        "reject",
        DocumentAutomationMode::SuggestionsOnly,
        None,
        None,
        "task_local",
    )
    .unwrap();
    assert_eq!(rejected["candidate_status"], "rejected");
    assert_eq!(rejected["review_reason"], "task_local");
    assert_eq!(rejected["repository_changed"], false);
    let rejected_page = candidate_page(&root, "rejected", 0, 10).unwrap();
    assert_eq!(
        rejected_page["candidates"][0]["review_feedback"]["reason"],
        "task_local"
    );

    let restored = review_candidates(
        &root,
        vec![candidate_id],
        "restore",
        DocumentAutomationMode::SuggestionsOnly,
        None,
        None,
        "",
    )
    .unwrap();
    assert_eq!(restored["candidate_status"], "pending");
    let pending = candidate_page(&root, "pending", 0, 10).unwrap();
    assert_eq!(pending["counts"]["pending"], 2);
    assert_eq!(
        pending["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|candidate| candidate["candidate_id"] == restored["candidate_ids"][0])
            .unwrap()["review_feedback"]["decision"],
        ""
    );

    assert!(!root.join(".elon").exists());
    assert_eq!(
        fs::read_to_string(root.join("src/first.rs")).unwrap(),
        "pub fn first_entry() {}\n"
    );
    cleanup(&root);
}

#[test]
fn attested_receipt_replay_preserves_human_revision_and_rejects_stale_cas() {
    let root = workspace("attested-revision");
    fs::write(root.join("src/authority.rs"), "pub fn authority() {}\n").unwrap();
    let original = memory(
        "src/authority.rs",
        "The native context authority is initially summarized by the task agent.",
        "native authority",
    );
    let session_id = "private-session-identifier";

    let first = record_attested_receipt(
        &root,
        vec![original.clone()],
        "codex_native_tools",
        session_id,
    )
    .unwrap();
    let candidate = &first["candidates"][0];
    let candidate_id = candidate["candidate_id"].as_str().unwrap();
    let updated_at_ms = candidate["updated_at_ms"].as_u64().unwrap();
    assert_eq!(candidate["provenance"]["source"], "receipt_profile");
    assert_eq!(
        candidate["provenance"]["assurance"],
        "local_mcp_session_attested"
    );
    assert_eq!(
        candidate["provenance"]["session_fingerprint"]
            .as_str()
            .unwrap()
            .len(),
        24
    );
    assert!(!serde_json::to_string(&first).unwrap().contains(session_id));

    let human_summary =
        "A human reviewer confirms the authority path and keeps the original evidence binding.";
    let revised = revise_candidate(
        &root,
        candidate_id,
        updated_at_ms,
        human_summary.into(),
        vec!["reviewed authority".into()],
    )
    .unwrap();
    assert_eq!(revised["candidate"]["summary"], human_summary);
    assert_eq!(
        revised["candidate"]["provenance"]["last_editor"],
        "pc_document_review"
    );
    assert!(revise_candidate(
        &root,
        candidate_id,
        updated_at_ms,
        "A stale editor must not overwrite the accepted human revision.".into(),
        vec!["stale edit".into()],
    )
    .is_err());

    let replay = record_attested_receipt(
        &root,
        vec![original],
        "codex_native_tools",
        "second-session-identifier",
    )
    .unwrap();
    assert_eq!(replay["status"], "no_new_candidate");
    assert_eq!(replay["deduplicated_count"], 1);
    assert_eq!(replay["candidates"][0]["summary"], human_summary);
    assert_eq!(
        replay["candidates"][0]["provenance"]["last_editor"],
        "pc_document_review"
    );
    let page = candidate_page(&root, "pending", 0, 10).unwrap();
    assert_eq!(page["pagination"]["total"], 1);
    assert_eq!(page["candidates"][0]["summary"], human_summary);
    assert_eq!(page["source_bodies_returned"], 0);
    cleanup(&root);
}

fn workspace(label: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "elon_native_candidate_{label}_{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(root.join("src")).unwrap();
    root
}

fn memory(path: &str, summary: &str, topic: &str) -> ProjectContextMemory {
    ProjectContextMemory {
        summary: summary.into(),
        topics: vec![topic.into()],
        evidence: vec![ProjectContextEvidence {
            path: path.into(),
            locator: "entry".into(),
            evidence_kind: "source".into(),
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn cleanup(workspace: &Path) {
    let canonical = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    let key = format!(
        "{:x}",
        Sha256::digest(canonical.to_string_lossy().replace('\\', "/").as_bytes())
    );
    let database = state_root().join("indexes").join(format!("{key}.sqlite3"));
    fs::remove_file(&database).ok();
    fs::remove_file(format!("{}-wal", database.display())).ok();
    fs::remove_file(format!("{}-shm", database.display())).ok();
    fs::remove_dir_all(workspace).ok();
}

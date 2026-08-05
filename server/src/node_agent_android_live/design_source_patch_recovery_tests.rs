use std::fs;

use serde_json::json;

use super::{
    broker::LiveUiBroker,
    design_source_patch_store::{self as store, SourcePatchProposal},
    design_tools,
};

fn fixture_root(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "elon_design_source_patch_recovery_{name}_{}",
        uuid::Uuid::new_v4().simple()
    ))
}

async fn fixture_session(root: &std::path::Path) -> std::sync::Arc<super::broker::LiveUiSession> {
    LiveUiBroker::new()
        .create_session(
            "design-source-patch-recovery-test".to_string(),
            "ui.design.source-patch.recovery.test".to_string(),
            Some(root.display().to_string()),
            38922,
        )
        .await
}

fn applying_proposal(
    root: &std::path::Path,
    id_suffix: char,
    source: &[u8],
) -> (SourcePatchProposal, Vec<u8>) {
    let edits = store::build_edits(
        source,
        &json!([
            {"start":3,"end":6,"expectedBeforeSha256":store::source_sha(&source[3..6]),"replacement":"x"},
            {"start":9,"end":12,"expectedBeforeSha256":store::source_sha(&source[9..12]),"replacement":"yyyyy"}
        ]),
    )
    .unwrap();
    let output = store::apply_edits(source, &edits).unwrap();
    let proposal_id = format!("sourcepatch_{}", id_suffix.to_string().repeat(32));
    let review_artifact_path =
        store::write_review_artifact(root, &proposal_id, "source.txt", &edits).unwrap();
    (
        SourcePatchProposal {
            schema_version: 1,
            proposal_id,
            revision: 2,
            writeback_plan_id: format!("writeplan_{}", "1".repeat(32)),
            draft_id: format!("draft_{}", "2".repeat(32)),
            draft_revision: 1,
            source_file: "source.txt".to_string(),
            source_sha_before: store::source_sha(source),
            source_sha_after: store::source_sha(&output),
            edits,
            status: "APPLYING".to_string(),
            decision_reason: None,
            review_artifact_path,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            applied_at: None,
        },
        output,
    )
}

#[tokio::test]
async fn applying_journal_recovers_before_and_after_source_states() {
    let root = fixture_root("known-states");
    fs::create_dir_all(&root).unwrap();
    let root = root.canonicalize().unwrap();
    let session = fixture_session(&root).await;
    let source = b"AAA111BBB222CCC";
    let (proposal, output) = applying_proposal(&root, 'a', source);
    fs::write(root.join("source.txt"), source).unwrap();
    store::persist(&root, &proposal).unwrap();

    let applied = design_tools::call(
        &session,
        "ui_apply_design_source_patch",
        json!({"proposalId":proposal.proposal_id,"expectedRevision":2}),
    )
    .await
    .unwrap();
    assert_eq!(applied["proposal"]["status"], "APPLIED");
    assert_eq!(fs::read(root.join("source.txt")).unwrap(), output);
    let rollback = design_tools::call(
        &session,
        "ui_plan_design_source_rollback",
        json!({"proposalId":proposal.proposal_id,"expectedRevision":3}),
    )
    .await
    .unwrap();
    assert_eq!(rollback["rollback"]["edits"][0]["start"], 3);
    assert_eq!(rollback["rollback"]["edits"][0]["end"], 4);
    assert_eq!(rollback["rollback"]["edits"][1]["start"], 7);
    assert_eq!(rollback["rollback"]["edits"][1]["end"], 12);
    assert_eq!(
        rollback["rollback"]["targetSourceRevision"],
        store::source_sha(source)
    );

    let (already_applied, already_output) = applying_proposal(&root, 'b', source);
    fs::write(root.join("source.txt"), &already_output).unwrap();
    store::persist(&root, &already_applied).unwrap();
    let recovered = design_tools::call(
        &session,
        "ui_apply_design_source_patch",
        json!({"proposalId":already_applied.proposal_id,"expectedRevision":2}),
    )
    .await
    .unwrap();
    assert_eq!(recovered["proposal"]["status"], "APPLIED");
    assert_eq!(fs::read(root.join("source.txt")).unwrap(), already_output);
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn applying_journal_fails_closed_on_unknown_source_state() {
    let root = fixture_root("drift");
    fs::create_dir_all(&root).unwrap();
    let root = root.canonicalize().unwrap();
    let session = fixture_session(&root).await;
    let source = b"AAA111BBB222CCC";
    let (proposal, _) = applying_proposal(&root, 'c', source);
    fs::write(root.join("source.txt"), b"unexpected drift").unwrap();
    store::persist(&root, &proposal).unwrap();

    let error = design_tools::call(
        &session,
        "ui_apply_design_source_patch",
        json!({"proposalId":proposal.proposal_id,"expectedRevision":2}),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("SOURCE_PATCH_RECOVERY_REQUIRED"));
    let stored = store::read(&root, &proposal.proposal_id).unwrap();
    assert_eq!(stored.status, "APPLYING");
    assert_eq!(stored.revision, 2);
    assert_eq!(
        fs::read(root.join("source.txt")).unwrap(),
        b"unexpected drift"
    );
    fs::remove_dir_all(root).unwrap();
}

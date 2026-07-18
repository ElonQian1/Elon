use super::*;

fn temp_store() -> (PathBuf, UpdateRecoveryStore) {
    let root = std::env::temp_dir().join(format!("elon-update-recovery-{}", uuid::Uuid::new_v4()));
    let store = UpdateRecoveryStore::new(root.join("ledger.json"));
    (root, store)
}

#[test]
fn durable_lifecycle_is_correlated_and_replayable() {
    let (root, store) = temp_store();
    let mut receipt = UpdateRecoveryReceipt::planned("update-1", "root-1", "task-1");
    receipt.from_release = ReleaseIdentity {
        version: "1.0.0".to_string(),
        git_sha: "old".to_string(),
    };
    receipt.to_release = ReleaseIdentity {
        version: "1.1.0".to_string(),
        git_sha: "new".to_string(),
    };
    receipt.codex_session_id = Some("thread-1".to_string());
    receipt.codex_session_scope = Some("project:conversation".to_string());
    receipt.sidecar_session_id = Some("sidecar-1".to_string());
    receipt.journal_cursor = 41;
    receipt.workspace = WorkspaceGitFingerprint {
        workspace_path: r"D:\project".to_string(),
        git_head: Some("abc123".to_string()),
        git_status_sha256: Some("digest".to_string()),
        git_status_clean: Some(true),
    };
    store.upsert(receipt).expect("persist planned receipt");

    for state in [
        UpdateRecoveryState::Downloaded,
        UpdateRecoveryState::CheckpointSaved,
        UpdateRecoveryState::Applying,
        UpdateRecoveryState::RuntimeOnline,
        UpdateRecoveryState::Reattaching,
        UpdateRecoveryState::Resumed,
        UpdateRecoveryState::Verified,
    ] {
        assert!(store
            .transition("update-1", "task-1", state, Some("test transition"))
            .expect("transition receipt"));
    }

    let ledger = store.load().expect("reload ledger");
    let receipt = &ledger.receipts[0];
    assert_eq!(receipt.root_task_id, "root-1");
    assert_eq!(receipt.state, UpdateRecoveryState::Verified);
    assert_eq!(receipt.events.len(), 8);
    assert_eq!(receipt.events[0].event_id, "update-1:1:planned");
    assert_eq!(receipt.events[7].event_id, "update-1:8:verified");
    assert_eq!(receipt.final_reason.as_deref(), Some("test transition"));
    assert!(store.active().expect("read active receipts").is_empty());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn duplicate_transition_and_upsert_are_idempotent() {
    let (root, store) = temp_store();
    let receipt = UpdateRecoveryReceipt::planned("update-2", "root-2", "task-2");
    store.upsert(receipt.clone()).expect("first upsert");
    store.upsert(receipt).expect("duplicate upsert");
    assert_eq!(store.load().expect("load ledger").receipts.len(), 1);

    assert!(store
        .transition("update-2", "task-2", UpdateRecoveryState::Downloaded, None,)
        .expect("first transition"));
    assert!(!store
        .transition("update-2", "task-2", UpdateRecoveryState::Downloaded, None,)
        .expect("duplicate transition"));
    assert_eq!(
        store.load().expect("load ledger").receipts[0].events.len(),
        2
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn interrupted_apply_can_continue_after_repeated_restart() {
    let mut receipt = UpdateRecoveryReceipt::planned("update-3", "root-3", "task-3");
    receipt
        .transition(UpdateRecoveryState::Downloaded, None)
        .unwrap();
    receipt
        .transition(UpdateRecoveryState::CheckpointSaved, None)
        .unwrap();
    receipt
        .transition(UpdateRecoveryState::Applying, None)
        .unwrap();
    receipt
        .transition(
            UpdateRecoveryState::RuntimeOnline,
            Some("restart recovered"),
        )
        .unwrap();
    receipt
        .transition(UpdateRecoveryState::Conflict, Some("workspace drift"))
        .unwrap();
    receipt
        .transition(UpdateRecoveryState::RuntimeOnline, Some("drift reviewed"))
        .unwrap();
    receipt
        .transition(UpdateRecoveryState::ResumeCreated, None)
        .unwrap();
    receipt
        .transition(UpdateRecoveryState::Resumed, None)
        .unwrap();
    assert_eq!(receipt.state, UpdateRecoveryState::Resumed);
}

#[test]
fn verified_transaction_rejects_reexecution() {
    let mut receipt = UpdateRecoveryReceipt::planned("update-4", "root-4", "task-4");
    receipt.state = UpdateRecoveryState::Resumed;
    receipt
        .transition(UpdateRecoveryState::Verified, Some("done"))
        .unwrap();
    let error = receipt
        .transition(UpdateRecoveryState::RuntimeOnline, None)
        .expect_err("verified receipt must be terminal");
    assert!(error
        .to_string()
        .contains("invalid update recovery transition"));
}

#[test]
fn old_and_remote_protocol_defaults_remain_compatible() {
    let old = serde_json::json!({
        "update_id": "legacy-update",
        "root_task_id": "legacy-root",
        "original_task_id": "legacy-task"
    });
    let parsed: UpdateRecoveryReceipt = serde_json::from_value(old).expect("parse legacy receipt");
    assert_eq!(parsed.schema_version, 1);
    assert_eq!(parsed.protocol, UPDATE_RECOVERY_PROTOCOL);
    assert_eq!(parsed.state, UpdateRecoveryState::Planned);
    assert_eq!(parsed.transport.kind, "local_loopback");
    assert!(parsed.transport.supports("event_replay"));
    assert_eq!(parsed.transport.auth_mode, "loopback_admin_token");
    assert!(parsed.transport.replay_from_cursor);
    assert_eq!(parsed.expected_downtime_ms, 45_000);

    let remote = RecoveryTransport::remote_v1();
    assert_eq!(remote.protocol, "elon.node.v1");
    assert!(!remote.supports("update_recovery_v1"));
    let round_trip: RecoveryTransport =
        serde_json::from_value(serde_json::to_value(remote).expect("serialize remote transport"))
            .expect("deserialize remote transport");
    assert_eq!(round_trip.kind, "remote_relay");
    assert_eq!(round_trip.auth_mode, "remote_transport_auth");
    assert!(!round_trip.replay_from_cursor);
    assert!(!round_trip.supports("update_recovery_v1"));
}

#[test]
fn lifecycle_lookup_and_final_review_share_the_same_receipt() {
    let (root, store) = temp_store();
    let mut receipt = UpdateRecoveryReceipt::planned("update-6", "root-6", "task-6");
    receipt.resume_task_id = Some("resume-6".to_string());
    store.upsert(receipt).unwrap();
    assert_eq!(
        store
            .receipt_for_task("resume-6")
            .unwrap()
            .unwrap()
            .original_task_id,
        "task-6"
    );
    assert!(store
        .record_final_review(
            "task-6",
            UpdateRecoveryReview {
                verdict: "accepted".to_string(),
                summary: "reviewed".to_string(),
                reviewed_by: "codex_desktop".to_string(),
                reviewed_at_ms: 9,
            },
        )
        .unwrap());
    let status = store.status_payload(20).unwrap();
    assert_eq!(status["protocol"], UPDATE_RECOVERY_PROTOCOL);
    assert_eq!(status["receipts"][0]["final_review"]["verdict"], "accepted");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn invalid_state_skip_is_rejected() {
    let mut receipt = UpdateRecoveryReceipt::planned("update-5", "root-5", "task-5");
    let error = receipt
        .transition(UpdateRecoveryState::Applying, None)
        .expect_err("planned cannot skip durable checkpoint");
    assert!(error.to_string().contains("Planned -> Applying"));
}

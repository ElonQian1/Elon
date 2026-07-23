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
        ..WorkspaceGitFingerprint::default()
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
fn compatible_terminal_receipts_are_canonicalized_without_losing_raw_audit() {
    let (root, store) = temp_store();
    let mut sparse =
        UpdateRecoveryReceipt::planned("update-terminal-a", "root-terminal", "task-terminal");
    sparse.state = UpdateRecoveryState::Failed;
    sparse.updated_at_ms = 20;
    let mut detailed =
        UpdateRecoveryReceipt::planned("update-terminal-b", "root-terminal", "task-terminal");
    detailed.state = UpdateRecoveryState::Failed;
    detailed.updated_at_ms = 10;
    detailed.terminal_task_status = Some("done".to_string());
    detailed.terminal_finished_at_ms = Some(99);
    detailed.terminal_success = Some(true);
    detailed.terminal_outcome = Some("completed".to_string());
    detailed.completion_event_id = Some("event-terminal".to_string());
    store.upsert(sparse).unwrap();
    store.upsert(detailed).unwrap();

    let canonical = store
        .receipt_for_task("task-terminal")
        .expect("compatible terminal facts")
        .expect("canonical receipt");
    assert_eq!(canonical.state, UpdateRecoveryState::Failed);
    assert_eq!(canonical.terminal_task_status.as_deref(), Some("done"));
    assert_eq!(canonical.terminal_finished_at_ms, Some(99));
    assert_eq!(
        canonical.completion_event_id.as_deref(),
        Some("event-terminal")
    );
    assert_eq!(
        store.receipts_for_task("task-terminal").unwrap().len(),
        2,
        "canonical lookup must preserve the append-only receipt audit"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn conflicting_terminal_receipts_are_readable_and_remain_fail_closed() {
    let (root, store) = temp_store();
    let mut first =
        UpdateRecoveryReceipt::planned("update-conflict-a", "root-conflict", "task-conflict");
    first.state = UpdateRecoveryState::Failed;
    first.terminal_task_status = Some("done".to_string());
    let mut second =
        UpdateRecoveryReceipt::planned("update-conflict-b", "root-conflict", "task-conflict");
    second.state = UpdateRecoveryState::Failed;
    second.terminal_task_status = Some("failed".to_string());
    store.upsert(first).unwrap();
    store.upsert(second).unwrap();

    let canonical = store
        .receipt_for_task("task-conflict")
        .expect("conflict must not make task detail fail")
        .expect("canonical conservative receipt");
    assert!(canonical.conflict_detected);
    assert_eq!(canonical.conflict_count, 2);
    assert!(canonical
        .conflict_reason
        .as_deref()
        .unwrap_or_default()
        .contains("terminal_task_status"));
    assert!(
        !canonical.state.is_terminal() || canonical.terminal_task_status.is_some(),
        "conflict view must preserve a real receipt rather than invent terminal facts"
    );
    assert_eq!(store.receipts_for_task("task-conflict").unwrap().len(), 2);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn compact_status_counts_all_receipts_and_pages_without_events() {
    let (root, store) = temp_store();
    let mut active = UpdateRecoveryReceipt::planned("update-page-a", "root-a", "task-a");
    active.updated_at_ms = 30;
    let mut failed = UpdateRecoveryReceipt::planned("update-page-b", "root-b", "task-b");
    failed.state = UpdateRecoveryState::Failed;
    failed.updated_at_ms = 20;
    let mut verified = UpdateRecoveryReceipt::planned("update-page-c", "root-c", "task-c");
    verified.state = UpdateRecoveryState::Verified;
    verified.updated_at_ms = 10;
    store.upsert(active).unwrap();
    store.upsert(failed).unwrap();
    store.upsert(verified).unwrap();

    let first = store.status_page_payload(0, 1, false).unwrap();
    assert_eq!(first["receipt_count"], 3);
    assert_eq!(first["active_count"], 1);
    assert_eq!(first["receipts"].as_array().unwrap().len(), 1);
    assert!(first["receipts"][0].get("events").is_none());
    assert_eq!(first["receipts"][0]["event_count"], 1);
    assert_eq!(first["next_cursor"], 1);
    assert_eq!(first["has_more"], true);

    let second = store.status_page_payload(1, 2, true).unwrap();
    assert_eq!(second["receipts"].as_array().unwrap().len(), 2);
    assert!(second["receipts"][0]["events"].is_array());
    assert_eq!(second["has_more"], false);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn terminal_binding_ignores_completed_parent_after_resume_child_exists() {
    let (root, store) = temp_store();
    let mut receipt = UpdateRecoveryReceipt::planned("update-7", "root-7", "task-7");
    receipt.resume_task_id = Some("resume-7".to_string());
    receipt.state = UpdateRecoveryState::Resumed;
    store.upsert(receipt).unwrap();

    assert_eq!(
        store
            .reconcile_terminal_completion(
                crate::node_agent_update_recovery_terminal::ExpectedRecovery::NotApplicable,
                "task-7",
                "old-canceled",
                "canceled",
                10,
                false,
                None,
            )
            .unwrap(),
        crate::node_agent_update_recovery_terminal::TerminalRecoveryDisposition::NotApplicable
    );
    let unchanged = store.load().unwrap().receipts.remove(0);
    assert!(unchanged.completion_event_id.is_none());
    assert!(unchanged.terminal_task_status.is_none());

    assert_eq!(
        store
            .reconcile_terminal_completion(
                crate::node_agent_update_recovery_terminal::ExpectedRecovery::Required,
                "resume-7",
                "resume-done",
                "done",
                20,
                true,
                None,
            )
            .unwrap(),
        crate::node_agent_update_recovery_terminal::TerminalRecoveryDisposition::Reconciled
    );
    let bound = store.load().unwrap().receipts.remove(0);
    assert_eq!(bound.completion_event_id.as_deref(), Some("resume-done"));
    assert_eq!(bound.terminal_task_status.as_deref(), Some("done"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn recovered_child_completion_idempotently_promotes_resumed_receipt() {
    let (root, store) = temp_store();
    let mut receipt =
        UpdateRecoveryReceipt::planned("update-terminal", "root-terminal", "parent-terminal");
    receipt.resume_task_id = Some("resume-terminal".to_string());
    receipt.state = UpdateRecoveryState::Resumed;
    store.upsert(receipt).unwrap();

    assert_eq!(
        store
            .reconcile_terminal_completion(
                crate::node_agent_update_recovery_terminal::ExpectedRecovery::Required,
                "resume-terminal",
                "event-terminal",
                "done",
                20,
                true,
                None,
            )
            .unwrap(),
        crate::node_agent_update_recovery_terminal::TerminalRecoveryDisposition::Reconciled
    );
    assert_eq!(
        store
            .reconcile_terminal_completion(
                crate::node_agent_update_recovery_terminal::ExpectedRecovery::Required,
                "resume-terminal",
                "event-terminal",
                "done",
                20,
                true,
                None,
            )
            .unwrap(),
        crate::node_agent_update_recovery_terminal::TerminalRecoveryDisposition::Reconciled
    );
    let verified = store.load().unwrap().receipts.remove(0);
    assert_eq!(verified.state, UpdateRecoveryState::Verified);
    assert_eq!(
        verified.completion_event_id.as_deref(),
        Some("event-terminal")
    );
    assert_eq!(verified.terminal_task_status.as_deref(), Some("done"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn terminal_preflight_rejects_unpromotable_recovery_without_writing() {
    let (root, store) = temp_store();
    store
        .upsert(UpdateRecoveryReceipt::planned(
            "update-preflight",
            "root-preflight",
            "task-preflight",
        ))
        .unwrap();
    let path = root.join("ledger.json");
    let before = std::fs::read(&path).unwrap();
    let error = store
        .preflight_terminal_completion(
            crate::node_agent_update_recovery_terminal::ExpectedRecovery::Required,
            "task-preflight",
            "event-preflight",
            "done",
            20,
            true,
            None,
        )
        .expect_err("planned recovery cannot skip directly to verified");
    assert!(error.to_string().contains("cannot accept"));
    assert_eq!(std::fs::read(&path).unwrap(), before);
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

#[test]
fn old_runtime_cannot_advance_target_install_gate() {
    let (root, store) = temp_store();
    let mut receipt = UpdateRecoveryReceipt::planned("update-target", "root", "task");
    receipt.state = UpdateRecoveryState::Applying;
    receipt.from_release = ReleaseIdentity {
        version: "1.0.0".into(),
        git_sha: "fromsha".into(),
    };
    receipt.to_release = ReleaseIdentity {
        version: "1.1.0".into(),
        git_sha: "tosha".into(),
    };
    store.upsert(receipt).unwrap();

    assert!(!store
        .mark_runtime_online_if_target("1.0.0+fromsha")
        .unwrap());
    assert_ne!(store.load().unwrap().install_gate.phase, "runtime_online");
    assert!(store.mark_runtime_online_if_target("1.1.0+tosha").unwrap());
    assert_eq!(store.load().unwrap().install_gate.phase, "runtime_online");
    let _ = std::fs::remove_dir_all(root);
}

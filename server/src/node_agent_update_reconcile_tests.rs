use super::*;

#[test]
fn stable_resume_child_is_idempotent_across_restart_callbacks() {
    let first = stable_resume_task_id("update-a", "task-a");
    let duplicate = stable_resume_task_id("update-a", "task-a");
    assert_eq!(first, duplicate);
    assert!(first.starts_with("local-recovery-"));
    assert_ne!(first, stable_resume_task_id("update-b", "task-a"));
}

#[test]
fn incomplete_publish_is_non_repeatable_until_result_arrives() {
    let call = crate::node_agent_task_journal::TaskJournalEventView {
        seq: 1,
        event: serde_json::json!({
            "type": "tool_call",
            "call_id": "publish-1",
            "tool": "publish_server"
        }),
    };
    assert_eq!(
        incomplete_non_repeatable_action(&[call.clone()]).as_deref(),
        Some("publish_server:publish-1")
    );
    let result = crate::node_agent_task_journal::TaskJournalEventView {
        seq: 2,
        event: serde_json::json!({"type":"tool_result","call_id":"publish-1"}),
    };
    assert!(incomplete_non_repeatable_action(&[call, result]).is_none());
}

#[test]
fn fingerprint_detects_git_or_workspace_drift() {
    let root = std::env::temp_dir().join(format!(
        "elon-update-fingerprint-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let status = crate::git_command_error::git_command()
        .arg("init")
        .current_dir(&root)
        .status();
    if status.as_ref().is_ok_and(|status| status.success()) {
        let before = fingerprint_workspace(&root);
        assert!(before.git_status_sha256.is_some());
        std::fs::write(root.join("new.txt"), b"drift").unwrap();
        let after = fingerprint_workspace(&root);
        assert_ne!(before.git_status_sha256, after.git_status_sha256);
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn recovery_state_keeps_restart_and_pause_paths_explicit() {
    let mut receipt = UpdateRecoveryReceipt::planned("update-c", "root-c", "task-c");
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
        .transition(UpdateRecoveryState::RuntimeOnline, None)
        .unwrap();
    receipt
        .transition(
            UpdateRecoveryState::ApprovalRequired,
            Some("approval pending"),
        )
        .unwrap();
    assert_eq!(receipt.state, UpdateRecoveryState::ApprovalRequired);
    assert!(!receipt.state.is_terminal());
}

#[test]
fn repeated_runtime_restart_can_reattach_same_resume_identity() {
    let mut receipt = UpdateRecoveryReceipt::planned("update-d", "root-d", "task-d");
    receipt.state = UpdateRecoveryState::Resumed;
    receipt.resume_task_id = Some("local-recovery-stable".to_string());
    receipt
        .transition(
            UpdateRecoveryState::Reattaching,
            Some("runtime restarted again"),
        )
        .unwrap();
    receipt
        .transition(UpdateRecoveryState::Resumed, Some("same child reattached"))
        .unwrap();
    assert_eq!(receipt.active_task_id(), "local-recovery-stable");
}

#[test]
fn startup_reconcile_advances_only_forward_from_applying() {
    let root = std::env::temp_dir().join(format!(
        "elon-update-advance-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let store = UpdateRecoveryStore::new(root.join("ledger.json"));
    let mut receipt = UpdateRecoveryReceipt::planned("update-e", "root-e", "task-e");
    receipt
        .transition(UpdateRecoveryState::Downloaded, None)
        .unwrap();
    receipt
        .transition(UpdateRecoveryState::CheckpointSaved, None)
        .unwrap();
    receipt
        .transition(UpdateRecoveryState::Applying, None)
        .unwrap();
    store.upsert(receipt).unwrap();

    advance_runtime_online(&store, "update-e", "task-e").unwrap();

    let saved = store.load().unwrap().receipts.remove(0);
    assert_eq!(saved.state, UpdateRecoveryState::RuntimeOnline);
    assert_eq!(
        saved.events.last().unwrap().state,
        UpdateRecoveryState::RuntimeOnline
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn local_current_protocol_recovers_while_remote_and_capability_drift_fail_closed() {
    let local = UpdateRecoveryReceipt::planned("update-local", "root", "task");
    assert!(local.allows_local_reconcile());

    let mut remote = local.clone();
    remote.transport = crate::node_agent_update_recovery::RecoveryTransport::remote_v1();
    assert!(!remote.allows_local_reconcile());

    let mut drifted = local;
    drifted
        .transport
        .capabilities
        .retain(|capability| capability != "sidecar_reattach");
    assert!(!drifted.allows_local_reconcile());
}

#[test]
fn remote_v1_identity_capability_lease_and_disconnect_fixture_stays_fail_closed() {
    let local = UpdateRecoveryReceipt::planned("update-remote-fixture", "root", "task");
    let mut remote = local.clone();
    remote.transport = crate::node_agent_update_recovery::RecoveryTransport::remote_v1();
    remote.transport.capabilities = local.transport.capabilities.clone();
    remote.transport.replay_from_cursor = true;
    remote.transport.lease_id = Some("remote-lease".to_string());
    remote.transport.lease_expires_at_ms = Some(u128::MAX);
    assert!(!remote.allows_local_reconcile(), "remote identity cannot borrow the local recovery authority even with capabilities and a live lease");

    remote.transport.lease_expires_at_ms = Some(1);
    assert!(
        !remote.allows_local_reconcile(),
        "expired remote lease fails closed"
    );
    remote.transport.lease_id = None;
    remote.transport.lease_expires_at_ms = None;
    assert!(
        !remote.allows_local_reconcile(),
        "disconnected remote transport without a lease fails closed"
    );
}

#[test]
fn release_and_task_identity_drift_fail_closed() {
    let expected = crate::node_agent_update_recovery::ReleaseIdentity {
        version: "0.3.70".to_string(),
        git_sha: "abc123".to_string(),
    };
    assert!(release_identity_matches(&expected, "0.3.70+abc123"));
    assert!(!release_identity_matches(&expected, "0.3.70+def456"));
    assert!(!release_identity_matches(&expected, "0.3.71+abc123"));

    assert!(recovery_task_identity_matches(
        "owner-a",
        "agent-a",
        "install-a",
        "owner-a",
        "agent-a",
        "install-a",
    ));
    assert!(!recovery_task_identity_matches(
        "owner-a",
        "agent-a",
        "install-a",
        "owner-a",
        "agent-other",
        "install-a",
    ));
}

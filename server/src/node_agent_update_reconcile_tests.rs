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

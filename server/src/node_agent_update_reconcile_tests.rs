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

#[test]
fn from_release_restart_is_deferred_instead_of_target_mismatch_failure() {
    let mut receipt = UpdateRecoveryReceipt::planned("update-release", "root", "task");
    receipt.from_release = crate::node_agent_update_recovery::ReleaseIdentity {
        version: "0.3.70".to_string(),
        git_sha: "from123".to_string(),
    };
    receipt.to_release = crate::node_agent_update_recovery::ReleaseIdentity {
        version: "0.3.71".to_string(),
        git_sha: "target456".to_string(),
    };

    assert_eq!(
        release_relation(&receipt, "0.3.70+from123"),
        ReleaseRelation::From
    );
    assert_eq!(
        release_relation(&receipt, "0.3.71+target456"),
        ReleaseRelation::Target
    );
    assert_eq!(
        release_relation(&receipt, "0.3.72+foreign"),
        ReleaseRelation::Other
    );
}

#[test]
fn acb3_to_3149_successor_supersedes_resumed_ticket_idempotently() {
    const RELEASE_79BE: &str = "79be8937cb13fd1dbd64c9b61a5444752b500f0f";
    const RELEASE_ACB3: &str = "acb3fb4f032943c0acc99e7aade38d550f2ecf59";
    const RELEASE_3149: &str = "3149557e92b93d70ef6ddaf3a03ef0828a0d061d";
    const UNKNOWN: &str = "22903fbe2db5d7ad0535683c956e8d37dc49b207";

    let root = std::env::temp_dir().join(format!(
        "elon-update-successor-race-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let store = UpdateRecoveryStore::new(root.join("ledger.json"));
    let mut old = UpdateRecoveryReceipt::planned(
        format!("node-update-{RELEASE_ACB3}"),
        "root-real-upgrade",
        "task-real-upgrade",
    );
    old.created_at_ms = 10;
    old.updated_at_ms = 20;
    old.from_release = ReleaseIdentity {
        version: format!("0.3.69+{RELEASE_79BE}"),
        git_sha: RELEASE_79BE.to_string(),
    };
    old.to_release = ReleaseIdentity {
        version: String::new(),
        git_sha: RELEASE_ACB3.to_string(),
    };
    old.state = UpdateRecoveryState::Resumed;
    old.resume_strategy = Some("sidecar_reattach".to_string());

    let mut successor = UpdateRecoveryReceipt::planned(
        format!("node-update-{RELEASE_3149}"),
        "root-real-upgrade",
        "task-real-upgrade",
    );
    successor.created_at_ms = 30;
    successor.updated_at_ms = 40;
    successor.from_release = ReleaseIdentity {
        version: format!("0.3.69+{RELEASE_ACB3}"),
        git_sha: RELEASE_ACB3.to_string(),
    };
    successor.to_release = ReleaseIdentity {
        version: String::new(),
        git_sha: RELEASE_3149.to_string(),
    };
    successor.state = UpdateRecoveryState::Applying;
    store.upsert(old.clone()).unwrap();
    store.upsert(successor.clone()).unwrap();

    let current = format!("0.3.69+{RELEASE_3149}");
    let evidence = superseding_release_evidence(&store, &old, &current)
        .unwrap()
        .expect("the exact chained successor is auditable");
    assert_eq!(evidence.update_id, successor.update_id);
    assert_eq!(evidence.source, "successor_update_receipt");
    assert!(
        superseding_release_evidence(&store, &old, &format!("0.3.69+{UNKNOWN}"))
            .unwrap()
            .is_none(),
        "an unknown runtime must remain fail-closed"
    );

    assert!(record_superseded_recovery(&store, &old, &evidence).unwrap());
    assert!(!record_superseded_recovery(&store, &old, &evidence).unwrap());
    let ledger = store.load().unwrap();
    let saved_old = ledger
        .receipts
        .iter()
        .find(|receipt| receipt.update_id == old.update_id)
        .unwrap();
    assert_eq!(saved_old.state, UpdateRecoveryState::Verified);
    assert_eq!(
        saved_old.superseded_by_update_id.as_deref(),
        Some(successor.update_id.as_str())
    );
    assert_eq!(
        saved_old.supersede_evidence.as_deref(),
        Some("successor_update_receipt")
    );
    assert!(saved_old
        .events
        .last()
        .and_then(|event| event.reason.as_deref())
        .is_some_and(|reason| reason.contains("no recovery action replayed")));
    assert_eq!(
        store
            .receipt_for_task("task-real-upgrade")
            .unwrap()
            .unwrap()
            .update_id,
        successor.update_id,
        "terminal reconciliation must ignore the superseded generation"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn task_bound_install_gate_accepts_only_its_exact_release() {
    const RELEASE_ACB3: &str = "acb3fb4f032943c0acc99e7aade38d550f2ecf59";
    const RELEASE_3149: &str = "3149557e92b93d70ef6ddaf3a03ef0828a0d061d";
    const UNKNOWN: &str = "22903fbe2db5d7ad0535683c956e8d37dc49b207";

    let root = std::env::temp_dir().join(format!(
        "elon-update-gate-race-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let store = UpdateRecoveryStore::new(root.join("ledger.json"));
    let mut old = UpdateRecoveryReceipt::planned(
        format!("node-update-{RELEASE_ACB3}"),
        "root-gate",
        "task-gate",
    );
    old.state = UpdateRecoveryState::Resumed;
    old.resume_strategy = Some("sidecar_reattach".to_string());
    store.upsert(old.clone()).unwrap();
    store
        .update_install_gate(crate::node_agent_update_recovery::UpdateInstallGate {
            phase: "checkpoint_saved".to_string(),
            target_git_sha: RELEASE_3149.to_string(),
            active_foreground_task_ids: vec!["task-gate".to_string()],
            safe_checkpoint_count: 1,
            ..Default::default()
        })
        .unwrap();

    let evidence = superseding_release_evidence(&store, &old, &format!("0.3.69+{RELEASE_3149}"))
        .unwrap()
        .expect("the exact task-bound install gate is auditable");
    assert_eq!(evidence.source, "task_bound_install_gate");
    assert!(
        superseding_release_evidence(&store, &old, &format!("0.3.69+{UNKNOWN}"))
            .unwrap()
            .is_none()
    );

    store
        .update_install_gate(crate::node_agent_update_recovery::UpdateInstallGate {
            phase: "checkpoint_saved".to_string(),
            target_git_sha: RELEASE_3149.to_string(),
            active_foreground_task_ids: vec!["different-task".to_string()],
            safe_checkpoint_count: 1,
            ..Default::default()
        })
        .unwrap();
    assert!(
        superseding_release_evidence(&store, &old, &format!("0.3.69+{RELEASE_3149}"))
            .unwrap()
            .is_none(),
        "an unbound global target cannot authorize this task"
    );
    let _ = std::fs::remove_dir_all(root);
}

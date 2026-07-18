use super::*;
use crate::node_agent_local_task_store::LocalTaskStart;

#[test]
fn install_gate_requires_a_checkpoint_for_every_foreground_task() {
    let safe = UpdateCheckpointDecision {
        active_foreground_task_ids: vec!["task-a".into(), "task-b".into()],
        checkpointed_task_ids: vec!["task-b".into(), "task-a".into()],
    };
    assert!(safe.install_may_proceed());
    let unsafe_decision = UpdateCheckpointDecision {
        checkpointed_task_ids: vec!["task-a".into()],
        ..safe
    };
    assert!(!unsafe_decision.install_may_proceed());
}

#[test]
fn incomplete_non_repeatable_action_blocks_until_its_result_is_durable() {
    let call = crate::node_agent_task_journal::TaskJournalEventView {
        seq: 1,
        event: serde_json::json!({"event": {"type": "tool_call", "call_id": "publish-1", "tool": "publish_server"}}),
    };
    assert_eq!(
        incomplete_non_repeatable_action(std::slice::from_ref(&call)).as_deref(),
        Some("publish_server:publish-1")
    );
    let result = crate::node_agent_task_journal::TaskJournalEventView {
        seq: 2,
        event: serde_json::json!({"event": {"type": "tool_result", "call_id": "publish-1"}}),
    };
    assert!(incomplete_non_repeatable_action(&[call, result]).is_none());
}

#[test]
fn legacy_active_task_without_supervision_checkpoint_defers_update() {
    let root = std::env::temp_dir().join(format!(
        "elon-update-gate-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let local_tasks =
        crate::node_agent_local_task_store::LocalTaskStore::new(root.join("tasks.db"));
    local_tasks
        .create(LocalTaskStart {
            task_id: "legacy-active",
            owner_user_id: "owner",
            agent_id: "agent",
            install_id: "install",
            project_id: "project",
            channel_id: None,
            conversation_id: "conversation",
            workspace_path: root.to_string_lossy().as_ref(),
            prompt: "legacy foreground",
            cli: "codex",
            runtime_permission: "full_access",
        })
        .unwrap();
    let decision = checkpoint_active_update_transactions(
        &UpdateRecoveryStore::new(root.join("recovery.json")),
        &local_tasks,
        &crate::node_agent_task_journal::TaskJournal::new(root.join("journal")),
        &crate::node_agent_cli_sidecar::CliSidecarRegistry::new(root.join("sidecars")),
        "old",
        "new",
    )
    .unwrap();
    assert_eq!(decision.active_foreground_task_ids, ["legacy-active"]);
    assert!(decision.checkpointed_task_ids.is_empty());
    assert!(!decision.install_may_proceed());
    let _ = std::fs::remove_dir_all(root);
}

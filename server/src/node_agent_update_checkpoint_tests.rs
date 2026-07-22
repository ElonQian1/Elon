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
fn checkpoint_preserves_platform_isolated_workspace_identity() {
    let mut fingerprint = WorkspaceGitFingerprint {
        workspace_path: "C:\\conversation-worktrees\\project\\conversation".to_string(),
        branch: Some("detected".to_string()),
        git_head: Some("0123456789abcdef0123456789abcdef01234567".to_string()),
        git_status_sha256: Some("status".to_string()),
        git_status_clean: Some(true),
        ..WorkspaceGitFingerprint::default()
    };
    preserve_platform_workspace_identity(
        &mut fingerprint,
        Some(&serde_json::json!({
            "isolated": true,
            "base_workspace_path": "D:\\project",
            "active_workspace_path": "C:\\conversation-worktrees\\project\\conversation",
            "branch": "ai/session/project/conversation",
            "git_head": "0123456789abcdef0123456789abcdef01234567"
        })),
    );
    assert!(fingerprint.isolated);
    assert_eq!(
        fingerprint.base_workspace_path.as_deref(),
        Some("D:\\project")
    );
    assert_eq!(
        fingerprint.branch.as_deref(),
        Some("ai/session/project/conversation")
    );
    assert!(fingerprint.has_sufficient_identity());
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
        &HashSet::new(),
    )
    .unwrap();
    assert_eq!(decision.active_foreground_task_ids, ["legacy-active"]);
    assert!(decision.checkpointed_task_ids.is_empty());
    assert!(!decision.install_may_proceed());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn low_priority_post_task_improvement_yields_without_blocking_update() {
    let root = std::env::temp_dir().join(format!(
        "elon-update-evolution-yield-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let local_tasks =
        crate::node_agent_local_task_store::LocalTaskStore::new(root.join("tasks.db"));
    local_tasks
        .create(LocalTaskStart {
            task_id: "evolution-active",
            owner_user_id: "owner",
            agent_id: "agent",
            install_id: "install",
            project_id: "project",
            channel_id: None,
            conversation_id: "self-evolution",
            workspace_path: root.to_string_lossy().as_ref(),
            prompt: "improve after user task",
            cli: "codex",
            runtime_permission: "full_access",
        })
        .unwrap();
    let journal = crate::node_agent_task_journal::TaskJournal::new(root.join("journal"));
    let contract = crate::node_agent_local_task_supervision::SupervisionContract {
        protocol: SUPERVISION_PROTOCOL.to_string(),
        supervisor: "codex_desktop".to_string(),
        task_role: "post_task_improvement".to_string(),
        parent_task_id: Some("user-task".to_string()),
        root_task_id: Some("root-task".to_string()),
        acceptance_criteria: Vec::new(),
        improvement_policy: "after_task_only".to_string(),
    };
    crate::node_agent_local_task_supervision::record_supervision_event(
        &journal,
        "evolution-active",
        "supervision_contract",
        crate::node_agent_local_task_supervision::contract_payload(&contract),
    )
    .unwrap();

    let recovery = UpdateRecoveryStore::new(root.join("recovery.json"));
    let sidecars = crate::node_agent_cli_sidecar::CliSidecarRegistry::new(root.join("sidecars"));
    let blocked = checkpoint_active_update_transactions(
        &recovery,
        &local_tasks,
        &journal,
        &sidecars,
        "old",
        "new",
        &HashSet::new(),
    )
    .expect_err("updater must fail closed before the self-evolution cancel audit is durable");
    assert!(blocked.to_string().contains("durable sidecar audit"));

    assert!(local_tasks
        .mark_cancel_requested("evolution-active")
        .unwrap());
    let confirmed_stale = HashSet::from(["evolution-active".to_string()]);
    let stale_decision = checkpoint_active_update_transactions(
        &recovery,
        &local_tasks,
        &journal,
        &sidecars,
        "old",
        "new",
        &confirmed_stale,
    )
    .expect("exact-target stale cancel_requested evolution should not block the installer");
    assert!(stale_decision.install_may_proceed());

    sidecars
        .upsert_session(
            crate::node_agent_cli_sidecar::CliSidecarSessionRecord::managed_conpty(
                "evolution-sidecar",
                "evolution-active",
                "codex",
                "route_a_external_cli",
                Some(root.to_string_lossy().into_owned()),
                Some("npipe://elon/evolution-sidecar".to_string()),
                Some(100),
                Some(200),
                crate::node_agent_cli_sidecar::now_ms(),
            ),
        )
        .unwrap();
    let decision = checkpoint_active_update_transactions(
        &recovery,
        &local_tasks,
        &journal,
        &sidecars,
        "old",
        "new",
        &HashSet::new(),
    )
    .unwrap();
    assert!(decision.active_foreground_task_ids.is_empty());
    assert!(decision.checkpointed_task_ids.is_empty());
    assert!(decision.install_may_proceed());
    let _ = std::fs::remove_dir_all(root);
}

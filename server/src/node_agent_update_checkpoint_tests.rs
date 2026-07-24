use super::*;

#[cfg(windows)]
#[test]
fn missing_windows_paths_compare_across_separator_and_long_path_prefix_forms() {
    assert!(same_path(
        Path::new(r"C:\missing\conversation-worktrees\elon-self\task"),
        Path::new("C:/missing/conversation-worktrees/elon-self/task"),
    ));
    assert!(same_path(
        Path::new(r"\\?\C:\missing\conversation-worktrees\elon-self\task"),
        Path::new("C:/missing/conversation-worktrees/elon-self/task"),
    ));
}
use crate::node_agent_local_task_store::LocalTaskStart;

#[test]
fn install_gate_requires_a_checkpoint_but_not_an_idle_runtime() {
    let safe = UpdateCheckpointDecision {
        active_foreground_task_ids: vec!["task-a".into(), "task-b".into()],
        checkpointed_task_ids: vec!["task-b".into(), "task-a".into()],
        live_execution_task_ids: Vec::new(),
    };
    assert!(safe.install_may_proceed());
    let unsafe_decision = UpdateCheckpointDecision {
        checkpointed_task_ids: vec!["task-a".into()],
        ..safe.clone()
    };
    assert!(!unsafe_decision.install_may_proceed());
    let live = UpdateCheckpointDecision {
        live_execution_task_ids: vec!["task-a".into()],
        ..safe
    };
    assert!(
        live.install_may_proceed(),
        "a live task with a complete durable checkpoint must reconnect after update"
    );
}

#[test]
fn reconciled_duplicate_receipts_follow_the_opt_in_install_gate() {
    let classification = crate::node_agent_update_recovery::UpdateGateTaskClassification {
        ambiguous_recovery_receipts: true,
        excluded_from_install_blockers: true,
        non_repeatable_action: Some("journal_exceeds_audit_limit".to_string()),
        ..Default::default()
    };
    assert!(reconciled_classification_allows_install(
        &classification,
        false,
        true,
        &[],
        None,
    ));
    assert!(!reconciled_classification_allows_install(
        &classification,
        true,
        true,
        &[],
        None,
    ));
    assert!(!reconciled_classification_allows_install(
        &classification,
        false,
        true,
        &["approval-1".to_string()],
        None,
    ));
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

    let fit_run = crate::node_agent_task_journal::TaskJournalEventView {
        seq: 3,
        event: serde_json::json!({"event": {
            "type": "tool_call",
            "call_id": "fit-1",
            "tool": "ui_start_fit_run"
        }}),
    };
    assert_eq!(
        incomplete_non_repeatable_action(std::slice::from_ref(&fit_run)).as_deref(),
        Some("ui_start_fit_run:fit-1"),
        "an in-flight real-renderer operation must keep update fail-closed"
    );
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
        &HashSet::new(),
    )
    .unwrap();
    assert_eq!(decision.active_foreground_task_ids, ["legacy-active"]);
    assert!(decision.checkpointed_task_ids.is_empty());
    assert!(!decision.install_may_proceed());

    let stale_without_persisted_cancel = checkpoint_active_update_transactions(
        &UpdateRecoveryStore::new(root.join("recovery-unpersisted-cancel.json")),
        &local_tasks,
        &crate::node_agent_task_journal::TaskJournal::new(root.join("journal")),
        &crate::node_agent_cli_sidecar::CliSidecarRegistry::new(root.join("sidecars")),
        "old",
        "new",
        &HashSet::from(["legacy-active".to_string()]),
        &HashSet::new(),
    )
    .expect("stale proof must not replace durable cancel_requested state");
    assert!(!stale_without_persisted_cancel.install_may_proceed());

    assert!(local_tasks.mark_cancel_requested("legacy-active").unwrap());
    let live_runtime = checkpoint_active_update_transactions(
        &UpdateRecoveryStore::new(root.join("recovery-runtime-handle.json")),
        &local_tasks,
        &crate::node_agent_task_journal::TaskJournal::new(root.join("journal")),
        &crate::node_agent_cli_sidecar::CliSidecarRegistry::new(root.join("sidecars")),
        "old",
        "new",
        &HashSet::from(["legacy-active".to_string()]),
        &HashSet::from(["legacy-active".to_string()]),
    )
    .expect("a fresh runtime handle must remain a blocking decision");
    assert_eq!(live_runtime.live_execution_task_ids, ["legacy-active"]);
    assert!(!live_runtime.install_may_proceed());

    let stale_decision = checkpoint_active_update_transactions(
        &UpdateRecoveryStore::new(root.join("recovery-stale.json")),
        &local_tasks,
        &crate::node_agent_task_journal::TaskJournal::new(root.join("journal")),
        &crate::node_agent_cli_sidecar::CliSidecarRegistry::new(root.join("sidecars")),
        "old",
        "new",
        &HashSet::from(["legacy-active".to_string()]),
        &HashSet::new(),
    )
    .expect("exact-target stale cancel_requested task should not block the installer");
    assert!(stale_decision.active_foreground_task_ids.is_empty());
    assert!(stale_decision.install_may_proceed());
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
        &HashSet::new(),
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
                Some(std::process::id()),
                None,
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
        &confirmed_stale,
        &HashSet::new(),
    )
    .unwrap();
    assert_eq!(decision.active_foreground_task_ids, ["evolution-active"]);
    assert!(decision.checkpointed_task_ids.is_empty());
    assert_eq!(decision.live_execution_task_ids, ["evolution-active"]);
    assert!(!decision.install_may_proceed());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn durable_cancel_without_live_runtime_or_sidecar_is_safe_for_update_only() {
    let root = std::env::temp_dir().join(format!(
        "elon-update-stale-cancel-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let local_tasks =
        crate::node_agent_local_task_store::LocalTaskStore::new(root.join("tasks.db"));
    local_tasks
        .create(LocalTaskStart {
            task_id: "stale-cancel",
            owner_user_id: "owner",
            agent_id: "agent",
            install_id: "install",
            project_id: "project",
            channel_id: None,
            conversation_id: "conversation",
            workspace_path: root.to_string_lossy().as_ref(),
            prompt: "cancel me",
            cli: "codex",
            runtime_permission: "full_access",
        })
        .unwrap();
    let journal = crate::node_agent_task_journal::TaskJournal::new(root.join("journal"));
    journal
        .record_started(crate::node_agent_task_journal::TaskJournalStart {
            req_id: "stale-cancel",
            cli_name: "codex",
            route: Some("route_a_external_cli"),
            run_handle_id: Some("stale-cancel"),
            cwd: root.to_str(),
            runtime_permission: Some("full_access"),
        })
        .unwrap();
    let contract = crate::node_agent_local_task_supervision::SupervisionContract {
        protocol: SUPERVISION_PROTOCOL.to_string(),
        supervisor: "codex_desktop".to_string(),
        task_role: "resume_original".to_string(),
        parent_task_id: Some("parent".to_string()),
        root_task_id: Some("root".to_string()),
        acceptance_criteria: Vec::new(),
        improvement_policy: "after_task_only".to_string(),
    };
    crate::node_agent_local_task_supervision::record_supervision_event(
        &journal,
        "stale-cancel",
        "supervision_contract",
        crate::node_agent_local_task_supervision::contract_payload(&contract),
    )
    .unwrap();
    assert!(local_tasks.mark_cancel_requested("stale-cancel").unwrap());
    let sidecars = crate::node_agent_cli_sidecar::CliSidecarRegistry::new(root.join("sidecars"));

    assert!(
        proven_stale_cancelled_tasks(&local_tasks, &journal, &sidecars, Some(&HashSet::new()))
            .unwrap()
            .is_empty()
    );
    journal.record_cancel_requested("stale-cancel").unwrap();

    local_tasks
        .create(LocalTaskStart {
            task_id: "stale-resume",
            owner_user_id: "owner",
            agent_id: "agent",
            install_id: "install",
            project_id: "project",
            channel_id: None,
            conversation_id: "conversation",
            workspace_path: root.to_string_lossy().as_ref(),
            prompt: "resume after cancel",
            cli: "codex",
            runtime_permission: "full_access",
        })
        .unwrap();
    journal
        .record_started(crate::node_agent_task_journal::TaskJournalStart {
            req_id: "stale-resume",
            cli_name: "codex",
            route: Some("route_a_external_cli"),
            run_handle_id: Some("stale-resume"),
            cwd: root.to_str(),
            runtime_permission: Some("full_access"),
        })
        .unwrap();
    crate::node_agent_local_task_supervision::record_supervision_event(
        &journal,
        "stale-resume",
        "supervision_contract",
        crate::node_agent_local_task_supervision::contract_payload(&contract),
    )
    .unwrap();
    journal.record_cancel_requested("stale-resume").unwrap();
    assert!(local_tasks
        .mark_recovery_blocked(
            "stale-resume",
            "historical canceled executor requires resume"
        )
        .unwrap());

    assert!(
        proven_stale_cancelled_tasks(&local_tasks, &journal, &sidecars, None)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        proven_stale_cancelled_tasks(&local_tasks, &journal, &sidecars, Some(&HashSet::new()))
            .unwrap(),
        HashSet::from(["stale-cancel".to_string(), "stale-resume".to_string()])
    );
    let confirmed = HashSet::from(["stale-cancel".to_string(), "stale-resume".to_string()]);
    let safe = checkpoint_active_update_transactions(
        &UpdateRecoveryStore::new(root.join("recovery-two-stale.json")),
        &local_tasks,
        &journal,
        &sidecars,
        "old",
        "new",
        &confirmed,
        &HashSet::new(),
    )
    .unwrap();
    assert!(safe.active_foreground_task_ids.is_empty());
    assert!(safe.install_may_proceed());

    let wrong_window = checkpoint_active_update_transactions(
        &UpdateRecoveryStore::new(root.join("recovery-wrong-window.json")),
        &local_tasks,
        &journal,
        &sidecars,
        "old",
        "new",
        &HashSet::new(),
        &HashSet::new(),
    )
    .unwrap();
    assert_eq!(
        wrong_window.active_foreground_task_ids,
        ["stale-cancel", "stale-resume"]
    );
    assert!(!wrong_window.install_may_proceed());
    assert_eq!(
        proven_stale_cancelled_tasks(
            &local_tasks,
            &journal,
            &sidecars,
            Some(&HashSet::from(["stale-cancel".to_string()]))
        )
        .unwrap(),
        HashSet::from(["stale-resume".to_string()]),
        "a fresh runtime handle must block only its exact task"
    );
    sidecars
        .upsert_session(
            crate::node_agent_cli_sidecar::CliSidecarSessionRecord::managed_conpty(
                "stale-resume-sidecar",
                "stale-resume",
                "codex",
                "route_a_external_cli",
                Some(root.to_string_lossy().into_owned()),
                Some("npipe://elon/stale-resume-sidecar".to_string()),
                Some(std::process::id()),
                None,
                crate::node_agent_cli_sidecar::now_ms(),
            ),
        )
        .unwrap();
    assert_eq!(
        proven_stale_cancelled_tasks(
            &local_tasks,
            &journal,
            &sidecars,
            Some(&HashSet::from(["stale-cancel".to_string()]))
        )
        .unwrap(),
        HashSet::new(),
        "a live sidecar and a fresh runtime handle must both fail closed"
    );
    assert_eq!(
        local_tasks.get("stale-cancel").unwrap().unwrap().status,
        "cancel_requested",
        "update proof must not rewrite cancellation semantics"
    );
    assert_eq!(
        local_tasks.get("stale-resume").unwrap().unwrap().status,
        "resume_required",
        "update proof must preserve the historical resume state"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn reconciled_terminal_history_is_excluded_but_fresh_ownership_still_blocks() {
    let root = std::env::temp_dir().join(format!(
        "elon-update-terminal-reconcile-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let local_tasks =
        crate::node_agent_local_task_store::LocalTaskStore::new(root.join("tasks.db"));
    local_tasks
        .create(LocalTaskStart {
            task_id: "historical-resume",
            owner_user_id: "owner",
            agent_id: "agent",
            install_id: "install",
            project_id: "project",
            channel_id: None,
            conversation_id: "conversation",
            workspace_path: root.to_string_lossy().as_ref(),
            prompt: "historical task",
            cli: "codex",
            runtime_permission: "full_access",
        })
        .unwrap();
    local_tasks
        .mark_recovery_blocked("historical-resume", "cannot be resumed")
        .unwrap();
    let task = local_tasks.get("historical-resume").unwrap().unwrap();
    let store = UpdateRecoveryStore::new(root.join("recovery.json"));
    store
        .update_install_gate(UpdateInstallGate {
            target_git_sha: "new".to_string(),
            classifications: vec![
                crate::node_agent_update_recovery::UpdateGateTaskClassification {
                    task_id: task.task_id.clone(),
                    status: task.status.clone(),
                    finished_at_ms: task.finished_at_ms,
                    resume_eligible: Some(false),
                    resume_ineligibility_proof: Some(
                        "terminal workspace snapshot was rejected".to_string(),
                    ),
                    excluded_from_install_blockers: true,
                    reason: "audited terminal history".to_string(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        })
        .unwrap();
    let journal = crate::node_agent_task_journal::TaskJournal::new(root.join("journal"));
    let sidecars = crate::node_agent_cli_sidecar::CliSidecarRegistry::new(root.join("sidecars"));
    let safe = checkpoint_active_update_transactions(
        &store,
        &local_tasks,
        &journal,
        &sidecars,
        "old",
        "new",
        &HashSet::new(),
        &HashSet::new(),
    )
    .unwrap();
    assert!(safe.install_may_proceed());
    assert!(safe.active_foreground_task_ids.is_empty());

    let fresh = checkpoint_active_update_transactions(
        &store,
        &local_tasks,
        &journal,
        &sidecars,
        "old",
        "new",
        &HashSet::new(),
        &HashSet::from(["historical-resume".to_string()]),
    )
    .unwrap();
    assert!(!fresh.install_may_proceed());
    assert_eq!(fresh.live_execution_task_ids, ["historical-resume"]);
    assert_eq!(fresh.active_foreground_task_ids, ["historical-resume"]);

    let mut resumable_gate = store.load().unwrap().install_gate;
    resumable_gate.classifications[0].resume_ineligibility_proof = None;
    store.update_install_gate(resumable_gate).unwrap();
    let resumable = checkpoint_active_update_transactions(
        &store,
        &local_tasks,
        &journal,
        &sidecars,
        "old",
        "new",
        &HashSet::new(),
        &HashSet::new(),
    )
    .unwrap();
    assert!(resumable.install_may_proceed());
    assert!(resumable.live_execution_task_ids.is_empty());
    assert!(resumable.active_foreground_task_ids.is_empty());
    let _ = std::fs::remove_dir_all(root);
}

use std::{fs, path::Path};

use tokio::sync::watch;

use super::*;
use crate::{
    node_agent_active_task::ActiveCliPromptHandle, node_agent_cli_sidecar::CliSidecarSessionRecord,
    node_agent_local_task_store::LocalTaskStart, node_agent_task_journal::TaskJournalStart,
};

#[tokio::test]
async fn crash_after_intent_replays_to_surviving_sidecar_once_after_restart() {
    let root = unique_root("restart-replay");
    let journal = TaskJournal::new(root.join("journal"));
    let sidecars = CliSidecarRegistry::new(root.join("sidecars"));
    let local_tasks = LocalTaskStore::new(root.join("local.sqlite3"));
    start_journal(&journal, "task-replay");
    start_local_task(&local_tasks, "task-replay");
    sidecars
        .upsert_session(sidecar("session-replay", "task-replay"))
        .unwrap();

    let active = ActiveCliPromptRegistry::new();
    let (cancel_tx, cancel_rx) = watch::channel(false);
    assert!(
        active
            .try_insert(ActiveCliPromptHandle::new(
                "task-replay",
                "codex",
                "route_a_external_cli",
                Some("D:/demo".to_string()),
                Some("full_access".to_string()),
                cancel_tx,
            ))
            .await
    );

    let crashed = request_cancel_crash_after_intent(
        &active,
        &sidecars,
        &journal,
        &local_tasks,
        "task-replay",
        &audit(),
    )
    .await
    .unwrap();
    assert!(matches!(&crashed, CancelDispatchOutcome::Pending { .. }));
    assert!(!crashed.accepted());
    assert!(!*cancel_rx.borrow());
    assert!(!sidecars.command_mailbox_path("task-replay").exists());
    assert_eq!(
        local_tasks.get("task-replay").unwrap().unwrap().status,
        "cancel_requested"
    );

    let intent = journal
        .snapshot("task-replay", 0, 20)
        .unwrap()
        .record
        .unwrap()
        .cancel_intent
        .unwrap();
    assert!(intent.action_id.starts_with("cancel-"));
    assert_eq!(intent.task_id, "task-replay");
    assert_eq!(intent.sidecar_session_id.as_deref(), Some("session-replay"));
    assert_eq!(intent.audit, audit());
    assert!(intent.side_effect.is_none());

    let restarted_active = ActiveCliPromptRegistry::new();
    let reloaded_sidecars = CliSidecarRegistry::new(root.join("sidecars"));
    let reloaded_local_tasks = LocalTaskStore::new(root.join("local.sqlite3"));
    let replayed = reconcile_intent(
        &restarted_active,
        &reloaded_sidecars,
        &journal,
        &reloaded_local_tasks,
        intent.clone(),
    )
    .await
    .unwrap();
    assert!(matches!(
        replayed,
        CancelDispatchOutcome::Dispatched {
            ref target_kind,
            ..
        } if target_kind == "sidecar_mailbox"
    ));
    let mailbox = fs::read_to_string(reloaded_sidecars.command_mailbox_path("task-replay"))
        .expect("reconcile should persist the sidecar cancel command");
    assert_eq!(mailbox.lines().count(), 1);
    assert!(mailbox.contains(&format!(r#""command_id":"{}""#, intent.action_id)));
    assert!(mailbox.contains(r#""target_session_id":"session-replay""#));

    let duplicate = reconcile_intent(
        &restarted_active,
        &reloaded_sidecars,
        &journal,
        &reloaded_local_tasks,
        intent,
    )
    .await
    .unwrap();
    assert!(matches!(
        duplicate,
        CancelDispatchOutcome::AlreadyCommitted { .. }
    ));
    let mailbox = fs::read_to_string(reloaded_sidecars.command_mailbox_path("task-replay"))
        .expect("mailbox should remain available");
    assert_eq!(mailbox.lines().count(), 1);
    let committed = journal
        .snapshot("task-replay", 0, 20)
        .unwrap()
        .record
        .unwrap()
        .cancel_intent
        .unwrap()
        .side_effect
        .unwrap();
    assert_eq!(committed.target_kind, "sidecar_mailbox");
    assert_eq!(committed.target_id, "session-replay");
    cleanup(&root);
}

#[tokio::test]
async fn active_watch_dispatch_commits_before_repeat_reports_success() {
    let root = unique_root("active-watch");
    let journal = TaskJournal::new(root.join("journal"));
    let sidecars = CliSidecarRegistry::new(root.join("sidecars"));
    let local_tasks = LocalTaskStore::new(root.join("local.sqlite3"));
    start_journal(&journal, "task-active");
    start_local_task(&local_tasks, "task-active");
    let active = ActiveCliPromptRegistry::new();
    let (cancel_tx, cancel_rx) = watch::channel(false);
    assert!(
        active
            .try_insert(ActiveCliPromptHandle::new(
                "task-active",
                "codex",
                "route_a_external_cli",
                None,
                Some("full_access".to_string()),
                cancel_tx,
            ))
            .await
    );

    let first = request_cancel(
        &active,
        &sidecars,
        &journal,
        &local_tasks,
        "task-active",
        &audit(),
    )
    .await
    .unwrap();
    assert!(matches!(
        &first,
        CancelDispatchOutcome::Dispatched {
            target_kind,
            ..
        } if target_kind == "active_watch"
    ));
    assert!(first.accepted());
    assert!(*cancel_rx.borrow());
    let repeated = request_cancel(
        &active,
        &sidecars,
        &journal,
        &local_tasks,
        "task-active",
        &audit(),
    )
    .await
    .unwrap();
    assert!(matches!(
        repeated,
        CancelDispatchOutcome::AlreadyCommitted { .. }
    ));
    assert_eq!(
        local_tasks.get("task-active").unwrap().unwrap().status,
        "cancel_requested"
    );
    cleanup(&root);
}

#[tokio::test]
async fn terminal_task_makes_pending_cancel_replay_a_noop_without_duplicate_terminal() {
    let root = unique_root("terminal-noop");
    let journal = TaskJournal::new(root.join("journal"));
    let sidecars = CliSidecarRegistry::new(root.join("sidecars"));
    let local_tasks = LocalTaskStore::new(root.join("local.sqlite3"));
    start_journal(&journal, "task-terminal");
    sidecars
        .upsert_session(sidecar("session-terminal", "task-terminal"))
        .unwrap();
    let active = ActiveCliPromptRegistry::new();
    let pending = request_cancel_crash_after_intent(
        &active,
        &sidecars,
        &journal,
        &local_tasks,
        "task-terminal",
        &audit(),
    )
    .await
    .unwrap();
    let action_id = match pending {
        CancelDispatchOutcome::Pending { action_id } => action_id,
        other => panic!("expected pending intent, got {other:?}"),
    };
    journal
        .record_finished_with_outcome("task-terminal", "done", None)
        .unwrap();

    for _ in 0..2 {
        assert_eq!(
            reconcile_intent(
                &active,
                &sidecars,
                &journal,
                &local_tasks,
                journal
                    .cancel_intents()
                    .unwrap()
                    .into_iter()
                    .find(|intent| intent.action_id == action_id)
                    .unwrap(),
            )
            .await
            .unwrap(),
            CancelDispatchOutcome::Terminal {
                status: "done".to_string()
            }
        );
    }
    assert!(!sidecars.command_mailbox_path("task-terminal").exists());
    let events = fs::read_to_string(root.join("journal/events.jsonl")).unwrap();
    assert_eq!(events.matches(r#""type":"finished""#).count(), 1);
    cleanup(&root);
}

#[tokio::test]
async fn replay_refuses_a_new_sidecar_with_the_same_task_id() {
    let root = unique_root("identity-mismatch");
    let journal = TaskJournal::new(root.join("journal"));
    let sidecars = CliSidecarRegistry::new(root.join("sidecars"));
    let local_tasks = LocalTaskStore::new(root.join("local.sqlite3"));
    start_journal(&journal, "task-identity");
    sidecars
        .upsert_session(sidecar("session-old", "task-identity"))
        .unwrap();
    let active = ActiveCliPromptRegistry::new();
    let pending = request_cancel_crash_after_intent(
        &active,
        &sidecars,
        &journal,
        &local_tasks,
        "task-identity",
        &audit(),
    )
    .await
    .unwrap();
    assert!(matches!(pending, CancelDispatchOutcome::Pending { .. }));
    sidecars
        .touch_session("session-old", Some("finished"), None)
        .unwrap();
    sidecars
        .upsert_session(sidecar("session-new", "task-identity"))
        .unwrap();
    let intent = journal.cancel_intents().unwrap().remove(0);

    let outcome = reconcile_intent(&active, &sidecars, &journal, &local_tasks, intent)
        .await
        .unwrap();
    assert!(matches!(outcome, CancelDispatchOutcome::Pending { .. }));
    assert!(!sidecars.command_mailbox_path("task-identity").exists());
    cleanup(&root);
}

#[tokio::test]
async fn intent_persistence_failure_never_sends_the_active_watch_signal() {
    let root = unique_root("persistence-failure");
    let journal_dir = root.join("journal");
    let journal = TaskJournal::new(&journal_dir);
    let sidecars = CliSidecarRegistry::new(root.join("sidecars"));
    let local_tasks = LocalTaskStore::new(root.join("local.sqlite3"));
    start_journal(&journal, "task-fail-closed");
    fs::create_dir(journal_dir.join(format!("registry.json.tmp-{}", std::process::id()))).unwrap();

    let active = ActiveCliPromptRegistry::new();
    let (cancel_tx, cancel_rx) = watch::channel(false);
    assert!(
        active
            .try_insert(ActiveCliPromptHandle::new(
                "task-fail-closed",
                "codex",
                "route_a_external_cli",
                None,
                Some("full_access".to_string()),
                cancel_tx,
            ))
            .await
    );
    let result = request_cancel(
        &active,
        &sidecars,
        &journal,
        &local_tasks,
        "task-fail-closed",
        &audit(),
    )
    .await;
    assert!(result.is_err());
    assert!(!*cancel_rx.borrow());
    assert_eq!(
        journal
            .snapshot("task-fail-closed", 0, 10)
            .unwrap()
            .record
            .unwrap()
            .status,
        "running"
    );
    cleanup(&root);
}

fn start_journal(journal: &TaskJournal, task_id: &str) {
    journal
        .record_started(TaskJournalStart {
            req_id: task_id,
            cli_name: "codex",
            route: Some("route_a_external_cli"),
            run_handle_id: Some(task_id),
            cwd: Some("D:/demo"),
            runtime_permission: Some("full_access"),
        })
        .unwrap();
}

fn start_local_task(store: &LocalTaskStore, task_id: &str) {
    store
        .create(LocalTaskStart {
            task_id,
            owner_user_id: "owner-a",
            agent_id: "node-a",
            install_id: "install-a",
            project_id: "project-a",
            channel_id: Some("dev"),
            conversation_id: "conversation-a",
            workspace_path: "D:/demo",
            prompt: "finish the task",
            cli: "codex",
            runtime_permission: "full_access",
        })
        .unwrap();
}

fn sidecar(session_id: &str, task_id: &str) -> CliSidecarSessionRecord {
    CliSidecarSessionRecord::managed_pipe_json(
        session_id,
        task_id,
        "codex",
        "route_a_external_cli",
        Some("D:/demo".to_string()),
        Some("D:/state/output.jsonl".to_string()),
        Some(std::process::id()),
        None,
        now_ms(),
    )
}

fn audit() -> CancelRequestAudit {
    CancelRequestAudit {
        requested_by: Some("owner-a".to_string()),
        source: Some("pc_ui".to_string()),
        reason: Some("user_requested".to_string()),
        requested_at_ms: Some(1234),
        interruption_source: None,
    }
}

fn unique_root(suffix: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "elon-cancel-saga-{suffix}-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ))
}

fn cleanup(root: &Path) {
    let _ = fs::remove_dir_all(root);
}

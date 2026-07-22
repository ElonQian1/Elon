use super::{completion_replay_backoff_ms, completion_replay_is_due, shutdown_session_task};
use crate::{
    node_agent_completion_outbox::{CliCompletionOutbox, PendingCliCompletion},
    node_agent_local_task_store::{LocalTaskStart, LocalTaskStore},
};
use homecli_proto::{CliCompletionEnvelope, CliProjectContext};
use std::{collections::HashSet, fs, path::PathBuf, time::Duration};

const OWNER: &str = "owner-a";

fn test_root() -> PathBuf {
    std::env::temp_dir().join(format!(
        "elon-completion-ack-test-{}",
        uuid::Uuid::new_v4().simple()
    ))
}

fn test_runtime(root: &PathBuf, local_tasks_path: PathBuf) -> crate::NodeRuntime {
    let mut runtime = crate::NodeRuntime::new(
        crate::node_agent_config::NodeConfig {
            cloud_url: "ws://127.0.0.1".into(),
            cloud_http_url: "http://127.0.0.1".into(),
            ollama_url: "http://127.0.0.1".into(),
            lm_studio_url: None,
            custom_url: None,
            price_per_1k: 0.0,
        },
        None,
        crate::pc_storage_repo::StorageSettings::default(),
        crate::node_agent_data_root::resolve(None, None, None),
        "install-a".into(),
    );
    runtime.completion_outbox = CliCompletionOutbox::new(root.join("outbox.sqlite3"));
    runtime.local_tasks = LocalTaskStore::new(local_tasks_path);
    runtime.task_journal = crate::node_agent_task_journal::TaskJournal::new(root.join("journal"));
    runtime.update_recovery =
        crate::node_agent_update_recovery::UpdateRecoveryStore::new(root.join("recovery.json"));
    runtime
}

fn completion(event_id: &str, req_id: &str) -> CliCompletionEnvelope {
    CliCompletionEnvelope {
        event_id: event_id.to_string(),
        req_id: req_id.to_string(),
        cli: "codex".to_string(),
        origin: "local_offline".to_string(),
        producer_identity: Some(homecli_proto::CliCompletionProducerIdentity {
            owner_user_id: OWNER.to_string(),
            agent_id: "node-a".to_string(),
            install_id: "install-a".to_string(),
        }),
        project_context: Some(CliProjectContext {
            project_id: "project-a".to_string(),
            conversation_id: "conversation-a".to_string(),
            runtime_permission: Some("full_access".to_string()),
        }),
        channel_id: Some("channel-a".to_string()),
        prompt: Some("离线任务".to_string()),
        final_output: "任务完成".to_string(),
        exit_ok: true,
        error: None,
        session_id: Some("session-a".to_string()),
        prompt_tokens: Some(10),
        cached_input_tokens: Some(2),
        completion_tokens: Some(3),
        reasoning_tokens: Some(1),
        total_tokens: Some(13),
        model: Some("gpt-5.4".to_string()),
        workspace_status: None,
        created_at_ms: 1_783_920_000_000,
    }
}

fn create_local_task(store: &LocalTaskStore, completion: &CliCompletionEnvelope) {
    store
        .create(LocalTaskStart {
            task_id: &completion.req_id,
            owner_user_id: OWNER,
            agent_id: "node-a",
            install_id: "install-a",
            project_id: "project-a",
            channel_id: Some("channel-a"),
            conversation_id: "conversation-a",
            workspace_path: "C:\\repo",
            prompt: "离线任务",
            cli: "codex",
            runtime_permission: "full_access",
        })
        .expect("create local task");
}

fn seed_local_task(store: &LocalTaskStore, completion: &CliCompletionEnvelope) {
    create_local_task(store, completion);
    assert!(store.finish(OWNER, completion).expect("finish local task"));
}

#[tokio::test]
async fn accepted_ack_updates_local_display_before_deleting_outbox_row() {
    let root = test_root();
    let runtime = test_runtime(&root, root.join("local-tasks.sqlite3"));
    let outbox = &runtime.completion_outbox;
    let local_tasks = &runtime.local_tasks;
    let completion = completion("event-accepted", "req-accepted");
    outbox.enqueue(&completion).expect("enqueue completion");
    seed_local_task(&local_tasks, &completion);

    crate::node_agent_session_completion_ack::apply(
        &runtime,
        completion.producer_identity.as_ref().unwrap(),
        &completion.event_id,
        &completion.req_id,
        true,
        false,
        None,
    )
    .await
    .expect("apply accepted ACK");

    assert!(outbox
        .latest_for_req_id(&completion.req_id)
        .unwrap()
        .is_none());
    let record = local_tasks
        .get_for_owner(OWNER, &completion.req_id)
        .unwrap()
        .expect("local task record");
    assert_eq!(record.sync_state, "synced");
    assert!(record.server_ack_at_ms.is_some());
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn ack_self_heals_local_display_and_real_storage_failure_keeps_outbox() {
    let root = test_root();
    let runtime = test_runtime(&root, root.join("local-tasks.sqlite3"));
    let outbox = &runtime.completion_outbox;
    let local_tasks = &runtime.local_tasks;
    let accepted = completion("event-accepted", "req-accepted");
    let rejected = completion("event-rejected", "req-rejected");
    outbox.enqueue(&accepted).unwrap();
    outbox.enqueue(&rejected).unwrap();
    create_local_task(&local_tasks, &accepted);

    assert!(crate::node_agent_session_completion_ack::apply(
        &runtime,
        accepted.producer_identity.as_ref().unwrap(),
        "wrong-event",
        &accepted.req_id,
        true,
        false,
        None,
    )
    .await
    .is_err());
    crate::node_agent_session_completion_ack::apply(
        &runtime,
        accepted.producer_identity.as_ref().unwrap(),
        &accepted.event_id,
        &accepted.req_id,
        true,
        false,
        None,
    )
    .await
    .expect("ACK should repair a crash before local display persistence");
    let repaired = local_tasks
        .get_for_owner(OWNER, &accepted.req_id)
        .unwrap()
        .unwrap();
    assert_eq!(repaired.status, "done");
    assert_eq!(repaired.sync_state, "synced");
    assert_eq!(outbox.pending_count().unwrap(), 1);

    // Opening a SQLite database at an existing directory fails deterministically.
    let broken_runtime = test_runtime(&root, root.clone());
    assert!(crate::node_agent_session_completion_ack::apply(
        &broken_runtime,
        rejected.producer_identity.as_ref().unwrap(),
        &rejected.event_id,
        &rejected.req_id,
        false,
        false,
        Some("permanent rejection"),
    )
    .await
    .is_err());

    assert_eq!(outbox.pending_count().unwrap(), 1);
    assert!(outbox
        .completion_for_binding(&accepted.event_id, &accepted.req_id)
        .unwrap()
        .is_none());
    assert!(outbox
        .completion_for_binding(&rejected.event_id, &rejected.req_id)
        .unwrap()
        .is_some());
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn late_local_finish_cannot_downgrade_synced_or_rejected_state() {
    let root = test_root();
    let runtime = test_runtime(&root, root.join("local-tasks.sqlite3"));
    let outbox = &runtime.completion_outbox;
    let local_tasks = &runtime.local_tasks;

    let accepted = completion("event-accepted", "req-accepted");
    outbox.enqueue(&accepted).unwrap();
    seed_local_task(&local_tasks, &accepted);
    crate::node_agent_session_completion_ack::apply(
        &runtime,
        accepted.producer_identity.as_ref().unwrap(),
        &accepted.event_id,
        &accepted.req_id,
        true,
        false,
        None,
    )
    .await
    .unwrap();
    assert!(local_tasks.finish(OWNER, &accepted).unwrap());
    let mut stale_fallback = accepted.clone();
    stale_fallback.event_id = "local-display-stale".to_string();
    let error = local_tasks
        .finish(OWNER, &stale_fallback)
        .expect_err("a late different event must fail closed");
    assert!(error.to_string().contains("different completion event"));
    let accepted_record = local_tasks
        .get_for_owner(OWNER, &accepted.req_id)
        .unwrap()
        .unwrap();
    assert_eq!(accepted_record.sync_state, "synced");
    assert_eq!(
        accepted_record.completion_event_id.as_deref(),
        Some(accepted.event_id.as_str())
    );

    let rejected = completion("event-rejected", "req-rejected");
    outbox.enqueue(&rejected).unwrap();
    seed_local_task(&local_tasks, &rejected);
    crate::node_agent_session_completion_ack::apply(
        &runtime,
        rejected.producer_identity.as_ref().unwrap(),
        &rejected.event_id,
        &rejected.req_id,
        false,
        false,
        Some("permanent rejection"),
    )
    .await
    .unwrap();
    assert!(local_tasks.finish(OWNER, &rejected).unwrap());
    let rejected_record = local_tasks
        .get_for_owner(OWNER, &rejected.req_id)
        .unwrap()
        .unwrap();
    assert_eq!(rejected_record.sync_state, "rejected");
    assert_eq!(
        rejected_record.completion_event_id.as_deref(),
        Some(rejected.event_id.as_str())
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn startup_interrupts_only_running_tasks_without_durable_completion() {
    let root = test_root();
    let local_tasks = LocalTaskStore::new(root.join("local-tasks.sqlite3"));
    let protected = completion("event-protected", "req-protected");
    let orphaned = completion("event-orphaned", "req-orphaned");
    create_local_task(&local_tasks, &protected);
    create_local_task(&local_tasks, &orphaned);

    let durable_req_ids = HashSet::from([protected.req_id.clone()]);
    assert_eq!(
        local_tasks
            .interrupt_lingering_running(&durable_req_ids, i64::MAX)
            .unwrap(),
        1
    );
    assert_eq!(
        local_tasks
            .get_for_owner(OWNER, &protected.req_id)
            .unwrap()
            .unwrap()
            .status,
        "running"
    );
    let resume_required = local_tasks
        .get_for_owner(OWNER, &orphaned.req_id)
        .unwrap()
        .unwrap();
    assert_eq!(resume_required.status, "resume_required");
    assert_eq!(resume_required.sync_state, "local_only");
    assert!(resume_required
        .error
        .as_deref()
        .is_some_and(|error| error.contains("Resume")));
    assert!(resume_required.finished_at_ms.is_some());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn startup_grace_blocks_marker_first_race_until_no_handle_evidence_is_old() {
    let root = test_root();
    let local_tasks = LocalTaskStore::new(root.join("local-tasks.sqlite3"));
    let task = completion("event-marker-first", "req-marker-first");
    create_local_task(&local_tasks, &task);
    let started_at = local_tasks
        .get_for_owner(OWNER, &task.req_id)
        .unwrap()
        .unwrap()
        .started_at_ms;

    assert_eq!(
        local_tasks
            .interrupt_lingering_running(&HashSet::new(), started_at.saturating_sub(1))
            .unwrap(),
        0,
        "a fresh durable marker must survive until handle registration can finish"
    );
    assert_eq!(
        local_tasks
            .get_for_owner(OWNER, &task.req_id)
            .unwrap()
            .unwrap()
            .status,
        "running"
    );
    assert_eq!(
        local_tasks
            .interrupt_lingering_running(&HashSet::new(), started_at)
            .unwrap(),
        1,
        "only an old task with no protected handle evidence may become resume_required"
    );
    assert_eq!(
        local_tasks
            .get_for_owner(OWNER, &task.req_id)
            .unwrap()
            .unwrap()
            .status,
        "resume_required"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn completion_replay_backoff_is_bounded_and_due_check_is_deterministic() {
    assert_eq!(completion_replay_backoff_ms(1), 3_000);
    assert_eq!(completion_replay_backoff_ms(2), 6_000);
    assert_eq!(completion_replay_backoff_ms(u32::MAX), 300_000);

    let pending = PendingCliCompletion {
        completion: completion("event-backoff", "req-backoff"),
        attempt_count: 2,
        last_attempt_at_ms: Some(1_000),
        last_error: None,
    };
    assert!(!completion_replay_is_due(&pending, 6_999));
    assert!(completion_replay_is_due(&pending, 7_000));
}

#[tokio::test]
async fn session_task_shutdown_aborts_a_noncooperative_task_after_timeout() {
    let mut task = tokio::spawn(std::future::pending::<()>());
    shutdown_session_task("test task", &mut task, Duration::from_millis(5)).await;
    assert!(task.is_finished());
}

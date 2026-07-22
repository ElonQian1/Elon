//! Periodic reconciliation for local tasks whose executor ownership vanished.
//!
//! A pending completion is a retry log, not proof that all terminal stores are
//! coherent. Each pass first replays the existing durable terminal reconciler;
//! only a live PID, sidecar, current handle, or fresh journal heartbeat may keep
//! a still-nonterminal row in `running` after that replay fails.

use std::{collections::HashSet, sync::Arc, time::Duration};

use anyhow::Result;
use tracing::{info, warn};

use crate::NodeRuntime;

const STALE_AFTER_MS: u128 = 2 * 60 * 1_000;
const RECONCILE_INTERVAL: Duration = Duration::from_secs(30);
const COMPLETION_SCAN_LIMIT: usize = 1_000;

pub(crate) async fn reconcile_once(runtime: &NodeRuntime) -> Result<usize> {
    reconcile_with_stale_after(runtime, STALE_AFTER_MS).await
}

async fn reconcile_with_stale_after(runtime: &NodeRuntime, stale_after_ms: u128) -> Result<usize> {
    for completion in runtime
        .completion_outbox
        .list_pending(COMPLETION_SCAN_LIMIT)?
        .into_iter()
        .filter(|completion| {
            completion.origin == crate::node_agent_completion_outbox::LOCAL_OFFLINE_ORIGIN
        })
    {
        match crate::node_agent_local_terminal_reconcile::LocalTerminalReconciler::from_runtime(
            runtime,
        )
        .reconcile(&completion)
        .await
        {
            Ok(()) => info!(
                req_id = %completion.req_id,
                event_id = %completion.event_id,
                "reconciled trusted local terminal state from durable outbox"
            ),
            Err(error) => warn!(
                req_id = %completion.req_id,
                event_id = %completion.event_id,
                %error,
                "durable local terminal reconciliation remains retryable"
            ),
        }
    }

    let now = crate::node_agent_cli_sidecar::now_ms();
    let mut protected = runtime
        .active_cli_prompts
        .views_without_approvals()
        .await
        .into_iter()
        .filter(|handle| handle.control_handle_live)
        .map(|handle| handle.req_id)
        .collect::<HashSet<_>>();
    protected.extend(
        runtime
            .cli_sidecars
            .all_sessions()?
            .into_iter()
            .filter(|session| session.protects_startup_reconcile_at(now))
            .map(|session| session.task_id),
    );

    for task in runtime.local_tasks.list_update_install_candidates()? {
        let Some(record) = runtime.task_journal.snapshot(&task.task_id, 0, 1)?.record else {
            continue;
        };
        if journal_record_protects(&record, now, stale_after_ms) {
            protected.insert(task.task_id);
        }
    }
    let cutoff = (now.min(i64::MAX as u128) as i64)
        .saturating_sub(stale_after_ms.min(i64::MAX as u128) as i64);
    runtime
        .local_tasks
        .mark_stale_without_runtime(&protected, cutoff)
}

fn journal_record_protects(
    record: &crate::node_agent_task_journal::TaskJournalRecord,
    now: u128,
    stale_after_ms: u128,
) -> bool {
    let process_live = record
        .os_pid
        .is_some_and(crate::node_agent_cli_worker::process_is_running);
    let heartbeat = record.heartbeat_at_ms.unwrap_or(record.updated_at_ms);
    let heartbeat_fresh = now >= heartbeat && now.saturating_sub(heartbeat) <= stale_after_ms;
    matches!(
        record.status.as_str(),
        "running" | "recovering" | "reattaching"
    ) && (process_live || heartbeat_fresh)
}

pub(crate) fn spawn_reconciler(runtime: Arc<NodeRuntime>) {
    let runtime_handle = tokio::runtime::Handle::current();
    std::thread::Builder::new()
        .name("elon-local-task-orphan-reconcile".to_string())
        .spawn(move || loop {
            std::thread::sleep(RECONCILE_INTERVAL);
            runtime_handle.block_on(async {
                match reconcile_once(&runtime).await {
                    Ok(0) => {}
                    Ok(changed) => {
                        warn!(changed, "periodic orphan reconciliation requested Resume")
                    }
                    Err(error) => warn!(%error, "periodic orphan reconciliation failed closed"),
                }
            });
        })
        .expect("local task orphan reconciler thread should start");
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use homecli_proto::{CliCompletionEnvelope, CliCompletionProducerIdentity, CliProjectContext};

    use super::*;
    use crate::node_agent_local_task_store::{LocalTaskStart, LocalTaskStore};

    #[tokio::test]
    async fn complete_evidence_finishes_but_incomplete_outbox_becomes_resume_required() {
        let root = std::env::temp_dir().join(format!(
            "elon-orphan-reconcile-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let runtime = test_runtime(&root);
        for task_id in ["complete", "incomplete"] {
            runtime
                .local_tasks
                .create(LocalTaskStart {
                    task_id,
                    owner_user_id: "owner",
                    agent_id: "agent",
                    install_id: "install",
                    project_id: "project",
                    channel_id: None,
                    conversation_id: task_id,
                    workspace_path: root.to_str().unwrap(),
                    prompt: "work",
                    cli: "codex",
                    runtime_permission: "full_access",
                })
                .unwrap();
        }
        let complete = completion("complete", "event-complete", "owner");
        let incomplete = completion("incomplete", "event-incomplete", "wrong-owner");
        runtime.completion_outbox.enqueue(&complete).unwrap();
        runtime.completion_outbox.enqueue(&incomplete).unwrap();

        assert_eq!(reconcile_with_stale_after(&runtime, 0).await.unwrap(), 1);
        assert_eq!(
            runtime.local_tasks.get("complete").unwrap().unwrap().status,
            "done"
        );
        assert_eq!(
            runtime
                .local_tasks
                .get("incomplete")
                .unwrap()
                .unwrap()
                .status,
            "resume_required"
        );
        assert_eq!(runtime.completion_outbox.pending_count().unwrap(), 2);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn live_pid_or_fresh_heartbeat_blocks_orphan_conversion() {
        let now = crate::node_agent_cli_sidecar::now_ms();
        let mut record = crate::node_agent_task_journal::TaskJournalRecord {
            req_id: "live".into(),
            cli_name: "codex".into(),
            route: None,
            run_handle_id: None,
            cwd: None,
            runtime_permission: None,
            os_pid: Some(std::process::id()),
            process_started_at_ms: Some(1),
            codex_session_id: None,
            codex_session_scope_key: None,
            codex_session_updated_at_ms: None,
            status: "running".into(),
            phase: "reasoning".into(),
            current_command: None,
            last_progress_ms: None,
            heartbeat_at_ms: Some(0),
            timeout_policy: None,
            started_at_ms: 0,
            updated_at_ms: 0,
            cancel_requested_at_ms: None,
            cancel_intent: None,
        };
        assert!(journal_record_protects(&record, now, 0));
        record.os_pid = None;
        record.heartbeat_at_ms = Some(now);
        assert!(journal_record_protects(&record, now, 1));
        record.heartbeat_at_ms = Some(0);
        assert!(!journal_record_protects(&record, now, 1));
    }

    fn test_runtime(root: &Path) -> NodeRuntime {
        let mut runtime = NodeRuntime::new(
            crate::node_agent_config::NodeConfig {
                cloud_url: "ws://127.0.0.1".into(),
                cloud_http_url: "http://127.0.0.1".into(),
                ollama_url: "http://127.0.0.1".into(),
                lm_studio_url: None,
                custom_url: None,
                price_per_1k: 0.0,
            },
            Some(crate::node_agent_config::Credentials {
                agent_id: "agent".into(),
                agent_secret: "unused".into(),
                owner_user_id: "owner".into(),
                user_token: None,
            }),
            crate::pc_storage_repo::StorageSettings::default(),
            crate::node_agent_data_root::resolve(None, None, None),
            "install".into(),
        );
        runtime.local_tasks = LocalTaskStore::new(root.join("tasks.sqlite3"));
        runtime.task_journal =
            crate::node_agent_task_journal::TaskJournal::new(root.join("journal"));
        runtime.completion_outbox = crate::node_agent_completion_outbox::CliCompletionOutbox::new(
            root.join("outbox.sqlite3"),
        );
        runtime.update_recovery =
            crate::node_agent_update_recovery::UpdateRecoveryStore::new(root.join("recovery.json"));
        runtime
    }

    fn completion(task_id: &str, event_id: &str, owner: &str) -> CliCompletionEnvelope {
        CliCompletionEnvelope {
            event_id: event_id.into(),
            req_id: task_id.into(),
            cli: "codex".into(),
            origin: crate::node_agent_completion_outbox::LOCAL_OFFLINE_ORIGIN.into(),
            producer_identity: Some(CliCompletionProducerIdentity {
                owner_user_id: owner.into(),
                agent_id: "agent".into(),
                install_id: "install".into(),
            }),
            project_context: Some(CliProjectContext {
                project_id: "project".into(),
                conversation_id: task_id.into(),
                runtime_permission: Some("full_access".into()),
            }),
            channel_id: None,
            prompt: None,
            final_output: "done".into(),
            exit_ok: true,
            error: None,
            session_id: None,
            prompt_tokens: None,
            cached_input_tokens: None,
            completion_tokens: None,
            reasoning_tokens: None,
            total_tokens: None,
            model: None,
            workspace_status: None,
            created_at_ms: 10,
        }
    }
}

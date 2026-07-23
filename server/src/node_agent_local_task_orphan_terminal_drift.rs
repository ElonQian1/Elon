//! Align durable terminal local rows with stale nonterminal journal records.

use anyhow::Result;
use tracing::{info, warn};

use crate::NodeRuntime;

pub(super) async fn sync(runtime: &NodeRuntime, now: u128, limit: usize) -> Result<usize> {
    let candidates = runtime
        .local_tasks
        .list_terminal_journal_drift_candidates(limit)?;
    let mut changed = 0;
    for task in candidates {
        let result = sync_one(runtime, &task.task_id, now).await;
        match result {
            Ok(true) => {
                changed += 1;
                info!(task_id = %task.task_id, "aligned stale journal with durable resume_required task");
            }
            Ok(false) => {}
            Err(error) => warn!(
                task_id = %task.task_id,
                %error,
                "terminal journal drift remains fail-closed while other candidates continue"
            ),
        }
    }
    Ok(changed)
}

async fn sync_one(runtime: &NodeRuntime, task_id: &str, now: u128) -> Result<bool> {
    if runtime
        .active_cli_prompt_view(task_id)
        .await
        .is_some_and(|handle| handle.control_handle_live)
    {
        return Ok(false);
    }
    if let Some(sidecar) = runtime.cli_sidecars.session_for_task(task_id)? {
        if super::sidecar_record_protects(&sidecar, now)? {
            return Ok(false);
        }
    }
    let Some(record) = runtime.task_journal.record(task_id)? else {
        return Ok(false);
    };
    if !matches!(
        record.status.as_str(),
        "running" | "recovering" | "reattaching" | "cancel_requested"
    ) || super::journal_record_protects(&record, now, 0)?
    {
        return Ok(false);
    }
    runtime.task_journal.record_finished_with_outcome(
        task_id,
        "resume_required",
        Some("本机任务已持久化为待继续，且执行器所有权已过期；journal 已自动对齐"),
    )?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::*;
    use crate::{
        node_agent_local_task_store::{LocalTaskStart, LocalTaskStore},
        node_agent_task_journal::TaskJournalStart,
    };

    #[tokio::test]
    async fn terminal_resume_required_aligns_a_stale_running_journal() {
        let root = unique_root("terminal-journal-drift");
        let runtime = test_runtime(&root);
        create_task(&runtime, "terminal-drift");
        start_journal(&runtime, "terminal-drift");
        assert!(runtime
            .local_tasks
            .mark_one_stale_without_runtime("terminal-drift", i64::MAX)
            .unwrap());

        assert_eq!(
            sync(
                &runtime,
                crate::node_agent_cli_sidecar::now_ms().saturating_add(1),
                10,
            )
            .await
            .unwrap(),
            1
        );
        assert_eq!(
            runtime
                .task_journal
                .snapshot("terminal-drift", 0, 10)
                .unwrap()
                .record
                .unwrap()
                .status,
            "resume_required"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn finished_success_journal_is_monotonic_terminal_evidence() {
        let root = unique_root("finished-journal");
        let runtime = test_runtime(&root);
        start_journal(&runtime, "finished");
        runtime.task_journal.record_finished("finished").unwrap();
        assert!(
            crate::node_agent_terminal_journal::has_finished_success(&runtime, "finished").unwrap()
        );
        overwrite_journal_status(&root, "finished", "resume_required");
        assert_eq!(
            runtime
                .task_journal
                .record("finished")
                .unwrap()
                .unwrap()
                .status,
            "resume_required",
            "the fixture must model a registry overwritten by an older node"
        );
        assert!(
            crate::node_agent_terminal_journal::has_finished_success(&runtime, "finished").unwrap(),
            "the immutable success event must recover an old overwritten registry"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn failed_finished_event_is_not_success_evidence() {
        let root = unique_root("failed-journal");
        let runtime = test_runtime(&root);
        start_journal(&runtime, "failed");
        runtime
            .task_journal
            .record_finished_with_outcome("failed", "failed", Some("test failure"))
            .unwrap();
        overwrite_journal_status(&root, "failed", "resume_required");
        assert!(
            !crate::node_agent_terminal_journal::has_finished_success(&runtime, "failed").unwrap(),
            "a failed immutable event must remain fail-closed"
        );
        let _ = fs::remove_dir_all(root);
    }

    fn overwrite_journal_status(root: &Path, task_id: &str, status: &str) {
        let registry_path = root.join("journal").join("registry.json");
        let mut registry: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&registry_path).unwrap()).unwrap();
        registry[task_id]["status"] = serde_json::Value::String(status.into());
        fs::write(
            &registry_path,
            serde_json::to_string_pretty(&registry).unwrap(),
        )
        .unwrap();
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

    fn create_task(runtime: &NodeRuntime, task_id: &str) {
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
                workspace_path: "D:/demo",
                prompt: "work",
                cli: "codex",
                runtime_permission: "full_access",
            })
            .unwrap();
    }

    fn start_journal(runtime: &NodeRuntime, task_id: &str) {
        runtime
            .task_journal
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

    fn unique_root(suffix: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "elon-orphan-reconcile-{suffix}-{}",
            uuid::Uuid::new_v4().simple()
        ))
    }
}

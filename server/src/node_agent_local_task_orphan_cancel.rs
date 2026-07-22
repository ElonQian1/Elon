//! Terminal convergence for stale `cancel_requested` local tasks.

use std::{collections::HashSet, path::Path};

use anyhow::Result;

use crate::NodeRuntime;

pub(super) async fn reconcile_candidate(
    runtime: &NodeRuntime,
    task: &crate::node_agent_local_task_store::LocalTaskRecord,
    now: u128,
    stale_after_ms: u128,
    cutoff: i64,
    observed_events: &HashSet<String>,
) -> Result<bool> {
    if task.status != "cancel_requested"
        || task.completion_event_id.is_some()
        || task.started_at_ms > cutoff
        || runtime
            .completion_outbox
            .latest_for_req_id(&task.task_id)?
            .is_some_and(|completion| !observed_events.contains(&completion.event_id))
    {
        return Ok(false);
    }
    let active_workspace = task
        .workspace_status
        .as_ref()
        .and_then(|status| status.get("active_workspace_path"))
        .and_then(serde_json::Value::as_str)
        .map(Path::new)
        .unwrap_or_else(|| Path::new(&task.workspace_path));
    if super::runtime_evidence::exact_runtime_protects(
        runtime,
        task,
        active_workspace,
        now,
        stale_after_ms,
    )
    .await?
    {
        return Ok(false);
    }
    reconcile_stale_cancel(runtime, &task.task_id, cutoff)
}

pub(super) fn reconcile_stale_cancel(
    runtime: &NodeRuntime,
    task_id: &str,
    cutoff: i64,
) -> Result<bool> {
    let durable_cancel_intent = runtime
        .task_journal
        .record(task_id)?
        .and_then(|record| record.cancel_intent)
        .is_some();
    let changed = runtime.local_tasks.mark_one_stale_cancel_requested(
        task_id,
        cutoff,
        durable_cancel_intent,
    )?;
    if changed {
        let (status, reason) = if durable_cancel_intent {
            (
                "canceled",
                "取消请求已持久化，且执行器所有权已过期；节点已确认任务停止",
            )
        } else {
            (
                "resume_required",
                "历史取消中任务缺少可验证的取消意图；现场已保留，请检查后继续",
            )
        };
        runtime
            .task_journal
            .record_finished_with_outcome(task_id, status, Some(reason))?;
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use homecli_proto::CancelRequestAudit;

    use super::*;
    use crate::{
        node_agent_local_task_store::{LocalTaskStart, LocalTaskStore},
        node_agent_task_journal::{CancelIntentTarget, TaskJournalStart},
    };

    #[test]
    fn durable_intent_becomes_canceled_in_both_stores() {
        let root = unique_root("intent");
        let runtime = test_runtime(&root);
        start_task(&runtime, "cancel-intent");
        runtime
            .task_journal
            .record_cancel_intent(
                "cancel-intent",
                CancelIntentTarget {
                    run_handle_id: Some("cancel-intent".into()),
                    active_started_at_ms: None,
                    sidecar_session_id: None,
                },
                &CancelRequestAudit {
                    requested_by: Some("owner".into()),
                    source: Some("pc_ui".into()),
                    reason: Some("user_stop_button".into()),
                    requested_at_ms: Some(1),
                    interruption_source: None,
                },
            )
            .unwrap();
        runtime
            .local_tasks
            .mark_cancel_requested("cancel-intent")
            .unwrap();
        assert!(reconcile_stale_cancel(&runtime, "cancel-intent", i64::MAX).unwrap());
        assert_statuses(&runtime, "cancel-intent", "canceled");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn legacy_cancel_without_intent_becomes_resume_required() {
        let root = unique_root("legacy");
        let runtime = test_runtime(&root);
        start_task(&runtime, "cancel-legacy");
        runtime
            .local_tasks
            .mark_cancel_requested("cancel-legacy")
            .unwrap();
        assert!(reconcile_stale_cancel(&runtime, "cancel-legacy", i64::MAX).unwrap());
        assert_statuses(&runtime, "cancel-legacy", "resume_required");
        let _ = fs::remove_dir_all(root);
    }

    fn assert_statuses(runtime: &NodeRuntime, task_id: &str, expected: &str) {
        assert_eq!(
            runtime.local_tasks.get(task_id).unwrap().unwrap().status,
            expected
        );
        assert_eq!(
            runtime
                .task_journal
                .snapshot(task_id, 0, 10)
                .unwrap()
                .record
                .unwrap()
                .status,
            expected
        );
    }

    fn start_task(runtime: &NodeRuntime, task_id: &str) {
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
        runtime
    }

    fn unique_root(suffix: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "elon-orphan-cancel-{suffix}-{}",
            uuid::Uuid::new_v4().simple()
        ))
    }
}

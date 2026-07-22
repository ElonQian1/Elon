//! Durable dispatch receipts for local HTTP acceptance and worker startup.

use anyhow::Result;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tracing::warn;

use crate::{
    node_agent_cli_done::CliCompletionContext,
    node_agent_task_journal::{TaskJournal, TaskJournalStart},
    NodeRuntime,
};

pub(super) fn prepare_local(
    journal: &TaskJournal,
    request: &super::CliTaskDispatchRequest,
) -> bool {
    if request.completion_context.origin
        != crate::node_agent_completion_outbox::LOCAL_OFFLINE_ORIGIN
    {
        return false;
    }
    let runtime_permission = request
        .project_context
        .as_ref()
        .and_then(|context| context.runtime_permission.as_deref());
    match persist_initial_dispatch_receipt(
        journal,
        &request.req_id,
        &request.cli,
        request.cwd.as_deref(),
        runtime_permission,
    ) {
        Ok(_) => true,
        Err(error) => {
            warn!(
                %error,
                task_id = %request.req_id,
                "failed to persist synchronous local dispatch receipt"
            );
            false
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn ensure_worker(
    already_persisted: bool,
    runtime: &NodeRuntime,
    completion_context: &CliCompletionContext,
    out_tx: &mpsc::UnboundedSender<Message>,
    task_id: &str,
    cli_name: &str,
    cwd: Option<&str>,
    runtime_permission: Option<&str>,
) -> bool {
    if already_persisted {
        return true;
    }
    if let Err(error) = persist_worker_dispatch_receipt(
        &runtime.task_journal,
        task_id,
        cli_name,
        cwd,
        runtime_permission,
    ) {
        super::failure::send_preflight_failure(
            runtime,
            completion_context,
            cli_name,
            out_tx,
            task_id.to_string(),
            format!("DISPATCH_PERSIST_FAILED: 无法持久化派发起点: {error}"),
        )
        .await;
        return false;
    }
    true
}

fn persist_worker_dispatch_receipt(
    journal: &TaskJournal,
    task_id: &str,
    cli_name: &str,
    cwd: Option<&str>,
    runtime_permission: Option<&str>,
) -> Result<()> {
    journal.record_started(TaskJournalStart {
        req_id: task_id,
        cli_name,
        route: Some(crate::node_agent_active_task::route_for_cli(cli_name)),
        run_handle_id: Some(task_id),
        cwd,
        runtime_permission,
    })
}

fn persist_initial_dispatch_receipt(
    journal: &TaskJournal,
    task_id: &str,
    cli_name: &str,
    cwd: Option<&str>,
    runtime_permission: Option<&str>,
) -> Result<bool> {
    journal.record_started_if_absent(TaskJournalStart {
        req_id: task_id,
        cli_name,
        route: Some(crate::node_agent_active_task::route_for_cli(cli_name)),
        run_handle_id: Some(task_id),
        cwd,
        runtime_permission,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_dispatch_receipt_is_visible_before_worker_spawn() {
        let root = std::env::temp_dir().join(format!(
            "elon-local-dispatch-receipt-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let journal = TaskJournal::new(&root);
        assert!(persist_initial_dispatch_receipt(
            &journal,
            "local-dispatch-receipt",
            "codex",
            Some("C:/workspace"),
            Some("full_access"),
        )
        .unwrap());
        let first = journal.snapshot("local-dispatch-receipt", 0, 10).unwrap();
        assert!(!persist_initial_dispatch_receipt(
            &journal,
            "local-dispatch-receipt",
            "codex",
            Some("C:/different"),
            Some("full_access"),
        )
        .unwrap());
        let second = journal.snapshot("local-dispatch-receipt", 0, 10).unwrap();
        assert_eq!(second.last_event_seq, first.last_event_seq);
        let record = second.record.expect("dispatch receipt");
        assert_eq!(record.status, "running");
        assert_eq!(record.cwd.as_deref(), Some("C:/workspace"));
        assert!(record.heartbeat_at_ms.is_some());
        assert_eq!(
            record.run_handle_id.as_deref(),
            Some("local-dispatch-receipt")
        );
        let _ = std::fs::remove_dir_all(root);
    }
}

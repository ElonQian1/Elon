use homecli_proto::AgentToServer;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tracing::warn;

pub(crate) fn spawn(
    runtime: Arc<crate::NodeRuntime>,
    out_tx: mpsc::UnboundedSender<Message>,
    req_id: String,
    task_id: String,
    since: usize,
    limit: usize,
) {
    tokio::spawn(async move {
        let reply = inspect_cli_task_journal(runtime.as_ref(), req_id, task_id, since, limit).await;
        let _ = out_tx.send(crate::ws_text(&reply));
    });
}

pub(crate) async fn inspect_cli_task_journal(
    runtime: &crate::NodeRuntime,
    req_id: String,
    task_id: String,
    since: usize,
    limit: usize,
) -> AgentToServer {
    let task_id = task_id.trim().to_string();
    if task_id.is_empty() {
        return AgentToServer::CliTaskJournalSnapshot {
            req_id,
            task_id,
            ok: false,
            snapshot: None,
            error: Some("task_id 不能为空".to_string()),
        };
    }

    let active = runtime.active_cli_prompt_view(&task_id).await;
    let sidecar = runtime
        .cli_sidecars
        .session_for_task(&task_id)
        .unwrap_or_else(|error| {
            warn!("读取 CLI sidecar 会话失败: {error}");
            None
        });

    match runtime.task_journal_snapshot(&task_id, since, limit.clamp(1, 200), None) {
        Ok(snapshot) => {
            let attach = crate::node_agent_task_resume::task_attach_state_with_sidecar(
                snapshot.record.as_ref(),
                active,
                sidecar,
            );
            let resume = crate::node_agent_task_resume::task_resume_contract_with_journal_approvals(
                &attach,
                &snapshot.approvals,
            );
            let task_status = snapshot
                .record
                .as_ref()
                .map(|record| record.status.as_str());
            let approval_state = snapshot.approvals.resolve_runtime_state_for_task_status(
                resume.active_approval_ids(),
                resume.can_approve_tools(),
                task_status,
            );
            let mut runtime_status =
                crate::node_agent_task_journal::runtime_status_payload(snapshot.record.as_ref());
            if approval_state.actionable_count > 0 {
                runtime_status["phase"] = serde_json::Value::String("approval".to_string());
            }
            let snapshot_task_id = snapshot.task_id.clone();
            let payload = serde_json::json!({
                "ok": true,
                "source": "local_task_journal",
                "task_id": snapshot.task_id,
                "record": snapshot.record,
                "events": snapshot.events,
                "last_event_seq": snapshot.last_event_seq,
                "has_more": snapshot.has_more,
                "attach": attach,
                "resume": resume,
                "approval_state": approval_state,
                "runtime": runtime_status,
            });
            AgentToServer::CliTaskJournalSnapshot {
                req_id,
                task_id: snapshot_task_id,
                ok: true,
                snapshot: Some(payload),
                error: None,
            }
        }
        Err(error) => AgentToServer::CliTaskJournalSnapshot {
            req_id,
            task_id,
            ok: false,
            snapshot: None,
            error: Some(error.to_string()),
        },
    }
}

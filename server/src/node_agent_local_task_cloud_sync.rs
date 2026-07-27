//! Periodic, acknowledged replay of node-local task starts to the cloud.

use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    time::Duration,
};

use homecli_proto::{
    AgentToServer, CliCompletionProducerIdentity, CliLocalTaskSnapshot, CliProjectContext,
};
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::tungstenite::Message;

use crate::{
    node_agent_local_task_store::LocalTaskRecord, node_agent_task_journal::TaskJournalRecord,
    ws_text, NodeRuntime,
};

const SYNC_INTERVAL: Duration = Duration::from_secs(3);
const SYNC_SCAN_LIMIT: usize = 100;

#[derive(Clone, Default)]
pub(crate) struct LocalTaskSyncAcks {
    settled: Arc<Mutex<HashSet<String>>>,
}

impl LocalTaskSyncAcks {
    pub(crate) fn settle(&self, task_id: &str, revision: &str, accepted: bool, retryable: bool) {
        if !accepted && retryable {
            return;
        }
        if let Ok(mut settled) = self.settled.lock() {
            settled.insert(ack_key(task_id, revision));
        }
    }

    fn contains(&self, task_id: &str, revision: &str) -> bool {
        self.settled
            .lock()
            .map(|settled| settled.contains(&ack_key(task_id, revision)))
            .unwrap_or(false)
    }
}

pub(crate) fn spawn(
    runtime: Arc<NodeRuntime>,
    out_tx: mpsc::UnboundedSender<Message>,
    producer: CliCompletionProducerIdentity,
    acks: LocalTaskSyncAcks,
    mut stop_rx: watch::Receiver<bool>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(SYNC_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                _ = stop_rx.changed() => return,
                _ = ticker.tick() => {}
            }
            let tasks = match runtime
                .local_tasks
                .list_for_owner(&producer.owner_user_id, SYNC_SCAN_LIMIT)
            {
                Ok(tasks) => tasks,
                Err(error) => {
                    tracing::warn!(%error, "读取本机任务云端同步候选失败");
                    continue;
                }
            };
            let task_ids = tasks
                .iter()
                .map(|task| task.task_id.clone())
                .collect::<Vec<_>>();
            let journal = runtime
                .task_journal
                .records_for_req_ids(&task_ids)
                .unwrap_or_default();
            for task in tasks.into_iter().filter(should_sync) {
                let snapshot = snapshot(&task, journal.get(&task.task_id), &producer);
                if acks.contains(&snapshot.task_id, &snapshot.revision) {
                    continue;
                }
                if out_tx
                    .send(ws_text(&AgentToServer::CliLocalTaskSync { snapshot }))
                    .is_err()
                {
                    return;
                }
            }
        }
    })
}

fn should_sync(task: &LocalTaskRecord) -> bool {
    task.execution_origin == "local_offline"
        && task.sync_state != "synced"
        && task.sync_state != "rejected"
}

fn snapshot(
    task: &LocalTaskRecord,
    journal: Option<&TaskJournalRecord>,
    producer: &CliCompletionProducerIdentity,
) -> CliLocalTaskSnapshot {
    let session_id = journal
        .and_then(|record| record.codex_session_id.clone())
        .or_else(|| task.codex_session_id.clone());
    let updated_at_ms = journal
        .map(|record| clamp_ms(record.updated_at_ms))
        .or_else(|| task.finished_at_ms.map(nonnegative_ms))
        .unwrap_or_else(|| nonnegative_ms(task.started_at_ms));
    let revision = revision(task, session_id.as_deref());
    CliLocalTaskSnapshot {
        task_id: task.task_id.clone(),
        revision,
        cli: task.cli.clone(),
        producer_identity: producer.clone(),
        project_context: CliProjectContext {
            project_id: task.project_id.clone(),
            conversation_id: task.conversation_id.clone(),
            runtime_permission: Some(task.runtime_permission.clone()),
        },
        channel_id: task.channel_id.clone(),
        prompt: task.prompt.clone(),
        workspace_path: task.workspace_path.clone(),
        status: task.status.clone(),
        session_id,
        started_at_ms: nonnegative_ms(task.started_at_ms),
        updated_at_ms,
    }
}

fn revision(task: &LocalTaskRecord, session_id: Option<&str>) -> String {
    format!(
        "{}:{}:{}:{}",
        task.status,
        task.sync_state,
        session_id.unwrap_or("-"),
        task.finished_at_ms.unwrap_or_default()
    )
}

fn ack_key(task_id: &str, revision: &str) -> String {
    format!("{task_id}\0{revision}")
}

fn clamp_ms(value: u128) -> u64 {
    value.min(u64::MAX as u128) as u64
}

fn nonnegative_ms(value: i64) -> u64 {
    value.max(0) as u64
}

#[cfg(test)]
mod tests {
    use super::LocalTaskSyncAcks;

    #[test]
    fn retryable_rejection_remains_replayable() {
        let acks = LocalTaskSyncAcks::default();
        acks.settle("local-a", "r1", false, true);
        assert!(!acks.contains("local-a", "r1"));
        acks.settle("local-a", "r1", true, false);
        assert!(acks.contains("local-a", "r1"));
    }
}

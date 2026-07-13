use std::{
    collections::{HashMap, HashSet},
    sync::{atomic::Ordering, Arc},
    time::{Duration, Instant},
};

use homecli_proto::{AgentToServer, CliCompletionEnvelope};
use tokio::sync::{mpsc, Mutex};

use super::AgentManager;

const RECOVERY_GRACE_ENV: &str = "ELON_PC_CLI_RECOVERY_GRACE_SECS";
const DEFAULT_RECOVERY_GRACE_SECS: u64 = 120;
const MIN_RECOVERY_GRACE_SECS: u64 = 10;
const MAX_RECOVERY_GRACE_SECS: u64 = 900;

pub(super) struct RecoveringCliRequest {
    node_id: String,
    sender: mpsc::UnboundedSender<AgentToServer>,
    deadline: Instant,
    disconnect_reason: String,
}

pub(super) type RecoveringCliRequests = Mutex<HashMap<String, RecoveringCliRequest>>;

impl AgentManager {
    /// Starts one lightweight deadline reaper per manager. Server restarts do not
    /// retain these receivers; durable completion reconciliation handles that case.
    pub(super) fn ensure_cli_recovery_worker(self: &Arc<Self>) {
        if self
            .recovery_worker_started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }

        let manager = Arc::downgrade(self);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                let Some(manager) = manager.upgrade() else {
                    break;
                };
                manager.expire_recovering_cli_at(Instant::now()).await;
            }
        });
    }

    /// Move only CLI receivers into the reconnect grace window. Other request
    /// families keep their historical fail-fast behavior.
    pub(super) async fn recover_session_pending(
        &self,
        node_id: &str,
        pending: &Arc<Mutex<HashMap<String, mpsc::UnboundedSender<AgentToServer>>>>,
        cli_pending_ids: &Arc<Mutex<HashSet<String>>>,
        reason: &str,
    ) {
        self.recover_session_pending_until(
            node_id,
            pending,
            cli_pending_ids,
            reason,
            Instant::now() + cli_recovery_grace(),
        )
        .await;
    }

    pub(super) async fn recover_session_pending_until(
        &self,
        node_id: &str,
        pending: &Arc<Mutex<HashMap<String, mpsc::UnboundedSender<AgentToServer>>>>,
        cli_pending_ids: &Arc<Mutex<HashSet<String>>>,
        reason: &str,
        deadline: Instant,
    ) {
        // Hold the recovery map across the drain so an accepted replay cannot
        // slip through the hand-off gap and get ACKed without waking its runner.
        let mut recovering = self.recovering_cli.lock().await;
        // All code that needs both session locks takes them in this order.
        let mut pending_guard = pending.lock().await;
        let mut cli_ids_guard = cli_pending_ids.lock().await;
        let stale: Vec<_> = pending_guard.drain().collect();
        let cli_ids = std::mem::take(&mut *cli_ids_guard);
        drop(cli_ids_guard);
        drop(pending_guard);

        for (req_id, sender) in stale {
            if cli_ids.contains(&req_id) {
                tracing::info!(
                    %node_id,
                    %req_id,
                    grace_seconds = deadline.saturating_duration_since(Instant::now()).as_secs(),
                    "PC CLI receiver entered reconnect recovery window"
                );
                recovering
                    .entry(req_id)
                    .or_insert_with(|| RecoveringCliRequest {
                        node_id: node_id.to_string(),
                        sender,
                        deadline,
                        disconnect_reason: reason.to_string(),
                    });
            } else {
                send_failed_cli_done(&req_id, &sender, reason);
            }
        }
    }

    /// Complete the in-memory runner after a durable replay was accepted.
    ///
    /// A reconnect retry may install a fresh active receiver for the same req_id
    /// while the disconnected session still has a stale recovery sender. Prefer
    /// the active receiver so the current caller cannot be left waiting after the
    /// node and server have already reconciled the durable completion.
    pub(crate) async fn deliver_accepted_cli_replay(
        &self,
        node_id: &str,
        completion: &CliCompletionEnvelope,
    ) -> bool {
        if let Some(sender) = self
            .take_active_cli(node_id, completion.req_id.as_str())
            .await
        {
            if deliver_completion(&sender, completion) {
                // Drop any stale sender retained by the session that preceded
                // this reconnect retry.
                let _ = self
                    .take_recovering_cli(node_id, completion.req_id.as_str())
                    .await;
                return true;
            }
        }

        if let Some(sender) = self
            .take_recovering_cli(node_id, completion.req_id.as_str())
            .await
        {
            if deliver_completion(&sender, completion) {
                return true;
            }
        }

        // A reconnect dispatch may have installed the active receiver after the
        // first lookup while disconnect recovery was also settling its handoff.
        if let Some(sender) = self
            .take_active_cli(node_id, completion.req_id.as_str())
            .await
        {
            return deliver_completion(&sender, completion);
        }
        false
    }

    async fn take_active_cli(
        &self,
        node_id: &str,
        req_id: &str,
    ) -> Option<mpsc::UnboundedSender<AgentToServer>> {
        let active = {
            let agents = self.agents.read().await;
            agents
                .get(node_id)
                .map(|entry| (entry.pending.clone(), entry.cli_pending_ids.clone()))
        }?;
        let (pending, cli_pending_ids) = active;
        let mut pending_guard = pending.lock().await;
        let mut cli_ids_guard = cli_pending_ids.lock().await;
        if !cli_ids_guard.remove(req_id) {
            return None;
        }
        pending_guard.remove(req_id)
    }

    pub(super) async fn take_recovering_cli(
        &self,
        node_id: &str,
        req_id: &str,
    ) -> Option<mpsc::UnboundedSender<AgentToServer>> {
        let mut recovering = self.recovering_cli.lock().await;
        if recovering
            .get(req_id)
            .is_some_and(|request| request.node_id == node_id)
        {
            return recovering.remove(req_id).map(|request| request.sender);
        }
        None
    }

    pub(super) async fn expire_recovering_cli_at(&self, now: Instant) -> usize {
        let expired = {
            let mut recovering = self.recovering_cli.lock().await;
            let expired_ids: Vec<_> = recovering
                .iter()
                .filter(|(_, request)| request.deadline <= now)
                .map(|(req_id, _)| req_id.clone())
                .collect();
            expired_ids
                .into_iter()
                .filter_map(|req_id| recovering.remove(&req_id).map(|request| (req_id, request)))
                .collect::<Vec<_>>()
        };
        let expired_count = expired.len();
        for (req_id, request) in expired {
            let message = format!(
                "PC 节点短线恢复等待超时；稍后到达的本机结果仍会通过离线账本同步。断线原因：{}",
                request.disconnect_reason
            );
            send_failed_cli_done(&req_id, &request.sender, &message);
            tracing::warn!(
                node_id = %request.node_id,
                %req_id,
                "PC CLI reconnect recovery window expired"
            );
        }
        expired_count
    }

    #[cfg(test)]
    pub(super) async fn recovering_cli_count(&self) -> usize {
        self.recovering_cli.lock().await.len()
    }
}

fn cli_recovery_grace() -> Duration {
    let seconds = std::env::var(RECOVERY_GRACE_ENV)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_RECOVERY_GRACE_SECS)
        .clamp(MIN_RECOVERY_GRACE_SECS, MAX_RECOVERY_GRACE_SECS);
    Duration::from_secs(seconds)
}

fn deliver_completion(
    sender: &mpsc::UnboundedSender<AgentToServer>,
    completion: &CliCompletionEnvelope,
) -> bool {
    if !completion.final_output.is_empty() {
        if sender
            .send(AgentToServer::CliChunk {
                req_id: completion.req_id.clone(),
                text: completion.final_output.clone(),
            })
            .is_err()
        {
            return false;
        }
    }
    sender.send(completion.to_cli_done()).is_ok()
}

fn send_failed_cli_done(
    req_id: &str,
    sender: &mpsc::UnboundedSender<AgentToServer>,
    message: &str,
) {
    let _ = sender.send(AgentToServer::CliDone {
        req_id: req_id.to_string(),
        exit_ok: false,
        error: Some(message.to_string()),
        session_id: None,
        prompt_tokens: None,
        cached_input_tokens: None,
        completion_tokens: None,
        reasoning_tokens: None,
        total_tokens: None,
        model: None,
        workspace_status: None,
    });
}

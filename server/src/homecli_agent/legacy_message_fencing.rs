use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use anyhow::{anyhow, Result};
use homecli_proto::{AgentToServer, ServerToAgent};
use tokio::sync::{mpsc, oneshot, Mutex};
use uuid::Uuid;

use crate::node_registry::AgentProcessSessionKey;

use super::{agent_session::tool_approval_ack_key, AgentManager};

const TOOL_APPROVAL_ACK_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LegacyMessageDispatchError {
    SessionChanged,
    RequestAlreadyPending,
    WriterClosed,
}

impl AgentManager {
    /// Dispatch an approval decision only while the selected process session is still exact.
    pub async fn send_tool_approval_decision(
        &self,
        req_id: &str,
        approval_id: &str,
        decision: &str,
    ) -> Result<bool> {
        let target = {
            let agents = self.agents.read().await;
            let mut target = None;
            for agent in agents.values() {
                if !agent.pending.lock().await.contains_key(req_id) {
                    continue;
                }
                target = Some((agent.process_session.clone(), agent.approval_acks.clone()));
                break;
            }
            target
        };
        let Some((process_session, approval_acks)) = target else {
            return Err(anyhow!(
                "pending CLI request not found for tool approval: {req_id}"
            ));
        };

        let dispatch_id = Uuid::new_v4().to_string();
        let ack_key = tool_approval_ack_key(req_id, approval_id, &dispatch_id);
        let (ack_tx, ack_rx) = oneshot::channel();
        // Global legacy message order is Manager exact read -> session-owned map. Root mutation
        // holds only the Manager writer and starts map cleanup after releasing it.
        let agents = self.agents.read().await;
        let dispatched = match agents.get(process_session.agent_id()).filter(|agent| {
            agent.process_session == process_session
                && Arc::ptr_eq(&agent.approval_acks, &approval_acks)
        }) {
            Some(agent) => {
                let mut approval_acks_guard = approval_acks.lock().await;
                approval_acks_guard.insert(ack_key.clone(), ack_tx);
                Some(
                    if agent
                        .cmd_tx
                        .send(ServerToAgent::ToolApprovalDecision {
                            req_id: req_id.to_string(),
                            approval_id: approval_id.to_string(),
                            dispatch_id,
                            decision: decision.to_string(),
                        })
                        .is_err()
                    {
                        approval_acks_guard.remove(&ack_key);
                        false
                    } else {
                        true
                    },
                )
            }
            None => None,
        };
        drop(agents);
        match dispatched {
            Some(true) => {}
            Some(false) => return Err(anyhow!("agent writer closed")),
            None => return Err(anyhow!("agent session changed before approval dispatch")),
        }

        match tokio::time::timeout(TOOL_APPROVAL_ACK_TIMEOUT, ack_rx).await {
            Ok(Ok(accepted)) => Ok(accepted),
            Ok(Err(_)) => Err(anyhow!(
                "tool approval ack channel closed: req_id={req_id}, approval_id={approval_id}"
            )),
            Err(_) => {
                approval_acks.lock().await.remove(&ack_key);
                Err(anyhow!(
                    "tool approval ack timeout: req_id={req_id}, approval_id={approval_id}"
                ))
            }
        }
    }

    /// Deliver a security-sensitive approval ACK only while this exact process session is
    /// current. The Manager exact read precedes the waiter lock, matching all legacy dispatches;
    /// root cleanup does not touch waiter maps until after releasing its Manager writer.
    pub(super) async fn deliver_current_tool_approval_ack(
        &self,
        process_session: &AgentProcessSessionKey,
        approval_acks: &Arc<Mutex<HashMap<String, oneshot::Sender<bool>>>>,
        ack_key: &str,
        accepted: bool,
    ) -> Option<bool> {
        let agents = self.agents.read().await;
        let _entry = agents.get(process_session.agent_id()).filter(|entry| {
            &entry.process_session == process_session
                && Arc::ptr_eq(&entry.approval_acks, approval_acks)
        })?;
        let mut approval_acks = approval_acks.lock().await;
        let Some(sender) = approval_acks.remove(ack_key) else {
            return Some(false);
        };
        let _ = sender.send(accepted);
        Some(true)
    }

    /// Route task-scoped data only through the exact process session that owns its waiter.
    pub(super) async fn deliver_current_task_message(
        &self,
        process_session: &AgentProcessSessionKey,
        pending: &Arc<Mutex<HashMap<String, mpsc::UnboundedSender<AgentToServer>>>>,
        message: AgentToServer,
    ) -> Option<bool> {
        let task_id = message.task_id()?.to_string();
        let is_final = matches!(
            &message,
            AgentToServer::TaskExit { .. } | AgentToServer::TaskError { .. }
        );
        let agents = self.agents.read().await;
        let _entry = agents.get(process_session.agent_id()).filter(|entry| {
            &entry.process_session == process_session && Arc::ptr_eq(&entry.pending, pending)
        })?;
        let mut pending = pending.lock().await;
        let sender = if is_final {
            pending.remove(&task_id)
        } else {
            pending.get(&task_id).cloned()
        };
        let Some(sender) = sender else {
            return Some(false);
        };
        Some(sender.send(message).is_ok())
    }

    /// Route request-scoped data only through the exact process session that owns its waiter.
    pub(super) async fn deliver_current_req_message(
        &self,
        process_session: &AgentProcessSessionKey,
        pending: &Arc<Mutex<HashMap<String, mpsc::UnboundedSender<AgentToServer>>>>,
        cli_pending_ids: &Arc<Mutex<HashSet<String>>>,
        message: AgentToServer,
    ) -> Option<bool> {
        let req_id = message.req_id()?.to_string();
        let is_final = message.is_final_req_msg();
        let agents = self.agents.read().await;
        let _entry = agents.get(process_session.agent_id()).filter(|entry| {
            &entry.process_session == process_session
                && Arc::ptr_eq(&entry.pending, pending)
                && Arc::ptr_eq(&entry.cli_pending_ids, cli_pending_ids)
        })?;
        let mut pending = pending.lock().await;
        let sender = if is_final {
            cli_pending_ids.lock().await.remove(&req_id);
            pending.remove(&req_id)
        } else {
            pending.get(&req_id).cloned()
        };
        let Some(sender) = sender else {
            return Some(false);
        };
        Some(sender.send(message).is_ok())
    }

    /// Install a request waiter and enqueue its frame under one exact-session read fence.
    pub(super) async fn install_current_req_waiter_and_dispatch(
        &self,
        process_session: &AgentProcessSessionKey,
        pending: &Arc<Mutex<HashMap<String, mpsc::UnboundedSender<AgentToServer>>>>,
        req_id: &str,
        waiter: mpsc::UnboundedSender<AgentToServer>,
        message: ServerToAgent,
    ) -> std::result::Result<(), LegacyMessageDispatchError> {
        let agents = self.agents.read().await;
        let entry = agents
            .get(process_session.agent_id())
            .filter(|entry| {
                &entry.process_session == process_session && Arc::ptr_eq(&entry.pending, pending)
            })
            .ok_or(LegacyMessageDispatchError::SessionChanged)?;
        let mut pending = pending.lock().await;
        if pending.contains_key(req_id) {
            return Err(LegacyMessageDispatchError::RequestAlreadyPending);
        }
        pending.insert(req_id.to_string(), waiter);
        if entry.cmd_tx.send(message).is_err() {
            pending.remove(req_id);
            return Err(LegacyMessageDispatchError::WriterClosed);
        }
        Ok(())
    }

    /// Install a durable CLI waiter and enqueue its prompt under one exact-session read fence.
    pub(super) async fn install_current_cli_waiter_and_dispatch(
        &self,
        process_session: &AgentProcessSessionKey,
        pending: &Arc<Mutex<HashMap<String, mpsc::UnboundedSender<AgentToServer>>>>,
        cli_pending_ids: &Arc<Mutex<HashSet<String>>>,
        req_id: &str,
        waiter: mpsc::UnboundedSender<AgentToServer>,
        message: ServerToAgent,
    ) -> std::result::Result<(), LegacyMessageDispatchError> {
        let agents = self.agents.read().await;
        let entry = agents
            .get(process_session.agent_id())
            .filter(|entry| {
                &entry.process_session == process_session
                    && Arc::ptr_eq(&entry.pending, pending)
                    && Arc::ptr_eq(&entry.cli_pending_ids, cli_pending_ids)
            })
            .ok_or(LegacyMessageDispatchError::SessionChanged)?;
        let mut pending = pending.lock().await;
        let mut cli_pending_ids = cli_pending_ids.lock().await;
        pending.insert(req_id.to_string(), waiter);
        cli_pending_ids.insert(req_id.to_string());
        if entry.cmd_tx.send(message).is_err() {
            pending.remove(req_id);
            cli_pending_ids.remove(req_id);
            return Err(LegacyMessageDispatchError::WriterClosed);
        }
        Ok(())
    }
}

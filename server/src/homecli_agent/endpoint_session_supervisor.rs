use std::{
    collections::HashMap,
    sync::{Arc, Mutex as StdMutex, MutexGuard},
};

use anyhow::{bail, Result};
use tokio::sync::{watch, Notify};
use uuid::Uuid;

use crate::{
    node_compute_sharing::endpoint_authority::{
        NodeEndpointSessionOpenRequest, VerifiedSecureNodeEndpointTransport,
    },
    store::{node_credentials::NodeEndpointSessionPermit, Store},
    types::AppState,
};

use super::AgentManager;

mod cleanup;
mod expiry;
pub(crate) use cleanup::NodeEndpointSessionCleanup;

struct EndpointSessionEntry {
    current: NodeEndpointSessionCurrent,
    shutdown: watch::Sender<bool>,
}

#[derive(Default)]
pub(super) struct EndpointSessionSupervisor {
    sessions: StdMutex<HashMap<String, EndpointSessionEntry>>,
    changed: Notify,
}

#[derive(Clone)]
pub(crate) struct NodeEndpointSessionCurrent {
    permit: NodeEndpointSessionPermit,
    process_key: Uuid,
}

impl NodeEndpointSessionCurrent {
    pub(crate) fn permit(&self) -> &NodeEndpointSessionPermit {
        &self.permit
    }

    fn matches(&self, other: &Self) -> bool {
        self.process_key == other.process_key && self.permit.binding() == other.permit.binding()
    }
}

pub(crate) struct NodeEndpointSessionLease {
    current: NodeEndpointSessionCurrent,
    shutdown: watch::Receiver<bool>,
}

impl NodeEndpointSessionLease {
    pub(crate) fn into_parts(self) -> (NodeEndpointSessionCurrent, watch::Receiver<bool>) {
        (self.current, self.shutdown)
    }
}

impl EndpointSessionSupervisor {
    fn lock(&self) -> Result<MutexGuard<'_, HashMap<String, EndpointSessionEntry>>> {
        self.sessions
            .lock()
            .map_err(|_| anyhow::anyhow!("NODE_ENDPOINT_SESSION_SUPERVISOR_POISONED"))
    }

    fn install(
        &self,
        permit: NodeEndpointSessionPermit,
    ) -> Result<(NodeEndpointSessionCurrent, watch::Receiver<bool>)> {
        let agent_id = permit.binding().agent_id().to_string();
        let current = NodeEndpointSessionCurrent {
            permit,
            process_key: Uuid::new_v4(),
        };
        let (shutdown, receiver) = watch::channel(false);
        if let Some(previous) = self.lock()?.insert(
            agent_id,
            EndpointSessionEntry {
                current: current.clone(),
                shutdown,
            },
        ) {
            let _ = previous.shutdown.send(true);
        }
        self.changed.notify_one();
        Ok((current, receiver))
    }

    fn contains_exact(&self, current: &NodeEndpointSessionCurrent) -> Result<bool> {
        Ok(self
            .lock()?
            .get(current.permit().binding().agent_id())
            .is_some_and(|entry| entry.current.matches(current)))
    }

    fn detach_exact(&self, current: &NodeEndpointSessionCurrent) -> Result<bool> {
        let mut sessions = self.lock()?;
        if !sessions
            .get(current.permit().binding().agent_id())
            .is_some_and(|entry| entry.current.matches(current))
        {
            return Ok(false);
        }
        let removed = sessions
            .remove(current.permit().binding().agent_id())
            .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_SESSION_SUPERVISOR_CURRENTNESS_LOST"))?;
        let _ = removed.shutdown.send(true);
        drop(sessions);
        self.changed.notify_one();
        Ok(true)
    }

    fn detach_agent(&self, agent_id: &str) -> Result<Option<NodeEndpointSessionPermit>> {
        let mut sessions = self.lock()?;
        let removed = sessions.remove(agent_id);
        drop(sessions);
        if let Some(removed) = removed {
            let _ = removed.shutdown.send(true);
            self.changed.notify_one();
            return Ok(Some(removed.current.permit));
        }
        Ok(None)
    }

    fn signal_exact(&self, current: &NodeEndpointSessionCurrent) -> Result<()> {
        let sessions = self.lock()?;
        if let Some(entry) = sessions
            .get(current.permit().binding().agent_id())
            .filter(|entry| entry.current.matches(current))
        {
            let _ = entry.shutdown.send(true);
        }
        Ok(())
    }
}

impl AgentManager {
    /// Atomically authenticate a durable session and install only its compute-inert socket lease.
    pub(crate) async fn authenticate_and_install_endpoint_session(
        &self,
        store: &Store,
        request: &NodeEndpointSessionOpenRequest,
        transport: &VerifiedSecureNodeEndpointTransport,
    ) -> Result<NodeEndpointSessionLease> {
        let agents = self.agents.write().await;
        if agents.contains_key(request.agent_id()) {
            bail!("NODE_ENDPOINT_LEGACY_SESSION_PRESENT");
        }
        let permit = store.authenticate_node_endpoint_session(request, transport)?;
        let (current, shutdown) = match self.endpoint_sessions.install(permit.clone()) {
            Ok(installed) => installed,
            Err(error) => {
                let _ = store.terminal_close_node_endpoint_session(&permit);
                return Err(error);
            }
        };
        drop(agents);
        Ok(NodeEndpointSessionLease { current, shutdown })
    }

    pub(crate) async fn inspect_endpoint_session(
        &self,
        store: &Store,
        current: &NodeEndpointSessionCurrent,
    ) -> Result<NodeEndpointSessionPermit> {
        let _agents = self.agents.write().await;
        if !self.endpoint_sessions.contains_exact(current)? {
            bail!("NODE_ENDPOINT_SESSION_PROCESS_CURRENTNESS_MISMATCH");
        }
        match store.inspect_node_endpoint_session_currentness(current.permit()) {
            Ok(permit) => Ok(permit),
            Err(error) => {
                let _ = self.endpoint_sessions.signal_exact(current);
                if store
                    .terminal_close_node_endpoint_session(current.permit())
                    .is_ok()
                {
                    let _ = self.endpoint_sessions.detach_exact(current);
                }
                Err(error)
            }
        }
    }

    pub(crate) async fn close_endpoint_session(
        &self,
        store: &Store,
        current: &NodeEndpointSessionCurrent,
    ) -> Result<bool> {
        let _agents = self.agents.write().await;
        if !self.endpoint_sessions.contains_exact(current)? {
            return Ok(false);
        }
        match store.terminal_close_node_endpoint_session(current.permit()) {
            Ok(changed) => {
                self.endpoint_sessions.detach_exact(current)?;
                Ok(changed)
            }
            Err(error) => {
                let _ = self.endpoint_sessions.signal_exact(current);
                Err(error)
            }
        }
    }

    pub(crate) async fn expire_endpoint_session(
        &self,
        store: &Store,
        current: &NodeEndpointSessionCurrent,
    ) -> Result<bool> {
        let _agents = self.agents.write().await;
        if !self.endpoint_sessions.contains_exact(current)? {
            return Ok(false);
        }
        match store.expire_node_endpoint_session(current.permit()) {
            Ok(false) => Ok(false),
            Ok(true) => self.endpoint_sessions.detach_exact(current),
            Err(error) => {
                let _ = self.endpoint_sessions.signal_exact(current);
                Err(error)
            }
        }
    }

    pub(super) fn detach_endpoint_session_for_authority_mutation(
        &self,
        agent_id: &str,
    ) -> Result<Option<NodeEndpointSessionPermit>> {
        self.endpoint_sessions.detach_agent(agent_id)
    }

    pub(super) fn retry_detached_endpoint_session_terminal_close(
        &self,
        state: &Arc<AppState>,
        permit: NodeEndpointSessionPermit,
    ) {
        cleanup::spawn_detached_terminal_retry(state, permit);
    }
}

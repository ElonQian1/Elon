use anyhow::{bail, Result};

use crate::{store::node_credentials::NodeEndpointSessionPermit, store::Store};

use super::{AgentManager, NodeEndpointSessionCurrent};

impl AgentManager {
    /// Runs one synchronous endpoint-planning Store transaction while the exact process-local
    /// socket remains fenced against replacement and credential mutation.
    ///
    /// The Store operation must independently revalidate the durable v216 head, receipt,
    /// credential and absolute expiry on the same SQLite transaction that records the stage.
    pub(crate) async fn with_current_endpoint_planning_session<T>(
        &self,
        store: &Store,
        current: &NodeEndpointSessionCurrent,
        operation: impl FnOnce(&Store, &NodeEndpointSessionPermit) -> Result<T>,
    ) -> Result<T> {
        let _agents = self.agents.write().await;
        if !self.endpoint_sessions.contains_exact(current)? {
            bail!("NODE_ENDPOINT_SESSION_PROCESS_CURRENTNESS_MISMATCH");
        }
        current.permit().require_planning_bootstrap_v14()?;
        let result = operation(store, current.permit());
        if result.is_err() {
            let _ = self.endpoint_sessions.signal_exact(current);
        }
        result
    }
}

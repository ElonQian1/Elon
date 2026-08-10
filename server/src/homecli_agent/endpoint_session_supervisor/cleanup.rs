use std::{sync::Arc, time::Duration};

use anyhow::Result;

use crate::{store::node_credentials::NodeEndpointSessionPermit, types::AppState};

use super::NodeEndpointSessionCurrent;

struct CleanupWork {
    state: Arc<AppState>,
    current: NodeEndpointSessionCurrent,
}

/// Cancellation-safe owner of one exact process-local endpoint session cleanup.
pub(crate) struct NodeEndpointSessionCleanup {
    state: Option<Arc<AppState>>,
    current: NodeEndpointSessionCurrent,
}

impl NodeEndpointSessionCleanup {
    pub(crate) fn new(state: &Arc<AppState>, current: NodeEndpointSessionCurrent) -> Self {
        Self {
            state: Some(Arc::clone(state)),
            current,
        }
    }

    pub(crate) fn current(&self) -> &NodeEndpointSessionCurrent {
        &self.current
    }

    pub(crate) async fn finish(mut self) -> Result<bool> {
        let state = self
            .state
            .take()
            .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_SESSION_CLEANUP_ALREADY_FINISHED"))?;
        let work = CleanupWork {
            state,
            current: self.current.clone(),
        };
        tokio::spawn(run(work)).await?
    }
}

impl Drop for NodeEndpointSessionCleanup {
    fn drop(&mut self) {
        let Some(state) = self.state.take() else {
            return;
        };
        let work = CleanupWork {
            state,
            current: self.current.clone(),
        };
        match tokio::runtime::Handle::try_current() {
            Ok(runtime) => {
                runtime.spawn(run(work));
            }
            Err(error) => {
                tracing::error!(%error, "endpoint session cleanup lost without a Tokio runtime");
            }
        }
    }
}

async fn run(work: CleanupWork) -> Result<bool> {
    let mut retry = Duration::from_secs(1);
    loop {
        match work
            .state
            .agent_manager
            .close_endpoint_session(&work.state.store, &work.current)
            .await
        {
            Ok(changed) => return Ok(changed),
            Err(error) => {
                tracing::error!(
                    %error,
                    agent_id = work.current.permit().binding().agent_id(),
                    "retrying exact endpoint session terminal close"
                );
                tokio::time::sleep(retry).await;
                retry = std::cmp::min(retry.saturating_mul(2), Duration::from_secs(30));
            }
        }
    }
}

pub(super) fn spawn_detached_terminal_retry(
    state: &Arc<AppState>,
    permit: NodeEndpointSessionPermit,
) {
    let state = Arc::clone(state);
    tokio::spawn(async move {
        let mut retry = Duration::from_secs(1);
        loop {
            let result = state
                .agent_manager
                .with_endpoint_authority_write_fence(|| {
                    state.store.terminal_close_node_endpoint_session(&permit)
                })
                .await;
            match result {
                Ok(_) => return,
                Err(error) => {
                    tracing::error!(
                        %error,
                        agent_id = permit.binding().agent_id(),
                        "retrying detached endpoint session terminal close"
                    );
                    tokio::time::sleep(retry).await;
                    retry = std::cmp::min(retry.saturating_mul(2), Duration::from_secs(30));
                }
            }
        }
    });
}

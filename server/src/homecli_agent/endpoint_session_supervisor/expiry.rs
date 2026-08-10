use std::time::Duration;

use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::{homecli_agent::AgentManager, store::Store};

use super::{EndpointSessionSupervisor, NodeEndpointSessionCurrent};

impl EndpointSessionSupervisor {
    fn next_expiry(&self) -> Result<Option<DateTime<Utc>>> {
        Ok(self
            .lock()?
            .values()
            .map(|entry| entry.current.permit().expires_at())
            .min())
    }

    fn due(&self, now: DateTime<Utc>) -> Result<Vec<NodeEndpointSessionCurrent>> {
        Ok(self
            .lock()?
            .values()
            .filter(|entry| entry.current.permit().expires_at() <= now)
            .map(|entry| entry.current.clone())
            .collect())
    }
}

impl AgentManager {
    pub(crate) async fn supervise_endpoint_session_expiry(&self, store: &Store) {
        loop {
            let changed = self.endpoint_sessions.changed.notified();
            match self.endpoint_sessions.next_expiry() {
                Ok(Some(expires_at)) => {
                    let delay = expires_at
                        .signed_duration_since(Utc::now())
                        .to_std()
                        .unwrap_or_default();
                    tokio::select! {
                        _ = tokio::time::sleep(delay) => {}
                        _ = changed => continue,
                    }
                }
                Ok(None) => {
                    changed.await;
                    continue;
                }
                Err(error) => {
                    tracing::error!(%error, "endpoint session supervisor read failed");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            }
            let due = match self.endpoint_sessions.due(Utc::now()) {
                Ok(due) => due,
                Err(error) => {
                    tracing::error!(%error, "endpoint session expiry scan failed");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };
            for current in due {
                if let Err(error) = self.expire_endpoint_session(store, &current).await {
                    tracing::error!(
                        %error,
                        agent_id = current.permit().binding().agent_id(),
                        "endpoint session expiry failed"
                    );
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }
}

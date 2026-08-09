use std::sync::atomic::Ordering;

use tokio::sync::watch;

use super::NodeRuntime;
use crate::node_agent_config::{load_persisted, save_persisted, Credentials};

impl NodeRuntime {
    pub(crate) async fn creds(&self) -> Option<Credentials> {
        self.creds.read().await.clone()
    }

    /// Returns credentials and the exact process-local epoch under the same read lock. A caller
    /// must carry the epoch for the whole cloud session; a later replacement is observed through
    /// the watch channel even when an ordinary `Notify` wakeup happened before a select began.
    pub(crate) async fn credential_session(&self) -> (Option<Credentials>, u64) {
        let credentials = self.creds.read().await;
        let epoch = self.credential_epoch.load(Ordering::Acquire);
        (credentials.clone(), epoch)
    }

    pub(crate) fn subscribe_credential_epoch(&self) -> watch::Receiver<u64> {
        self.credential_epoch_tx.subscribe()
    }

    pub(crate) fn require_credential_epoch(&self, expected_epoch: u64) -> anyhow::Result<()> {
        if expected_epoch == 0 || self.credential_epoch.load(Ordering::Acquire) != expected_epoch {
            anyhow::bail!("NODE_AGENT_CREDENTIAL_SESSION_STALE");
        }
        Ok(())
    }

    /// Holds the credential read lock across one synchronous control-plane handler. Credential
    /// replacement takes the write lock before revoking Bootstrap state and advancing the epoch,
    /// so an old sharing/preparation/planning request cannot race between a scalar check and ACK.
    pub(crate) async fn with_current_credential_session<T>(
        &self,
        expected_epoch: u64,
        expected: &Credentials,
        operation: impl FnOnce() -> T,
    ) -> anyhow::Result<T> {
        let current = self.creds.read().await;
        if self.credential_epoch.load(Ordering::Acquire) != expected_epoch
            || current
                .as_ref()
                .is_none_or(|credentials| !same_credentials(credentials, expected))
        {
            anyhow::bail!("NODE_AGENT_CREDENTIAL_SESSION_STALE");
        }
        Ok(operation())
    }

    pub(crate) async fn user_token(&self) -> Option<String> {
        self.creds
            .read()
            .await
            .as_ref()
            .and_then(|creds| creds.user_token.clone())
    }

    pub(crate) async fn set_creds(&self, next: Option<Credentials>) -> anyhow::Result<()> {
        let _transition = self.persisted_state_transition.lock().await;
        let mut current = self.creds.write().await;
        if same_optional_credentials(current.as_ref(), next.as_ref()) {
            return Ok(());
        }
        let old_epoch = self.credential_epoch.load(Ordering::Acquire);
        let next_epoch = old_epoch
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("NODE_AGENT_CREDENTIAL_SESSION_EPOCH_EXHAUSTED"))?;

        let mut persisted = load_persisted()?;
        persisted.set_install_id(&self.install_id);
        persisted.set_credentials(next.as_ref());
        save_persisted(&persisted)?;

        // The credential write lease excludes compute control handlers across the durable commit.
        // Publish every process-local authority fact without an await only after persistence
        // succeeds, so a failed save leaves the previous identity completely intact.
        self.compute_plugin_bootstrap
            .note_credentials_replaced(next.as_ref().map(|credentials| {
                (
                    credentials.agent_id.as_str(),
                    credentials.owner_user_id.as_str(),
                )
            }));
        self.credential_epoch.store(next_epoch, Ordering::Release);
        self.credential_epoch_tx.send_replace(next_epoch);
        *current = next;
        self.wake.notify_waiters();
        Ok(())
    }
}

fn same_optional_credentials(left: Option<&Credentials>, right: Option<&Credentials>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => same_credentials(left, right),
        _ => false,
    }
}

fn same_credentials(left: &Credentials, right: &Credentials) -> bool {
    left.agent_id == right.agent_id
        && left.agent_secret == right.agent_secret
        && left.owner_user_id == right.owner_user_id
        && left.user_token == right.user_token
}

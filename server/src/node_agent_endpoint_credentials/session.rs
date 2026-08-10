use std::sync::atomic::Ordering;

use anyhow::{bail, Context, Result};
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::{
    client::IntoClientRequest,
    http::{header::AUTHORIZATION, HeaderValue, Request},
};

use super::{EndpointAuthorityBinding, EndpointCredentialManager};

/// Non-secret proof that a transient WebSocket request was created from one
/// exact in-memory credential generation.
pub(crate) struct EndpointSessionLease {
    endpoint_https_origin: String,
    binding: EndpointAuthorityBinding,
    epoch: u64,
}

impl EndpointSessionLease {
    pub(crate) fn binding(&self) -> &EndpointAuthorityBinding {
        &self.binding
    }
}

impl EndpointCredentialManager {
    pub(crate) async fn endpoint_session_available(&self) -> bool {
        let state = self.state.lock().await;
        self.session_epoch.load(Ordering::SeqCst) != 0
            && !self.session_suspended.load(Ordering::SeqCst)
            && state.endpoint_required
            && state.endpoint_https_origin.is_some()
            && state.pending_mutation.is_none()
            && state.current.as_ref().is_some_and(|current| {
                current.binding.status == "active"
                    && current.binding.validate().is_ok()
                    && current.secret.is_some()
            })
    }

    pub(crate) async fn prepare_wss_request(
        &self,
        configured_origin: Option<&str>,
    ) -> Result<(Request<()>, EndpointSessionLease)> {
        let configured_origin = configured_origin
            .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_CONFIG_ORIGIN_REQUIRED"))?;
        let configured_origin = super::normalize_endpoint_https_origin(configured_origin)?;
        let state = self.state.lock().await;
        let epoch = self.session_epoch.load(Ordering::SeqCst);
        if epoch == 0 {
            bail!("NODE_ENDPOINT_SESSION_EPOCH_EXHAUSTED");
        }
        if self.session_suspended.load(Ordering::SeqCst)
            || !state.endpoint_required
            || state.pending_mutation.is_some()
        {
            bail!("NODE_ENDPOINT_SESSION_CREDENTIAL_UNAVAILABLE");
        }
        let persisted_origin = state
            .endpoint_https_origin
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_PERSISTED_ORIGIN_REQUIRED"))?;
        if persisted_origin != configured_origin {
            bail!("NODE_ENDPOINT_HTTPS_ORIGIN_DRIFT");
        }
        let current = state
            .current
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_CURRENT_CREDENTIAL_REQUIRED"))?;
        current.binding.validate()?;
        if current.binding.status != "active" {
            bail!("NODE_ENDPOINT_ACTIVE_CREDENTIAL_REQUIRED");
        }
        let secret = current
            .secret
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_SECRET_REQUIRED"))?;

        let mut websocket_url = reqwest::Url::parse(&configured_origin)
            .context("NODE_ENDPOINT_CONFIG_ORIGIN_INVALID")?;
        websocket_url
            .set_scheme("wss")
            .map_err(|_| anyhow::anyhow!("NODE_ENDPOINT_WSS_SCHEME_INVALID"))?;
        websocket_url.set_path("/agent/ws");
        websocket_url.set_query(None);
        websocket_url.set_fragment(None);
        let mut request = websocket_url.as_str().into_client_request()?;

        let mut authorization = Vec::with_capacity(7 + secret.plaintext_bytes().len());
        authorization.extend_from_slice(b"Bearer ");
        authorization.extend_from_slice(secret.plaintext_bytes());
        let authorization_header = HeaderValue::from_bytes(&authorization)
            .context("NODE_ENDPOINT_AUTHORIZATION_HEADER_INVALID");
        authorization.fill(0);
        let mut authorization_header = authorization_header?;
        authorization_header.set_sensitive(true);
        request
            .headers_mut()
            .insert(AUTHORIZATION, authorization_header);

        Ok((
            request,
            EndpointSessionLease {
                endpoint_https_origin: configured_origin,
                binding: current.binding.clone(),
                epoch,
            },
        ))
    }

    pub(crate) fn subscribe_endpoint_session_epoch(&self) -> watch::Receiver<u64> {
        self.session_epoch_tx.subscribe()
    }

    pub(crate) async fn require_current_endpoint_session(
        &self,
        lease: &EndpointSessionLease,
    ) -> Result<()> {
        let state = self.state.lock().await;
        self.require_current_endpoint_session_locked(&state, lease)
    }

    /// Runs one synchronous endpoint action while the exact credential generation remains under
    /// the manager read fence. The closure cannot await; the required lock order is credential
    /// state first, then any Bootstrap lock acquired by the closure.
    pub(crate) async fn with_current_endpoint_session_read_fence<T>(
        &self,
        lease: &EndpointSessionLease,
        operation: impl FnOnce() -> Result<T>,
    ) -> Result<T> {
        let state = self.state.lock().await;
        self.require_current_endpoint_session_locked(&state, lease)?;
        let result = operation();
        drop(state);
        result
    }

    fn require_current_endpoint_session_locked(
        &self,
        state: &super::types::EndpointCredentialState,
        lease: &EndpointSessionLease,
    ) -> Result<()> {
        if lease.epoch == 0
            || self.session_epoch.load(Ordering::SeqCst) != lease.epoch
            || self.session_suspended.load(Ordering::SeqCst)
            || !state.endpoint_required
            || state.endpoint_https_origin.as_deref() != Some(lease.endpoint_https_origin.as_str())
            || state.pending_mutation.is_some()
        {
            bail!("NODE_ENDPOINT_SESSION_LEASE_STALE");
        }
        let Some(current) = state.current.as_ref() else {
            bail!("NODE_ENDPOINT_SESSION_LEASE_STALE");
        };
        if current.secret.is_none()
            || current.binding.status != "active"
            || !current.binding.same_credential(&lease.binding)
        {
            bail!("NODE_ENDPOINT_SESSION_LEASE_STALE");
        }
        Ok(())
    }
}

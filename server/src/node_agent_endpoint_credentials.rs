//! Durable, non-downgradable NodeAgent endpoint credential authority.
//!
//! This module deliberately does not wire the WebSocket protocol. Once an
//! endpoint root is pending or present, the legacy cloud socket is gated off
//! until the separate WSS/Register protocol batch is installed.

use std::time::Duration;

use anyhow::{bail, Result};
use serde::de::DeserializeOwned;
use tokio::sync::Mutex;

mod admin;
mod bootstrap;
mod login;
mod owner_api;
mod persistence;
mod secure_store;
mod startup;
mod types;

pub(crate) use admin::{login as admin_login, logout as admin_logout};
pub(crate) use login::{bind_persisted_endpoint_origin, cloud_login, endpoint_origin_from_env};
pub(crate) use startup::{
    clear_legacy_credentials_before_startup, initial_runtime_credentials,
    load_and_bind as load_and_bind_startup, wait_if_legacy_connection_forbidden,
};
pub(crate) use types::EndpointAuthorityBinding;

use bootstrap::BootstrapTarget;
use types::EndpointCredentialState;

pub(crate) struct EndpointCredentialManager {
    state: Mutex<EndpointCredentialState>,
}

impl EndpointCredentialManager {
    pub(crate) fn absent() -> Self {
        Self {
            state: Mutex::new(EndpointCredentialState::absent()),
        }
    }

    pub(crate) fn load_default() -> Result<Self> {
        Ok(Self {
            state: Mutex::new(persistence::load_default()?),
        })
    }

    pub(crate) async fn endpoint_required(&self) -> bool {
        self.state.lock().await.endpoint_required
    }

    pub(crate) async fn endpoint_https_origin(&self) -> Option<String> {
        self.state.lock().await.endpoint_https_origin.clone()
    }

    /// Persist the permanent no-downgrade tombstone before secure
    /// registration can create or renew any server-side legacy anchor.
    pub(crate) async fn arm_endpoint_required(&self, origin: &str) -> Result<()> {
        secure_store::require_available()?;
        let origin = normalize_endpoint_https_origin(origin)?;
        let mut state = self.state.lock().await;
        if state
            .endpoint_https_origin
            .as_deref()
            .is_some_and(|current| current != origin)
        {
            bail!("NODE_ENDPOINT_HTTPS_ORIGIN_DRIFT");
        }
        persistence::save_parts(
            true,
            Some(&origin),
            state
                .current
                .as_ref()
                .map(|current| (&current.binding, current.secret.as_ref())),
            state.pending_mutation.as_ref(),
        )?;
        state.endpoint_required = true;
        state.endpoint_https_origin = Some(origin);
        Ok(())
    }

    pub(crate) async fn bootstrap_after_legacy_registration(
        &self,
        origin: &str,
        bearer: &str,
        password: &str,
        agent_id: &str,
        owner_user_id: &str,
        install_id: &str,
    ) -> Result<EndpointAuthorityBinding> {
        let mut state = self.state.lock().await;
        bootstrap::bootstrap(
            &mut state,
            origin,
            bearer,
            password,
            BootstrapTarget {
                agent_id,
                owner_user_id,
                install_id,
                current_hint: None,
            },
        )
        .await
    }

    pub(crate) async fn recover_existing_authority(
        &self,
        origin: &str,
        bearer: &str,
        password: &str,
        current: EndpointAuthorityBinding,
    ) -> Result<EndpointAuthorityBinding> {
        let mut state = self.state.lock().await;
        let target_agent_id = current.agent_id.clone();
        let target_owner_user_id = current.owner_user_id.clone();
        let target_install_id = current.install_id.clone();
        bootstrap::bootstrap(
            &mut state,
            origin,
            bearer,
            password,
            BootstrapTarget {
                agent_id: &target_agent_id,
                owner_user_id: &target_owner_user_id,
                install_id: &target_install_id,
                current_hint: Some(current),
            },
        )
        .await
    }

    /// Logout removes the local plaintext capability but preserves the
    /// endpoint-required tombstone and exact current binding forever.
    pub(crate) async fn clear_secret_for_logout(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        if !state.endpoint_required {
            return Ok(());
        }
        let origin = state
            .endpoint_https_origin
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_HTTPS_ORIGIN_REQUIRED"))?;
        let current_binding = state.current.as_ref().map(|current| &current.binding);
        persistence::save_parts(
            true,
            Some(origin),
            current_binding.map(|binding| (binding, None)),
            state.pending_mutation.as_ref(),
        )?;
        if let Some(current) = state.current.as_mut() {
            current.secret = None;
        }
        Ok(())
    }
}

pub(crate) fn normalize_endpoint_https_origin(raw: &str) -> Result<String> {
    if raw.is_empty() || raw != raw.trim() {
        bail!("NODE_ENDPOINT_HTTPS_ORIGIN_INVALID: origin 不能为空或包含首尾空白");
    }
    let parsed = reqwest::Url::parse(raw)
        .map_err(|error| anyhow::anyhow!("NODE_ENDPOINT_HTTPS_ORIGIN_INVALID: {error}"))?;
    if parsed.scheme() != "https"
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.path() != "/"
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        bail!("NODE_ENDPOINT_HTTPS_ORIGIN_INVALID: 必须是无凭据、路径、查询和片段的 https origin");
    }
    Ok(parsed.origin().ascii_serialization())
}

pub(crate) fn secure_https_client(timeout: Duration) -> Result<reqwest::Client> {
    owner_api::secure_https_client(timeout)
}

pub(crate) async fn read_https_json_limited<T: DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T> {
    owner_api::read_json_limited(response).await
}

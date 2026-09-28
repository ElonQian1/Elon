//! Online authority for independently registered project workspaces.
//! Never treats a caller-supplied root id or a local receipt as registration.
use anyhow::{bail, Context, Result};
use std::{path::Path, time::Duration};

mod validation;
pub(super) use validation::RegisteredProjectIdentity;
pub(super) use validation::{bind_runtime, RegisteredRuntime};

pub(super) async fn resolve(root: &Path) -> Result<RegisteredProjectIdentity> {
    let state = crate::node_agent_config::load_persisted()
        .context("RUNTIME_BINDING_REGISTRY_UNAVAILABLE: node credentials unavailable")?;
    let node_id = state
        .agent_id
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .context("RUNTIME_BINDING_REGISTRY_UNAVAILABLE: node is not registered")?;
    let owner_id = state
        .owner_user_id
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .context("RUNTIME_BINDING_REGISTRY_UNAVAILABLE: node owner is missing")?;
    let token = state
        .user_token
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .context("RUNTIME_BINDING_REGISTRY_UNAVAILABLE: login is required")?;
    let config = crate::node_agent_config::NodeConfig::from_env()?;
    let base = reqwest::Url::parse(&config.cloud_http_url)?;
    if !matches!(base.scheme(), "http" | "https")
        || !base.username().is_empty()
        || base.password().is_some()
        || base.query().is_some()
        || base.fragment().is_some()
    {
        bail!("RUNTIME_BINDING_REGISTRY_UNAVAILABLE: invalid configured registry origin");
    }
    // Same account-scoped authoritative API as the PC project workbench. Do not
    // follow a redirect or read project-provided registry URLs/credentials.
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .build()?;
    let mut response = client
        .get(base.join("/api/me/projects")?)
        .bearer_auth(token)
        .query(&[("node_id", node_id), ("include_system", "false")])
        .send()
        .await
        .map_err(|_| {
            anyhow::anyhow!("RUNTIME_BINDING_REGISTRY_UNAVAILABLE: registry request failed")
        })?;
    if !response.status().is_success() {
        bail!(
            "RUNTIME_BINDING_REGISTRY_UNAVAILABLE: registry status {}",
            response.status().as_u16()
        );
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .context("RUNTIME_BINDING_REGISTRY_UNAVAILABLE: registry response failed")?
    {
        if bytes.len() + chunk.len() > 2 * 1024 * 1024 {
            bail!("RUNTIME_BINDING_REGISTRY_INVALID: response exceeds limit");
        }
        bytes.extend_from_slice(&chunk);
    }
    let payload = serde_json::from_slice(&bytes)
        .context("RUNTIME_BINDING_REGISTRY_INVALID: response is not JSON")?;
    validation::select(
        root,
        node_id,
        owner_id,
        &base.origin().ascii_serialization(),
        &payload,
    )
}

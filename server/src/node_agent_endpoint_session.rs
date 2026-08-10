use std::{sync::Arc, time::Duration};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use futures::{SinkExt, StreamExt};
use homecli_proto::{
    NodeEndpointSessionAcceptedV2, NodeEndpointSessionAcceptedV2Fields,
    NodeEndpointSessionRegisterV2, NodeEndpointSessionRegisterV2Fields,
    CAP_NODE_ENDPOINT_PLANNING_SNAPSHOT_BOOTSTRAP_V1, NODE_ENDPOINT_SESSION_MAX_LIFETIME_MS,
    NODE_ENDPOINT_SESSION_RENEWAL_MARGIN_MS, NODE_ENDPOINT_SESSION_V2_CAPABILITIES,
    NODE_ENDPOINT_SESSION_V2_CAPABILITY_SET_DIGEST,
};
use sha2::{Digest, Sha256};
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::warn;

use crate::{
    node_agent_endpoint_credentials::{EndpointAuthorityBinding, EndpointSessionLease},
    NodeRuntime,
};

mod bootstrap;

const BASE_RECONNECT_BACKOFF: Duration = Duration::from_secs(2);
const MAX_RECONNECT_BACKOFF: Duration = Duration::from_secs(60);
const ACCEPTED_TIMEOUT: Duration = Duration::from_secs(10);
const ENDPOINT_CLOSE_TIMEOUT: Duration = Duration::from_secs(1);
const PLANNING_BOOTSTRAP_CHAIN_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_ACCEPTED_MESSAGE_BYTES: usize = 64 * 1024;
const MAX_ACCEPTED_CLOCK_SKEW_MS: i64 = 2 * 60 * 1_000;
const INSTALLATION_BINDING_DOMAIN: &[u8] = b"ELON_NODE_ENDPOINT_INSTALLATION_BINDING_V1";

pub(super) type EndpointWebSocket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

pub(super) enum EndpointSessionEnd {
    RemoteClose,
    RefreshRequired,
}

/// Linear proof that one strict Accepted V2 was checked against this exact local credential lease.
/// Only this endpoint adapter can construct it; the compute Bootstrap host may only consume it.
pub(crate) struct ValidatedEndpointSessionProvenance<'lease> {
    _lease: &'lease EndpointSessionLease,
    accepted: NodeEndpointSessionAcceptedV2Fields,
}

impl<'lease> ValidatedEndpointSessionProvenance<'lease> {
    fn seal(
        lease: &'lease EndpointSessionLease,
        accepted: NodeEndpointSessionAcceptedV2Fields,
    ) -> Self {
        Self {
            _lease: lease,
            accepted,
        }
    }

    pub(crate) fn into_accepted_fields(self) -> NodeEndpointSessionAcceptedV2Fields {
        self.accepted
    }
}

pub(crate) async fn run_if_required(runtime: &Arc<NodeRuntime>, backoff: &mut Duration) -> bool {
    if runtime.cfg.endpoint_https_origin.is_none()
        && !runtime.endpoint_credentials.endpoint_required().await
    {
        return false;
    }
    if !runtime
        .endpoint_credentials
        .endpoint_session_available()
        .await
    {
        runtime
            .set_connection_stage("endpoint_waiting_credentials")
            .await;
        runtime
            .set_connected(
                false,
                "安全 endpoint 模式已固定；等待账号与密码完成凭据恢复",
            )
            .await;
        *backoff = BASE_RECONNECT_BACKOFF;
        let _ = tokio::time::timeout(BASE_RECONNECT_BACKOFF, runtime.wake.notified()).await;
        return true;
    }

    runtime.begin_connection_attempt().await;
    runtime
        .set_connection_stage("endpoint_session_connect")
        .await;
    runtime
        .set_connected(false, "正在建立安全 endpoint 会话；算力能力保持关闭")
        .await;
    match run_endpoint_session(runtime).await {
        Ok(EndpointSessionEnd::RefreshRequired) => {
            runtime
                .set_connected(false, "安全 endpoint 会话正在刷新；算力能力保持关闭")
                .await;
            *backoff = BASE_RECONNECT_BACKOFF;
            return true;
        }
        Ok(EndpointSessionEnd::RemoteClose) => {
            runtime
                .set_connected(false, "安全 endpoint 会话已关闭，等待重连")
                .await;
        }
        Err(error) => {
            warn!(
                "安全 endpoint 会话错误: {error:#}，{:.1}s 后重连",
                backoff.as_secs_f32()
            );
            runtime
                .set_connected(false, &format!("安全 endpoint 会话错误: {error}"))
                .await;
            if !runtime
                .endpoint_credentials
                .endpoint_session_available()
                .await
            {
                *backoff = BASE_RECONNECT_BACKOFF;
                return true;
            }
        }
    }
    runtime.set_connection_backoff(*backoff).await;
    let was_woken = tokio::select! {
        _ = tokio::time::sleep(*backoff) => false,
        _ = runtime.wake.notified() => true,
    };
    if was_woken {
        *backoff = BASE_RECONNECT_BACKOFF;
    } else {
        *backoff = (*backoff * 2).min(MAX_RECONNECT_BACKOFF);
    }
    true
}

async fn run_endpoint_session(runtime: &Arc<NodeRuntime>) -> Result<EndpointSessionEnd> {
    let Some((mut websocket, lease)) =
        crate::node_agent_cloud_connection::connect_endpoint(runtime).await?
    else {
        return Ok(EndpointSessionEnd::RefreshRequired);
    };
    let result = run_connected_session(runtime, &mut websocket, &lease).await;
    let _ = tokio::time::timeout(ENDPOINT_CLOSE_TIMEOUT, websocket.close(None)).await;
    result
}

async fn run_connected_session(
    runtime: &Arc<NodeRuntime>,
    websocket: &mut EndpointWebSocket,
    lease: &EndpointSessionLease,
) -> Result<EndpointSessionEnd> {
    let mut epoch = runtime
        .endpoint_credentials
        .subscribe_endpoint_session_epoch();
    if !lease_is_current(runtime, lease).await {
        return Ok(EndpointSessionEnd::RefreshRequired);
    }

    let agent_version = crate::node_agent_release_identity::current();
    let binding = lease.binding();
    let register = NodeEndpointSessionRegisterV2::new(NodeEndpointSessionRegisterV2Fields {
        agent_id: binding.agent_id.clone(),
        owner_user_id: binding.owner_user_id.clone(),
        install_id: binding.install_id.clone(),
        credential_id: binding.credential_id.clone(),
        credential_revision: binding.credential_revision,
        credential_digest: binding.credential_digest.clone(),
        agent_version: agent_version.clone(),
        capabilities: NODE_ENDPOINT_SESSION_V2_CAPABILITIES
            .into_iter()
            .map(str::to_string)
            .collect(),
    })
    .map_err(anyhow::Error::msg)?;
    let register_json = serde_json::to_string(&register)?;
    tokio::time::timeout(
        ACCEPTED_TIMEOUT,
        websocket.send(Message::Text(register_json)),
    )
    .await
    .context("NODE_ENDPOINT_SESSION_REGISTER_SEND_TIMEOUT")??;
    if !lease_is_current(runtime, lease).await {
        return Ok(EndpointSessionEnd::RefreshRequired);
    }
    runtime
        .set_connection_stage("endpoint_session_acceptance")
        .await;

    let first_frame = tokio::select! {
        changed = epoch.changed() => {
            changed.context("NODE_ENDPOINT_SESSION_EPOCH_CLOSED")?;
            return Ok(EndpointSessionEnd::RefreshRequired);
        }
        frame = tokio::time::timeout(ACCEPTED_TIMEOUT, websocket.next()) => {
            frame
                .context("NODE_ENDPOINT_SESSION_ACCEPTED_TIMEOUT")?
                .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_SESSION_CLOSED_BEFORE_ACCEPTED"))??
        }
    };
    let Message::Text(text) = first_frame else {
        bail!("NODE_ENDPOINT_SESSION_FIRST_FRAME_NOT_ACCEPTED_TEXT");
    };
    if text.len() > MAX_ACCEPTED_MESSAGE_BYTES {
        bail!("NODE_ENDPOINT_SESSION_ACCEPTED_TOO_LARGE");
    }
    let chain_started_at = tokio::time::Instant::now();
    let accepted: NodeEndpointSessionAcceptedV2 =
        serde_json::from_str(&text).context("NODE_ENDPOINT_SESSION_ACCEPTED_INVALID")?;

    let (renewal_after, accepted) = validate_accepted(accepted, binding, &agent_version)?;
    if !lease_is_current(runtime, lease).await {
        return Ok(EndpointSessionEnd::RefreshRequired);
    }
    let provenance = ValidatedEndpointSessionProvenance::seal(lease, accepted);
    let chain_deadline = chain_started_at + PLANNING_BOOTSTRAP_CHAIN_TIMEOUT;
    let witness = tokio::time::timeout_at(
        chain_deadline,
        runtime
            .endpoint_credentials
            .with_current_endpoint_session_read_fence(lease, || {
                runtime
                    .compute_plugin_bootstrap
                    .bind_endpoint_session_provenance(provenance)
            }),
    )
    .await
    .context("NODE_ENDPOINT_PLANNING_BOOTSTRAP_CHAIN_TIMEOUT")??;
    runtime
        .set_connection_stage("endpoint_session_planning_bootstrap_blocked")
        .await;
    runtime
        .set_connected(false, "安全 endpoint 规划引导已认证；算力能力保持关闭")
        .await;
    let result = bootstrap::run(
        runtime,
        websocket,
        lease,
        &witness,
        &mut epoch,
        chain_started_at,
        renewal_after,
    )
    .await;
    runtime
        .compute_plugin_bootstrap
        .release_endpoint_session_provenance(witness);
    result
}

async fn lease_is_current(runtime: &Arc<NodeRuntime>, lease: &EndpointSessionLease) -> bool {
    runtime
        .endpoint_credentials
        .require_current_endpoint_session(lease)
        .await
        .is_ok()
}

fn validate_accepted(
    accepted: NodeEndpointSessionAcceptedV2,
    binding: &EndpointAuthorityBinding,
    agent_version: &str,
) -> Result<(Duration, NodeEndpointSessionAcceptedV2Fields)> {
    let accepted = accepted.into_fields().map_err(anyhow::Error::msg)?;
    if accepted.agent_id != binding.agent_id
        || accepted.owner_user_id != binding.owner_user_id
        || accepted.install_id != binding.install_id
        || accepted.credential_id != binding.credential_id
        || accepted.credential_revision != binding.credential_revision
        || accepted.credential_digest != binding.credential_digest
        || accepted.agent_version != agent_version
    {
        bail!("NODE_ENDPOINT_SESSION_ACCEPTED_IDENTITY_MISMATCH");
    }
    if accepted.installation_binding_digest
        != installation_binding_digest(
            &binding.agent_id,
            &binding.owner_user_id,
            &binding.install_id,
        )?
    {
        bail!("NODE_ENDPOINT_SESSION_ACCEPTED_INSTALLATION_BINDING_MISMATCH");
    }
    if accepted.capabilities.len() != 1
        || accepted.capabilities[0] != CAP_NODE_ENDPOINT_PLANNING_SNAPSHOT_BOOTSTRAP_V1
        || accepted.capability_set_digest != NODE_ENDPOINT_SESSION_V2_CAPABILITY_SET_DIGEST
    {
        bail!("NODE_ENDPOINT_SESSION_ACCEPTED_CAPABILITY_SET_MISMATCH");
    }

    let authenticated_at = DateTime::parse_from_rfc3339(&accepted.authenticated_at)
        .context("NODE_ENDPOINT_SESSION_AUTHENTICATED_AT_INVALID")?
        .with_timezone(&Utc);
    let expires_at = DateTime::parse_from_rfc3339(&accepted.expires_at)
        .context("NODE_ENDPOINT_SESSION_EXPIRES_AT_INVALID")?
        .with_timezone(&Utc);
    let lifetime_ms = expires_at
        .signed_duration_since(authenticated_at)
        .num_milliseconds();
    if lifetime_ms != NODE_ENDPOINT_SESSION_MAX_LIFETIME_MS as i64
        || accepted.expires_in_ms > NODE_ENDPOINT_SESSION_MAX_LIFETIME_MS
        || accepted.expires_in_ms <= NODE_ENDPOINT_SESSION_RENEWAL_MARGIN_MS
    {
        bail!("NODE_ENDPOINT_SESSION_ACCEPTED_EXPIRY_INVALID");
    }
    let now = Utc::now();
    let accepted_clock_delta_ms = now
        .signed_duration_since(authenticated_at)
        .num_milliseconds();
    if !(-MAX_ACCEPTED_CLOCK_SKEW_MS..=MAX_ACCEPTED_CLOCK_SKEW_MS)
        .contains(&accepted_clock_delta_ms)
    {
        bail!("NODE_ENDPOINT_SESSION_ACCEPTED_NOT_FRESH");
    }
    let wall_remaining = expires_at
        .signed_duration_since(now)
        .to_std()
        .context("NODE_ENDPOINT_SESSION_ACCEPTED_EXPIRED")?;
    let declared_remaining = Duration::from_millis(accepted.expires_in_ms);
    let renewal_after = wall_remaining
        .min(declared_remaining)
        .checked_sub(Duration::from_millis(
            NODE_ENDPOINT_SESSION_RENEWAL_MARGIN_MS,
        ))
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_SESSION_ACCEPTED_RENEWAL_EXPIRED"))?;
    Ok((renewal_after, accepted))
}

fn installation_binding_digest(
    agent_id: &str,
    owner_user_id: &str,
    install_id: &str,
) -> Result<String> {
    let agent_id = serde_json::to_string(agent_id)?;
    let install_id = serde_json::to_string(install_id)?;
    let owner_user_id = serde_json::to_string(owner_user_id)?;
    let canonical_json = format!(
        "{{\"agent_id\":{agent_id},\"install_id\":{install_id},\"owner_user_id\":{owner_user_id}}}"
    );
    Ok(domain_digest(
        INSTALLATION_BINDING_DOMAIN,
        canonical_json.as_bytes(),
    ))
}

fn domain_digest(domain: &[u8], canonical_json: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(canonical_json);
    hex::encode(digest.finalize())
}

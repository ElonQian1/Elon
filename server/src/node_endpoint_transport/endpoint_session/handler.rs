use std::{sync::Arc, time::Duration};

use anyhow::{bail, Context, Result};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Extension, OriginalUri, State,
    },
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::{SecondsFormat, Utc};
use homecli_proto::{
    NodeEndpointSessionAcceptedV1, NodeEndpointSessionAcceptedV1Fields,
    NodeEndpointSessionRegisterV1, NODE_ENDPOINT_SESSION_PROTO_VERSION,
};
use tokio::time::{interval, timeout, Instant, MissedTickBehavior};
use uuid::Uuid;

use crate::{
    homecli_agent::{NodeEndpointSessionCleanup, NodeEndpointSessionCurrent},
    node_compute_sharing::endpoint_authority::{
        bind_direct_tls_node_endpoint_transport, NodeEndpointSessionOpenRequest,
        VerifiedSecureNodeEndpointTransport,
    },
    store::node_credentials::NodeEndpointSessionPermit,
    types::AppState,
};

use super::rate_limit;
use crate::node_endpoint_transport::{
    direct_tls::DirectTlsPeerAddress, evidence_slot::VerifiedSecureTransportSlot,
};

const SESSION_PATH: &str = "/agent/ws";
const FIRST_FRAME_TIMEOUT: Duration = Duration::from_secs(10);
const CURRENTNESS_INTERVAL: Duration = Duration::from_secs(5);
const MAX_MESSAGE_BYTES: usize = 64 * 1024;

pub(in crate::node_endpoint_transport) async fn session_ws(
    State(state): State<Arc<AppState>>,
    Extension(slot): Extension<VerifiedSecureTransportSlot>,
    Extension(peer): Extension<DirectTlsPeerAddress>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    if let Err(retry_after_seconds) = rate_limit::check_peer(peer) {
        return reject_with_retry(
            StatusCode::TOO_MANY_REQUESTS,
            "NODE_ENDPOINT_SESSION_RATE_LIMITED",
            retry_after_seconds,
        );
    }
    if uri.path() != SESSION_PATH || uri.query().is_some() || forbidden_headers(&headers) {
        return reject(
            StatusCode::BAD_REQUEST,
            "NODE_ENDPOINT_SESSION_REQUEST_INVALID",
        );
    }
    let presented_secret = match endpoint_bearer(&headers) {
        Some(secret) => secret,
        None => {
            return reject(
                StatusCode::UNAUTHORIZED,
                "NODE_ENDPOINT_SESSION_CREDENTIAL_REQUIRED",
            )
        }
    };
    let transport = match slot
        .take()
        .and_then(bind_direct_tls_node_endpoint_transport)
    {
        Ok(transport) => transport,
        Err(_) => {
            return reject(
                StatusCode::UNAUTHORIZED,
                "NODE_ENDPOINT_SESSION_TRANSPORT_INVALID",
            )
        }
    };

    let mut response = ws
        .max_frame_size(MAX_MESSAGE_BYTES)
        .max_message_size(MAX_MESSAGE_BYTES)
        .on_upgrade(move |socket| run_socket(socket, state, transport, presented_secret));
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

async fn run_socket(
    mut socket: WebSocket,
    state: Arc<AppState>,
    transport: VerifiedSecureNodeEndpointTransport,
    presented_secret: String,
) {
    let result = run_authenticated_socket(&mut socket, &state, transport, presented_secret).await;
    if let Err(error) = result {
        tracing::info!(%error, "compute-inert endpoint session closed");
    }
    let _ = socket.close().await;
}

async fn run_authenticated_socket(
    socket: &mut WebSocket,
    state: &Arc<AppState>,
    transport: VerifiedSecureNodeEndpointTransport,
    presented_secret: String,
) -> Result<()> {
    let fields = read_register(socket)
        .await?
        .into_fields()
        .map_err(anyhow::Error::msg)?;
    let agent_version = fields.agent_version.clone();
    let session_id = format!("nes_{}", Uuid::new_v4().simple());
    let server_instance_id = transport.server_instance_id().to_string();
    let request = NodeEndpointSessionOpenRequest::new(
        fields.agent_id,
        fields.owner_user_id,
        fields.install_id,
        fields.credential_id,
        fields.credential_revision,
        fields.credential_digest,
        session_id,
        server_instance_id,
        u64::from(NODE_ENDPOINT_SESSION_PROTO_VERSION),
        agent_version.clone(),
        presented_secret,
    )?;
    let lease = state
        .agent_manager
        .authenticate_and_install_endpoint_session(&state.store, &request, &transport)
        .await?;
    let (current, mut shutdown) = lease.into_parts();
    let cleanup = NodeEndpointSessionCleanup::new(state, current);

    let result = serve_current_session(
        socket,
        state,
        cleanup.current(),
        &mut shutdown,
        agent_version,
    )
    .await;
    // Stop network use before waiting for durable terminalization. The cleanup guard remains
    // armed across this await and will continue in a detached task if the handler is cancelled.
    let _ = socket.send(Message::Close(None)).await;
    let agent_id = cleanup.current().permit().binding().agent_id().to_string();
    if let Err(error) = cleanup.finish().await {
        tracing::warn!(
            %error,
            agent_id,
            "endpoint session terminal close failed"
        );
    }
    result
}

async fn read_register(socket: &mut WebSocket) -> Result<NodeEndpointSessionRegisterV1> {
    let frame = timeout(FIRST_FRAME_TIMEOUT, socket.recv())
        .await
        .context("NODE_ENDPOINT_SESSION_REGISTER_TIMEOUT")?
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_SESSION_REGISTER_MISSING"))??;
    let Message::Text(text) = frame else {
        bail!("NODE_ENDPOINT_SESSION_REGISTER_FRAME_INVALID");
    };
    if text.len() > MAX_MESSAGE_BYTES {
        bail!("NODE_ENDPOINT_SESSION_REGISTER_TOO_LARGE");
    }
    Ok(serde_json::from_str(&text)?)
}

async fn serve_current_session(
    socket: &mut WebSocket,
    state: &Arc<AppState>,
    current: &NodeEndpointSessionCurrent,
    shutdown: &mut tokio::sync::watch::Receiver<bool>,
    agent_version: String,
) -> Result<()> {
    if *shutdown.borrow() {
        bail!("NODE_ENDPOINT_SESSION_SUPERSEDED_BEFORE_ACCEPTED");
    }
    state
        .agent_manager
        .inspect_endpoint_session(&state.store, current)
        .await?;
    let permit = current.permit();
    let remaining = remaining_lifetime(permit)?;
    let expires = Instant::now() + remaining;
    let accepted = accepted_message(permit, agent_version, remaining)?;
    socket
        .send(Message::Text(serde_json::to_string(&accepted)?))
        .await?;

    let mut currentness = interval(CURRENTNESS_INTERVAL);
    currentness.set_missed_tick_behavior(MissedTickBehavior::Delay);
    currentness.tick().await;
    let mut ping_nonce = 0_u64;
    let mut awaiting_pong: Option<Vec<u8>> = None;

    loop {
        tokio::select! {
            changed = shutdown.changed() => {
                changed.context("NODE_ENDPOINT_SESSION_SUPERVISOR_CLOSED")?;
                bail!("NODE_ENDPOINT_SESSION_SUPERSEDED");
            }
            _ = tokio::time::sleep_until(expires) => {
                state.agent_manager.expire_endpoint_session(&state.store, current).await?;
                return Ok(());
            }
            _ = currentness.tick() => {
                if awaiting_pong.is_some() {
                    bail!("NODE_ENDPOINT_SESSION_PONG_TIMEOUT");
                }
                state.agent_manager.inspect_endpoint_session(&state.store, current).await?;
                ping_nonce = ping_nonce.checked_add(1).context("NODE_ENDPOINT_SESSION_PING_EXHAUSTED")?;
                let payload = ping_nonce.to_be_bytes().to_vec();
                socket.send(Message::Ping(payload.clone())).await?;
                awaiting_pong = Some(payload);
            }
            frame = socket.recv() => match frame {
                Some(Ok(Message::Pong(payload))) if awaiting_pong.as_ref() == Some(&payload) => {
                    awaiting_pong = None;
                }
                Some(Ok(Message::Close(_))) | None => return Ok(()),
                Some(Ok(_)) => bail!("NODE_ENDPOINT_SESSION_FRAME_FORBIDDEN"),
                Some(Err(error)) => return Err(error.into()),
            }
        }
    }
}

fn accepted_message(
    permit: &NodeEndpointSessionPermit,
    agent_version: String,
    remaining: Duration,
) -> Result<NodeEndpointSessionAcceptedV1> {
    let binding = permit.binding();
    let expires_in_ms = u64::try_from(remaining.as_millis())?;
    NodeEndpointSessionAcceptedV1::new(NodeEndpointSessionAcceptedV1Fields {
        agent_id: binding.agent_id().to_string(),
        owner_user_id: permit.owner_user_id().to_string(),
        install_id: permit.install_id().to_string(),
        credential_id: binding.credential_id().to_string(),
        credential_revision: binding.credential_revision(),
        credential_digest: binding.credential_digest().to_string(),
        installation_binding_digest: permit.installation_binding_digest().to_string(),
        agent_version,
        session_id: binding.session_id().to_string(),
        session_generation: binding.session_generation(),
        authentication_receipt_id: binding.authentication_receipt_id().to_string(),
        authentication_digest: binding.authentication_digest().to_string(),
        server_instance_id: binding.server_instance_id().to_string(),
        capability_set_digest: permit.capability_set_digest().to_string(),
        authenticated_at: permit
            .authenticated_at()
            .to_rfc3339_opts(SecondsFormat::Nanos, true),
        expires_at: permit
            .expires_at()
            .to_rfc3339_opts(SecondsFormat::Nanos, true),
        expires_in_ms,
    })
    .map_err(anyhow::Error::msg)
}

fn remaining_lifetime(permit: &NodeEndpointSessionPermit) -> Result<Duration> {
    let wall_remaining = permit
        .expires_at()
        .signed_duration_since(Utc::now())
        .to_std()
        .context("NODE_ENDPOINT_SESSION_ALREADY_EXPIRED")?;
    let sealed_lifetime = permit
        .expires_at()
        .signed_duration_since(permit.authenticated_at())
        .to_std()
        .context("NODE_ENDPOINT_SESSION_LIFETIME_INVALID")?;
    Ok(std::cmp::min(wall_remaining, sealed_lifetime))
}

fn endpoint_bearer(headers: &HeaderMap) -> Option<String> {
    let mut values = headers.get_all(header::AUTHORIZATION).iter();
    let value = values.next()?;
    if values.next().is_some() {
        return None;
    }
    let value = value.to_str().ok()?;
    let secret = value.strip_prefix("Bearer ")?;
    if secret.len() != 43 || secret.trim() != secret || !secret.is_ascii() {
        return None;
    }
    Some(secret.to_string())
}

fn forbidden_headers(headers: &HeaderMap) -> bool {
    [
        header::COOKIE.as_str(),
        header::ORIGIN.as_str(),
        header::SEC_WEBSOCKET_PROTOCOL.as_str(),
        "forwarded",
        "x-forwarded-for",
        "x-forwarded-host",
        "x-forwarded-proto",
        "x-real-ip",
    ]
    .iter()
    .any(|name| headers.contains_key(*name))
}

fn reject(status: StatusCode, code: &'static str) -> Response {
    let mut response = (status, code).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn reject_with_retry(status: StatusCode, code: &'static str, retry_after: u64) -> Response {
    let mut response = reject(status, code);
    if let Ok(value) = HeaderValue::from_str(&retry_after.to_string()) {
        response.headers_mut().insert(header::RETRY_AFTER, value);
    }
    response
}

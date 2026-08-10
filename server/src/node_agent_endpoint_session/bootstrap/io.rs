use std::{pin::Pin, sync::Arc, time::Duration};

use anyhow::{bail, Context, Result};
use futures::{SinkExt, StreamExt};
use tokio::{
    sync::watch,
    time::{Instant, Sleep},
};
use tokio_tungstenite::tungstenite::Message;

use crate::{
    node_agent_compute_plugin_host::ComputePluginEndpointSessionWitness,
    node_agent_endpoint_credentials::EndpointSessionLease, NodeRuntime, CLOUD_WS_READ_TIMEOUT,
};

use super::super::{EndpointSessionEnd, EndpointWebSocket, PLANNING_BOOTSTRAP_CHAIN_TIMEOUT};

const STAGE_TIMEOUT: Duration = Duration::from_secs(10);
const CONTROL_FRAME_SEND_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_BOOTSTRAP_MESSAGE_BYTES: usize = 576 * 1024;

pub(super) enum NextText {
    Message(String),
    End(EndpointSessionEnd),
}

pub(super) fn chain_deadline(started_at: Instant) -> Instant {
    started_at + PLANNING_BOOTSTRAP_CHAIN_TIMEOUT
}

pub(super) fn require_chain_live(deadline: Instant) -> Result<()> {
    if Instant::now() >= deadline {
        bail!("NODE_ENDPOINT_PLANNING_BOOTSTRAP_CHAIN_TIMEOUT");
    }
    Ok(())
}

pub(super) fn stage_deadline(chain_deadline: Instant) -> Result<Instant> {
    require_chain_live(chain_deadline)?;
    Ok((Instant::now() + STAGE_TIMEOUT).min(chain_deadline))
}

pub(super) fn require_stage_live(deadline: Instant) -> Result<()> {
    if Instant::now() >= deadline {
        bail!("NODE_ENDPOINT_PLANNING_BOOTSTRAP_STAGE_TIMEOUT");
    }
    Ok(())
}

pub(super) async fn next_request_text(
    runtime: &Arc<NodeRuntime>,
    websocket: &mut EndpointWebSocket,
    lease: &EndpointSessionLease,
    witness: &ComputePluginEndpointSessionWitness,
    epoch: &mut watch::Receiver<u64>,
    stage_deadline: Instant,
    mut renewal: Pin<&mut Sleep>,
) -> Result<NextText> {
    require_stage_live(stage_deadline)?;
    let stage_timeout = tokio::time::sleep_until(stage_deadline);
    tokio::pin!(stage_timeout);
    loop {
        let frame = tokio::select! {
            changed = epoch.changed() => {
                changed.context("NODE_ENDPOINT_SESSION_EPOCH_CLOSED")?;
                return Ok(NextText::End(EndpointSessionEnd::RefreshRequired));
            }
            _ = &mut renewal => {
                return Ok(NextText::End(EndpointSessionEnd::RefreshRequired));
            }
            _ = &mut stage_timeout => {
                bail!("NODE_ENDPOINT_PLANNING_BOOTSTRAP_STAGE_TIMEOUT");
            }
            frame = websocket.next() => {
                frame.ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_SESSION_STREAM_ENDED"))??
            }
        };
        let current = tokio::time::timeout_at(stage_deadline, is_current(runtime, lease, witness))
            .await
            .context("NODE_ENDPOINT_PLANNING_BOOTSTRAP_STAGE_TIMEOUT")?;
        require_stage_live(stage_deadline)?;
        if !current {
            return Ok(NextText::End(EndpointSessionEnd::RefreshRequired));
        }
        match frame {
            Message::Text(text) if text.len() <= MAX_BOOTSTRAP_MESSAGE_BYTES => {
                return Ok(NextText::Message(text));
            }
            Message::Text(_) => bail!("NODE_ENDPOINT_PLANNING_BOOTSTRAP_MESSAGE_TOO_LARGE"),
            Message::Ping(payload) => {
                tokio::time::timeout_at(stage_deadline, websocket.send(Message::Pong(payload)))
                    .await
                    .context("NODE_ENDPOINT_PLANNING_BOOTSTRAP_STAGE_TIMEOUT")??;
            }
            Message::Close(_) => return Ok(NextText::End(EndpointSessionEnd::RemoteClose)),
            Message::Binary(_) => bail!("NODE_ENDPOINT_SESSION_BINARY_AFTER_ACCEPTED_FORBIDDEN"),
            Message::Pong(_) => bail!("NODE_ENDPOINT_SESSION_UNSOLICITED_PONG_FORBIDDEN"),
            _ => bail!("NODE_ENDPOINT_SESSION_FRAME_AFTER_ACCEPTED_FORBIDDEN"),
        }
    }
}

pub(super) async fn keepalive(
    runtime: &Arc<NodeRuntime>,
    websocket: &mut EndpointWebSocket,
    lease: &EndpointSessionLease,
    witness: &ComputePluginEndpointSessionWitness,
    epoch: &mut watch::Receiver<u64>,
    mut renewal: Pin<&mut Sleep>,
) -> Result<EndpointSessionEnd> {
    loop {
        let frame = tokio::select! {
            changed = epoch.changed() => {
                changed.context("NODE_ENDPOINT_SESSION_EPOCH_CLOSED")?;
                return Ok(EndpointSessionEnd::RefreshRequired);
            }
            _ = &mut renewal => return Ok(EndpointSessionEnd::RefreshRequired),
            frame = tokio::time::timeout(CLOUD_WS_READ_TIMEOUT, websocket.next()) => {
                frame
                    .context("NODE_ENDPOINT_SESSION_READ_TIMEOUT")?
                    .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_SESSION_STREAM_ENDED"))??
            }
        };
        if !is_current(runtime, lease, witness).await {
            return Ok(EndpointSessionEnd::RefreshRequired);
        }
        match frame {
            Message::Ping(payload) => {
                tokio::time::timeout(
                    CONTROL_FRAME_SEND_TIMEOUT,
                    websocket.send(Message::Pong(payload)),
                )
                .await
                .context("NODE_ENDPOINT_SESSION_CONTROL_FRAME_SEND_TIMEOUT")??;
            }
            Message::Close(_) => return Ok(EndpointSessionEnd::RemoteClose),
            Message::Text(_) => {
                bail!("NODE_ENDPOINT_PLANNING_BOOTSTRAP_TEXT_AFTER_TERMINAL_FORBIDDEN")
            }
            Message::Binary(_) => bail!("NODE_ENDPOINT_SESSION_BINARY_AFTER_ACCEPTED_FORBIDDEN"),
            Message::Pong(_) => bail!("NODE_ENDPOINT_SESSION_UNSOLICITED_PONG_FORBIDDEN"),
            _ => bail!("NODE_ENDPOINT_SESSION_FRAME_AFTER_ACCEPTED_FORBIDDEN"),
        }
    }
}

pub(super) async fn require_current(
    runtime: &Arc<NodeRuntime>,
    lease: &EndpointSessionLease,
    witness: &ComputePluginEndpointSessionWitness,
) -> Result<()> {
    runtime
        .endpoint_credentials
        .require_current_endpoint_session(lease)
        .await?;
    runtime
        .compute_plugin_bootstrap
        .require_endpoint_session_provenance(witness)
}

pub(super) async fn require_current_stage(
    runtime: &Arc<NodeRuntime>,
    lease: &EndpointSessionLease,
    witness: &ComputePluginEndpointSessionWitness,
    stage_deadline: Instant,
) -> Result<()> {
    require_stage_live(stage_deadline)?;
    tokio::time::timeout_at(stage_deadline, require_current(runtime, lease, witness))
        .await
        .context("NODE_ENDPOINT_PLANNING_BOOTSTRAP_STAGE_TIMEOUT")??;
    require_stage_live(stage_deadline)
}

pub(super) async fn with_current_stage<T>(
    runtime: &Arc<NodeRuntime>,
    lease: &EndpointSessionLease,
    witness: &ComputePluginEndpointSessionWitness,
    stage_deadline: Instant,
    operation: impl FnOnce() -> T,
) -> Result<T> {
    require_stage_live(stage_deadline)?;
    let value = tokio::time::timeout_at(
        stage_deadline,
        runtime
            .endpoint_credentials
            .with_current_endpoint_session_read_fence(lease, || {
                runtime
                    .compute_plugin_bootstrap
                    .require_endpoint_session_provenance(witness)?;
                Ok(operation())
            }),
    )
    .await
    .context("NODE_ENDPOINT_PLANNING_BOOTSTRAP_STAGE_TIMEOUT")??;
    require_stage_live(stage_deadline)?;
    Ok(value)
}

pub(super) async fn send_observation<T: serde::Serialize>(
    runtime: &Arc<NodeRuntime>,
    websocket: &mut EndpointWebSocket,
    lease: &EndpointSessionLease,
    witness: &ComputePluginEndpointSessionWitness,
    stage_deadline: Instant,
    message: T,
) -> Result<()> {
    let message = serde_json::to_string(&message)?;
    if message.len() > MAX_BOOTSTRAP_MESSAGE_BYTES {
        bail!("NODE_ENDPOINT_PLANNING_BOOTSTRAP_OBSERVATION_TOO_LARGE");
    }
    require_current_stage(runtime, lease, witness, stage_deadline).await?;
    tokio::time::timeout_at(stage_deadline, websocket.send(Message::Text(message)))
        .await
        .context("NODE_ENDPOINT_PLANNING_BOOTSTRAP_STAGE_TIMEOUT")??;
    require_current_stage(runtime, lease, witness, stage_deadline).await?;
    Ok(())
}

async fn is_current(
    runtime: &Arc<NodeRuntime>,
    lease: &EndpointSessionLease,
    witness: &ComputePluginEndpointSessionWitness,
) -> bool {
    require_current(runtime, lease, witness).await.is_ok()
}

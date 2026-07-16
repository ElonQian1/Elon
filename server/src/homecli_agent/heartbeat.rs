use anyhow::{anyhow, Result};
use axum::extract::ws::Message;
use homecli_proto::ServerToAgent;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{mpsc, oneshot, watch, Mutex};
use uuid::Uuid;

use crate::ws_transport::try_json_text_message;

pub(super) const AGENT_HEARTBEAT_MAX_MISSED_ACKS: u32 = 3;

const AGENT_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);
const AGENT_HEARTBEAT_ACK_TIMEOUT: Duration = Duration::from_secs(8);

pub(super) fn spawn_agent_heartbeat(
    agent_id: String,
    control_tx: mpsc::UnboundedSender<Message>,
    ping_acks: Arc<Mutex<HashMap<String, oneshot::Sender<()>>>>,
    session_shutdown: watch::Sender<bool>,
    mut session_shutdown_rx: watch::Receiver<bool>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(AGENT_HEARTBEAT_INTERVAL);
        let mut missed_acks = 0u32;
        loop {
            tokio::select! {
                _ = session_shutdown_rx.changed() => break,
                _ = interval.tick() => {}
            }
            if let Err(error) = send_protocol_ping_control(
                &agent_id,
                &control_tx,
                &ping_acks,
                AGENT_HEARTBEAT_ACK_TIMEOUT,
            )
            .await
            {
                missed_acks += 1;
                tracing::warn!(
                    agent_id = %agent_id,
                    missed_acks,
                    max_missed_acks = AGENT_HEARTBEAT_MAX_MISSED_ACKS,
                    error = %error,
                    "agent heartbeat ack missed"
                );
                if heartbeat_should_close_session(missed_acks) {
                    tracing::warn!(agent_id = %agent_id, "agent heartbeat failed too many times");
                    let _ = session_shutdown.send(true);
                    break;
                }
                continue;
            }
            missed_acks = 0;
            tracing::trace!(agent_id = %agent_id, "agent ping sent");
        }
    });
}

pub(super) async fn send_protocol_ping_control(
    agent_id: &str,
    control_tx: &mpsc::UnboundedSender<Message>,
    ping_acks: &Arc<Mutex<HashMap<String, oneshot::Sender<()>>>>,
    timeout: Duration,
) -> Result<()> {
    let nonce = Uuid::new_v4().to_string();
    let (ack_tx, ack_rx) = oneshot::channel();
    ping_acks.lock().await.insert(nonce.clone(), ack_tx);
    let frame = try_json_text_message(&ServerToAgent::Ping {
        nonce: Some(nonce.clone()),
    })?;
    if control_tx.send(frame).is_err() {
        ping_acks.lock().await.remove(&nonce);
        return Err(anyhow!("agent writer closed before ping: {agent_id}"));
    }
    match tokio::time::timeout(timeout, ack_rx).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(_)) => Err(anyhow!("agent ping waiter closed: {agent_id}")),
        Err(_) => {
            ping_acks.lock().await.remove(&nonce);
            Err(anyhow!(
                "agent ping ack timeout after {}s: {agent_id}",
                timeout.as_secs()
            ))
        }
    }
}

pub(super) fn heartbeat_should_close_session(missed_acks: u32) -> bool {
    missed_acks >= AGENT_HEARTBEAT_MAX_MISSED_ACKS
}

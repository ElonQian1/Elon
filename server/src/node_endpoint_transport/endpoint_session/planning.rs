use std::{future::Future, sync::Arc, time::Duration};

use anyhow::{bail, Context, Result};
use axum::extract::ws::{Message, WebSocket};
use homecli_proto::{
    NodeEndpointPlanningBootstrapPreparationObservedV1,
    NodeEndpointPlanningBootstrapPreparationRequestV1,
    NodeEndpointPlanningBootstrapSharingObservedV1, NodeEndpointPlanningBootstrapSharingRequestV1,
    NodeEndpointPlanningBootstrapSnapshotObservedV1,
    NodeEndpointPlanningBootstrapSnapshotRequestV1,
};
use serde::{de::DeserializeOwned, Serialize};
use tokio::{sync::watch, time::timeout};

use crate::{homecli_agent::NodeEndpointSessionCurrent, types::AppState};

use super::MAX_MESSAGE_BYTES;

const STAGE_TIMEOUT: Duration = Duration::from_secs(10);
const CHAIN_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) async fn run_bootstrap(
    socket: &mut WebSocket,
    state: &Arc<AppState>,
    current: &NodeEndpointSessionCurrent,
    shutdown: &mut watch::Receiver<bool>,
) -> Result<()> {
    timeout(
        CHAIN_TIMEOUT,
        run_bootstrap_chain(socket, state, current, shutdown),
    )
    .await
    .context("NODE_ENDPOINT_PLANNING_BOOTSTRAP_CHAIN_TIMEOUT")?
}

async fn run_bootstrap_chain(
    socket: &mut WebSocket,
    state: &Arc<AppState>,
    current: &NodeEndpointSessionCurrent,
    shutdown: &mut watch::Receiver<bool>,
) -> Result<()> {
    let preparation = within_stage(
        "NODE_ENDPOINT_PLANNING_BOOTSTRAP_SHARING_STAGE_TIMEOUT",
        async {
            ensure_current(shutdown)?;
            let sharing = state
                .agent_manager
                .with_current_endpoint_planning_session(&state.store, current, |store, permit| {
                    store.prepare_node_compute_plugin_endpoint_planning_bootstrap_v1(permit)
                })
                .await?;
            ensure_current(shutdown)?;
            let Some(sharing) = sharing else {
                bail!("NODE_ENDPOINT_PLANNING_BOOTSTRAP_SHARING_INTENT_MISSING");
            };
            let observed: NodeEndpointPlanningBootstrapSharingObservedV1 =
                exchange(socket, sharing.message(), shutdown).await?;
            ensure_current(shutdown)?;
            state
                .agent_manager
                .with_current_endpoint_planning_session(&state.store, current, |store, permit| {
                    store.observe_node_compute_plugin_endpoint_planning_bootstrap_sharing_v1(
                        permit, &sharing, &observed,
                    )
                })
                .await
        },
    )
    .await?;
    let Some(preparation) = preparation else {
        return Ok(());
    };

    let snapshot = within_stage(
        "NODE_ENDPOINT_PLANNING_BOOTSTRAP_PREPARATION_STAGE_TIMEOUT",
        async {
            ensure_current(shutdown)?;
            let observed: NodeEndpointPlanningBootstrapPreparationObservedV1 =
                exchange(socket, preparation.message(), shutdown).await?;
            ensure_current(shutdown)?;
            state
                .agent_manager
                .with_current_endpoint_planning_session(&state.store, current, |store, permit| {
                    store.observe_node_compute_plugin_endpoint_planning_bootstrap_preparation_v1(
                        permit,
                        &preparation,
                        &observed,
                    )
                })
                .await
        },
    )
    .await?;
    let Some(snapshot) = snapshot else {
        return Ok(());
    };

    within_stage(
        "NODE_ENDPOINT_PLANNING_BOOTSTRAP_SNAPSHOT_STAGE_TIMEOUT",
        async {
            ensure_current(shutdown)?;
            let observed: NodeEndpointPlanningBootstrapSnapshotObservedV1 =
                exchange(socket, snapshot.message(), shutdown).await?;
            ensure_current(shutdown)?;
            state
                .agent_manager
                .with_current_endpoint_planning_session(&state.store, current, |store, permit| {
                    store.observe_node_compute_plugin_endpoint_planning_bootstrap_snapshot_v1(
                        permit, &snapshot, &observed,
                    )
                })
                .await?;
            Ok(())
        },
    )
    .await
}

async fn within_stage<T>(
    timeout_code: &'static str,
    operation: impl Future<Output = Result<T>>,
) -> Result<T> {
    timeout(STAGE_TIMEOUT, operation)
        .await
        .context(timeout_code)?
}

async fn exchange<Request, Observation>(
    socket: &mut WebSocket,
    request: &Request,
    shutdown: &mut watch::Receiver<bool>,
) -> Result<Observation>
where
    Request: PlanningMessage,
    Observation: PlanningMessage,
{
    request.validate_message().map_err(anyhow::Error::msg)?;
    let encoded = serde_json::to_string(request)?;
    if encoded.len() > MAX_MESSAGE_BYTES {
        bail!("NODE_ENDPOINT_PLANNING_BOOTSTRAP_REQUEST_TOO_LARGE");
    }
    ensure_current(shutdown)?;
    tokio::select! {
        biased;
        changed = shutdown.changed() => {
            changed.context("NODE_ENDPOINT_SESSION_SUPERVISOR_CLOSED")?;
            bail!("NODE_ENDPOINT_PLANNING_BOOTSTRAP_SESSION_SUPERSEDED");
        }
        sent = socket.send(Message::Text(encoded)) => sent?,
    }

    let frame = tokio::select! {
        biased;
        changed = shutdown.changed() => {
            changed.context("NODE_ENDPOINT_SESSION_SUPERVISOR_CLOSED")?;
            bail!("NODE_ENDPOINT_PLANNING_BOOTSTRAP_SESSION_SUPERSEDED");
        }
        received = socket.recv() => received,
    };
    let frame = frame
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_PLANNING_BOOTSTRAP_OBSERVATION_MISSING"))??;
    let Message::Text(text) = frame else {
        bail!("NODE_ENDPOINT_PLANNING_BOOTSTRAP_FRAME_FORBIDDEN");
    };
    if text.len() > MAX_MESSAGE_BYTES {
        bail!("NODE_ENDPOINT_PLANNING_BOOTSTRAP_OBSERVATION_TOO_LARGE");
    }
    let observed: Observation = serde_json::from_str(&text)
        .context("NODE_ENDPOINT_PLANNING_BOOTSTRAP_OBSERVATION_INVALID")?;
    observed.validate_message().map_err(anyhow::Error::msg)?;
    Ok(observed)
}

fn ensure_current(shutdown: &watch::Receiver<bool>) -> Result<()> {
    if *shutdown.borrow() {
        bail!("NODE_ENDPOINT_PLANNING_BOOTSTRAP_SESSION_SUPERSEDED");
    }
    Ok(())
}

trait PlanningMessage: Serialize + DeserializeOwned {
    fn validate_message(&self) -> std::result::Result<(), &'static str>;
}

macro_rules! impl_planning_message {
    ($($message:ty),+ $(,)?) => {
        $(
            impl PlanningMessage for $message {
                fn validate_message(&self) -> std::result::Result<(), &'static str> {
                    self.validate()
                }
            }
        )+
    };
}

impl_planning_message!(
    NodeEndpointPlanningBootstrapSharingRequestV1,
    NodeEndpointPlanningBootstrapSharingObservedV1,
    NodeEndpointPlanningBootstrapPreparationRequestV1,
    NodeEndpointPlanningBootstrapPreparationObservedV1,
    NodeEndpointPlanningBootstrapSnapshotRequestV1,
    NodeEndpointPlanningBootstrapSnapshotObservedV1,
);

use std::{collections::BTreeSet, sync::Arc, time::Duration};

use anyhow::{bail, Context, Result};
use homecli_proto::{
    NodeEndpointPlanningBootstrapMessageBindingV1Fields,
    NodeEndpointPlanningBootstrapPreparationObservedV1,
    NodeEndpointPlanningBootstrapPreparationObservedV1Fields,
    NodeEndpointPlanningBootstrapPreparationRequestV1,
    NodeEndpointPlanningBootstrapPreparationRequestV1Fields,
    NodeEndpointPlanningBootstrapSharingObservedV1,
    NodeEndpointPlanningBootstrapSharingObservedV1Fields,
    NodeEndpointPlanningBootstrapSharingRequestV1,
    NodeEndpointPlanningBootstrapSharingRequestV1Fields,
    NodeEndpointPlanningBootstrapSnapshotObservedV1,
    NodeEndpointPlanningBootstrapSnapshotObservedV1Fields,
    NodeEndpointPlanningBootstrapSnapshotRequestV1,
    NodeEndpointPlanningBootstrapSnapshotRequestV1Fields,
};
use tokio::{sync::watch, time::Instant};

use crate::{
    compute_plugin_sharing_directive::compute_plugin_endpoint_planning_message_json_and_digest,
    node_agent_compute_plugin_host::ComputePluginEndpointSessionWitness,
    node_agent_endpoint_credentials::EndpointSessionLease, NodeRuntime,
};

use super::{EndpointSessionEnd, EndpointWebSocket};
use io::{
    keepalive, next_request_text, require_current_stage, send_observation, with_current_stage,
    NextText,
};

mod io;

const ZERO_DIGEST: &str = "0000000000000000000000000000000000000000000000000000000000000000";

struct ChainState {
    bootstrap_id: Option<String>,
    last_observation_digest: Option<String>,
    delivery_ids: BTreeSet<String>,
}

pub(super) async fn run(
    runtime: &Arc<NodeRuntime>,
    websocket: &mut EndpointWebSocket,
    lease: &EndpointSessionLease,
    witness: &ComputePluginEndpointSessionWitness,
    epoch: &mut watch::Receiver<u64>,
    chain_started_at: Instant,
    renewal_after: Duration,
) -> Result<EndpointSessionEnd> {
    let renewal = tokio::time::sleep(renewal_after);
    tokio::pin!(renewal);
    let mut chain = ChainState::new();
    let chain_deadline = io::chain_deadline(chain_started_at);
    let sharing_deadline = io::stage_deadline(chain_deadline)?;

    let sharing = match next_request_text(
        runtime,
        websocket,
        lease,
        witness,
        epoch,
        sharing_deadline,
        renewal.as_mut(),
    )
    .await?
    {
        NextText::Message(text) => parse_sharing_request(&text)?,
        NextText::End(end) => return Ok(end),
    };
    chain.accept_request(&sharing.binding, witness, 1)?;
    io::require_stage_live(sharing_deadline)?;
    let runtime_requested = sharing.snapshot.plugin_runtime_requested;
    let observed = with_current_stage(runtime, lease, witness, sharing_deadline, || {
        runtime
            .compute_plugin_bootstrap
            .apply_sharing_policy_snapshot_v1(
                &sharing.snapshot,
                witness.node_id(),
                witness.owner_user_id(),
            )
    })
    .await?;
    ensure_sharing_inert(&observed)?;
    let accepted = observed.accepted;
    io::require_stage_live(sharing_deadline)?;
    require_current_stage(runtime, lease, witness, sharing_deadline).await?;
    let (message, digest) = sharing_observation(&mut chain, sharing.binding, observed)?;
    send_observation(
        runtime,
        websocket,
        lease,
        witness,
        sharing_deadline,
        message,
    )
    .await?;
    chain.finish_observation(digest);
    if !accepted || !runtime_requested {
        return keepalive(runtime, websocket, lease, witness, epoch, renewal.as_mut()).await;
    }

    let preparation_deadline = io::stage_deadline(chain_deadline)?;
    let preparation = match next_request_text(
        runtime,
        websocket,
        lease,
        witness,
        epoch,
        preparation_deadline,
        renewal.as_mut(),
    )
    .await?
    {
        NextText::Message(text) => parse_preparation_request(&text)?,
        NextText::End(end) => return Ok(end),
    };
    chain.accept_request(&preparation.binding, witness, 3)?;
    io::require_stage_live(preparation_deadline)?;
    let observed = with_current_stage(runtime, lease, witness, preparation_deadline, || {
        runtime
            .compute_plugin_bootstrap
            .observe_install_plan_preparation_v1(
                &preparation.request,
                &preparation.binding.delivery_id,
                witness.node_id(),
                witness.owner_user_id(),
            )
    })
    .await?;
    ensure_preparation_blocked(&observed)?;
    let accepted = observed.accepted;
    io::require_stage_live(preparation_deadline)?;
    require_current_stage(runtime, lease, witness, preparation_deadline).await?;
    let (message, digest) = preparation_observation(&mut chain, preparation.binding, observed)?;
    send_observation(
        runtime,
        websocket,
        lease,
        witness,
        preparation_deadline,
        message,
    )
    .await?;
    chain.finish_observation(digest);
    if !accepted {
        return keepalive(runtime, websocket, lease, witness, epoch, renewal.as_mut()).await;
    }

    let snapshot_deadline = io::stage_deadline(chain_deadline)?;
    let snapshot = match next_request_text(
        runtime,
        websocket,
        lease,
        witness,
        epoch,
        snapshot_deadline,
        renewal.as_mut(),
    )
    .await?
    {
        NextText::Message(text) => parse_snapshot_request(&text)?,
        NextText::End(end) => return Ok(end),
    };
    chain.accept_request(&snapshot.binding, witness, 5)?;
    io::require_stage_live(snapshot_deadline)?;
    let observed = with_current_stage(runtime, lease, witness, snapshot_deadline, || {
        runtime
            .compute_plugin_bootstrap
            .observe_install_plan_planning_snapshot_v2(
                &snapshot.request,
                witness.node_id(),
                witness.owner_user_id(),
            )
    })
    .await?;
    ensure_snapshot_blocked(&observed)?;
    io::require_stage_live(snapshot_deadline)?;
    require_current_stage(runtime, lease, witness, snapshot_deadline).await?;
    let (message, digest) = snapshot_observation(&mut chain, snapshot.binding, observed)?;
    send_observation(
        runtime,
        websocket,
        lease,
        witness,
        snapshot_deadline,
        message,
    )
    .await?;
    chain.finish_observation(digest);
    keepalive(runtime, websocket, lease, witness, epoch, renewal.as_mut()).await
}

impl ChainState {
    fn new() -> Self {
        Self {
            bootstrap_id: None,
            last_observation_digest: None,
            delivery_ids: BTreeSet::new(),
        }
    }

    fn accept_request(
        &mut self,
        binding: &NodeEndpointPlanningBootstrapMessageBindingV1Fields,
        witness: &ComputePluginEndpointSessionWitness,
        expected_sequence: u32,
    ) -> Result<()> {
        witness.require_session_binding(&binding.session_binding)?;
        if binding.message_sequence != expected_sequence
            || !self.delivery_ids.insert(binding.delivery_id.clone())
        {
            bail!("NODE_ENDPOINT_PLANNING_BOOTSTRAP_REQUEST_SEQUENCE_INVALID");
        }
        match (&self.bootstrap_id, expected_sequence) {
            (None, 1) => self.bootstrap_id = Some(binding.bootstrap_id.clone()),
            (Some(current), 3 | 5)
                if current == &binding.bootstrap_id
                    && self.last_observation_digest.as_deref()
                        == Some(binding.previous_message_digest.as_str()) => {}
            _ => bail!("NODE_ENDPOINT_PLANNING_BOOTSTRAP_CHAIN_MISMATCH"),
        }
        Ok(())
    }

    fn observation_binding(
        &self,
        request: NodeEndpointPlanningBootstrapMessageBindingV1Fields,
        message_sequence: u32,
    ) -> NodeEndpointPlanningBootstrapMessageBindingV1Fields {
        NodeEndpointPlanningBootstrapMessageBindingV1Fields {
            bootstrap_id: request.bootstrap_id,
            message_sequence,
            session_binding: request.session_binding,
            delivery_id: request.delivery_id,
            previous_message_digest: request.message_digest,
            message_digest: ZERO_DIGEST.to_string(),
        }
    }

    fn finish_observation(&mut self, digest: String) {
        self.last_observation_digest = Some(digest);
    }
}

fn parse_sharing_request(
    text: &str,
) -> Result<NodeEndpointPlanningBootstrapSharingRequestV1Fields> {
    let message: NodeEndpointPlanningBootstrapSharingRequestV1 =
        serde_json::from_str(text).context("NODE_ENDPOINT_PLANNING_BOOTSTRAP_SHARING_INVALID")?;
    let (_, digest) = compute_plugin_endpoint_planning_message_json_and_digest(
        &message.digest_material().map_err(anyhow::Error::msg)?,
    )?;
    let fields = message.into_fields().map_err(anyhow::Error::msg)?;
    if fields.binding.message_digest != digest {
        bail!("NODE_ENDPOINT_PLANNING_BOOTSTRAP_SHARING_DIGEST_MISMATCH");
    }
    Ok(fields)
}

fn parse_preparation_request(
    text: &str,
) -> Result<NodeEndpointPlanningBootstrapPreparationRequestV1Fields> {
    let message: NodeEndpointPlanningBootstrapPreparationRequestV1 = serde_json::from_str(text)
        .context("NODE_ENDPOINT_PLANNING_BOOTSTRAP_PREPARATION_INVALID")?;
    let (_, digest) = compute_plugin_endpoint_planning_message_json_and_digest(
        &message.digest_material().map_err(anyhow::Error::msg)?,
    )?;
    let fields = message.into_fields().map_err(anyhow::Error::msg)?;
    if fields.binding.message_digest != digest {
        bail!("NODE_ENDPOINT_PLANNING_BOOTSTRAP_PREPARATION_DIGEST_MISMATCH");
    }
    Ok(fields)
}

fn parse_snapshot_request(
    text: &str,
) -> Result<NodeEndpointPlanningBootstrapSnapshotRequestV1Fields> {
    let message: NodeEndpointPlanningBootstrapSnapshotRequestV1 =
        serde_json::from_str(text).context("NODE_ENDPOINT_PLANNING_BOOTSTRAP_SNAPSHOT_INVALID")?;
    let (_, digest) = compute_plugin_endpoint_planning_message_json_and_digest(
        &message.digest_material().map_err(anyhow::Error::msg)?,
    )?;
    let fields = message.into_fields().map_err(anyhow::Error::msg)?;
    if fields.binding.message_digest != digest {
        bail!("NODE_ENDPOINT_PLANNING_BOOTSTRAP_SNAPSHOT_DIGEST_MISMATCH");
    }
    Ok(fields)
}

fn sharing_observation(
    chain: &mut ChainState,
    binding: NodeEndpointPlanningBootstrapMessageBindingV1Fields,
    observed: homecli_proto::ComputePluginSharingPolicyObservedV1,
) -> Result<(NodeEndpointPlanningBootstrapSharingObservedV1, String)> {
    let placeholder = NodeEndpointPlanningBootstrapSharingObservedV1::new(
        NodeEndpointPlanningBootstrapSharingObservedV1Fields {
            binding: chain.observation_binding(binding, 2),
            observed,
        },
    )
    .map_err(anyhow::Error::msg)?;
    let (_, digest) = compute_plugin_endpoint_planning_message_json_and_digest(
        &placeholder.digest_material().map_err(anyhow::Error::msg)?,
    )?;
    let mut fields = placeholder.into_fields().map_err(anyhow::Error::msg)?;
    fields.binding.message_digest = digest.clone();
    Ok((
        NodeEndpointPlanningBootstrapSharingObservedV1::new(fields).map_err(anyhow::Error::msg)?,
        digest,
    ))
}

fn preparation_observation(
    chain: &mut ChainState,
    binding: NodeEndpointPlanningBootstrapMessageBindingV1Fields,
    observed: homecli_proto::ComputePluginInstallPlanPreparationObservedV1,
) -> Result<(NodeEndpointPlanningBootstrapPreparationObservedV1, String)> {
    let placeholder = NodeEndpointPlanningBootstrapPreparationObservedV1::new(
        NodeEndpointPlanningBootstrapPreparationObservedV1Fields {
            binding: chain.observation_binding(binding, 4),
            observed,
        },
    )
    .map_err(anyhow::Error::msg)?;
    let (_, digest) = compute_plugin_endpoint_planning_message_json_and_digest(
        &placeholder.digest_material().map_err(anyhow::Error::msg)?,
    )?;
    let mut fields = placeholder.into_fields().map_err(anyhow::Error::msg)?;
    fields.binding.message_digest = digest.clone();
    Ok((
        NodeEndpointPlanningBootstrapPreparationObservedV1::new(fields)
            .map_err(anyhow::Error::msg)?,
        digest,
    ))
}

fn snapshot_observation(
    chain: &mut ChainState,
    binding: NodeEndpointPlanningBootstrapMessageBindingV1Fields,
    observed: homecli_proto::ComputePluginInstallPlanPlanningSnapshotObservedV2,
) -> Result<(NodeEndpointPlanningBootstrapSnapshotObservedV1, String)> {
    let placeholder = NodeEndpointPlanningBootstrapSnapshotObservedV1::new(
        NodeEndpointPlanningBootstrapSnapshotObservedV1Fields {
            binding: chain.observation_binding(binding, 6),
            observed,
        },
    )
    .map_err(anyhow::Error::msg)?;
    let (_, digest) = compute_plugin_endpoint_planning_message_json_and_digest(
        &placeholder.digest_material().map_err(anyhow::Error::msg)?,
    )?;
    let mut fields = placeholder.into_fields().map_err(anyhow::Error::msg)?;
    fields.binding.message_digest = digest.clone();
    Ok((
        NodeEndpointPlanningBootstrapSnapshotObservedV1::new(fields).map_err(anyhow::Error::msg)?,
        digest,
    ))
}

fn ensure_sharing_inert(value: &homecli_proto::ComputePluginSharingPolicyObservedV1) -> Result<()> {
    if value.side_effects_started {
        bail!("NODE_ENDPOINT_PLANNING_BOOTSTRAP_SHARING_SIDE_EFFECT_FORBIDDEN");
    }
    Ok(())
}

fn ensure_preparation_blocked(
    value: &homecli_proto::ComputePluginInstallPlanPreparationObservedV1,
) -> Result<()> {
    if value.context_ready
        || value.context.is_some()
        || value.compute_plugin_root_lock_acquired
        || value.trusted_time_authority_configured
        || value.rollback_anchor_witness_configured
        || value.root_pinned
        || value.authority_opened
        || value.process_fence_acquired
        || value.new_work_admission_enabled
        || value.downloads_allowed
        || value.side_effects_started
    {
        bail!("NODE_ENDPOINT_PLANNING_BOOTSTRAP_PREPARATION_AUTHORITY_FORBIDDEN");
    }
    Ok(())
}

fn ensure_snapshot_blocked(
    value: &homecli_proto::ComputePluginInstallPlanPlanningSnapshotObservedV2,
) -> Result<()> {
    if value.snapshot_ready
        || value.snapshot.is_some()
        || value.phase != "blocked"
        || value.local_confirmation_available
        || value.compute_plugin_root_lock_acquired
        || value.trusted_time_authority_configured
        || value.rollback_anchor_witness_configured
        || value.root_pinned
        || value.authority_opened
        || value.process_fence_acquired
        || value.plan_apply_allowed
        || value.new_work_admission_enabled
        || value.downloads_allowed
        || value.sidecar_launch_allowed
        || value.side_effects_started
    {
        bail!("NODE_ENDPOINT_PLANNING_BOOTSTRAP_SNAPSHOT_AUTHORITY_FORBIDDEN");
    }
    Ok(())
}

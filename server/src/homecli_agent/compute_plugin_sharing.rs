use std::{sync::Arc, time::Duration};

use tokio::sync::mpsc;

use super::{AgentManager, AgentToServer, ServerToAgent};
use crate::{
    node_registry::AgentProcessSessionKey, store::NodeComputePluginSharingDispatchIntent,
    types::AppState,
};

const ACK_TIMEOUT: Duration = Duration::from_secs(10);

mod install_plan_planning_snapshot;
mod install_plan_preparation;

pub(crate) struct ComputePluginSharingSessionAck {
    observed: homecli_proto::ComputePluginSharingPolicyObservedV1,
    process_session: AgentProcessSessionKey,
}

pub(super) fn spawn_current_compute_plugin_sharing_session_replay(
    state: Arc<AppState>,
    process_session: AgentProcessSessionKey,
    proto_version: u32,
    capabilities: &[String],
) {
    if proto_version < homecli_proto::COMPUTE_PLUGIN_SHARING_PROTO_VERSION
        || !capabilities
            .iter()
            .any(|capability| capability == homecli_proto::CAP_COMPUTE_PLUGIN_SHARING_V1)
    {
        return;
    }
    let agent_id = process_session.agent_id().to_string();
    tokio::spawn(async move {
        match state
            .store
            .prepare_node_compute_plugin_sharing_session_delivery(&agent_id)
        {
            Ok(Some(intent)) => {
                dispatch_durable_compute_plugin_sharing_intent_for_session(
                    &state,
                    &intent,
                    Some(&process_session),
                )
                .await;
            }
            Ok(None) => {}
            Err(error) => tracing::warn!(
                %agent_id,
                error = %error,
                "failed to prepare durable compute plugin sharing replay"
            ),
        }
    });
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ComputePluginSharingDispatchFailure {
    pub(crate) event_kind: &'static str,
    pub(crate) detail_code: &'static str,
}

impl AgentManager {
    /// Send one already-durable sharing snapshot and await its single protocol ACK.
    pub(crate) async fn dispatch_compute_plugin_sharing_policy(
        &self,
        agent_id: &str,
        req_id: &str,
        expected_process_session: Option<&AgentProcessSessionKey>,
        snapshot: homecli_proto::ComputePluginSharingPolicySnapshotV1,
    ) -> std::result::Result<ComputePluginSharingSessionAck, ComputePluginSharingDispatchFailure>
    {
        let (cmd_tx, pending, process_session) = {
            let agents = self.agents.read().await;
            let Some(agent) = agents.get(agent_id) else {
                return Err(failure(
                    "agent_offline",
                    "COMPUTE_PLUGIN_SHARING_AGENT_OFFLINE",
                ));
            };
            if expected_process_session.is_some_and(|expected| &agent.process_session != expected) {
                return Err(failure(
                    "dispatch_failed",
                    "COMPUTE_PLUGIN_SHARING_SESSION_REPLACED",
                ));
            }
            if agent.proto_version < homecli_proto::COMPUTE_PLUGIN_SHARING_PROTO_VERSION
                || !agent
                    .capabilities
                    .iter()
                    .any(|capability| capability == homecli_proto::CAP_COMPUTE_PLUGIN_SHARING_V1)
            {
                return Err(failure(
                    "capability_missing",
                    "COMPUTE_PLUGIN_SHARING_CAPABILITY_MISSING",
                ));
            }
            (
                agent.cmd_tx.clone(),
                agent.pending.clone(),
                agent.process_session.clone(),
            )
        };
        let (tx, mut rx) = mpsc::unbounded_channel();
        {
            let mut waiters = pending.lock().await;
            if waiters.contains_key(req_id) {
                return Err(failure(
                    "dispatch_failed",
                    "COMPUTE_PLUGIN_SHARING_REQUEST_ALREADY_PENDING",
                ));
            }
            waiters.insert(req_id.to_string(), tx);
        }
        if cmd_tx
            .send(ServerToAgent::ApplyComputePluginSharingPolicyV1 {
                req_id: req_id.to_string(),
                snapshot,
            })
            .is_err()
        {
            pending.lock().await.remove(req_id);
            return Err(failure(
                "writer_closed",
                "COMPUTE_PLUGIN_SHARING_WRITER_CLOSED",
            ));
        }
        let received = tokio::time::timeout(ACK_TIMEOUT, rx.recv()).await;
        pending.lock().await.remove(req_id);
        match received {
            Ok(Some(AgentToServer::ComputePluginSharingPolicyObservedV1 {
                req_id: observed_req_id,
                observed,
            })) if observed_req_id == req_id => Ok(ComputePluginSharingSessionAck {
                observed,
                process_session,
            }),
            Ok(Some(_)) => Err(failure(
                "dispatch_failed",
                "COMPUTE_PLUGIN_SHARING_ACK_TYPE_INVALID",
            )),
            Ok(None) => Err(failure(
                "writer_closed",
                "COMPUTE_PLUGIN_SHARING_ACK_CHANNEL_CLOSED",
            )),
            Err(_) => Err(failure("ack_timeout", "COMPUTE_PLUGIN_SHARING_ACK_TIMEOUT")),
        }
    }
}

pub(crate) async fn dispatch_durable_compute_plugin_sharing_intent(
    state: &AppState,
    intent: &NodeComputePluginSharingDispatchIntent,
) {
    dispatch_durable_compute_plugin_sharing_intent_for_session(state, intent, None).await;
}

async fn dispatch_durable_compute_plugin_sharing_intent_for_session(
    state: &AppState,
    intent: &NodeComputePluginSharingDispatchIntent,
    expected_process_session: Option<&AgentProcessSessionKey>,
) {
    if !intent.dispatchable {
        return;
    }
    let policy_revision = match u64::try_from(intent.policy_revision) {
        Ok(value) => value,
        Err(_) => {
            record_failure(state, intent, "COMPUTE_PLUGIN_SHARING_REVISION_INVALID");
            return;
        }
    };
    let authorization = intent
        .authorization
        .as_ref()
        .map(|value| {
            u64::try_from(value.revision).map(|revision| {
                homecli_proto::ComputePluginSharingAuthorizationBindingV1 {
                    authorization_ref: value.authorization_ref.clone(),
                    revision,
                    digest: value.digest.clone(),
                }
            })
        })
        .transpose();
    let authorization = match authorization {
        Ok(value) => value,
        Err(_) => {
            record_failure(
                state,
                intent,
                "COMPUTE_PLUGIN_SHARING_AUTH_REVISION_INVALID",
            );
            return;
        }
    };
    let snapshot = match crate::compute_plugin_sharing_directive::
        build_compute_plugin_sharing_policy_snapshot_v1(
            intent.node_id.clone(), intent.owner_user_id.clone(),
            intent.installation_identity_digest.clone(), policy_revision,
            intent.policy_digest.clone(), intent.plugin_runtime_requested, authorization,
        ) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            record_failure(state, intent, error.code());
            return;
        }
    };
    let snapshot_digest =
        match crate::compute_plugin_sharing_directive::compute_plugin_sharing_policy_snapshot_digest(
            &snapshot,
        ) {
            Ok(digest) => digest,
            Err(error) => {
                record_failure(state, intent, error.code());
                return;
            }
        };
    match state
        .agent_manager
        .dispatch_compute_plugin_sharing_policy(
            &intent.node_id,
            &intent.delivery_id,
            expected_process_session,
            snapshot,
        )
        .await
    {
        Ok(session_ack) => {
            let observed = session_ack.observed;
            if let Err(code) = validate_observed(intent, &snapshot_digest, &observed) {
                record_failure(state, intent, code);
                return;
            }
            let observed_json = match serde_json::to_value(&observed) {
                Ok(value) => value,
                Err(error) => {
                    tracing::warn!(node_id = %intent.node_id, error = %error,
                        "failed to serialize compute plugin sharing observation");
                    return;
                }
            };
            let commit = state
                .agent_manager
                .with_current_process_session(&session_ack.process_session, |_| {
                    state.store.record_node_compute_plugin_sharing_delivery(
                        intent,
                        "dispatched",
                        None,
                    )?;
                    state.store.record_node_compute_plugin_sharing_observation(
                        intent,
                        observed.accepted,
                        &observed_json,
                    )?;
                    if observed.accepted && intent.plugin_runtime_requested {
                        state
                            .store
                            .prepare_node_compute_plugin_install_plan_preparation_delivery(
                                intent,
                                &snapshot_digest,
                            )
                    } else {
                        Ok(None)
                    }
                })
                .await;
            let preparation = match commit {
                None => {
                    record_failure(
                        state,
                        intent,
                        "COMPUTE_PLUGIN_SHARING_SESSION_REPLACED_BEFORE_ACK_COMMIT",
                    );
                    return;
                }
                Some(Err(error)) => {
                    tracing::warn!(node_id = %intent.node_id, error = %error,
                        "failed to persist process-session-fenced sharing ACK closure");
                    return;
                }
                Some(Ok(preparation)) => preparation,
            };
            if let Some(preparation) = preparation {
                install_plan_preparation::dispatch_durable_install_plan_preparation(
                    state,
                    &preparation,
                    &session_ack.process_session,
                )
                .await;
            }
        }
        Err(failure) => {
            if let Err(error) = state.store.record_node_compute_plugin_sharing_delivery(
                intent,
                failure.event_kind,
                Some(failure.detail_code),
            ) {
                tracing::warn!(node_id = %intent.node_id, error = %error,
                    "failed to persist compute plugin sharing dispatch failure");
            }
        }
    }
}

fn validate_observed(
    intent: &NodeComputePluginSharingDispatchIntent,
    snapshot_digest: &str,
    observed: &homecli_proto::ComputePluginSharingPolicyObservedV1,
) -> std::result::Result<(), &'static str> {
    if observed.schema != homecli_proto::COMPUTE_PLUGIN_SHARING_POLICY_OBSERVED_V1_SCHEMA
        || observed.node_id != intent.node_id
        || observed.owner_user_id != intent.owner_user_id
        || observed.installation_identity_digest.as_deref()
            != Some(intent.installation_identity_digest.as_str())
        || observed.side_effects_started
    {
        return Err("COMPUTE_PLUGIN_SHARING_OBSERVED_IDENTITY_INVALID");
    }
    if observed.observed_policy_revision != u64::try_from(intent.policy_revision).ok()
        || observed.observed_policy_digest.as_deref() != Some(intent.policy_digest.as_str())
        || observed.observed_snapshot_digest.as_deref() != Some(snapshot_digest)
    {
        return Err("COMPUTE_PLUGIN_SHARING_OBSERVED_FACTS_MISMATCH");
    }
    if observed.accepted && observed.error_code.is_some() {
        return Err("COMPUTE_PLUGIN_SHARING_ACCEPTED_ERROR_PRESENT");
    }
    if !observed.accepted && observed.error_code.is_none() {
        return Err("COMPUTE_PLUGIN_SHARING_REJECTION_CODE_MISSING");
    }
    Ok(())
}

fn record_failure(
    state: &AppState,
    intent: &NodeComputePluginSharingDispatchIntent,
    detail_code: &str,
) {
    if let Err(error) = state.store.record_node_compute_plugin_sharing_delivery(
        intent,
        "dispatch_failed",
        Some(detail_code),
    ) {
        tracing::warn!(node_id = %intent.node_id, error = %error,
            "failed to persist compute plugin sharing validation failure");
    }
}

fn failure(
    event_kind: &'static str,
    detail_code: &'static str,
) -> ComputePluginSharingDispatchFailure {
    ComputePluginSharingDispatchFailure {
        event_kind,
        detail_code,
    }
}

use tokio::sync::mpsc;

use super::{failure, AgentManager, AgentToServer, ComputePluginSharingDispatchFailure};
use crate::{
    store::{
        NodeComputePluginInstallPlanPlanningDispatchIntentV2, PlanningSnapshotObservationCommitV2,
    },
    types::AppState,
};

impl AgentManager {
    async fn dispatch_compute_plugin_install_plan_planning_snapshot_v2(
        &self,
        agent_id: &str,
        req_id: &str,
        expected_session_id: &str,
        request: homecli_proto::ComputePluginInstallPlanPlanningSnapshotRequestV2,
    ) -> std::result::Result<
        homecli_proto::ComputePluginInstallPlanPlanningSnapshotObservedV2,
        ComputePluginSharingDispatchFailure,
    > {
        let (cmd_tx, pending) = {
            let agents = self.agents.read().await;
            let Some(agent) = agents.get(agent_id) else {
                return Err(planning_failure(
                    "agent_offline",
                    "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_AGENT_OFFLINE",
                ));
            };
            if agent.session_id != expected_session_id {
                return Err(planning_failure(
                    "session_replaced",
                    "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_SESSION_REPLACED",
                ));
            }
            if agent.proto_version
                < homecli_proto::COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_PROTO_VERSION
                || !agent.capabilities.iter().any(|capability| {
                    capability
                        == homecli_proto::CAP_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_V2
                })
            {
                return Err(planning_failure(
                    "capability_missing",
                    "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_CAPABILITY_MISSING",
                ));
            }
            (agent.cmd_tx.clone(), agent.pending.clone())
        };
        let (tx, mut rx) = mpsc::unbounded_channel();
        {
            let mut waiters = pending.lock().await;
            if waiters.contains_key(req_id) {
                return Err(planning_failure(
                    "dispatch_failed",
                    "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_REQUEST_ALREADY_PENDING",
                ));
            }
            waiters.insert(req_id.to_string(), tx);
        }
        if cmd_tx
            .send(
                homecli_proto::ServerToAgent::ReadComputePluginInstallPlanPlanningSnapshotV2 {
                    req_id: req_id.to_string(),
                    request,
                },
            )
            .is_err()
        {
            pending.lock().await.remove(req_id);
            return Err(planning_failure(
                "writer_closed",
                "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_WRITER_CLOSED",
            ));
        }
        let received = tokio::time::timeout(super::ACK_TIMEOUT, rx.recv()).await;
        pending.lock().await.remove(req_id);
        match received {
            Ok(Some(AgentToServer::ComputePluginInstallPlanPlanningSnapshotObservedV2 {
                req_id: observed_req_id,
                observed,
            })) if observed_req_id == req_id => Ok(observed),
            Ok(Some(_)) => Err(planning_failure(
                "dispatch_failed",
                "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_ACK_TYPE_INVALID",
            )),
            Ok(None) => Err(planning_failure(
                "writer_closed",
                "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_ACK_CHANNEL_CLOSED",
            )),
            Err(_) => Err(planning_failure(
                "ack_timeout",
                "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_ACK_TIMEOUT",
            )),
        }
    }
}

pub(super) async fn dispatch_durable_install_plan_planning_snapshot_v2(
    state: &AppState,
    intent: &NodeComputePluginInstallPlanPlanningDispatchIntentV2,
    expected_session_id: &str,
) {
    if !intent.dispatchable {
        return;
    }
    let observed = match state
        .agent_manager
        .dispatch_compute_plugin_install_plan_planning_snapshot_v2(
            &intent.request.node_id,
            &intent.planning_delivery_id,
            expected_session_id,
            intent.request.clone(),
        )
        .await
    {
        Ok(observed) => observed,
        Err(dispatch_failure) => {
            record_failure(state, intent, dispatch_failure);
            return;
        }
    };
    let committed = match state
        .store
        .record_node_compute_plugin_install_plan_planning_observation_v2(intent, &observed)
    {
        Ok(committed) => committed,
        Err(error) => {
            tracing::warn!(node_id = %intent.request.node_id, error = %error,
                "failed to persist Planning Snapshot V2 observation");
            record_failure(
                state,
                intent,
                planning_failure(
                    "dispatch_failed",
                    "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_ACK_INVALID_OR_PERSIST_FAILED",
                ),
            );
            return;
        }
    };
    let PlanningSnapshotObservationCommitV2::Snapshot(_) = committed else {
        return;
    };
    tracing::info!(
        node_id = %intent.request.node_id,
        planning_delivery_id = %intent.planning_delivery_id,
        "Planning Snapshot V2 and signer_unavailable generation outcome committed atomically"
    );
}

fn record_failure(
    state: &AppState,
    intent: &NodeComputePluginInstallPlanPlanningDispatchIntentV2,
    dispatch_failure: ComputePluginSharingDispatchFailure,
) {
    if let Err(error) = state
        .store
        .record_node_compute_plugin_install_plan_planning_delivery_failure_v2(
            intent,
            dispatch_failure.event_kind,
            dispatch_failure.detail_code,
        )
    {
        tracing::warn!(node_id = %intent.request.node_id, error = %error,
            "failed to persist Planning Snapshot V2 dispatch failure");
    }
}

fn planning_failure(
    event_kind: &'static str,
    detail_code: &'static str,
) -> ComputePluginSharingDispatchFailure {
    failure(event_kind, detail_code)
}

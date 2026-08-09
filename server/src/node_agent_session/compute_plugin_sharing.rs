use homecli_proto::{
    AgentToServer, ComputePluginInstallPlanPlanningSnapshotRequestV2,
    ComputePluginInstallPlanPreparationRequestV1, ComputePluginSharingPolicySnapshotV1,
    ServerToAgent,
};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};

use super::{ws_text, Credentials, NodeRuntime};

pub(super) async fn handle_compute_plugin_message(
    runtime: &NodeRuntime,
    credentials: &Credentials,
    credential_epoch: u64,
    out_tx: &mpsc::UnboundedSender<Message>,
    message: ServerToAgent,
) -> anyhow::Result<Option<ServerToAgent>> {
    match message {
        ServerToAgent::ApplyComputePluginSharingPolicyV1 { req_id, snapshot } => {
            handle_policy_snapshot_v1(
                runtime,
                credentials,
                credential_epoch,
                out_tx,
                req_id,
                snapshot,
            )
            .await?;
            Ok(None)
        }
        ServerToAgent::PrepareComputePluginInstallPlanV1 { req_id, request } => {
            handle_install_plan_preparation_v1(
                runtime,
                credentials,
                credential_epoch,
                out_tx,
                req_id,
                request,
            )
            .await?;
            Ok(None)
        }
        ServerToAgent::ReadComputePluginInstallPlanPlanningSnapshotV2 { req_id, request } => {
            handle_install_plan_planning_snapshot_v2(
                runtime,
                credentials,
                credential_epoch,
                out_tx,
                req_id,
                request,
            )
            .await?;
            Ok(None)
        }
        other => Ok(Some(other)),
    }
}

pub(super) async fn handle_policy_snapshot_v1(
    runtime: &NodeRuntime,
    credentials: &Credentials,
    credential_epoch: u64,
    out_tx: &mpsc::UnboundedSender<Message>,
    req_id: String,
    snapshot: ComputePluginSharingPolicySnapshotV1,
) -> anyhow::Result<()> {
    runtime
        .with_current_credential_session(credential_epoch, credentials, || {
            handle_policy_snapshot_v1_current(runtime, credentials, out_tx, req_id, snapshot)
        })
        .await
}

fn handle_policy_snapshot_v1_current(
    runtime: &NodeRuntime,
    credentials: &Credentials,
    out_tx: &mpsc::UnboundedSender<Message>,
    req_id: String,
    snapshot: ComputePluginSharingPolicySnapshotV1,
) {
    let observed = runtime
        .compute_plugin_bootstrap
        .apply_sharing_policy_snapshot_v1(
            &snapshot,
            &credentials.agent_id,
            &credentials.owner_user_id,
        );
    if observed.accepted {
        info!(
            replayed = observed.replayed,
            phase = %observed.phase,
            configuration_generation = observed.configuration_generation,
            cancellation_generation = observed.cancellation_generation,
            "已记录算力插件共享期望策略；未启动任何本机副作用"
        );
    } else {
        warn!(
            error_code = observed.error_code.as_deref().unwrap_or("unknown"),
            "拒绝算力插件共享期望策略"
        );
    }
    let _ = out_tx.send(ws_text(
        &AgentToServer::ComputePluginSharingPolicyObservedV1 { req_id, observed },
    ));
}

pub(super) async fn handle_install_plan_preparation_v1(
    runtime: &NodeRuntime,
    credentials: &Credentials,
    credential_epoch: u64,
    out_tx: &mpsc::UnboundedSender<Message>,
    req_id: String,
    request: ComputePluginInstallPlanPreparationRequestV1,
) -> anyhow::Result<()> {
    runtime
        .with_current_credential_session(credential_epoch, credentials, || {
            handle_install_plan_preparation_v1_current(
                runtime,
                credentials,
                out_tx,
                req_id,
                request,
            )
        })
        .await
}

fn handle_install_plan_preparation_v1_current(
    runtime: &NodeRuntime,
    credentials: &Credentials,
    out_tx: &mpsc::UnboundedSender<Message>,
    req_id: String,
    request: ComputePluginInstallPlanPreparationRequestV1,
) {
    let observed = runtime
        .compute_plugin_bootstrap
        .observe_install_plan_preparation_v1(
            &request,
            &req_id,
            &credentials.agent_id,
            &credentials.owner_user_id,
        );
    if observed.accepted {
        info!(
            preparation_id = %observed.preparation_id,
            replayed = observed.replayed,
            context_ready = observed.context_ready,
            bootstrap_instance_id = %observed.bootstrap_instance_id,
            "已核对 InstallPlan 准备请求；本机权威上下文仍不可用且未启动任何副作用"
        );
    } else {
        warn!(
            preparation_id = %observed.preparation_id,
            error_code = observed.error_code.as_deref().unwrap_or("unknown"),
            "拒绝 InstallPlan 准备请求"
        );
    }
    let _ = out_tx.send(ws_text(
        &AgentToServer::ComputePluginInstallPlanPreparationObservedV1 { req_id, observed },
    ));
}

pub(super) async fn handle_install_plan_planning_snapshot_v2(
    runtime: &NodeRuntime,
    credentials: &Credentials,
    credential_epoch: u64,
    out_tx: &mpsc::UnboundedSender<Message>,
    req_id: String,
    request: ComputePluginInstallPlanPlanningSnapshotRequestV2,
) -> anyhow::Result<()> {
    runtime
        .with_current_credential_session(credential_epoch, credentials, || {
            handle_install_plan_planning_snapshot_v2_current(
                runtime,
                credentials,
                out_tx,
                req_id,
                request,
            )
        })
        .await
}

fn handle_install_plan_planning_snapshot_v2_current(
    runtime: &NodeRuntime,
    credentials: &Credentials,
    out_tx: &mpsc::UnboundedSender<Message>,
    req_id: String,
    request: ComputePluginInstallPlanPlanningSnapshotRequestV2,
) {
    let observed = runtime
        .compute_plugin_bootstrap
        .observe_install_plan_planning_snapshot_v2(
            &request,
            &credentials.agent_id,
            &credentials.owner_user_id,
        );
    if observed.accepted {
        info!(
            preparation_id = %observed.preparation_id,
            replayed = observed.replayed,
            snapshot_ready = observed.snapshot_ready,
            bootstrap_instance_id = %observed.bootstrap_instance_id,
            "已核对 InstallPlan Planning Snapshot V2 请求；快照源仍不可用且未启动任何副作用"
        );
    } else {
        warn!(
            preparation_id = %observed.preparation_id,
            error_code = observed.error_code.as_deref().unwrap_or("unknown"),
            "拒绝 InstallPlan Planning Snapshot V2 请求"
        );
    }
    let _ = out_tx.send(ws_text(
        &AgentToServer::ComputePluginInstallPlanPlanningSnapshotObservedV2 { req_id, observed },
    ));
}

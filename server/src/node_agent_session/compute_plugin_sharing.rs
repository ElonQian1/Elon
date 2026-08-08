use homecli_proto::{AgentToServer, ComputePluginSharingPolicySnapshotV1};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};

use super::{ws_text, Credentials, NodeRuntime};

pub(super) fn handle_policy_snapshot_v1(
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

use std::sync::Arc;

use anyhow::Result;
use homecli_proto::{
    AgentToServer, ModelCapability, NodeDevRuntimeProfile, NodeHardwareProfile, NodeStorageProfile,
    CAP_ANDROID_DEVICE_HOST_V1, CAP_COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_V1,
    CAP_COMPUTE_PLUGIN_SHARING_V1, CAP_LOCAL_TASK_PROJECT_SYNC_V1, CAP_PROJECT_BUILD_CACHE_V1,
    CAP_PROJECT_DOCUMENT_FEDERATION_V1, PROTO_VERSION,
};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use super::{ws_text, Credentials, NodeRuntime};
use crate::node_agent_config::machine_label;

#[allow(clippy::too_many_arguments)]
pub(super) async fn send_initial_registration(
    runtime: &Arc<NodeRuntime>,
    credentials: &Credentials,
    credential_epoch: u64,
    out_tx: &mpsc::UnboundedSender<Message>,
    models: Vec<ModelCapability>,
    allowed_clis: Vec<String>,
    hardware: NodeHardwareProfile,
    storage: NodeStorageProfile,
    dev_runtime: NodeDevRuntimeProfile,
) -> Result<()> {
    // The cloud requires Register as the first WebSocket frame. Both frames are enqueued while a
    // credential read lease excludes replacement, so a superseded identity cannot register after
    // `set_creds` has begun its fail-closed transition.
    runtime.set_connection_stage("cloud_register").await;
    let lifecycle =
        crate::node_agent_lifecycle::runtime_report(runtime, true, true, "正在注册云端会话").await;
    let tts_worker_url = runtime.tts_worker_url.read().await.clone();
    let register = ws_text(&AgentToServer::Register {
        agent_id: credentials.agent_id.clone(),
        version: crate::node_agent_release_identity::current(),
        proto_version: PROTO_VERSION,
        capabilities: vec![
            CAP_PROJECT_BUILD_CACHE_V1.to_string(),
            CAP_ANDROID_DEVICE_HOST_V1.to_string(),
            CAP_PROJECT_DOCUMENT_FEDERATION_V1.to_string(),
            CAP_LOCAL_TASK_PROJECT_SYNC_V1.to_string(),
            CAP_COMPUTE_PLUGIN_SHARING_V1.to_string(),
            CAP_COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_V1.to_string(),
            homecli_proto::CAP_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_V2.to_string(),
        ],
        allowed_clis: allowed_clis.clone(),
        allowed_cwds: vec![],
        owner_user_id: Some(credentials.owner_user_id.clone()),
        device_name: Some(machine_label()),
        install_id: Some(runtime.install_id.clone()),
        hardware: Some(hardware.clone()),
        storage: Some(storage.clone()),
        dev_runtime: Some(dev_runtime.clone()),
        lifecycle: Some(lifecycle.clone()),
    });
    let capabilities = ws_text(&AgentToServer::RegisterCapabilities {
        models,
        allowed_clis,
        tts_worker_url,
        hardware: Some(hardware),
        storage: Some(storage),
        dev_runtime: Some(dev_runtime),
        lifecycle: Some(lifecycle),
    });
    runtime
        .with_current_credential_session(credential_epoch, credentials, || -> Result<()> {
            out_tx.send(register)?;
            out_tx.send(capabilities)?;
            Ok(())
        })
        .await?
}

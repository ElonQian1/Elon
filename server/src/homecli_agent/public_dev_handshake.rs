use std::sync::Arc;

use homecli_proto::NodeDevRuntimeProfile;

use crate::types::AppState;

pub(super) async fn record_node_public_dev_handshake(
    state: &Arc<AppState>,
    agent_id: &str,
    owner_user_id: &str,
    agent_version: &str,
    allowed_clis: &[String],
    dev_runtime: Option<&NodeDevRuntimeProfile>,
    warn_message: &'static str,
) {
    if owner_user_id.trim().is_empty() {
        return;
    }
    let handshake_clis = if !allowed_clis.is_empty() {
        allowed_clis.to_vec()
    } else {
        let agents = state.agent_manager.agents.read().await;
        agents
            .get(agent_id)
            .map(|entry| entry.allowed_clis.clone())
            .unwrap_or_default()
    };
    if let Err(e) = state.store.record_node_handshake(
        agent_id,
        owner_user_id,
        agent_version,
        &handshake_clis,
        dev_runtime,
    ) {
        tracing::warn!(%agent_id, error = %e, "{}", warn_message);
    }
}

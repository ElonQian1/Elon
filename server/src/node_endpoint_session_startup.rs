use std::sync::Arc;

use anyhow::Result;

use crate::types::AppState;

/// Reconcile durable heads before any listener or background producer can observe this process.
pub(crate) fn prepare(state: Arc<AppState>) -> Result<Arc<AppState>> {
    let restarted = state.store.restart_node_endpoint_sessions()?;
    if restarted > 0 {
        tracing::info!(
            restarted,
            "staled active endpoint sessions after server restart"
        );
    }
    let worker_state = Arc::clone(&state);
    tokio::spawn(async move {
        worker_state
            .agent_manager
            .supervise_endpoint_session_expiry(&worker_state.store)
            .await;
    });
    Ok(state)
}

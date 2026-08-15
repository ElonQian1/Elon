use std::sync::Arc;

use anyhow::Result;

use crate::{
    compute_federation::{
        external_pool_adapter_runtime_compatibility_signing_handoff_runtime::initialize_external_pool_adapter_runtime_compatibility_signing_handoff_runtime,
        external_pool_adapter_task_worker::initialize_external_pool_adapter_task_worker_runtime,
    },
    store::{
        initialize_external_pool_adapter_provider_runtime_readiness_runtime,
        initialize_external_pool_adapter_task_protocol_conformance_runtime,
    },
    types::AppState,
};

/// Reconcile durable heads before any listener or background producer can observe this process.
pub(crate) fn prepare(state: Arc<AppState>) -> Result<Arc<AppState>> {
    initialize_external_pool_adapter_runtime_compatibility_signing_handoff_runtime()?;
    initialize_external_pool_adapter_provider_runtime_readiness_runtime()?;
    initialize_external_pool_adapter_task_protocol_conformance_runtime()?;
    initialize_external_pool_adapter_task_worker_runtime()?;
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

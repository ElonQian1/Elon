//! Startup wiring for server-owned background workers.

use std::sync::Arc;

use crate::{
    billing_lifecycle, billing_monitor, codex_health, compute_federation,
    open_commerce_webhook_worker, pc_relay_client, project_document_maintenance,
    project_workspace_health_monitor, AppState,
};

pub(crate) fn spawn(state: Arc<AppState>) {
    codex_health::spawn_codex_network_monitor(state.clone());
    billing_lifecycle::spawn_reservation_janitor(state.clone());
    compute_federation::external_pool_adapter_task_worker::spawn(state.clone());
    compute_federation::delivery_allocation_expiry_worker::spawn(state.clone());
    billing_monitor::spawn_reconciliation_monitor(state.clone());
    project_workspace_health_monitor::spawn_project_workspace_health_monitor(state.clone());
    project_document_maintenance::spawn_maintenance_worker();
    open_commerce_webhook_worker::spawn(state);
    // 本地模式：作为 agent 连回云端，实现 APK→云端→PC 全双工中继
    pc_relay_client::spawn_if_configured();
}

pub(crate) fn spawn_stale_task_cleanup(state: Arc<AppState>) {
    const STALE_RUNNING_TASK_TIMEOUT_SECS: u64 = 45 * 60;
    // Preserve the build/upload/restart allowance and active-channel exclusion.
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(120));
        loop {
            interval.tick().await;
            let active = crate::project_space::active_channel_ai_task_ids();
            match state
                .store
                .mark_stale_running_tasks_with_channel_results_excluding(
                    STALE_RUNNING_TASK_TIMEOUT_SECS,
                    &active,
                ) {
                Ok(n) if n > 0 => tracing::info!("{n} 个超时 running 任务已自动标记为 failed"),
                Ok(_) => {}
                Err(e) => tracing::warn!("stale task cleanup error: {e}"),
            }
        }
    });
}

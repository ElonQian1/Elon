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
    compute_federation::delivery_allocation_expiry_worker::spawn(state.clone());
    billing_monitor::spawn_reconciliation_monitor(state.clone());
    project_workspace_health_monitor::spawn_project_workspace_health_monitor(state.clone());
    project_document_maintenance::spawn_maintenance_worker();
    open_commerce_webhook_worker::spawn(state);
    // 本地模式：作为 agent 连回云端，实现 APK→云端→PC 全双工中继
    pc_relay_client::spawn_if_configured();
}

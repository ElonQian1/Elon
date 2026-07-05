use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

use crate::{
    node_runtime::user_node_runtimes, pc_node_capacity::assess_pc_node_capacity,
    project_auth::auth_from_headers, types::AppState,
};

use super::{
    hardware_for_response, hardware_summary, public_dev::public_dev_handshake_state_for_runtime,
    responses::MyNodeResponse, runtime_route_flags, storage_can_cross_pc,
};
/// GET /api/me/nodes — 本用户自己的节点列表（含在线状态）
pub async fn my_nodes(State(state): State<Arc<AppState>>, headers: HeaderMap) -> impl IntoResponse {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    let runtimes = match user_node_runtimes(&state, &user.id).await {
        Ok(nodes) => nodes,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    let nodes = runtimes
        .into_iter()
        .map(|node| {
            let cli_project_ready = node.cli_project_ready();
            let workspace_provision_ready = node.workspace_provision_ready();
            let ai_cli_ready = node
                .dev_runtime
                .as_ref()
                .map(|runtime| runtime.ai_cli_ready)
                .unwrap_or(cli_project_ready);
            let (route_a_ready, api_runtime_ready, server_runtime_ready) =
                runtime_route_flags(node.dev_runtime.as_ref(), cli_project_ready);
            let global_project_count = state
                .store
                .count_active_pc_projects_for_node(&node.node_id)
                .unwrap_or(node.project_count);
            let mut capacity_node = node.clone();
            capacity_node.project_count = global_project_count;
            let latest_snapshot = state
                .store
                .latest_workspace_health_snapshot_for_node(&node.node_id)
                .ok()
                .flatten();
            let capacity = assess_pc_node_capacity(&capacity_node, latest_snapshot.as_ref());
            let storage = node.storage.clone();
            let storage_ready = node.storage_ready();
            let storage_repo_url_configured =
                storage.as_ref().map(storage_can_cross_pc).unwrap_or(false);
            let hardware = hardware_for_response(&state, &node.node_id, node.hardware.clone());
            let hardware_summary = hardware_summary(hardware.as_ref());
            let (public_dev_handshake_ready, public_dev_handshake_status) =
                public_dev_handshake_state_for_runtime(&node);
            MyNodeResponse {
                agent_id: node.node_id.clone(),
                node_id: node.node_id,
                owner_user_id: node.owner_user_id,
                label: node.label,
                device_name: node.device_name,
                hardware,
                hardware_summary,
                storage,
                dev_runtime: node.dev_runtime,
                lifecycle: node.lifecycle,
                storage_ready,
                storage_repo_url_configured,
                display_name: node.display_name,
                short_id: node.short_id,
                models: node.models,
                allowed_clis: node.allowed_clis,
                allowed_cwds: node.allowed_cwds,
                agent_version: node.agent_version,
                public_dev_enabled: node.public_dev_enabled,
                public_dev_allowed_clis: node.public_dev_allowed_clis,
                public_dev_permission_level: node.public_dev_permission_level,
                public_dev_handshake_ready,
                public_dev_handshake_status,
                last_handshake_at: node.last_handshake_at,
                last_handshake_agent_version: node.last_handshake_agent_version,
                last_handshake_allowed_clis: node.last_handshake_allowed_clis,
                cli_project_ready,
                workspace_provision_ready,
                ai_cli_ready,
                route_a_ready,
                api_runtime_ready,
                server_runtime_ready,
                project_count: capacity.project_count,
                project_limit: capacity.project_limit,
                project_slots_remaining: capacity.project_slots_remaining,
                disk_free_bytes: capacity.disk_free_bytes,
                can_accept_project: capacity.can_accept_project,
                capacity_label: capacity.label,
                capacity_tone: capacity.tone,
                capacity_warnings: capacity.warnings,
                connected_at: node.connected_at,
                created_at: node.created_at,
                online: node.online,
                registry_online: node.registry_online,
                cli_connected: node.cli_connected,
            }
        })
        .collect::<Vec<_>>();

    Json(serde_json::json!({ "nodes": nodes })).into_response()
}

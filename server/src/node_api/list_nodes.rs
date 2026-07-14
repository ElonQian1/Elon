use super::{
    public_dev::{public_dev_handshake_state, public_dev_handshake_value},
    responses::PublicNodeResponse,
    runtime_response::{
        capacity_for_response, hardware_for_response, hardware_summary, project_counts_for_user,
        runtime_route_flags,
    },
    storage_can_cross_pc,
};
use crate::{
    admin,
    node_runtime::{clean_string, display_node_name, short_node_id, supports_project_cli},
    project_auth::auth_from_headers,
    types::AppState,
};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use std::{collections::HashMap, sync::Arc};

// ── /api/nodes ────────────────────────────────────────────────────────────────

/// GET /api/nodes — 列出所有用户可发现的在线节点（含在线状态）
/// 需要有效用户 token（不要求管理员权限，普通用户可见）
pub async fn list_nodes(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };
    let project_counts = project_counts_for_user(&state, &user.id);
    let mut cli_by_id: HashMap<_, _> = state
        .agent_manager
        .list()
        .await
        .into_iter()
        .map(|agent| (agent.agent_id.clone(), agent))
        .collect();

    let mut nodes = Vec::new();
    for node in state.node_registry.list_online().await {
        let node_id = node.node_id.clone();
        let cli_agent = cli_by_id.remove(&node_id);
        let allowed_clis = cli_agent
            .as_ref()
            .map(|agent| agent.allowed_clis.clone())
            .unwrap_or_default();
        let dev_runtime = node.dev_runtime.clone().or_else(|| {
            cli_agent
                .as_ref()
                .and_then(|agent| agent.dev_runtime.clone())
        });
        let cli_project_ready = supports_project_cli(&allowed_clis);
        let workspace_provision_ready = dev_runtime
            .as_ref()
            .map(|runtime| runtime.workspace_provision_ready)
            .unwrap_or(cli_project_ready);
        let ai_cli_ready = dev_runtime
            .as_ref()
            .map(|runtime| runtime.ai_cli_ready)
            .unwrap_or(cli_project_ready);
        let (route_a_ready, api_runtime_ready, server_runtime_ready) =
            runtime_route_flags(dev_runtime.as_ref(), cli_project_ready);
        let short_id = short_node_id(&node_id);
        let device_name = clean_string(node.device_name.as_deref()).or_else(|| {
            cli_agent
                .as_ref()
                .and_then(|agent| clean_string(agent.device_name.as_deref()))
        });
        let display_name = display_node_name("", device_name.as_deref(), &short_id);
        let project_count = state
            .store
            .count_active_pc_projects_for_node(&node_id)
            .unwrap_or_else(|_| project_counts.get(&node_id).copied().unwrap_or(0));
        let capacity = capacity_for_response(
            &state,
            &node_id,
            &node.owner_user_id,
            "",
            device_name.as_deref(),
            &display_name,
            node.online || cli_agent.is_some(),
            cli_agent.is_some(),
            &allowed_clis,
            dev_runtime.clone(),
            project_count,
        );
        let hardware = hardware_for_response(&state, &node_id, node.hardware);
        let hardware_summary = hardware_summary(hardware.as_ref());
        let credential = state.store.get_node_credential(&node_id).ok().flatten();
        let agent_version = cli_agent.as_ref().map(|agent| agent.version.clone());
        let (public_dev_handshake_ready, public_dev_handshake_status) = public_dev_handshake_state(
            credential.as_ref(),
            node.online || cli_agent.is_some(),
            agent_version.as_deref(),
            &allowed_clis,
            dev_runtime.as_ref(),
        );
        nodes.push(PublicNodeResponse {
            agent_id: node_id.clone(),
            node_id: node_id.clone(),
            owner_user_id: node.owner_user_id,
            device_name,
            hardware,
            hardware_summary,
            storage: node.storage.clone(),
            dev_runtime,
            lifecycle: node
                .lifecycle
                .clone()
                .or_else(|| cli_agent.as_ref().and_then(|agent| agent.lifecycle.clone())),
            storage_ready: node
                .storage
                .as_ref()
                .map(|storage| storage.enabled)
                .unwrap_or(false),
            storage_repo_url_configured: node
                .storage
                .as_ref()
                .map(storage_can_cross_pc)
                .unwrap_or(false),
            display_name,
            short_id,
            models: node.models,
            allowed_clis: allowed_clis.clone(),
            agent_version,
            public_dev_enabled: credential
                .as_ref()
                .map(|credential| credential.public_dev_enabled)
                .unwrap_or(false),
            public_dev_allowed_clis: credential
                .as_ref()
                .map(|credential| credential.public_dev_allowed_clis.clone())
                .unwrap_or_default(),
            public_dev_permission_level: credential
                .as_ref()
                .map(|credential| credential.public_dev_permission_level.clone())
                .unwrap_or_else(|| "project_write".to_string()),
            public_dev_handshake_ready,
            public_dev_handshake_status,
            last_handshake_at: credential
                .as_ref()
                .and_then(|credential| credential.last_handshake_at.clone()),
            last_handshake_agent_version: credential
                .as_ref()
                .and_then(|credential| credential.last_handshake_agent_version.clone()),
            last_handshake_allowed_clis: credential
                .as_ref()
                .map(|credential| credential.last_handshake_allowed_clis.clone())
                .unwrap_or_default(),
            cli_project_ready,
            workspace_provision_ready,
            ai_cli_ready,
            route_a_ready,
            api_runtime_ready,
            server_runtime_ready,
            project_count,
            project_limit: capacity.project_limit,
            project_slots_remaining: capacity.project_slots_remaining,
            disk_free_bytes: capacity.disk_free_bytes,
            can_accept_project: capacity.can_accept_project,
            capacity_label: capacity.label,
            capacity_tone: capacity.tone,
            capacity_warnings: capacity.warnings,
            tts_worker_url: node.tts_worker_url,
            connected_at: node.connected_at,
            online: node.online || cli_agent.is_some(),
        });
    }

    for agent in cli_by_id.into_values() {
        let node_id = agent.agent_id.clone();
        let allowed_clis = agent.allowed_clis.clone();
        let dev_runtime = agent.dev_runtime.clone();
        let cli_project_ready = supports_project_cli(&allowed_clis);
        let workspace_provision_ready = dev_runtime
            .as_ref()
            .map(|runtime| runtime.workspace_provision_ready)
            .unwrap_or(cli_project_ready);
        let ai_cli_ready = dev_runtime
            .as_ref()
            .map(|runtime| runtime.ai_cli_ready)
            .unwrap_or(cli_project_ready);
        let (route_a_ready, api_runtime_ready, server_runtime_ready) =
            runtime_route_flags(dev_runtime.as_ref(), cli_project_ready);
        let short_id = short_node_id(&node_id);
        let device_name = clean_string(agent.device_name.as_deref());
        let display_name = display_node_name("", device_name.as_deref(), &short_id);
        let owner_user_id = state
            .store
            .get_node_credential_owner(&node_id)
            .ok()
            .flatten()
            .unwrap_or_default();
        let project_count = state
            .store
            .count_active_pc_projects_for_node(&node_id)
            .unwrap_or_else(|_| project_counts.get(&node_id).copied().unwrap_or(0));
        let capacity = capacity_for_response(
            &state,
            &node_id,
            &owner_user_id,
            "",
            device_name.as_deref(),
            &display_name,
            true,
            true,
            &allowed_clis,
            dev_runtime.clone(),
            project_count,
        );
        let hardware = hardware_for_response(&state, &node_id, agent.hardware);
        let hardware_summary = hardware_summary(hardware.as_ref());
        let credential = state.store.get_node_credential(&node_id).ok().flatten();
        let agent_version = Some(agent.version.clone());
        let (public_dev_handshake_ready, public_dev_handshake_status) = public_dev_handshake_state(
            credential.as_ref(),
            true,
            agent_version.as_deref(),
            &allowed_clis,
            dev_runtime.as_ref(),
        );
        nodes.push(PublicNodeResponse {
            agent_id: node_id.clone(),
            node_id: node_id.clone(),
            owner_user_id,
            device_name,
            hardware,
            hardware_summary,
            storage: agent.storage.clone(),
            dev_runtime,
            lifecycle: agent.lifecycle.clone(),
            storage_ready: agent
                .storage
                .as_ref()
                .map(|storage| storage.enabled)
                .unwrap_or(false),
            storage_repo_url_configured: agent
                .storage
                .as_ref()
                .map(storage_can_cross_pc)
                .unwrap_or(false),
            display_name,
            short_id,
            models: Vec::new(),
            allowed_clis: allowed_clis.clone(),
            agent_version,
            public_dev_enabled: credential
                .as_ref()
                .map(|credential| credential.public_dev_enabled)
                .unwrap_or(false),
            public_dev_allowed_clis: credential
                .as_ref()
                .map(|credential| credential.public_dev_allowed_clis.clone())
                .unwrap_or_default(),
            public_dev_permission_level: credential
                .as_ref()
                .map(|credential| credential.public_dev_permission_level.clone())
                .unwrap_or_else(|| "project_write".to_string()),
            public_dev_handshake_ready,
            public_dev_handshake_status,
            last_handshake_at: credential
                .as_ref()
                .and_then(|credential| credential.last_handshake_at.clone()),
            last_handshake_agent_version: credential
                .as_ref()
                .and_then(|credential| credential.last_handshake_agent_version.clone()),
            last_handshake_allowed_clis: credential
                .as_ref()
                .map(|credential| credential.last_handshake_allowed_clis.clone())
                .unwrap_or_default(),
            cli_project_ready,
            workspace_provision_ready,
            ai_cli_ready,
            route_a_ready,
            api_runtime_ready,
            server_runtime_ready,
            project_count,
            project_limit: capacity.project_limit,
            project_slots_remaining: capacity.project_slots_remaining,
            disk_free_bytes: capacity.disk_free_bytes,
            can_accept_project: capacity.can_accept_project,
            capacity_label: capacity.label,
            capacity_tone: capacity.tone,
            capacity_warnings: capacity.warnings,
            tts_worker_url: None,
            connected_at: agent.connected_at,
            online: true,
        });
    }

    Json(serde_json::json!({ "nodes": nodes })).into_response()
}

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use homecli_proto::NodeDevRuntimeProfile;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, collections::HashMap, sync::Arc};

use crate::{
    admin,
    node_runtime::{node_runtime_by_id, NodeRuntime},
    project_auth::auth_from_headers,
    store::NodeCredential,
    types::AppState,
};

#[derive(Deserialize)]
pub struct UpdateNodePublicDevSharingRequest {
    pub enabled: Option<bool>,
    pub allowed_clis: Option<Vec<String>>,
    pub permission_level: Option<String>,
}

/// PATCH /api/me/nodes/:node_id/sharing - toggle dev-phase public node sharing.
pub async fn update_my_node_sharing(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(node_id): Path<String>,
    Json(req): Json<UpdateNodePublicDevSharingRequest>,
) -> impl IntoResponse {
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

    let credential = match state.store.get_node_credential(&node_id) {
        Ok(Some(credential)) if credential.owner_user_id == user.id => credential,
        Ok(Some(_)) => {
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({"error": "只能修改自己的节点开放授权"})),
            )
                .into_response()
        }
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "节点不存在"})),
            )
                .into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };
    let enabled = req.enabled.unwrap_or(credential.public_dev_enabled);
    let allowed_clis = req
        .allowed_clis
        .unwrap_or(credential.public_dev_allowed_clis);
    let permission_level = req
        .permission_level
        .as_deref()
        .unwrap_or(&credential.public_dev_permission_level);

    match state.store.update_node_public_dev_sharing(
        &user.id,
        &credential.agent_id,
        enabled,
        &allowed_clis,
        permission_level,
    ) {
        Ok(Some(sharing)) => {
            Json(serde_json::json!({"ok": true, "sharing": sharing})).into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "节点不存在"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[derive(Serialize)]
struct AdminPublicDevHandshakeReport {
    generated_at: String,
    summary: AdminPublicDevHandshakeSummary,
    nodes: Vec<AdminPublicDevHandshakeNode>,
}

#[derive(Serialize)]
struct AdminPublicDevHandshakeSummary {
    total_nodes: usize,
    public_dev_enabled: usize,
    online_public_dev: usize,
    ready_public_dev: usize,
    pending_online_public_dev: usize,
    offline_public_dev: usize,
    status_counts: BTreeMap<String, usize>,
}

#[derive(Serialize)]
struct AdminPublicDevHandshakeNode {
    node_id: String,
    owner_user_id: String,
    owner_account: Option<String>,
    owner_nickname: Option<String>,
    label: String,
    device_name: Option<String>,
    display_name: String,
    short_id: String,
    install_id: Option<String>,
    public_dev_enabled: bool,
    public_dev_allowed_clis: Vec<String>,
    public_dev_permission_level: String,
    public_dev_handshake_ready: bool,
    public_dev_handshake_status: String,
    last_handshake_at: Option<String>,
    last_handshake_agent_version: Option<String>,
    last_handshake_allowed_clis: Vec<String>,
    last_handshake_route_a_ready: bool,
    last_handshake_api_runtime_ready: bool,
    last_handshake_server_runtime_ready: bool,
    last_handshake_ai_cli_ready: bool,
    online: bool,
    registry_online: bool,
    cli_connected: bool,
    connected_at: u64,
    agent_version: Option<String>,
    allowed_clis: Vec<String>,
    cli_project_ready: bool,
    workspace_provision_ready: bool,
    ai_cli_ready: bool,
    route_a_ready: bool,
    api_runtime_ready: bool,
    server_runtime_ready: bool,
    project_count: i64,
    project_limit: i64,
    project_slots_remaining: i64,
    can_accept_project: bool,
    capacity_label: String,
    capacity_tone: String,
    capacity_warnings: Vec<String>,
}

/// GET /api/admin/nodes/public-dev-handshake - admin diagnostic snapshot.
pub async fn admin_public_dev_handshake(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !admin::check_auth(&headers, &state.admin_token) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error":"admin token required"})),
        )
            .into_response();
    }

    match collect_public_dev_handshake_report(&state).await {
        Ok(report) => {
            Json(serde_json::json!({"ok": true, "public_dev_handshake": report})).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub(super) async fn public_dev_handshake_value(
    state: &AppState,
) -> anyhow::Result<serde_json::Value> {
    Ok(serde_json::json!(
        collect_public_dev_handshake_report(state).await?
    ))
}

async fn collect_public_dev_handshake_report(
    state: &AppState,
) -> anyhow::Result<AdminPublicDevHandshakeReport> {
    let credentials = state.store.list_all_node_credentials()?;
    let owner_map: HashMap<_, _> = match state.store.list_users() {
        Ok(users) => users
            .into_iter()
            .map(|user| (user.id.clone(), (Some(user.account), user.nickname)))
            .collect(),
        Err(e) => {
            tracing::warn!(error = %e, "failed to load users for node public dev handshake report");
            HashMap::new()
        }
    };

    let mut nodes = Vec::new();
    let mut status_counts = BTreeMap::new();
    let mut public_dev_enabled = 0usize;
    let mut online_public_dev = 0usize;
    let mut ready_public_dev = 0usize;
    let mut pending_online_public_dev = 0usize;
    let mut offline_public_dev = 0usize;

    for credential in credentials {
        let Some(runtime) = node_runtime_by_id(state, &credential.agent_id).await? else {
            continue;
        };
        let (public_dev_handshake_ready, public_dev_handshake_status) =
            public_dev_handshake_state_for_runtime(&runtime);
        *status_counts
            .entry(public_dev_handshake_status.clone())
            .or_insert(0) += 1;

        if runtime.public_dev_enabled {
            public_dev_enabled += 1;
            if runtime.online {
                online_public_dev += 1;
                if public_dev_handshake_ready {
                    ready_public_dev += 1;
                } else {
                    pending_online_public_dev += 1;
                }
            } else {
                offline_public_dev += 1;
            }
        }

        let cli_project_ready = runtime.cli_project_ready();
        let workspace_provision_ready = runtime.workspace_provision_ready();
        let ai_cli_ready = runtime
            .dev_runtime
            .as_ref()
            .map(|runtime| runtime.ai_cli_ready)
            .unwrap_or(cli_project_ready);
        let (route_a_ready, api_runtime_ready, server_runtime_ready) =
            super::runtime_route_flags(runtime.dev_runtime.as_ref(), cli_project_ready);
        let capacity = super::capacity_for_response(
            state,
            &runtime.node_id,
            &runtime.owner_user_id,
            &runtime.label,
            runtime.device_name.as_deref(),
            &runtime.display_name,
            runtime.online,
            runtime.cli_connected,
            &runtime.allowed_clis,
            runtime.dev_runtime.clone(),
            runtime.project_count,
        );
        let (owner_account, owner_nickname) = owner_map
            .get(&runtime.owner_user_id)
            .cloned()
            .unwrap_or((None, None));

        nodes.push(AdminPublicDevHandshakeNode {
            node_id: runtime.node_id,
            owner_user_id: runtime.owner_user_id,
            owner_account,
            owner_nickname,
            label: runtime.label,
            device_name: runtime.device_name,
            display_name: runtime.display_name,
            short_id: runtime.short_id,
            install_id: runtime.install_id,
            public_dev_enabled: runtime.public_dev_enabled,
            public_dev_allowed_clis: runtime.public_dev_allowed_clis,
            public_dev_permission_level: runtime.public_dev_permission_level,
            public_dev_handshake_ready,
            public_dev_handshake_status,
            last_handshake_at: runtime.last_handshake_at,
            last_handshake_agent_version: runtime.last_handshake_agent_version,
            last_handshake_allowed_clis: runtime.last_handshake_allowed_clis,
            last_handshake_route_a_ready: runtime.last_handshake_route_a_ready,
            last_handshake_api_runtime_ready: runtime.last_handshake_api_runtime_ready,
            last_handshake_server_runtime_ready: runtime.last_handshake_server_runtime_ready,
            last_handshake_ai_cli_ready: runtime.last_handshake_ai_cli_ready,
            online: runtime.online,
            registry_online: runtime.registry_online,
            cli_connected: runtime.cli_connected,
            connected_at: runtime.connected_at,
            agent_version: runtime.agent_version,
            allowed_clis: runtime.allowed_clis,
            cli_project_ready,
            workspace_provision_ready,
            ai_cli_ready,
            route_a_ready,
            api_runtime_ready,
            server_runtime_ready,
            project_count: runtime.project_count,
            project_limit: capacity.project_limit,
            project_slots_remaining: capacity.project_slots_remaining,
            can_accept_project: capacity.can_accept_project,
            capacity_label: capacity.label,
            capacity_tone: capacity.tone,
            capacity_warnings: capacity.warnings,
        });
    }

    nodes.sort_by(|left, right| {
        right
            .online
            .cmp(&left.online)
            .then_with(|| {
                right
                    .public_dev_handshake_ready
                    .cmp(&left.public_dev_handshake_ready)
            })
            .then_with(|| left.display_name.cmp(&right.display_name))
    });

    Ok(AdminPublicDevHandshakeReport {
        generated_at: chrono::Utc::now().to_rfc3339(),
        summary: AdminPublicDevHandshakeSummary {
            total_nodes: nodes.len(),
            public_dev_enabled,
            online_public_dev,
            ready_public_dev,
            pending_online_public_dev,
            offline_public_dev,
            status_counts,
        },
        nodes,
    })
}

pub(in crate::node_api) fn public_dev_handshake_state_for_runtime(
    node: &NodeRuntime,
) -> (bool, String) {
    public_dev_handshake_state_from_parts(
        node.public_dev_enabled,
        &node.public_dev_allowed_clis,
        node.last_handshake_at.as_deref(),
        node.last_handshake_agent_version.as_deref(),
        &node.last_handshake_allowed_clis,
        node.online,
        node.agent_version.as_deref(),
        &node.allowed_clis,
        node.dev_runtime.as_ref(),
    )
}

pub(super) fn public_dev_handshake_state(
    credential: Option<&NodeCredential>,
    online: bool,
    agent_version: Option<&str>,
    allowed_clis: &[String],
    dev_runtime: Option<&NodeDevRuntimeProfile>,
) -> (bool, String) {
    public_dev_handshake_state_from_parts(
        credential
            .map(|credential| credential.public_dev_enabled)
            .unwrap_or(false),
        credential
            .map(|credential| credential.public_dev_allowed_clis.as_slice())
            .unwrap_or(&[]),
        credential.and_then(|credential| credential.last_handshake_at.as_deref()),
        credential.and_then(|credential| credential.last_handshake_agent_version.as_deref()),
        credential
            .map(|credential| credential.last_handshake_allowed_clis.as_slice())
            .unwrap_or(&[]),
        online,
        agent_version,
        allowed_clis,
        dev_runtime,
    )
}

fn public_dev_handshake_state_from_parts(
    enabled: bool,
    public_allowed_clis: &[String],
    last_handshake_at: Option<&str>,
    last_handshake_agent_version: Option<&str>,
    last_handshake_allowed_clis: &[String],
    online: bool,
    agent_version: Option<&str>,
    allowed_clis: &[String],
    dev_runtime: Option<&NodeDevRuntimeProfile>,
) -> (bool, String) {
    if !enabled {
        return (false, "sharing_disabled".to_string());
    }
    if !online {
        return (false, "offline".to_string());
    }
    if last_handshake_at.is_none() {
        return (false, "waiting_for_handshake".to_string());
    }
    if let (Some(current), Some(last)) = (agent_version, last_handshake_agent_version) {
        if !current.trim().is_empty() && current.trim() != last.trim() {
            return (
                false,
                "version_reconnected_waiting_capabilities".to_string(),
            );
        }
    }
    let advertised_clis = if allowed_clis.is_empty() {
        last_handshake_allowed_clis
    } else {
        allowed_clis
    };
    if !cli_lists_intersect(public_allowed_clis, advertised_clis) {
        return (false, "no_allowed_cli".to_string());
    }
    let route_a_ready = dev_runtime
        .map(|runtime| runtime.route_a_ready)
        .unwrap_or_else(|| !advertised_clis.is_empty());
    let api_runtime_ready = dev_runtime
        .map(|runtime| runtime.api_runtime_ready)
        .unwrap_or(false);
    if route_a_ready || api_runtime_ready {
        (true, "ready".to_string())
    } else {
        (false, "runtime_not_ready".to_string())
    }
}

fn cli_lists_intersect(allowed: &[String], advertised: &[String]) -> bool {
    if allowed.is_empty() {
        return !advertised.is_empty();
    }
    allowed.iter().any(|allowed_cli| {
        advertised
            .iter()
            .any(|advertised_cli| allowed_cli.eq_ignore_ascii_case(advertised_cli))
    })
}

#[cfg(test)]
#[path = "public_dev_tests.rs"]
mod public_dev_tests;

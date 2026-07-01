use anyhow::{anyhow, Result};
use axum::http::StatusCode;
use homecli_proto::AgentToServer;

use crate::{
    node_runtime::{node_runtime_by_id, NodeRuntime},
    pc_node_capacity::{assess_pc_node_capacity, capacity_block_message},
    types::AppState,
};

pub struct PcProjectWorkspace {
    pub workspace_path: String,
    pub git_head: Option<String>,
    pub git_remote_origin: Option<String>,
    pub git_branch: Option<String>,
    pub created: bool,
}

pub async fn resolve_pc_project_node(
    state: &AppState,
    user_id: &str,
    requested_node_id: Option<&str>,
) -> Result<String, (StatusCode, String)> {
    let requested_node_id = requested_node_id
        .map(str::trim)
        .filter(|value| !value.is_empty());

    if let Some(node_id) = requested_node_id {
        let runtime = match node_runtime_by_id(state, node_id).await {
            Ok(Some(runtime)) => runtime,
            Ok(None) => {
                return Err((
                    StatusCode::SERVICE_UNAVAILABLE,
                    format!("PC 节点不在线或未连接 CLI 通道: {node_id}"),
                ))
            }
            Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
        };
        if !runtime.cli_connected {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                format!("PC 节点不在线或未连接 CLI 通道: {node_id}"),
            ));
        }
        if runtime.owner_user_id.trim() != user_id {
            return Err((
                StatusCode::FORBIDDEN,
                format!("PC 节点不属于当前账号: {node_id}"),
            ));
        }
        if !runtime.workspace_provision_ready() {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("PC 节点未启用可创建项目工作区的开发运行时能力: {node_id}"),
            ));
        }
        enforce_capacity(state, &runtime)?;
        return Ok(node_id.to_string());
    }

    let cli_nodes = connected_project_workspace_nodes(state).await;
    let mut candidates = Vec::new();
    let mut blocked_messages = Vec::new();
    for node in cli_nodes {
        match enforce_capacity(state, &node) {
            Ok(()) => candidates.push(node),
            Err((_, message)) => blocked_messages.push(message),
        }
    }
    match candidates.len() {
        1 => Ok(candidates[0].node_id.clone()),
        0 if blocked_messages.is_empty() => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "当前没有在线 PC 节点，无法新建代码项目。请先启动 PC 节点后重试。".into(),
        )),
        0 => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            format!(
                "当前在线 PC 节点暂不能创建新项目：{}",
                blocked_messages.join("；")
            ),
        )),
        _ => Err((
            StatusCode::CONFLICT,
            "检测到多个在线 PC 开发运行时节点，请在请求中指定 node_id".into(),
        )),
    }
}

pub async fn provision_project_workspace(
    state: &AppState,
    node_id: &str,
    user_id: &str,
    project_id: &str,
    name: &str,
    template: &str,
    repo_url: Option<&str>,
    branch: Option<&str>,
) -> Result<PcProjectWorkspace> {
    let msg = state
        .agent_manager
        .dispatch_project_workspace_provision(
            node_id,
            project_id.to_string(),
            user_id.to_string(),
            name.to_string(),
            template.to_string(),
            repo_url.map(ToOwned::to_owned),
            branch.map(ToOwned::to_owned),
        )
        .await?;

    match msg {
        AgentToServer::ProjectWorkspaceProvisioned {
            project_id: returned_project_id,
            workspace_path,
            git_head,
            git_remote_origin,
            git_branch,
            created,
            ..
        } if returned_project_id == project_id => Ok(PcProjectWorkspace {
            workspace_path,
            git_head,
            git_remote_origin,
            git_branch,
            created,
        }),
        AgentToServer::ProjectWorkspaceProvisioned {
            project_id: returned_project_id,
            ..
        } => Err(anyhow!(
            "PC 节点返回了不匹配的 project_id: expected {project_id}, got {returned_project_id}"
        )),
        AgentToServer::ProjectWorkspaceProvisionError { message, .. } => Err(anyhow!(message)),
        other => Err(anyhow!("PC 节点返回了非预期 provisioning 消息: {other:?}")),
    }
}

fn enforce_capacity(
    state: &AppState,
    runtime: &NodeRuntime,
) -> std::result::Result<(), (StatusCode, String)> {
    let latest_snapshot = state
        .store
        .latest_workspace_health_snapshot_for_node(&runtime.node_id)
        .ok()
        .flatten();
    let capacity = assess_pc_node_capacity(runtime, latest_snapshot.as_ref());
    if capacity.can_accept_project {
        Ok(())
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            capacity_block_message(runtime, &capacity),
        ))
    }
}

async fn connected_project_workspace_nodes(state: &AppState) -> Vec<NodeRuntime> {
    let mut nodes = Vec::new();
    for agent in state.agent_manager.list().await {
        match node_runtime_by_id(state, &agent.agent_id).await {
            Ok(Some(runtime)) if runtime.workspace_provision_ready() => nodes.push(runtime),
            Ok(None) => {}
            Ok(Some(_)) => {}
            Err(e) => tracing::warn!(
                node_id = %agent.agent_id,
                error = %e,
                "failed to build node runtime for project provisioning"
            ),
        }
    }
    nodes
}

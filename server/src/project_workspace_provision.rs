use anyhow::{anyhow, Result};
use axum::http::StatusCode;
use homecli_proto::AgentToServer;

use crate::{homecli_agent::AgentSummary, types::AppState};

pub struct PcProjectWorkspace {
    pub workspace_path: String,
    pub git_head: Option<String>,
    pub created: bool,
}

pub async fn resolve_pc_project_node(
    state: &AppState,
    user_id: &str,
    requested_node_id: Option<&str>,
) -> Result<String, (StatusCode, String)> {
    let cli_agents = connected_cli_agents(state).await;
    let requested_node_id = requested_node_id
        .map(str::trim)
        .filter(|value| !value.is_empty());

    if let Some(node_id) = requested_node_id {
        ensure_node_owned_by_user(state, user_id, node_id)?;
        let Some(agent) = cli_agents.iter().find(|agent| agent.agent_id == node_id) else {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                format!("PC 节点不在线或未连接 CLI 通道: {node_id}"),
            ));
        };
        if !supports_project_cli(agent) {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("PC 节点不支持 Codex/Copilot CLI: {node_id}"),
            ));
        }
        return Ok(node_id.to_string());
    }

    let mut owned = Vec::new();
    for agent in cli_agents {
        if matches!(
            state.store.get_node_credential_owner(&agent.agent_id),
            Ok(Some(owner)) if owner == user_id
        ) {
            owned.push(agent.agent_id);
        }
    }

    match owned.len() {
        1 => Ok(owned.remove(0)),
        0 => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "当前没有属于你的在线 PC 节点，无法新建代码项目。请先启动 PC 节点后重试。".into(),
        )),
        _ => Err((
            StatusCode::CONFLICT,
            "检测到多个属于你的在线 PC 节点，请在请求中指定 node_id".into(),
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
) -> Result<PcProjectWorkspace> {
    let msg = state
        .agent_manager
        .dispatch_project_workspace_provision(
            node_id,
            project_id.to_string(),
            user_id.to_string(),
            name.to_string(),
            template.to_string(),
        )
        .await?;

    match msg {
        AgentToServer::ProjectWorkspaceProvisioned {
            project_id: returned_project_id,
            workspace_path,
            git_head,
            created,
            ..
        } if returned_project_id == project_id => Ok(PcProjectWorkspace {
            workspace_path,
            git_head,
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

async fn connected_cli_agents(state: &AppState) -> Vec<AgentSummary> {
    state
        .agent_manager
        .list()
        .await
        .into_iter()
        .filter(supports_project_cli)
        .collect()
}

fn supports_project_cli(agent: &AgentSummary) -> bool {
    agent
        .allowed_clis
        .iter()
        .any(|cli| cli.eq_ignore_ascii_case("copilot") || cli.eq_ignore_ascii_case("codex"))
}

fn ensure_node_owned_by_user(
    state: &AppState,
    user_id: &str,
    node_id: &str,
) -> Result<(), (StatusCode, String)> {
    match state.store.get_node_credential_owner(node_id) {
        Ok(Some(owner)) if owner == user_id => Ok(()),
        Ok(Some(_)) => Err((StatusCode::FORBIDDEN, "不能使用其他用户的 PC 节点".into())),
        Ok(None) => Err((
            StatusCode::FORBIDDEN,
            "该 PC 节点没有绑定到当前登录用户，请在 PC 节点管理页重新登录".into(),
        )),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

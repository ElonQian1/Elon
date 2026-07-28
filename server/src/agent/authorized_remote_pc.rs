use std::sync::Arc;

use tokio::sync::mpsc::UnboundedSender;
use tracing::warn;

use crate::{node_runtime::node_runtime_by_id, store::ProjectAccess, types::AppState};

use super::{
    inspect_pc_agent_workspace, pc_workspace_inspect_error_allows_bound_dispatch,
    pc_workspace_inspect_problem, pc_workspace_inspect_usable, send_optional_progress,
    PcProjectBinding,
};

#[derive(Debug, Clone)]
struct AuthorizedRemotePcNode {
    provider_user_id: String,
    node_id: String,
    display_name: String,
    runtime_permission: String,
}

pub(crate) async fn authorized_remote_project_pc_node_id(
    state: &Arc<AppState>,
    project: &ProjectAccess,
) -> Option<String> {
    let node = first_authorized_remote_project_pc_node(state, project).await?;
    let binding = state
        .store
        .get_project_pc_workspace_binding(&node.provider_user_id, &project.id, &node.node_id)
        .ok()
        .flatten()?;
    if binding.workspace_path.trim().is_empty() {
        return None;
    }
    Some(node.node_id)
}

pub(super) async fn authorized_remote_project_pc_binding(
    state: &Arc<AppState>,
    project: &ProjectAccess,
    tx: Option<&UnboundedSender<String>>,
) -> Option<PcProjectBinding> {
    let node = first_authorized_remote_project_pc_node(state, project).await?;
    let recorded = match state.store.get_project_pc_workspace_binding(
        &node.provider_user_id,
        &project.id,
        &node.node_id,
    ) {
        Ok(binding) => binding,
        Err(error) => {
            warn!(
                project_id = %project.id,
                provider_user_id = %node.provider_user_id,
                agent_id = %node.node_id,
                error = %error,
                "failed to read authorized remote PC workspace binding"
            );
            None
        }
    };
    let workspace = recorded
        .as_ref()
        .map(|binding| binding.workspace_path.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())?;

    send_optional_progress(
        tx,
        &format!(
            "已选中项目授权的远程 PC 节点 {}，正在检查它的项目目录。",
            node.display_name
        ),
    );
    match inspect_pc_agent_workspace(state, &node.node_id, workspace).await {
        Ok(status) if pc_workspace_inspect_usable(&status) => Some(PcProjectBinding {
            agent_id: node.node_id,
            workspace: workspace.to_string(),
            runtime_permission: Some(node.runtime_permission),
        }),
        Ok(status) => {
            warn!(
                project_id = %project.id,
                provider_user_id = %node.provider_user_id,
                agent_id = %node.node_id,
                workspace_path = %workspace,
                problem = %pc_workspace_inspect_problem(&status),
                "authorized remote PC workspace binding is not usable"
            );
            send_optional_progress(
                tx,
                &format!("远程 PC 节点 {} 的项目目录暂不可用。", node.display_name),
            );
            None
        }
        Err(error) if pc_workspace_inspect_error_allows_bound_dispatch(&error) => {
            warn!(
                project_id = %project.id,
                provider_user_id = %node.provider_user_id,
                agent_id = %node.node_id,
                workspace_path = %workspace,
                error = %error,
                "authorized remote PC workspace inspect timed out; dispatching anyway"
            );
            send_optional_progress(
                tx,
                &format!(
                    "远程 PC 节点 {} 的目录巡检未及时返回，已继续直连该节点。",
                    node.display_name
                ),
            );
            Some(PcProjectBinding {
                agent_id: node.node_id,
                workspace: workspace.to_string(),
                runtime_permission: Some(node.runtime_permission),
            })
        }
        Err(error) => {
            warn!(
                project_id = %project.id,
                provider_user_id = %node.provider_user_id,
                agent_id = %node.node_id,
                workspace_path = %workspace,
                error = %error,
                "failed to inspect authorized remote PC workspace binding"
            );
            send_optional_progress(
                tx,
                &format!("远程 PC 节点 {} 暂时无法确认项目目录。", node.display_name),
            );
            None
        }
    }
}

async fn first_authorized_remote_project_pc_node(
    state: &Arc<AppState>,
    project: &ProjectAccess,
) -> Option<AuthorizedRemotePcNode> {
    let authorizations = match state.store.list_project_ai_node_authorizations(&project.id) {
        Ok(authorizations) => authorizations,
        Err(error) => {
            warn!(
                project_id = %project.id,
                error = %error,
                "failed to list project AI node authorizations"
            );
            return None;
        }
    };

    for authorization in authorizations
        .into_iter()
        .filter(|authorization| authorization.enabled)
    {
        let runtime = match node_runtime_by_id(state.as_ref(), &authorization.node_id).await {
            Ok(Some(runtime)) => runtime,
            Ok(None) => continue,
            Err(error) => {
                warn!(
                    project_id = %project.id,
                    agent_id = %authorization.node_id,
                    error = %error,
                    "failed to read authorized remote PC node runtime"
                );
                continue;
            }
        };
        if !runtime.cli_connected {
            continue;
        }
        if !runtime.owner_user_id.trim().is_empty()
            && runtime.owner_user_id != authorization.provider_user_id
        {
            warn!(
                project_id = %project.id,
                agent_id = %authorization.node_id,
                runtime_owner = %runtime.owner_user_id,
                authorization_provider = %authorization.provider_user_id,
                "authorized remote PC node owner mismatch; skipping"
            );
            continue;
        }
        if !route_c3_cli_allowed(&authorization.allowed_clis, &runtime.allowed_clis) {
            continue;
        }
        return Some(AuthorizedRemotePcNode {
            provider_user_id: authorization.provider_user_id,
            node_id: authorization.node_id,
            display_name: runtime.display_name,
            runtime_permission: authorization.permission_level,
        });
    }
    None
}

pub(super) fn route_c3_cli_allowed(authorization_clis: &[String], runtime_clis: &[String]) -> bool {
    let clis = if authorization_clis.is_empty() {
        runtime_clis
    } else {
        authorization_clis
    };
    clis.iter().any(|cli| {
        matches!(
            cli.trim().to_ascii_lowercase().as_str(),
            "codex" | "copilot" | "claude" | "gemini"
        )
    })
}

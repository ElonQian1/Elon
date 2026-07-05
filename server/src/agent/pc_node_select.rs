use std::sync::Arc;

use tracing::warn;

use crate::{
    pc_agent_runtime_choice::PcRuntimeRoutePreference, project_workspace_provision,
    store::ProjectAccess, types::AppState,
};

use super::{
    inspect_pc_agent_workspace, pc_agent_authorized_for_route, pc_agent_belongs_to_user_quiet,
    pc_agent_public_dev_enabled_for_consumer, pc_agent_runtime_ready_for_route,
    pc_workspace_inspect_problem, pc_workspace_inspect_usable, route_allows_public_dev_node,
    usable_project_binding_for_agent, PcProjectBinding,
};

pub(super) async fn connected_pc_agent_for_route(
    state: &Arc<AppState>,
    user_id: &str,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
) -> Option<String> {
    for agent in state.agent_manager.list().await {
        if pc_agent_authorized_for_route(state, user_id, &agent.agent_id, pc_runtime_route)
            && pc_agent_runtime_ready_for_route(state, user_id, &agent.agent_id, pc_runtime_route)
                .await
        {
            return Some(agent.agent_id);
        }
    }
    None
}

pub(super) async fn connected_pc_project_agent_for_route(
    state: &Arc<AppState>,
    user_id: &str,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
) -> Option<String> {
    for agent in state.agent_manager.list().await {
        if !pc_agent_authorized_for_route(state, user_id, &agent.agent_id, pc_runtime_route)
            || !pc_agent_runtime_ready_for_route(state, user_id, &agent.agent_id, pc_runtime_route)
                .await
        {
            continue;
        }
        if pc_agent_can_provision_project_workspace(
            state,
            user_id,
            &agent.agent_id,
            pc_runtime_route,
        )
        .await
        {
            return Some(agent.agent_id);
        }
    }
    None
}

async fn pc_agent_can_provision_project_workspace(
    state: &Arc<AppState>,
    user_id: &str,
    agent_id: &str,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
) -> bool {
    if pc_agent_belongs_to_user_quiet(state, user_id, agent_id) {
        return project_workspace_provision::resolve_pc_project_node(
            state,
            user_id,
            Some(agent_id),
        )
        .await
        .is_ok();
    }
    if !route_allows_public_dev_node(pc_runtime_route)
        || !pc_agent_public_dev_enabled_for_consumer(state, user_id, agent_id)
    {
        return false;
    }
    match crate::node_runtime::node_runtime_by_id(state, agent_id).await {
        Ok(Some(runtime)) => {
            runtime.cli_connected
                && runtime.workspace_provision_ready()
                && pc_agent_runtime_ready_for_route(state, user_id, agent_id, pc_runtime_route)
                    .await
        }
        Ok(None) => false,
        Err(error) => {
            warn!(
                agent_id = %agent_id,
                error = %error,
                "failed to inspect public dev node runtime"
            );
            false
        }
    }
}

pub(super) async fn connected_pc_agent_with_recorded_workspace_binding(
    state: &Arc<AppState>,
    user_id: &str,
    project: &ProjectAccess,
    conversation_id: Option<&str>,
    skip_agent_id: Option<&str>,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
) -> Option<PcProjectBinding> {
    let skip_agent_id = skip_agent_id
        .map(str::trim)
        .filter(|value| !value.is_empty());
    for agent in state.agent_manager.list().await {
        if skip_agent_id == Some(agent.agent_id.as_str()) {
            continue;
        }
        if !pc_agent_authorized_for_route(state, user_id, &agent.agent_id, pc_runtime_route)
            || !pc_agent_runtime_ready_for_route(state, user_id, &agent.agent_id, pc_runtime_route)
                .await
        {
            continue;
        }
        if let Some(binding) = usable_project_binding_for_agent(
            state,
            user_id,
            project,
            conversation_id,
            &agent.agent_id,
            false,
            None,
        )
        .await
        {
            warn!(
                project_id = %project.id,
                user_id = %user_id,
                fallback_agent_id = %binding.agent_id,
                workspace_path = %binding.workspace,
                "PC project will run on another online node using that node's recorded workspace"
            );
            return Some(binding);
        }
    }
    None
}

pub(super) async fn connected_pc_agent_with_existing_workspace(
    state: &Arc<AppState>,
    user_id: &str,
    workspace: &str,
    skip_agent_id: Option<&str>,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
) -> Option<String> {
    let skip_agent_id = skip_agent_id
        .map(str::trim)
        .filter(|value| !value.is_empty());
    for agent in state.agent_manager.list().await {
        if skip_agent_id == Some(agent.agent_id.as_str()) {
            continue;
        }
        if !pc_agent_authorized_for_route(state, user_id, &agent.agent_id, pc_runtime_route) {
            continue;
        }
        match inspect_pc_agent_workspace(state, &agent.agent_id, workspace).await {
            Ok(status) if pc_workspace_inspect_usable(&status) => return Some(agent.agent_id),
            Ok(status) => {
                warn!(
                    agent_id = %agent.agent_id,
                    workspace_path = %workspace,
                    problem = %pc_workspace_inspect_problem(&status),
                    "online PC node does not have a usable matching workspace"
                );
            }
            Err(error) => {
                warn!(
                    agent_id = %agent.agent_id,
                    workspace_path = %workspace,
                    error = %error,
                    "failed to inspect matching workspace on online PC node"
                );
            }
        }
    }
    None
}

use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{info, warn};

use homecli_proto::{AgentToServer, ProjectWorkspaceInspectStatus};

pub(super) const BOUND_PC_NODE_RECONNECT_WAIT_SECS: u64 = 120;
pub(super) const AUTO_BOUND_PC_NODE_RECONNECT_WAIT_SECS: u64 = 15;
const BOUND_PC_NODE_RECONNECT_POLL_MS: u64 = 1_000;

use crate::{
    agent_pc_workspace::{project_cli_runtime_permission, project_requires_pc_workspace},
    agent_routing::requested_agent_for_runtime_route,
    pc_agent_runtime_choice::PcRuntimeRoutePreference,
    pc_node_display::pc_node_progress_name,
    project_workspace_provision,
    route_a_session_lease::{self, RouteARuntimePrewarmResult},
    store::{ProjectAccess, ProjectDevProfile},
    types::{AppState, WsMessage},
};

use super::pc_node_select::{
    connected_pc_agent_for_route, connected_pc_agent_with_existing_workspace,
    connected_pc_agent_with_recorded_workspace_binding, connected_pc_project_agent_for_route,
};
use super::public_dev::{
    pc_agent_authorized_for_bound_node, pc_agent_authorized_for_route,
    pc_agent_belongs_to_user_quiet, pc_agent_runtime_ready_for_route,
    route_targets_public_dev_node,
};
use super::pc_binding_utils::*;
pub(super) use super::pc_binding_utils::{
    append_project_dev_profile_context, clone_url_for_project_access, inspect_pc_agent_workspace,
    is_codex_fallback_error, node_cli_available, pc_agent_is_connected,
    pc_workspace_inspect_error_allows_bound_dispatch, pc_workspace_inspect_problem,
    pc_workspace_inspect_usable, pc_workspace_inspect_usable_for_route,
    send_optional_progress, send_pc_workspace_unavailable_error,
};

#[derive(Debug, Clone)]
pub(super) struct PcProjectBinding {
    pub(super) agent_id: String,
    pub(super) workspace: String,
}

pub(super) async fn resolve_pc_chat_agent(
    state: &Arc<AppState>,
    user_id: &str,
    project: &ProjectAccess,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
) -> Option<String> {
    if let Some(agent_id) = project
        .node_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if route_targets_public_dev_node(pc_runtime_route)
            && pc_agent_belongs_to_user_quiet(state, user_id, agent_id)
        {
            return connected_pc_agent_for_route(state, user_id, pc_runtime_route).await;
        }
        if pc_agent_authorized_for_bound_node(state, user_id, agent_id)
            && pc_agent_is_connected(state, agent_id).await
        {
            return Some(agent_id.to_string());
        }
    }
    connected_pc_agent_for_route(state, user_id, pc_runtime_route).await
}

pub(super) async fn resolve_pc_project_binding(
    state: &Arc<AppState>,
    user_id: &str,
    project: &ProjectAccess,
    conversation_id: Option<&str>,
    tx: Option<&UnboundedSender<String>>,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
    auto_runtime_route: bool,
) -> Option<PcProjectBinding> {
    let reconnect_wait = if auto_runtime_route {
        Some(Duration::from_secs(AUTO_BOUND_PC_NODE_RECONNECT_WAIT_SECS))
    } else {
        Some(Duration::from_secs(BOUND_PC_NODE_RECONNECT_WAIT_SECS))
    };
    resolve_pc_project_binding_with_options(
        state,
        user_id,
        project,
        conversation_id,
        tx,
        reconnect_wait,
        true,
        pc_runtime_route,
    )
    .await
}

pub(crate) async fn prewarm_route_a_runtime_for_project(
    state: &Arc<AppState>,
    user_id: &str,
    project: &ProjectAccess,
    conversation_id: &str,
) -> Option<RouteARuntimePrewarmResult> {
    let binding = resolve_pc_project_binding_with_options(
        state,
        user_id,
        project,
        Some(conversation_id),
        None,
        None,
        false,
        Some(PcRuntimeRoutePreference::RouteA),
    )
    .await?;
    route_a_session_lease::prewarm_result(
        state,
        user_id,
        project,
        conversation_id,
        binding.agent_id,
        binding.workspace,
    )
    .await
}

pub(super) async fn resolve_pc_project_binding_with_options(
    state: &Arc<AppState>,
    user_id: &str,
    project: &ProjectAccess,
    conversation_id: Option<&str>,
    tx: Option<&UnboundedSender<String>>,
    bound_reconnect_wait: Option<Duration>,
    allow_provision: bool,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
) -> Option<PcProjectBinding> {
    let workspace = project
        .workspace_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let mut bound_agent_wrong_owner = false;
    let bound_agent_missing = project
        .node_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none();

    if let Some(agent_id) = project
        .node_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let skip_own_node_for_remote_route = route_targets_public_dev_node(pc_runtime_route)
            && pc_agent_belongs_to_user_quiet(state, user_id, agent_id);
        if skip_own_node_for_remote_route {
            info!(
                project_id = %project.id,
                user_id = %user_id,
                bound_agent_id = %agent_id,
                "remote PC route skips the current user's bound node"
            );
        } else {
            let authorized = pc_agent_authorized_for_bound_node(state, user_id, agent_id)
                || pc_agent_authorized_for_route(state, user_id, agent_id, pc_runtime_route);
            if authorized {
                let connected = if pc_agent_is_connected(state, agent_id).await {
                    true
                } else if let Some(wait) = bound_reconnect_wait {
                    wait_for_bound_pc_agent_reconnect(state, agent_id, tx, wait).await
                } else {
                    false
                };
                if connected {
                    if let Some(binding) = usable_project_binding_for_agent(
                        state,
                        user_id,
                        project,
                        conversation_id,
                        agent_id,
                        true,
                        tx,
                        pc_runtime_route,
                    )
                    .await
                    {
                        return Some(binding);
                    }
                }
            } else {
                bound_agent_wrong_owner = true;
            }
            warn!(
                project_id = %project.id,
                user_id = %user_id,
                bound_agent_id = %agent_id,
                "PC project bound node is not usable for the current user; trying an online user node"
            );
        }
    }

    if let Some(binding) = connected_pc_agent_with_recorded_workspace_binding(
        state,
        user_id,
        project,
        conversation_id,
        project.node_id.as_deref(),
        pc_runtime_route,
    )
    .await
    {
        send_optional_progress(tx, "已找到当前节点记录的项目路径，正在切换执行。");
        return Some(binding);
    }

    if let Some(workspace) = workspace {
        if let Some(fallback_agent_id) = connected_pc_agent_with_existing_workspace(
            state,
            user_id,
            workspace,
            project.node_id.as_deref(),
            pc_runtime_route,
        )
        .await
        {
            warn!(
                project_id = %project.id,
                user_id = %user_id,
                fallback_agent_id = %fallback_agent_id,
                workspace_path = %workspace,
                "PC project will run on another online node that has the same workspace path"
            );
            send_optional_progress(tx, "已找到同一路径可用的在线 PC 节点，正在切换执行。");
            route_a_session_lease::record_verified(
                state,
                user_id,
                project,
                conversation_id,
                &fallback_agent_id,
                workspace,
            )
            .await;
            return Some(PcProjectBinding {
                agent_id: fallback_agent_id,
                workspace: workspace.to_string(),
            });
        }
    }

    if project.source_type != "pc_managed" {
        warn!(
            project_id = %project.id,
            user_id = %user_id,
            workspace_path = ?workspace,
            "local path PC project has no online node with the recorded workspace"
        );
        return None;
    }

    let fallback_agent_id =
        connected_pc_project_agent_for_route(state, user_id, pc_runtime_route).await?;
    warn!(
        project_id = %project.id,
        user_id = %user_id,
        fallback_agent_id = %fallback_agent_id,
        "PC project will run on the current user's online node"
    );
    if project.source_type == "pc_managed" {
        if !allow_provision {
            return None;
        }
        let clone_url = clone_url_for_project_access(project, &fallback_agent_id);
        let can_recreate_without_remote = workspace.is_none()
            || clone_url.is_some()
            || (project.role == "owner" && (bound_agent_wrong_owner || bound_agent_missing));
        if can_recreate_without_remote {
            return provision_pc_project_binding(
                state,
                user_id,
                project,
                &fallback_agent_id,
                clone_url,
                tx,
            )
            .await;
        }

        warn!(
            project_id = %project.id,
            user_id = %user_id,
            fallback_agent_id = %fallback_agent_id,
            workspace_path = ?workspace,
            "PC managed project cannot move to fallback node because no portable git/storage source is available"
        );
        return None;
    }
    let workspace = workspace?;
    Some(PcProjectBinding {
        agent_id: fallback_agent_id,
        workspace: workspace.to_string(),
    })
}

pub(super) async fn wait_for_bound_pc_agent_reconnect(
    state: &Arc<AppState>,
    agent_id: &str,
    tx: Option<&UnboundedSender<String>>,
    wait: Duration,
) -> bool {
    let wait_secs = wait.as_secs().max(1);
    let message = if wait_secs <= 30 {
        format!(
            "自动模式正在等待绑定的 PC 节点短暂重连，最多等待 {wait_secs} 秒；未恢复会继续尝试远程节点或平台 AI。"
        )
    } else {
        format!(
            "绑定的 PC 节点正在重连，最长等待 {} 分钟让原节点恢复，避免把同一项目错误切到其它电脑。",
            (wait_secs + 59) / 60
        )
    };
    send_optional_progress(tx, &message);
    let deadline = tokio::time::Instant::now() + wait;
    loop {
        tokio::time::sleep(Duration::from_millis(BOUND_PC_NODE_RECONNECT_POLL_MS)).await;
        if pc_agent_is_connected(state, agent_id).await {
            send_optional_progress(tx, "绑定的 PC 节点已恢复连接，继续使用原本项目路径执行。");
            return true;
        }
        if tokio::time::Instant::now() >= deadline {
            return false;
        }
    }
}

pub(super) async fn usable_project_binding_for_agent(
    state: &Arc<AppState>,
    user_id: &str,
    project: &ProjectAccess,
    conversation_id: Option<&str>,
    agent_id: &str,
    is_bound_agent: bool,
    tx: Option<&UnboundedSender<String>>,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
) -> Option<PcProjectBinding> {
    let recorded =
        match state
            .store
            .get_project_pc_workspace_binding(user_id, &project.id, agent_id)
        {
            Ok(binding) => binding,
            Err(error) => {
                warn!(
                    project_id = %project.id,
                    user_id = %user_id,
                    agent_id = %agent_id,
                    error = %error,
                    "failed to read node-specific PC workspace binding"
                );
                None
            }
        };

    let workspace = recorded
        .as_ref()
        .map(|binding| binding.workspace_path.as_str())
        .or_else(|| {
            if project.node_id.as_deref() == Some(agent_id) {
                project.workspace_path.as_deref()
            } else {
                None
            }
        })
        .map(str::trim)
        .filter(|value| !value.is_empty())?;

    if route_a_session_lease::is_hot(
        state,
        user_id,
        project,
        conversation_id,
        agent_id,
        workspace,
    )
    .await
    {
        if is_bound_agent {
            send_optional_progress(tx, "已命中 PC 会话热状态，跳过重复工作区巡检。");
        }
        return Some(PcProjectBinding {
            agent_id: agent_id.to_string(),
            workspace: workspace.to_string(),
        });
    }

    let bound_agent_progress_name = if is_bound_agent {
        Some(pc_node_progress_name(state.as_ref(), agent_id).await)
    } else {
        None
    };
    if let Some(name) = bound_agent_progress_name.as_deref() {
        send_optional_progress(
            tx,
            &format!(
                "正在快速检查绑定 PC 节点 {name} 的项目目录；若巡检未及时返回，将继续直连该节点。"
            ),
        );
    }

    match inspect_pc_agent_workspace(state, agent_id, workspace).await {
        Ok(status) if pc_workspace_inspect_usable_for_route(&status, pc_runtime_route) => {
            route_a_session_lease::record_verified(
                state,
                user_id,
                project,
                conversation_id,
                agent_id,
                workspace,
            )
            .await;
            Some(PcProjectBinding {
                agent_id: agent_id.to_string(),
                workspace: workspace.to_string(),
            })
        }
        Ok(status) => {
            let problem = pc_workspace_inspect_problem(&status);
            warn!(
                project_id = %project.id,
                user_id = %user_id,
                agent_id = %agent_id,
                workspace_path = %workspace,
                problem = %problem,
                "PC project workspace binding is not usable"
            );
            if is_bound_agent {
                let message = bound_agent_progress_name
                    .as_deref()
                    .map(|name| {
                        format!("绑定的 PC 节点 {name} 工作区不可用，正在查找其它在线 PC 节点。")
                    })
                    .unwrap_or_else(|| {
                        "绑定的 PC 节点工作区不可用，正在查找其它在线 PC 节点。".to_string()
                    });
                send_optional_progress(tx, &message);
            }
            None
        }
        Err(error) => {
            warn!(
                project_id = %project.id,
                user_id = %user_id,
                agent_id = %agent_id,
                workspace_path = %workspace,
                error = %error,
                "could not inspect PC project workspace binding"
            );
            if is_bound_agent {
                if pc_workspace_inspect_error_allows_bound_dispatch(&error) {
                    let message = bound_agent_progress_name
                        .as_deref()
                        .map(|name| {
                            format!(
                                "绑定的 PC 节点 {name} 工作区检查未及时返回，已跳过巡检并继续直连，避免自动切换到其它电脑。"
                            )
                        })
                        .unwrap_or_else(|| {
                            "绑定的 PC 节点工作区检查未及时返回，已跳过巡检并继续直连，避免自动切换到其它电脑。".to_string()
                        });
                    send_optional_progress(tx, &message);
                    return Some(PcProjectBinding {
                        agent_id: agent_id.to_string(),
                        workspace: workspace.to_string(),
                    });
                }
                let message = bound_agent_progress_name
                    .as_deref()
                    .map(|name| {
                        format!(
                            "绑定的 PC 节点 {name} 暂时无法确认工作区状态，正在查找其它在线 PC 节点。"
                        )
                    })
                    .unwrap_or_else(|| {
                        "绑定的 PC 节点暂时无法确认工作区状态，正在查找其它在线 PC 节点。".to_string()
                    });
                send_optional_progress(tx, &message);
            }
            None
        }
    }
}

pub(super) async fn provision_pc_project_binding(
    state: &Arc<AppState>,
    user_id: &str,
    project: &ProjectAccess,
    agent_id: &str,
    clone_url: Option<String>,
    tx: Option<&UnboundedSender<String>>,
) -> Option<PcProjectBinding> {
    send_optional_progress(
        tx,
        if clone_url.is_some() {
            "当前 PC 节点没有可用项目目录，正在从代码源重建本机工作区。"
        } else {
            "当前 PC 节点没有可用项目目录，正在重新创建本机托管工作区。"
        },
    );
    let template = if project.template.trim().is_empty() {
        "android"
    } else {
        project.template.as_str()
    };
    let provisioned = match project_workspace_provision::provision_project_workspace(
        state,
        agent_id,
        user_id,
        &project.id,
        &project.name,
        template,
        clone_url.as_deref(),
        project.branch.as_deref(),
    )
    .await
    {
        Ok(workspace) => workspace,
        Err(error) => {
            warn!(
                project_id = %project.id,
                user_id = %user_id,
                agent_id = %agent_id,
                error = %error,
                "failed to provision PC project workspace before dispatch"
            );
            return None;
        }
    };

    let local_storage_path = project.storage_repo_path.as_deref().filter(|path| {
        project.storage_repo_url.is_none()
            && project.storage_node_id.as_deref() == Some(agent_id)
            && clone_url.as_deref() == Some(*path)
    });
    let persisted_remote_origin = provisioned
        .git_remote_origin
        .as_deref()
        .filter(|origin| Some(*origin) != local_storage_path)
        .or(project.repo_url.as_deref());

    let persist_result =
        if pc_agent_belongs_to_user_quiet(state, user_id, agent_id) && project.role == "owner" {
            state.store.bind_project_to_pc_workspace(
                user_id,
                &project.id,
                &provisioned.workspace_path,
                agent_id,
                provisioned.git_head.as_deref(),
                persisted_remote_origin,
                provisioned
                    .git_branch
                    .as_deref()
                    .or(project.branch.as_deref()),
            )
        } else {
            state.store.bind_project_member_to_pc_workspace(
                user_id,
                &project.id,
                &provisioned.workspace_path,
                agent_id,
                provisioned.git_head.as_deref(),
                persisted_remote_origin,
                provisioned
                    .git_branch
                    .as_deref()
                    .or(project.branch.as_deref()),
            )
        };
    if let Err(error) = persist_result {
        warn!(
            project_id = %project.id,
            user_id = %user_id,
            agent_id = %agent_id,
            workspace_path = %provisioned.workspace_path,
            error = %error,
            "failed to persist PC project workspace binding"
        );
        return None;
    }

    Some(PcProjectBinding {
        agent_id: agent_id.to_string(),
        workspace: provisioned.workspace_path,
    })
}


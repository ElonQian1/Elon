use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;

use crate::{
    conversation_router::{ensure_user_system_conversation_routes, ConversationRoute},
    node_runtime::user_node_runtimes,
    pc_node_capacity::assess_pc_node_capacity,
    project_workspace_lifecycle::workspace_lifecycle,
    store::{
        ProjectWorkspaceHealthSnapshot, UserArchiveConversationRoute, UserArchiveNode,
        UserArchiveProject, UserArchiveSummary, UserArchiveWorkspaceStatus, MEMORY_SCOPE_PROJECT,
    },
    types::AppState,
};

#[derive(Serialize)]
pub struct UserArchiveResponse {
    pub projects: Vec<UserArchiveProject>,
    pub personal_projects: Vec<UserArchiveProject>,
    pub system_projects: Vec<UserArchiveProject>,
    pub owned_projects: Vec<UserArchiveProject>,
    pub shared_projects: Vec<UserArchiveProject>,
    pub nodes: Vec<UserArchiveNode>,
    pub summary: UserArchiveSummary,
}

pub async fn build_user_archive_response(
    state: &AppState,
    user_id: &str,
) -> Result<UserArchiveResponse> {
    let system_routes = ensure_user_system_conversation_routes(&state.store, user_id)?;
    let mut projects = state.store.list_archive_projects_for_user(user_id)?;
    let nodes = build_archive_nodes(state, user_id).await?;
    let node_by_id = nodes
        .iter()
        .map(|node| (node.node_id.clone(), node.clone()))
        .collect::<HashMap<_, _>>();
    let system_route_by_project = system_routes
        .into_iter()
        .map(|route| (route.project_id.clone(), route))
        .collect::<HashMap<_, _>>();
    let project_ids = projects
        .iter()
        .map(|project| project.project.id.clone())
        .collect::<Vec<_>>();
    let snapshots = state
        .store
        .latest_project_workspace_health_snapshots(&project_ids)?;

    enrich_archive_projects(
        &mut projects,
        &node_by_id,
        &system_route_by_project,
        &snapshots,
        state,
    );

    let mut system_projects = Vec::new();
    let mut owned_projects = Vec::new();
    let mut shared_projects = Vec::new();
    for project in projects.iter().cloned() {
        if project.system_key.is_some() {
            system_projects.push(project);
        } else if project.project.role == "owner" {
            owned_projects.push(project);
        } else {
            shared_projects.push(project);
        }
    }

    let personal_projects = system_projects
        .iter()
        .cloned()
        .chain(owned_projects.iter().cloned())
        .collect::<Vec<_>>();
    let online_node_count = nodes.iter().filter(|node| node.online).count() as i64;

    Ok(UserArchiveResponse {
        projects,
        personal_projects: personal_projects.clone(),
        system_projects: system_projects.clone(),
        owned_projects: owned_projects.clone(),
        shared_projects: shared_projects.clone(),
        summary: UserArchiveSummary {
            total_projects: (personal_projects.len() + shared_projects.len()) as i64,
            system_project_count: system_projects.len() as i64,
            owned_project_count: owned_projects.len() as i64,
            shared_project_count: shared_projects.len() as i64,
            node_count: nodes.len() as i64,
            online_node_count,
        },
        nodes,
    })
}

pub async fn build_user_archive_project_response(
    state: &AppState,
    user_id: &str,
    project_id: &str,
) -> Result<Option<UserArchiveProject>> {
    let system_routes = ensure_user_system_conversation_routes(&state.store, user_id)?;
    let mut projects = state.store.list_archive_projects_for_user(user_id)?;
    let nodes = build_archive_nodes(state, user_id).await?;
    let node_by_id = nodes
        .iter()
        .map(|node| (node.node_id.clone(), node.clone()))
        .collect::<HashMap<_, _>>();
    let system_route_by_project = system_routes
        .into_iter()
        .map(|route| (route.project_id.clone(), route))
        .collect::<HashMap<_, _>>();
    let project_ids = projects
        .iter()
        .map(|project| project.project.id.clone())
        .collect::<Vec<_>>();
    let snapshots = state
        .store
        .latest_project_workspace_health_snapshots(&project_ids)?;

    enrich_archive_projects(
        &mut projects,
        &node_by_id,
        &system_route_by_project,
        &snapshots,
        state,
    );

    Ok(projects
        .into_iter()
        .find(|project| project.project.id == project_id))
}

fn enrich_archive_projects(
    projects: &mut [UserArchiveProject],
    node_by_id: &HashMap<String, UserArchiveNode>,
    system_route_by_project: &HashMap<String, ConversationRoute>,
    snapshots: &HashMap<String, ProjectWorkspaceHealthSnapshot>,
    state: &AppState,
) {
    for project in projects {
        project.conversation_route = Some(conversation_route_for_project(
            project,
            system_route_by_project.get(&project.project.id),
        ));
        project.workspace_status = Some(workspace_status_for_project(
            project,
            node_by_id,
            snapshots.get(&project.project.id),
            state,
        ));
    }
}

fn conversation_route_for_project(
    project: &UserArchiveProject,
    system_route: Option<&ConversationRoute>,
) -> UserArchiveConversationRoute {
    if let Some(route) = system_route {
        return UserArchiveConversationRoute {
            entry_key: route.entry_key.clone(),
            project_id: route.project_id.clone(),
            project_name: route.project_name.clone(),
            conversation_title: route.conversation_title.clone(),
            memory_scope_type: route.memory_scope_type.clone(),
            memory_scope_id: route.memory_scope_id.clone(),
            project_created: route.project_created,
        };
    }

    UserArchiveConversationRoute {
        entry_key: "project".to_string(),
        project_id: project.project.id.clone(),
        project_name: project.project.name.clone(),
        conversation_title: "项目开发会话".to_string(),
        memory_scope_type: MEMORY_SCOPE_PROJECT.to_string(),
        memory_scope_id: Some(project.project.id.clone()),
        project_created: false,
    }
}

fn workspace_status_for_project(
    project: &UserArchiveProject,
    node_by_id: &HashMap<String, UserArchiveNode>,
    snapshot: Option<&ProjectWorkspaceHealthSnapshot>,
    state: &AppState,
) -> UserArchiveWorkspaceStatus {
    let node = project
        .project
        .node_id
        .as_deref()
        .and_then(|node_id| node_by_id.get(node_id));
    let latest_execution = state
        .store
        .latest_project_execution_session(&project.project.id)
        .ok()
        .flatten();
    let mut warnings = Vec::new();
    let execution_target = execution_target_for_workspace_kind(&project.workspace_kind).to_string();

    if project.workspace_kind == "pc_node_workspace" {
        if project
            .project
            .node_id
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
        {
            warnings.push("缺少 PC 节点绑定".to_string());
        }
        if project
            .project
            .workspace_path
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
        {
            warnings.push("缺少 PC 工作区路径".to_string());
        }
        if !node.map(|node| node.online).unwrap_or(false) {
            warnings.push("PC 节点离线".to_string());
        } else if !node.map(|node| node.cli_connected).unwrap_or(false) {
            warnings.push("PC CLI 通道未连接".to_string());
        } else if !node.map(|node| node.cli_project_ready).unwrap_or(false) {
            warnings.push("PC 节点未上报 Codex/Copilot CLI 能力".to_string());
        }
    }
    if matches!(
        latest_execution
            .as_ref()
            .and_then(|session| session.merge_status.as_deref()),
        Some("legacy_no_workspace_status")
    ) {
        warnings.push("最近一次执行来自旧版节点，缺少工作区状态".to_string());
    }
    if let Some(snapshot) = snapshot {
        for warning in &snapshot.warnings {
            push_warning(&mut warnings, warning.clone());
        }
        if snapshot.inspect_error.is_some() {
            push_warning(&mut warnings, "最近一次 PC 工作区巡检失败".to_string());
        }
    }

    let cached_verified_can_run_on_pc =
        snapshot.and_then(|snapshot| snapshot.verified_can_run_on_pc);
    let can_run_on_pc = project.workspace_kind == "pc_node_workspace"
        && project
            .project
            .workspace_path
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && node
            .map(|node| node.cli_connected && node.cli_project_ready)
            .unwrap_or(false)
        && cached_verified_can_run_on_pc.unwrap_or(true);
    let lifecycle = workspace_lifecycle(
        &project.workspace_kind,
        project.project.node_id.as_deref(),
        project.project.workspace_path.as_deref(),
        node.map(|node| node.online).unwrap_or(false),
        can_run_on_pc,
        cached_verified_can_run_on_pc
            .or_else(|| node.map(|node| node.cli_connected && node.cli_project_ready)),
        snapshot.and_then(|snapshot| snapshot.live_inspect.as_ref()),
        warnings.len(),
    );

    UserArchiveWorkspaceStatus {
        project_id: project.project.id.clone(),
        workspace_kind: project.workspace_kind.clone(),
        execution_target,
        health_label: lifecycle.health_label,
        health_tone: lifecycle.health_tone.to_string(),
        recommended_action: lifecycle.recommended_action,
        node_id: project.project.node_id.clone(),
        node_online: node.map(|node| node.online).unwrap_or(false),
        node_cli_connected: node.map(|node| node.cli_connected).unwrap_or(false),
        node_cli_project_ready: node.map(|node| node.cli_project_ready).unwrap_or(false),
        node_display_name: node.map(|node| node.display_name.clone()),
        can_run_on_pc,
        cached_verified_can_run_on_pc,
        latest_health_checked_at: snapshot.map(|snapshot| snapshot.captured_at.clone()),
        latest_health_disk_free_bytes: snapshot.and_then(|snapshot| snapshot.disk_free_bytes),
        latest_execution_status: latest_execution
            .as_ref()
            .map(|session| session.status.clone()),
        latest_execution_merge_status: latest_execution
            .as_ref()
            .and_then(|session| session.merge_status.clone()),
        latest_execution_active_workspace_path: latest_execution
            .as_ref()
            .and_then(|session| session.active_workspace_path.clone()),
        warning_count: warnings.len() as i64,
        warnings,
        recovery_actions: lifecycle.recovery_actions,
    }
}

fn execution_target_for_workspace_kind(workspace_kind: &str) -> &'static str {
    match workspace_kind {
        "system_archive" => "archive_only",
        "pc_node_workspace" => "pc_node",
        "external_workspace" => "external_workspace",
        _ => "server_workspace",
    }
}

async fn build_archive_nodes(state: &AppState, user_id: &str) -> Result<Vec<UserArchiveNode>> {
    Ok(user_node_runtimes(state, user_id)
        .await?
        .into_iter()
        .map(|node| {
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
            let cli_project_ready = node.cli_project_ready();
            UserArchiveNode {
                node_id: node.node_id,
                label: node.label,
                device_name: node.device_name,
                display_name: node.display_name,
                short_id: node.short_id,
                online: node.online,
                cli_connected: node.cli_connected,
                cli_project_ready,
                allowed_clis: node.allowed_clis,
                project_count: capacity.project_count,
                project_limit: capacity.project_limit,
                project_slots_remaining: capacity.project_slots_remaining,
                disk_free_bytes: capacity.disk_free_bytes,
                capacity_label: capacity.label,
                capacity_tone: capacity.tone,
                capacity_warnings: capacity.warnings,
            }
        })
        .collect())
}

fn push_warning(warnings: &mut Vec<String>, warning: String) {
    if !warnings.iter().any(|existing| existing == &warning) {
        warnings.push(warning);
    }
}

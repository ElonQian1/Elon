use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;

use crate::{
    conversation_router::{ensure_user_system_conversation_routes, ConversationRoute},
    store::{
        UserArchiveConversationRoute, UserArchiveNode, UserArchiveProject, UserArchiveSummary,
        UserArchiveWorkspaceStatus, MEMORY_SCOPE_PROJECT,
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
    let nodes = build_archive_nodes(state, user_id, &projects).await?;
    let node_by_id = nodes
        .iter()
        .map(|node| (node.node_id.clone(), node.clone()))
        .collect::<HashMap<_, _>>();
    let system_route_by_project = system_routes
        .into_iter()
        .map(|route| (route.project_id.clone(), route))
        .collect::<HashMap<_, _>>();

    enrich_archive_projects(&mut projects, &node_by_id, &system_route_by_project, state);

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

fn enrich_archive_projects(
    projects: &mut [UserArchiveProject],
    node_by_id: &HashMap<String, UserArchiveNode>,
    system_route_by_project: &HashMap<String, ConversationRoute>,
    state: &AppState,
) {
    for project in projects {
        project.conversation_route = Some(conversation_route_for_project(
            project,
            system_route_by_project.get(&project.project.id),
        ));
        project.workspace_status = Some(workspace_status_for_project(project, node_by_id, state));
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

    UserArchiveWorkspaceStatus {
        project_id: project.project.id.clone(),
        workspace_kind: project.workspace_kind.clone(),
        execution_target,
        node_id: project.project.node_id.clone(),
        node_online: node.map(|node| node.online).unwrap_or(false),
        node_display_name: node.map(|node| node.display_name.clone()),
        can_run_on_pc: project.workspace_kind == "pc_node_workspace"
            && project
                .project
                .workspace_path
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
            && node.map(|node| node.online).unwrap_or(false),
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

async fn build_archive_nodes(
    state: &AppState,
    user_id: &str,
    projects: &[UserArchiveProject],
) -> Result<Vec<UserArchiveNode>> {
    let project_counts = projects
        .iter()
        .filter_map(|project| project.project.node_id.as_deref())
        .fold(HashMap::<String, i64>::new(), |mut counts, node_id| {
            *counts.entry(node_id.to_string()).or_insert(0) += 1;
            counts
        });

    let credentials = state.store.list_node_credentials(user_id)?;
    let mut online_by_id: HashMap<_, _> = state
        .node_registry
        .list_by_owner(user_id)
        .await
        .into_iter()
        .map(|node| (node.node_id.clone(), node))
        .collect();

    let mut nodes = Vec::new();
    for credential in credentials {
        let node_id = credential.agent_id.clone();
        let online = online_by_id.remove(&node_id);
        let short_id = short_node_id(&node_id);
        let label = credential.label.trim().to_string();
        let device_name = online
            .as_ref()
            .and_then(|node| clean_string(node.device_name.as_deref()))
            .or_else(|| clean_string(credential.device_name.as_deref()));
        let display_label = if label == node_id { "" } else { &label };
        let display_name = display_node_name(display_label, device_name.as_deref(), &short_id);
        nodes.push(UserArchiveNode {
            node_id: node_id.clone(),
            label,
            device_name,
            display_name,
            short_id,
            online: online.as_ref().map(|node| node.online).unwrap_or(false),
            project_count: project_counts.get(&node_id).copied().unwrap_or(0),
        });
    }

    for node in online_by_id.into_values() {
        let short_id = short_node_id(&node.node_id);
        let device_name = clean_string(node.device_name.as_deref());
        let display_name = display_node_name("", device_name.as_deref(), &short_id);
        nodes.push(UserArchiveNode {
            node_id: node.node_id.clone(),
            label: String::new(),
            device_name,
            display_name,
            short_id,
            online: node.online,
            project_count: project_counts.get(&node.node_id).copied().unwrap_or(0),
        });
    }

    nodes.sort_by(|left, right| {
        right
            .online
            .cmp(&left.online)
            .then(right.project_count.cmp(&left.project_count))
            .then_with(|| left.display_name.cmp(&right.display_name))
    });
    Ok(nodes)
}

fn clean_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn display_node_name(label: &str, device_name: Option<&str>, short_id: &str) -> String {
    clean_string(Some(label))
        .or_else(|| clean_string(device_name))
        .unwrap_or_else(|| short_id.to_string())
}

fn short_node_id(id: &str) -> String {
    let chars: Vec<char> = id.chars().collect();
    if chars.len() > 16 {
        let tail: String = chars[chars.len() - 14..].iter().collect();
        format!("...{tail}")
    } else {
        id.to_string()
    }
}

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::{collections::HashMap, sync::Arc};

use crate::{
    project_auth::{auth_from_headers, json_error},
    store::{UserArchiveNode, UserArchiveProject, UserArchiveSummary},
    types::AppState,
};

#[derive(Serialize)]
struct UserArchiveResponse {
    projects: Vec<UserArchiveProject>,
    system_projects: Vec<UserArchiveProject>,
    owned_projects: Vec<UserArchiveProject>,
    shared_projects: Vec<UserArchiveProject>,
    nodes: Vec<UserArchiveNode>,
    summary: UserArchiveSummary,
}

pub async fn get_user_archive(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };

    if let Err(e) = state.store.ensure_balloon_project_for_user(&user.id) {
        return json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("初始化手机控制档案失败: {e}"),
        );
    }
    if let Err(e) = state.store.ensure_chat_memory_project_for_user(&user.id) {
        return json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("初始化聊天记忆档案失败: {e}"),
        );
    }

    let projects = match state.store.list_archive_projects_for_user(&user.id) {
        Ok(projects) => projects,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

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

    let nodes = match build_archive_nodes(&state, &user.id, &projects).await {
        Ok(nodes) => nodes,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    let online_node_count = nodes.iter().filter(|node| node.online).count() as i64;

    Json(UserArchiveResponse {
        projects,
        system_projects: system_projects.clone(),
        owned_projects: owned_projects.clone(),
        shared_projects: shared_projects.clone(),
        summary: UserArchiveSummary {
            total_projects: (system_projects.len() + owned_projects.len() + shared_projects.len())
                as i64,
            system_project_count: system_projects.len() as i64,
            owned_project_count: owned_projects.len() as i64,
            shared_project_count: shared_projects.len() as i64,
            node_count: nodes.len() as i64,
            online_node_count,
        },
        nodes,
    })
    .into_response()
}

async fn build_archive_nodes(
    state: &AppState,
    user_id: &str,
    projects: &[UserArchiveProject],
) -> anyhow::Result<Vec<UserArchiveNode>> {
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

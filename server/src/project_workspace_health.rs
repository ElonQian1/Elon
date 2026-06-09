use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use std::sync::Arc;

use homecli_proto::{AgentToServer, ProjectWorkspaceInspectStatus};

use crate::{
    project_auth::{auth_from_headers, json_error, project_access},
    store::ProjectExecutionSession,
    types::AppState,
};

#[derive(Debug, Serialize)]
pub struct ProjectWorkspaceHealthResponse {
    pub project: ProjectWorkspaceHealthProject,
    pub node: Option<ProjectWorkspaceHealthNode>,
    pub latest_execution: Option<ProjectExecutionSession>,
    pub can_run_on_pc: bool,
    pub verified_can_run_on_pc: Option<bool>,
    pub live_inspect: Option<ProjectWorkspaceInspectStatus>,
    pub inspect_error: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ProjectWorkspaceHealthProject {
    pub id: String,
    pub name: String,
    pub source_type: String,
    pub workspace_path: Option<String>,
    pub node_id: Option<String>,
    pub role: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct ProjectWorkspaceHealthNode {
    pub node_id: String,
    pub owner_user_id: Option<String>,
    pub device_name: Option<String>,
    pub online: bool,
    pub connected_at: Option<u64>,
    pub model_count: usize,
}

pub async fn get_project_workspace_health(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e),
    };
    let access = match project_access(&state, &user.id, &project_id) {
        Ok(access) => access,
        Err(e) => return json_error(StatusCode::NOT_FOUND, e),
    };

    let node = match access.node_id.as_deref() {
        Some(node_id) => Some(node_health(&state, node_id).await),
        None => None,
    };
    let latest_execution = match state.store.latest_project_execution_session(&access.id) {
        Ok(session) => session,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e),
    };
    let (live_inspect, inspect_error) = inspect_workspace(
        &state,
        node.as_ref(),
        access.node_id.as_deref(),
        access.workspace_path.as_deref(),
    )
    .await;

    let mut warnings = Vec::new();
    if access.node_id.as_deref().unwrap_or_default().is_empty() {
        warnings.push("项目未绑定 PC 节点".to_string());
    }
    if access
        .workspace_path
        .as_deref()
        .unwrap_or_default()
        .is_empty()
    {
        warnings.push("项目缺少 workspace_path".to_string());
    }
    if matches!(node.as_ref(), Some(node) if !node.online) {
        warnings.push("PC 节点当前不在线".to_string());
    }
    if matches!(
        latest_execution
            .as_ref()
            .and_then(|session| session.merge_status.as_deref()),
        Some("legacy_no_workspace_status")
    ) {
        warnings.push("最近一次执行来自旧版节点，未返回工作区状态".to_string());
    }
    if let Some(error) = inspect_error.as_deref() {
        warnings.push(format!("PC 工作区实时巡检失败：{error}"));
    }
    if let Some(status) = live_inspect.as_ref() {
        append_live_inspect_warnings(status, &mut warnings);
    }

    let can_run_on_pc = access.node_id.is_some()
        && access.workspace_path.is_some()
        && node.as_ref().map(|node| node.online).unwrap_or(false);
    let verified_can_run_on_pc = live_inspect
        .as_ref()
        .map(|status| status.path_exists && status.is_dir && cli_available(status));

    Json(ProjectWorkspaceHealthResponse {
        project: ProjectWorkspaceHealthProject {
            id: access.id,
            name: access.name,
            source_type: access.source_type,
            workspace_path: access.workspace_path,
            node_id: access.node_id,
            role: access.role,
            status: access.status,
        },
        node,
        latest_execution,
        can_run_on_pc,
        verified_can_run_on_pc,
        live_inspect,
        inspect_error,
        warnings,
    })
    .into_response()
}

async fn inspect_workspace(
    state: &AppState,
    node: Option<&ProjectWorkspaceHealthNode>,
    node_id: Option<&str>,
    workspace_path: Option<&str>,
) -> (Option<ProjectWorkspaceInspectStatus>, Option<String>) {
    let Some(node_id) = node_id.filter(|value| !value.trim().is_empty()) else {
        return (None, None);
    };
    let Some(workspace_path) = workspace_path.filter(|value| !value.trim().is_empty()) else {
        return (None, None);
    };
    if !node.map(|node| node.online).unwrap_or(false) {
        return (None, None);
    }

    match state
        .agent_manager
        .dispatch_project_workspace_inspect(node_id, workspace_path.to_string())
        .await
    {
        Ok(AgentToServer::ProjectWorkspaceInspected { status, .. }) => (Some(status), None),
        Ok(AgentToServer::ProjectWorkspaceInspectError { message, .. }) => (None, Some(message)),
        Ok(other) => (
            None,
            Some(format!("unexpected inspect response: {other:?}")),
        ),
        Err(e) => (None, Some(e.to_string())),
    }
}

fn append_live_inspect_warnings(
    status: &ProjectWorkspaceInspectStatus,
    warnings: &mut Vec<String>,
) {
    if !status.path_exists {
        warnings.push("PC 工作区目录不存在，项目无法在该节点继续执行".to_string());
    } else if !status.is_dir {
        warnings.push("PC workspace_path 不是目录".to_string());
    } else if !status.is_git_worktree {
        warnings.push("PC 工作区不是 Git worktree，后续合并/回收能力受限".to_string());
    }

    if status.has_uncommitted_changes {
        let count = status.uncommitted_count.unwrap_or(0);
        warnings.push(format!("PC 工作区存在 {count} 个未提交改动"));
    }
    if !cli_available(status) {
        warnings.push("PC 节点未检测到 codex 或 copilot CLI".to_string());
    }
    if matches!(status.disk_free_bytes, Some(bytes) if bytes < 2 * 1024 * 1024 * 1024) {
        warnings.push("PC 工作区所在磁盘剩余空间低于 2GB".to_string());
    }
}

fn cli_available(status: &ProjectWorkspaceInspectStatus) -> bool {
    status.codex_available || status.copilot_available
}

async fn node_health(state: &AppState, node_id: &str) -> ProjectWorkspaceHealthNode {
    let online_node = state
        .node_registry
        .list_online()
        .await
        .into_iter()
        .find(|node| node.node_id == node_id);
    let owner_user_id = online_node
        .as_ref()
        .map(|node| node.owner_user_id.clone())
        .or_else(|| {
            state
                .store
                .get_node_credential_owner(node_id)
                .ok()
                .flatten()
        });

    ProjectWorkspaceHealthNode {
        node_id: node_id.to_string(),
        owner_user_id,
        device_name: online_node
            .as_ref()
            .and_then(|node| node.device_name.clone()),
        online: online_node
            .as_ref()
            .map(|node| node.online)
            .unwrap_or(false),
        connected_at: online_node.as_ref().map(|node| node.connected_at),
        model_count: online_node
            .as_ref()
            .map(|node| node.models.len())
            .unwrap_or(0),
    }
}

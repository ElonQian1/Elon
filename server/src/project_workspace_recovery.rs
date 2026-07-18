use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use homecli_proto::{AgentToServer, ProjectWorkspaceInspectStatus};

use crate::{
    project_auth::{auth_from_headers, can_edit, json_error, project_access},
    project_storage, project_workspace_provision,
    store::{is_system_project_source_type, ProjectSummary, UserArchiveProject},
    types::AppState,
    user_archive_profile::build_user_archive_project_response,
};

#[derive(Debug, Deserialize)]
pub struct RecoverProjectWorkspaceRequest {
    pub action: String,
    pub node_id: Option<String>,
    #[serde(default, alias = "workspacePath", alias = "localWorkspacePath")]
    pub workspace_path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RecoverProjectWorkspaceResponse {
    pub action: String,
    pub project: ProjectSummary,
    pub archive_project: Option<UserArchiveProject>,
    pub node_id: String,
    pub workspace_path: String,
    pub workspace_created: bool,
    pub message: String,
}

pub async fn recover_project_workspace(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(req): Json<RecoverProjectWorkspaceRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e),
    };
    let access = match project_access(&state, &user.id, &project_id) {
        Ok(access) => access,
        Err(e) => return json_error(StatusCode::NOT_FOUND, e),
    };
    let action = req.action.trim();
    if action == "bind_pc_node" && !can_edit(&access.role) {
        return json_error(
            StatusCode::FORBIDDEN,
            "只有项目 owner、管理员或协作者可以绑定自己的 PC 工作区",
        );
    }
    if action != "bind_pc_node" && access.role != "owner" {
        return json_error(StatusCode::FORBIDDEN, "只有项目 owner 可以修复 PC 工作区");
    }

    let project = match project_summary_for_user(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e),
    };
    if is_system_project_source_type(&project.source_type) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "个人归档项目只保存会话和记忆，不需要创建 PC 工作区",
        );
    }

    let requested_node = req
        .node_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let requested_workspace = req
        .workspace_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let target_node =
        match resolve_recovery_node(&state, &user.id, &project, action, requested_node).await {
            Ok(node_id) => node_id,
            Err((status, message)) => return json_error(status, message),
        };
    if action == "bind_pc_node" {
        if let Some(workspace_path) = requested_workspace {
            return match bind_existing_pc_workspace(
                &state,
                &user.id,
                &project.id,
                &access.role,
                &target_node,
                workspace_path,
            )
            .await
            {
                Ok((rebound, status)) => Json(RecoverProjectWorkspaceResponse {
                    action: action.to_string(),
                    archive_project: build_user_archive_project_response(
                        &state,
                        &user.id,
                        &project.id,
                    )
                    .await
                    .ok()
                    .flatten(),
                    project: rebound,
                    node_id: target_node,
                    workspace_path: status.workspace_path,
                    workspace_created: false,
                    message: "已绑定当前 PC 节点上的现有项目目录".into(),
                })
                .into_response(),
                Err((status, message)) => json_error(status, message),
            };
        }
    }
    let rebuild_repo_url = project_storage::clone_url_for_project_storage(&project, &target_node);
    if requires_git_remote_for_cross_node(action, &project, &target_node)
        && rebuild_repo_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
    {
        return json_error(
            StatusCode::BAD_REQUEST,
            "该项目没有可跨 PC 访问的 Git 仓库，不能在其它 PC 节点重建。请配置外部 repo_url，或在硬盘节点管理页配置 Git 服务基础地址。",
        );
    }

    let provisioned = match project_workspace_provision::provision_project_workspace(
        &state,
        &target_node,
        &user.id,
        &project.id,
        &project.name,
        &project.template,
        rebuild_repo_url.as_deref(),
        project.branch.as_deref(),
    )
    .await
    {
        Ok(workspace) => workspace,
        Err(e) => {
            return json_error(
                StatusCode::SERVICE_UNAVAILABLE,
                format!("PC 节点准备项目工作区失败：{e}"),
            )
        }
    };

    let local_storage_path = project.storage_repo_path.as_deref().filter(|path| {
        project.storage_repo_url.is_none()
            && project.storage_node_id.as_deref() == Some(target_node.as_str())
            && rebuild_repo_url.as_deref() == Some(*path)
    });
    let persisted_remote_origin = provisioned
        .git_remote_origin
        .as_deref()
        .filter(|origin| Some(*origin) != local_storage_path)
        .or(project.repo_url.as_deref());
    let rebound = match persist_project_workspace_binding(
        &state,
        &user.id,
        &access.role,
        &project.id,
        &provisioned.workspace_path,
        &target_node,
        provisioned.git_head.as_deref(),
        persisted_remote_origin,
        provisioned
            .git_branch
            .as_deref()
            .or(project.branch.as_deref()),
    ) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e),
    };
    let archive_project =
        match build_user_archive_project_response(&state, &user.id, &project.id).await {
            Ok(project) => project,
            Err(e) => {
                tracing::warn!(
                    "构造工作区恢复后的项目归档响应失败 project_id={}: {e}",
                    project.id
                );
                None
            }
        };

    Json(RecoverProjectWorkspaceResponse {
        action: action.to_string(),
        project: rebound,
        archive_project,
        node_id: target_node,
        workspace_path: provisioned.workspace_path,
        workspace_created: provisioned.created,
        message: recovery_message(action, provisioned.created),
    })
    .into_response()
}

pub async fn bind_existing_pc_workspace(
    state: &AppState,
    user_id: &str,
    project_id: &str,
    project_role: &str,
    node_id: &str,
    workspace_path: &str,
) -> Result<(ProjectSummary, ProjectWorkspaceInspectStatus), (StatusCode, String)> {
    let target_node =
        project_workspace_provision::resolve_pc_project_node(state, user_id, Some(node_id)).await?;
    let status = match state
        .agent_manager
        .dispatch_project_workspace_inspect(&target_node, workspace_path.to_string())
        .await
    {
        Ok(AgentToServer::ProjectWorkspaceInspected { status, .. }) => status,
        Ok(AgentToServer::ProjectWorkspaceInspectError { message, .. }) => {
            return Err((StatusCode::BAD_REQUEST, message))
        }
        Ok(other) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("PC 节点返回了非预期工作区检查结果: {other:?}"),
            ))
        }
        Err(error) => {
            let message = error.to_string();
            if is_workspace_inspect_timeout(&message) {
                tracing::warn!(
                    project_id = %project_id,
                    user_id = %user_id,
                    node_id = %target_node,
                    workspace_path = %workspace_path,
                    error = %message,
                    "workspace inspect timed out while binding PC node; persisting requested binding so current browser can prefer this node"
                );
                return persist_pc_workspace_binding_after_inspect_timeout(
                    state,
                    user_id,
                    project_id,
                    project_role,
                    &target_node,
                    workspace_path,
                );
            }
            return Err((StatusCode::SERVICE_UNAVAILABLE, message));
        }
    };
    if !workspace_usable_for_binding(&status) {
        return Err((
            StatusCode::BAD_REQUEST,
            workspace_binding_problem(&status).to_string(),
        ));
    }
    let rebound = persist_project_workspace_binding(
        state,
        user_id,
        project_role,
        project_id,
        &status.workspace_path,
        &target_node,
        status.git_head.as_deref(),
        status.git_remote_origin.as_deref(),
        status.git_branch.as_deref(),
    )
    .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    Ok((rebound, status))
}

pub async fn inspect_transient_pc_workspace(
    state: &AppState,
    user_id: &str,
    node_id: &str,
    workspace_path: &str,
) -> Result<(String, ProjectWorkspaceInspectStatus), (StatusCode, String)> {
    let target_node =
        project_workspace_provision::resolve_pc_project_node(state, user_id, Some(node_id)).await?;
    let status = match state
        .agent_manager
        .dispatch_project_workspace_inspect(&target_node, workspace_path.to_string())
        .await
    {
        Ok(AgentToServer::ProjectWorkspaceInspected { status, .. }) => status,
        Ok(AgentToServer::ProjectWorkspaceInspectError { message, .. }) => {
            return Err((StatusCode::BAD_REQUEST, message))
        }
        Ok(other) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("PC 节点返回了非预期临时工作区检查结果: {other:?}"),
            ))
        }
        Err(error) => return Err((StatusCode::SERVICE_UNAVAILABLE, error.to_string())),
    };
    if !workspace_usable_for_binding(&status) {
        return Err((
            StatusCode::BAD_REQUEST,
            workspace_binding_problem(&status).to_string(),
        ));
    }
    Ok((target_node, status))
}

#[allow(clippy::too_many_arguments)]
fn persist_project_workspace_binding(
    state: &AppState,
    user_id: &str,
    project_role: &str,
    project_id: &str,
    workspace_path: &str,
    node_id: &str,
    git_head: Option<&str>,
    repo_url: Option<&str>,
    branch: Option<&str>,
) -> anyhow::Result<ProjectSummary> {
    if project_role == "owner" {
        state.store.bind_project_to_pc_workspace(
            user_id,
            project_id,
            workspace_path,
            node_id,
            git_head,
            repo_url,
            branch,
        )
    } else {
        state.store.bind_project_member_to_pc_workspace(
            user_id,
            project_id,
            workspace_path,
            node_id,
            git_head,
            repo_url,
            branch,
        )
    }
}

fn persist_pc_workspace_binding_after_inspect_timeout(
    state: &AppState,
    user_id: &str,
    project_id: &str,
    project_role: &str,
    node_id: &str,
    workspace_path: &str,
) -> Result<(ProjectSummary, ProjectWorkspaceInspectStatus), (StatusCode, String)> {
    let rebound = persist_project_workspace_binding(
        state,
        user_id,
        project_role,
        project_id,
        workspace_path,
        node_id,
        None,
        None,
        None,
    )
    .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    Ok((rebound, unconfirmed_workspace_status(workspace_path)))
}

fn unconfirmed_workspace_status(workspace_path: &str) -> ProjectWorkspaceInspectStatus {
    ProjectWorkspaceInspectStatus {
        workspace_path: workspace_path.to_string(),
        path_exists: false,
        is_dir: false,
        is_git_worktree: false,
        git_branch: None,
        git_head: None,
        git_remote_origin: None,
        has_uncommitted_changes: false,
        uncommitted_count: None,
        disk_free_bytes: None,
        codex_available: false,
        copilot_available: false,
    }
}

fn is_workspace_inspect_timeout(message: &str) -> bool {
    message.contains("project workspace inspect timeout")
}

fn workspace_usable_for_binding(status: &ProjectWorkspaceInspectStatus) -> bool {
    status.path_exists && status.is_dir && (status.codex_available || status.copilot_available)
}

fn workspace_binding_problem(status: &ProjectWorkspaceInspectStatus) -> &'static str {
    if !status.path_exists {
        "当前 PC 节点上的项目目录不存在"
    } else if !status.is_dir {
        "当前 PC 节点上的 workspace_path 不是目录"
    } else if !status.codex_available && !status.copilot_available {
        "当前 PC 节点未检测到可用 Codex/Copilot CLI"
    } else {
        "当前 PC 节点项目目录不可用"
    }
}

async fn resolve_recovery_node(
    state: &AppState,
    user_id: &str,
    project: &ProjectSummary,
    action: &str,
    requested_node: Option<&str>,
) -> Result<String, (StatusCode, String)> {
    match action {
        "recreate_workspace" => {
            let node_id = requested_node
                .or(project.node_id.as_deref())
                .ok_or_else(|| {
                    (
                        StatusCode::BAD_REQUEST,
                        "项目尚未绑定 PC 节点，请选择在线 PC 节点后执行绑定".to_string(),
                    )
                })?;
            project_workspace_provision::resolve_pc_project_node(state, user_id, Some(node_id))
                .await
        }
        "migrate_workspace" | "bind_pc_node" => {
            project_workspace_provision::resolve_pc_project_node(state, user_id, requested_node)
                .await
        }
        "repair_cli" => Err((
            StatusCode::BAD_REQUEST,
            "CLI 环境需要在 PC 节点上安装或修复 Codex/Copilot 后重试，服务器不能远程代装。".into(),
        )),
        _ => Err((
            StatusCode::BAD_REQUEST,
            "action 必须为 recreate_workspace / migrate_workspace / bind_pc_node / repair_cli"
                .into(),
        )),
    }
}

fn project_summary_for_user(
    state: &AppState,
    user_id: &str,
    project_id: &str,
) -> anyhow::Result<ProjectSummary> {
    state
        .store
        .list_projects_for_user(user_id)?
        .into_iter()
        .find(|project| project.id == project_id)
        .ok_or_else(|| anyhow::anyhow!("项目不存在，或当前用户无权访问"))
}

fn requires_git_remote_for_cross_node(
    action: &str,
    project: &ProjectSummary,
    target_node: &str,
) -> bool {
    match action {
        "migrate_workspace" => true,
        "bind_pc_node" => {
            project
                .workspace_path
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
                && project.node_id.as_deref() != Some(target_node)
        }
        _ => false,
    }
}

fn recovery_message(action: &str, created: bool) -> String {
    match (action, created) {
        ("recreate_workspace", true) => "已在绑定 PC 节点重新创建项目目录".into(),
        ("recreate_workspace", false) => "绑定 PC 节点上的项目目录已存在，已重新确认绑定".into(),
        ("migrate_workspace", true) => "已迁移到新的 PC 节点并创建项目目录".into(),
        ("migrate_workspace", false) => "已迁移到新的 PC 节点并复用现有项目目录".into(),
        ("bind_pc_node", true) => "已绑定 PC 节点并创建项目目录".into(),
        ("bind_pc_node", false) => "已绑定 PC 节点并复用现有项目目录".into(),
        _ => "PC 工作区已处理".into(),
    }
}

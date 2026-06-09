use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, json_error, project_access},
    project_workspace_provision,
    store::{is_system_project_source_type, ProjectSummary, UserArchiveProject},
    types::AppState,
    user_archive_profile::build_user_archive_project_response,
};

#[derive(Debug, Deserialize)]
pub struct RecoverProjectWorkspaceRequest {
    pub action: String,
    pub node_id: Option<String>,
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
    if access.role != "owner" {
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

    let action = req.action.trim();
    let requested_node = req
        .node_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let target_node =
        match resolve_recovery_node(&state, &user.id, &project, action, requested_node).await {
            Ok(node_id) => node_id,
            Err((status, message)) => return json_error(status, message),
        };

    let provisioned = match project_workspace_provision::provision_project_workspace(
        &state,
        &target_node,
        &user.id,
        &project.id,
        &project.name,
        &project.template,
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

    let rebound = match state.store.bind_project_to_pc_workspace(
        &user.id,
        &project.id,
        &provisioned.workspace_path,
        &target_node,
        provisioned.git_head.as_deref(),
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

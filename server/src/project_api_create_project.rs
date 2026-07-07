use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;
use crate::{
    project_auth::{auth_from_headers, json_error},
    project_storage, project_workspace_provision,
    types::AppState,
};
use super::{CreateProjectRequest, archive_project_payload, clean_optional_string};

pub async fn create_project(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<CreateProjectRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };

    let execution_target = req
        .execution_target
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("pc_node");
    if !["pc_node", "pc"].contains(&execution_target) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "服务器磁盘不再承载新代码项目，请选择在线 PC 节点创建项目",
        );
    }
    let requested_repo_url = clean_optional_string(req.repo_url.as_deref());
    let requested_branch = clean_optional_string(req.branch.as_deref());
    if requested_repo_url.is_some()
        && req
            .storage_node_id
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
    {
        return json_error(
            StatusCode::BAD_REQUEST,
            "repo_url 和 storage_node_id 不能同时指定：外部 Git 远端与平台硬盘节点二选一",
        );
    }

    let create_result = match state.store.create_project(
        &user.id,
        &req.name,
        req.description.as_deref(),
        req.template.as_deref(),
    ) {
        Ok(result) => result,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };

    let reused_existing = create_result.reused_existing;
    let mut project = create_result.project;
    if requested_repo_url.is_some() || requested_branch.is_some() {
        project = match state.store.update_project_git_metadata(
            &user.id,
            &project.id,
            requested_repo_url.as_deref(),
            requested_branch.as_deref(),
        ) {
            Ok(project) => project,
            Err(e) => {
                if !reused_existing {
                    let _ = state.store.purge_project_records(&user.id, &project.id);
                }
                return json_error(StatusCode::BAD_REQUEST, e.to_string());
            }
        };
    }
    if reused_existing {
        if project
            .node_id
            .as_deref()
            .is_some_and(|value| !value.is_empty())
            && project
                .workspace_path
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
        {
            let existing_node_id = project.node_id.clone();
            let archive_project = archive_project_payload(&state, &user.id, &project.id).await;
            return Json(serde_json::json!({
                "project": project,
                "archive_project": archive_project,
                "reused_existing": true,
                "node_id": existing_node_id,
                "provisioned": false,
            }))
            .into_response();
        }
        return json_error(
            StatusCode::CONFLICT,
            "同名项目已存在但尚未绑定 PC 工作区，请先绑定 PC 节点或更换项目名称",
        );
    }

    let node_id = match project_workspace_provision::resolve_pc_project_node(
        &state,
        &user.id,
        req.node_id.as_deref(),
    )
    .await
    {
        Ok(node_id) => node_id,
        // 节点暂时离线 → 保留项目记录，返回 pending 状态。
        // 用户可以先进入项目，等节点上线后工作区会在首次发起任务时自动初始化。
        Err((StatusCode::SERVICE_UNAVAILABLE, _)) => {
            let archive_project = archive_project_payload(&state, &user.id, &project.id).await;
            return (
                StatusCode::ACCEPTED,
                Json(serde_json::json!({
                    "project": project,
                    "archive_project": archive_project,
                    "reused_existing": false,
                    "workspace_status": "pending",
                    "node_id": req.node_id,
                    "message": "项目已创建，PC 节点上线后工作区将自动初始化",
                })),
            )
                .into_response();
        }
        // 其他错误（如节点配置错误、不支持 CLI）→ 回滚项目记录，返回错误
        Err((status, message)) => {
            let _ = state.store.purge_project_records(&user.id, &project.id);
            return json_error(status, message);
        }
    };

    let mut provision_repo_url = project.repo_url.clone();
    let mut provision_branch = project.branch.clone();
    let mut storage_repo_created = None;
    let mut local_storage_clone_path = None;
    let skip_storage = req.skip_storage.unwrap_or(false);
    if provision_repo_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
        && !skip_storage
    {
        let prepared_storage = match project_storage::maybe_prepare_project_storage_repo(
            &state,
            &user.id,
            &project.id,
            &project.name,
            project.branch.as_deref(),
            req.storage_node_id.as_deref(),
            Some(&node_id),
        )
        .await
        {
            Ok(repo) => repo,
            Err((status, message)) => {
                let _ = state.store.purge_project_records(&user.id, &project.id);
                return json_error(status, message);
            }
        };
        if let Some(storage) = prepared_storage {
            let clone_url = match project_storage::clone_url_for_prepared_storage(
                &storage, &node_id,
            ) {
                Some(url) => url,
                None => {
                    let _ = state.store.purge_project_records(&user.id, &project.id);
                    return json_error(
                        StatusCode::BAD_REQUEST,
                        "硬盘节点已创建项目仓库，但没有可用的跨 PC Git 地址。请升级硬盘节点 node-agent、配置外部 Git 服务基础地址，或选择同一台 PC 同时作为硬盘和计算节点。",
                    );
                }
            };
            if clone_url == storage.storage_repo_path {
                local_storage_clone_path = Some(clone_url.clone());
            }
            storage_repo_created = Some(storage.created);
            project = match state.store.bind_project_storage_repo(
                &user.id,
                &project.id,
                &storage.node_id,
                &storage.storage_repo_path,
                storage.storage_repo_url.as_deref(),
                storage.storage_worktree_path.as_deref(),
                storage.branch.as_deref(),
            ) {
                Ok(project) => project,
                Err(e) => {
                    let _ = state.store.purge_project_records(&user.id, &project.id);
                    return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
                }
            };
            provision_repo_url = Some(clone_url);
            provision_branch = storage.branch.or(project.branch.clone());
        }
    }

    let provisioned = match project_workspace_provision::provision_project_workspace(
        &state,
        &node_id,
        &user.id,
        &project.id,
        &project.name,
        &project.template,
        provision_repo_url.as_deref(),
        provision_branch.as_deref(),
    )
    .await
    {
        Ok(workspace) => workspace,
        Err(e) => {
            let _ = state.store.purge_project_records(&user.id, &project.id);
            return json_error(
                StatusCode::SERVICE_UNAVAILABLE,
                format!("PC 节点创建项目工作区失败：{e}"),
            );
        }
    };

    let persisted_remote_origin = provisioned
        .git_remote_origin
        .as_deref()
        .filter(|origin| Some(*origin) != local_storage_clone_path.as_deref())
        .or(project.repo_url.as_deref());
    let project = match state.store.bind_project_to_pc_workspace(
        &user.id,
        &project.id,
        &provisioned.workspace_path,
        &node_id,
        provisioned.git_head.as_deref(),
        persisted_remote_origin,
        provisioned
            .git_branch
            .as_deref()
            .or(provision_branch.as_deref())
            .or(project.branch.as_deref()),
    ) {
        Ok(project) => project,
        Err(e) => {
            let _ = state.store.purge_project_records(&user.id, &project.id);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
        }
    };

    let storage_node_id = project.storage_node_id.clone();
    let storage_worktree_path = project.storage_worktree_path.clone();
    let archive_project = archive_project_payload(&state, &user.id, &project.id).await;
    Json(serde_json::json!({
        "project": project,
        "archive_project": archive_project,
        "reused_existing": false,
        "node_id": node_id,
        "storage_node_id": storage_node_id,
        "storage_worktree_path": storage_worktree_path,
        "provisioned": true,
        "workspace_created": provisioned.created,
        "storage_repo_created": storage_repo_created,
    }))
    .into_response()
}


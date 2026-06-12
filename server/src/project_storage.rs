use anyhow::Result;
use axum::http::StatusCode;
use homecli_proto::AgentToServer;

use crate::{
    node_runtime::{node_runtime_by_id, user_node_runtimes, NodeRuntime},
    store::ProjectSummary,
    types::AppState,
};

pub struct PreparedStorageRepo {
    pub node_id: String,
    pub storage_repo_path: String,
    pub storage_repo_url: Option<String>,
    pub branch: Option<String>,
    pub created: bool,
}

pub async fn maybe_prepare_project_storage_repo(
    state: &AppState,
    user_id: &str,
    project_id: &str,
    name: &str,
    branch: Option<&str>,
    requested_storage_node_id: Option<&str>,
    compute_node_id: Option<&str>,
) -> Result<Option<PreparedStorageRepo>, (StatusCode, String)> {
    let node =
        match resolve_storage_node(state, user_id, requested_storage_node_id, compute_node_id)
            .await?
        {
            Some(node) => node,
            None => return Ok(None),
        };
    if compute_node_id.is_some_and(|compute| compute != node.node_id)
        && node
            .storage
            .as_ref()
            .and_then(|storage| storage.git_base_url.as_ref())
            .is_none()
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "硬盘节点没有配置 Git 服务基础地址，不能给其它 PC 节点提供跨机 clone。请配置后重试，或选择同一台 PC 同时作为硬盘和计算节点。"
                .into(),
        ));
    }
    let msg = state
        .agent_manager
        .dispatch_project_storage_repo_prepare(
            &node.node_id,
            project_id.to_string(),
            user_id.to_string(),
            name.to_string(),
            branch.map(ToOwned::to_owned),
        )
        .await
        .map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, e.to_string()))?;

    match msg {
        AgentToServer::ProjectStorageRepoReady {
            project_id: returned_project_id,
            storage_repo_path,
            storage_repo_url,
            branch,
            created,
            ..
        } if returned_project_id == project_id => Ok(Some(PreparedStorageRepo {
            node_id: node.node_id,
            storage_repo_path,
            storage_repo_url,
            branch,
            created,
        })),
        AgentToServer::ProjectStorageRepoReady {
            project_id: returned_project_id,
            ..
        } => Err((
            StatusCode::BAD_GATEWAY,
            format!(
                "硬盘节点返回了不匹配的 project_id: expected {project_id}, got {returned_project_id}"
            ),
        )),
        AgentToServer::ProjectStorageRepoError { message, .. } => {
            Err((StatusCode::SERVICE_UNAVAILABLE, message))
        }
        other => Err((
            StatusCode::BAD_GATEWAY,
            format!("硬盘节点返回了非预期消息: {other:?}"),
        )),
    }
}

pub fn clone_url_for_prepared_storage(
    storage: &PreparedStorageRepo,
    compute_node_id: &str,
) -> Option<String> {
    storage
        .storage_repo_url
        .clone()
        .or_else(|| (storage.node_id == compute_node_id).then(|| storage.storage_repo_path.clone()))
}

pub fn clone_url_for_project_storage(
    project: &ProjectSummary,
    target_node_id: &str,
) -> Option<String> {
    project
        .repo_url
        .clone()
        .or_else(|| project.storage_repo_url.clone())
        .or_else(|| {
            (project.storage_node_id.as_deref() == Some(target_node_id))
                .then(|| project.storage_repo_path.clone())
                .flatten()
        })
}

async fn resolve_storage_node(
    state: &AppState,
    user_id: &str,
    requested_storage_node_id: Option<&str>,
    preferred_node_id: Option<&str>,
) -> Result<Option<NodeRuntime>, (StatusCode, String)> {
    let requested = requested_storage_node_id
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(node_id) = requested {
        let node = node_runtime_by_id(state, node_id)
            .await
            .map_err(internal_error)?
            .ok_or_else(|| {
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    format!("硬盘节点不在线或未连接: {node_id}"),
                )
            })?;
        ensure_storage_node_owner(user_id, &node)?;
        ensure_storage_ready(&node)?;
        return Ok(Some(node));
    }

    let nodes = user_node_runtimes(state, user_id)
        .await
        .map_err(internal_error)?;
    let mut candidates = nodes
        .into_iter()
        .filter(|node| node.owner_user_id == user_id && node.storage_ready())
        .collect::<Vec<_>>();
    if let Some(preferred) = preferred_node_id {
        if let Some(index) = candidates.iter().position(|node| node.node_id == preferred) {
            return Ok(Some(candidates.swap_remove(index)));
        }
    }
    candidates.sort_by(|left, right| {
        right
            .storage
            .as_ref()
            .and_then(|storage| storage.disk_free_bytes)
            .cmp(
                &left
                    .storage
                    .as_ref()
                    .and_then(|storage| storage.disk_free_bytes),
            )
    });
    Ok(candidates.into_iter().next())
}

fn ensure_storage_node_owner(
    user_id: &str,
    node: &NodeRuntime,
) -> Result<(), (StatusCode, String)> {
    if node.owner_user_id == user_id {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            "只能使用自己账号注册的硬盘节点保存项目代码".into(),
        ))
    }
}

fn ensure_storage_ready(node: &NodeRuntime) -> Result<(), (StatusCode, String)> {
    if !node.cli_connected {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            format!("硬盘节点未连接 PC 通道: {}", node.node_id),
        ));
    }
    if !node.storage_ready() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("该 PC 节点未启用硬盘服务: {}", node.node_id),
        ));
    }
    Ok(())
}

fn internal_error(error: anyhow::Error) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

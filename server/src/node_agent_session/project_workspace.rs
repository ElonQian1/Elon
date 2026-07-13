//! PC 项目工作区与存储协议任务。
//! 会话入口只负责协议分发和日志，本模块持有数据根迁移锁并完成阻塞 I/O 与响应构造。

use std::sync::Arc;

use homecli_proto::AgentToServer;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use crate::{pc_storage_repo, pc_workspace_provisioner, ws_text, NodeRuntime};

pub(super) struct ProvisionRequest {
    pub(super) req_id: String,
    pub(super) project_id: String,
    pub(super) user_id: String,
    pub(super) name: String,
    pub(super) template: String,
    pub(super) repo_url: Option<String>,
    pub(super) branch: Option<String>,
}

pub(super) struct PrepareStorageRequest {
    pub(super) req_id: String,
    pub(super) project_id: String,
    pub(super) user_id: String,
    pub(super) name: String,
    pub(super) branch: Option<String>,
    pub(super) access_token: Option<String>,
    pub(super) prepare_worktree: bool,
}

pub(super) struct CleanupRequest {
    pub(super) req_id: String,
    pub(super) project_id: String,
    pub(super) workspace_path: String,
}

pub(super) fn spawn_provision(
    runtime: Arc<NodeRuntime>,
    out_tx: mpsc::UnboundedSender<Message>,
    request: ProvisionRequest,
) {
    tokio::spawn(async move {
        let ProvisionRequest {
            req_id,
            project_id,
            user_id,
            name,
            template,
            repo_url,
            branch,
        } = request;
        let project_id_for_error = project_id.clone();
        let transition = runtime
            .node_data_root_transition
            .clone()
            .lock_owned()
            .await;
        let workspace_root = runtime
            .node_data_root
            .read()
            .await
            .paths
            .as_ref()
            .map(elon_pc_dev_runtime::NodeDataPaths::workspaces);
        let response = if let Some(workspace_root) = workspace_root {
            match tokio::task::spawn_blocking(move || {
                let _transition = transition;
                pc_workspace_provisioner::provision_project_workspace_in(
                    &workspace_root,
                    pc_workspace_provisioner::ProjectWorkspaceRequest {
                        project_id,
                        user_id,
                        name,
                        template,
                        repo_url,
                        branch,
                    },
                )
            })
            .await
            {
                Ok(Ok(result)) => AgentToServer::ProjectWorkspaceProvisioned {
                    req_id,
                    project_id: project_id_for_error,
                    workspace_path: result.workspace_path,
                    git_head: result.git_head,
                    git_remote_origin: result.git_remote_origin,
                    git_branch: result.git_branch,
                    created: result.created,
                },
                Ok(Err(error)) => AgentToServer::ProjectWorkspaceProvisionError {
                    req_id,
                    project_id: project_id_for_error,
                    message: error.to_string(),
                },
                Err(error) => AgentToServer::ProjectWorkspaceProvisionError {
                    req_id,
                    project_id: project_id_for_error,
                    message: format!("PC 项目工作区准备任务异常结束: {error}"),
                },
            }
        } else {
            drop(transition);
            AgentToServer::ProjectWorkspaceProvisionError {
                req_id,
                project_id: project_id_for_error,
                message: "PC 节点尚未配置有效的统一数据根，已阻止项目工作区回落到系统盘".to_string(),
            }
        };
        let _ = out_tx.send(ws_text(&response));
    });
}

pub(super) fn spawn_prepare_storage(
    runtime: Arc<NodeRuntime>,
    out_tx: mpsc::UnboundedSender<Message>,
    request: PrepareStorageRequest,
) {
    tokio::spawn(async move {
        let PrepareStorageRequest {
            req_id,
            project_id,
            user_id,
            name,
            branch,
            access_token,
            prepare_worktree,
        } = request;
        let project_id_for_error = project_id.clone();
        let transition = runtime
            .node_data_root_transition
            .clone()
            .lock_owned()
            .await;
        let data_paths = runtime.node_data_root.read().await.paths.clone();
        let response = if let Some(data_paths) = data_paths {
            let mut storage_settings = runtime.storage_settings.read().await.clone();
            storage_settings.root_path =
                Some(data_paths.storage().to_string_lossy().to_string());
            match tokio::task::spawn_blocking(move || {
                let _transition = transition;
                pc_storage_repo::prepare_project_storage_repo(
                    &storage_settings,
                    pc_storage_repo::StorageRepoRequest {
                        project_id,
                        user_id,
                        name,
                        branch,
                        access_token,
                        prepare_worktree,
                    },
                )
            })
            .await
            {
                Ok(Ok(result)) => AgentToServer::ProjectStorageRepoReady {
                    req_id,
                    project_id: project_id_for_error,
                    storage_repo_path: result.storage_repo_path,
                    storage_repo_url: result.storage_repo_url,
                    storage_worktree_path: result.storage_worktree_path,
                    branch: result.branch,
                    created: result.created,
                },
                Ok(Err(error)) => AgentToServer::ProjectStorageRepoError {
                    req_id,
                    project_id: project_id_for_error,
                    message: error.to_string(),
                },
                Err(error) => AgentToServer::ProjectStorageRepoError {
                    req_id,
                    project_id: project_id_for_error,
                    message: format!("PC 项目存储准备任务异常结束: {error}"),
                },
            }
        } else {
            drop(transition);
            AgentToServer::ProjectStorageRepoError {
                req_id,
                project_id: project_id_for_error,
                message: "PC 节点尚未配置有效的统一数据根，已阻止项目存储回落到系统盘".to_string(),
            }
        };
        let _ = out_tx.send(ws_text(&response));
    });
}

pub(super) fn spawn_cleanup(
    runtime: Arc<NodeRuntime>,
    out_tx: mpsc::UnboundedSender<Message>,
    request: CleanupRequest,
) {
    tokio::spawn(async move {
        let CleanupRequest {
            req_id,
            project_id,
            workspace_path,
        } = request;
        let project_id_for_error = project_id.clone();
        let transition = runtime
            .node_data_root_transition
            .clone()
            .lock_owned()
            .await;
        let workspace_root = runtime
            .node_data_root
            .read()
            .await
            .paths
            .as_ref()
            .map(elon_pc_dev_runtime::NodeDataPaths::workspaces);
        let response = if let Some(workspace_root) = workspace_root {
            match tokio::task::spawn_blocking(move || {
                let _transition = transition;
                pc_workspace_provisioner::cleanup_project_workspace_in(
                    &workspace_root,
                    &project_id,
                    &workspace_path,
                )
            })
            .await
            {
                Ok(Ok(result)) => AgentToServer::ProjectWorkspaceCleaned {
                    req_id,
                    project_id: project_id_for_error,
                    removed_paths: result.removed_paths,
                    skipped_paths: result.skipped_paths,
                },
                Ok(Err(error)) => AgentToServer::ProjectWorkspaceCleanupError {
                    req_id,
                    project_id: project_id_for_error,
                    message: error.to_string(),
                },
                Err(error) => AgentToServer::ProjectWorkspaceCleanupError {
                    req_id,
                    project_id: project_id_for_error,
                    message: format!("PC 项目工作区清理任务异常结束: {error}"),
                },
            }
        } else {
            drop(transition);
            AgentToServer::ProjectWorkspaceCleanupError {
                req_id,
                project_id: project_id_for_error,
                message: "PC 节点尚未配置有效的统一数据根，拒绝按旧用户目录清理项目工作区".to_string(),
            }
        };
        let _ = out_tx.send(ws_text(&response));
    });
}

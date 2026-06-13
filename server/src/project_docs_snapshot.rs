//! Unified project-document snapshot loading for all project-doc surfaces.
use homecli_proto::{AgentToServer, ProjectDocumentsSnapshot};
use std::path::Path;

use crate::{
    project_docs_scan::{build_snapshot, collect_project_documents},
    store::ProjectAccess,
    types::AppState,
};

pub(crate) async fn load_project_documents_snapshot(
    state: &AppState,
    project: &ProjectAccess,
) -> ProjectDocumentsSnapshot {
    let workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
    if let (Some(node_id), Some(workspace_path)) = (
        project
            .node_id
            .as_deref()
            .filter(|value| !value.trim().is_empty()),
        project
            .workspace_path
            .as_deref()
            .filter(|value| !value.trim().is_empty()),
    ) {
        match state
            .agent_manager
            .dispatch_project_documents_read(node_id, workspace_path.to_string())
            .await
        {
            Ok(AgentToServer::ProjectDocumentsRead { mut snapshot, .. }) => {
                let source = if snapshot.source.is_empty() {
                    "pc_node".to_string()
                } else {
                    format!("pc_node:{}", snapshot.source)
                };
                snapshot.revision = build_snapshot(
                    snapshot.workspace_path.clone(),
                    &source,
                    snapshot.documents.clone(),
                    snapshot.warnings.clone(),
                )
                .revision;
                snapshot.source = source;
                return snapshot;
            }
            Ok(AgentToServer::ProjectDocumentsReadError { message, .. }) => {
                return fallback_snapshot(&workspace, format!("PC 节点读取文档失败：{message}"));
            }
            Ok(other) => {
                return fallback_snapshot(
                    &workspace,
                    format!("PC 节点返回了非文档响应：{other:?}"),
                );
            }
            Err(error) => {
                return fallback_snapshot(&workspace, format!("PC 节点暂不可读取文档：{error}"));
            }
        }
    }

    collect_project_documents(&workspace).unwrap_or_else(|error| {
        build_snapshot(
            workspace.to_string_lossy().to_string(),
            "read_error",
            Vec::new(),
            vec![format!("读取项目文档失败：{error}")],
        )
    })
}

fn fallback_snapshot(workspace: &Path, warning: String) -> ProjectDocumentsSnapshot {
    let mut snapshot = collect_project_documents(workspace).unwrap_or_else(|error| {
        build_snapshot(
            workspace.to_string_lossy().to_string(),
            "read_error",
            Vec::new(),
            vec![format!("服务器本地读取也失败：{error}")],
        )
    });
    snapshot.warnings.insert(0, warning);
    snapshot.source = format!("server_fallback:{}", snapshot.source);
    snapshot.revision = build_snapshot(
        snapshot.workspace_path.clone(),
        &snapshot.source,
        snapshot.documents.clone(),
        snapshot.warnings.clone(),
    )
    .revision;
    snapshot
}

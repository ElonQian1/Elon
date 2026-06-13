//! Unified project-document snapshot loading for all project-doc surfaces.
use homecli_proto::{AgentToServer, ProjectDocumentsSnapshot};
use std::path::Path;

use crate::{
    project_auth::can_edit,
    project_docs_scan::{
        build_snapshot, collect_project_documents_with_options, ProjectDocumentScanOptions,
    },
    store::ProjectAccess,
    types::AppState,
};

pub(crate) async fn load_project_documents_snapshot(
    state: &AppState,
    project: &ProjectAccess,
) -> ProjectDocumentsSnapshot {
    let seed_defaults = should_seed_default_documents(project);
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
            .dispatch_project_documents_read(node_id, workspace_path.to_string(), seed_defaults)
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
                return fallback_snapshot(
                    &workspace,
                    seed_defaults,
                    format!("PC 节点读取文档失败：{message}"),
                );
            }
            Ok(other) => {
                return fallback_snapshot(
                    &workspace,
                    seed_defaults,
                    format!("PC 节点返回了非文档响应：{other:?}"),
                );
            }
            Err(error) => {
                return fallback_snapshot(
                    &workspace,
                    seed_defaults,
                    format!("PC 节点暂不可读取文档：{error}"),
                );
            }
        }
    }

    collect_project_documents_with_options(&workspace, scan_options(seed_defaults)).unwrap_or_else(
        |error| {
            build_snapshot(
                workspace.to_string_lossy().to_string(),
                "read_error",
                Vec::new(),
                vec![format!("读取项目文档失败：{error}")],
            )
        },
    )
}

fn fallback_snapshot(
    workspace: &Path,
    seed_defaults: bool,
    warning: String,
) -> ProjectDocumentsSnapshot {
    let mut snapshot =
        collect_project_documents_with_options(workspace, scan_options(seed_defaults))
            .unwrap_or_else(|error| {
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

fn should_seed_default_documents(project: &ProjectAccess) -> bool {
    !project.id.eq_ignore_ascii_case("elon-self") && can_edit(&project.role)
}

fn scan_options(seed_missing_defaults: bool) -> ProjectDocumentScanOptions {
    ProjectDocumentScanOptions {
        seed_missing_defaults,
    }
}

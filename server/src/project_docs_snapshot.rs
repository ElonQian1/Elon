//! Unified project-document snapshot loading for all project-doc surfaces.
use homecli_proto::{AgentToServer, ProjectDocumentsSnapshot};
use std::path::Path;

use crate::{
    project_auth::can_edit,
    project_docs_scan::{
        build_snapshot, collect_project_documents_with_options, ProjectDocumentScanOptions,
    },
    project_document_policy::classify_project_document,
    store::ProjectAccess,
    types::AppState,
};

pub(crate) async fn load_project_documents_snapshot(
    state: &AppState,
    project: &ProjectAccess,
) -> ProjectDocumentsSnapshot {
    load_project_documents_snapshot_mode(state, project, false).await
}

pub(crate) async fn load_project_documents_catalog_snapshot(
    state: &AppState,
    project: &ProjectAccess,
) -> ProjectDocumentsSnapshot {
    load_project_documents_snapshot_mode(state, project, true).await
}

async fn load_project_documents_snapshot_mode(
    state: &AppState,
    project: &ProjectAccess,
    catalog_only: bool,
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
            .dispatch_project_documents_read(
                node_id,
                workspace_path.to_string(),
                seed_defaults,
                catalog_only,
            )
            .await
        {
            Ok(AgentToServer::ProjectDocumentsRead { mut snapshot, .. }) => {
                normalize_remote_snapshot(&mut snapshot, catalog_only);
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
                    catalog_only,
                    format!("PC 节点读取文档失败：{message}"),
                );
            }
            Ok(other) => {
                return fallback_snapshot(
                    &workspace,
                    seed_defaults,
                    catalog_only,
                    format!("PC 节点返回了非文档响应：{other:?}"),
                );
            }
            Err(error) => {
                return fallback_snapshot(
                    &workspace,
                    seed_defaults,
                    catalog_only,
                    format!("PC 节点暂不可读取文档：{error}"),
                );
            }
        }
    }

    collect_project_documents_with_options(&workspace, scan_options(seed_defaults, catalog_only))
        .unwrap_or_else(|error| {
            build_snapshot(
                workspace.to_string_lossy().to_string(),
                "read_error",
                Vec::new(),
                vec![format!("读取项目文档失败：{error}")],
            )
        })
}

fn normalize_remote_snapshot(snapshot: &mut ProjectDocumentsSnapshot, catalog_only: bool) {
    for document in &mut snapshot.documents {
        if document.metadata.role.trim().is_empty() {
            document.metadata = classify_project_document(
                &document.path,
                &document.content,
                document.content.chars().count(),
            );
        }
        if catalog_only {
            document.content.clear();
            document.truncated = false;
        }
    }
}

fn fallback_snapshot(
    workspace: &Path,
    seed_defaults: bool,
    catalog_only: bool,
    warning: String,
) -> ProjectDocumentsSnapshot {
    let mut snapshot = collect_project_documents_with_options(
        workspace,
        scan_options(seed_defaults, catalog_only),
    )
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

fn scan_options(seed_missing_defaults: bool, catalog_only: bool) -> ProjectDocumentScanOptions {
    ProjectDocumentScanOptions {
        seed_missing_defaults,
        catalog_only,
        include_analysis: true,
    }
}

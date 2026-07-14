//! Unified project-document file access for server-local and bound PC workspaces.

use axum::http::StatusCode;
use homecli_proto::AgentToServer;

use crate::{
    project_document_files::{
        read_project_document_file, write_project_document_file, ProjectDocumentFile,
        ProjectDocumentWriteError,
    },
    store::ProjectAccess,
    types::AppState,
};

pub(crate) type ProjectDocumentGatewayError = (StatusCode, String);

pub(crate) async fn read_project_file(
    state: &AppState,
    access: &ProjectAccess,
    document_path: &str,
) -> Result<ProjectDocumentFile, ProjectDocumentGatewayError> {
    if let Some((node_id, workspace_path)) = bound_pc_workspace(access) {
        return match state
            .agent_manager
            .dispatch_project_document_file_read(
                node_id,
                workspace_path.to_string(),
                document_path.to_string(),
            )
            .await
        {
            Ok(AgentToServer::ProjectDocumentFileRead {
                path,
                content,
                revision,
                byte_len,
                ..
            }) => Ok(ProjectDocumentFile {
                path,
                content,
                revision,
                byte_len,
            }),
            Ok(AgentToServer::ProjectDocumentFileReadError { message, .. }) => {
                Err((StatusCode::BAD_REQUEST, message))
            }
            Ok(other) => Err((
                StatusCode::BAD_GATEWAY,
                format!("PC 节点返回了非文档响应：{other:?}"),
            )),
            Err(error) => Err((StatusCode::BAD_GATEWAY, error.to_string())),
        };
    }
    let workspace =
        state.resolve_project_workspace(&access.workspace_key, access.workspace_path.as_deref());
    read_project_document_file(&workspace, document_path)
        .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))
}

pub(crate) async fn read_optional_project_file(
    state: &AppState,
    access: &ProjectAccess,
    document_path: &str,
) -> Result<Option<ProjectDocumentFile>, ProjectDocumentGatewayError> {
    match read_project_file(state, access, document_path).await {
        Ok(file) => Ok(Some(file)),
        Err((StatusCode::BAD_REQUEST, message))
            if message.contains("不存在") || message.contains("not found") =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

pub(crate) async fn write_project_file(
    state: &AppState,
    access: &ProjectAccess,
    document_path: &str,
    content: &str,
    expected_revision: Option<&str>,
) -> Result<ProjectDocumentFile, ProjectDocumentGatewayError> {
    if let Some((node_id, workspace_path)) = bound_pc_workspace(access) {
        return match state
            .agent_manager
            .dispatch_project_document_file_write(
                node_id,
                workspace_path.to_string(),
                document_path.to_string(),
                content.to_string(),
                expected_revision.map(str::to_string),
            )
            .await
        {
            Ok(AgentToServer::ProjectDocumentFileWritten {
                path,
                revision,
                byte_len,
                ..
            }) => Ok(ProjectDocumentFile {
                path,
                content: content.to_string(),
                revision,
                byte_len,
            }),
            Ok(AgentToServer::ProjectDocumentFileWriteError {
                message, conflict, ..
            }) => Err((
                if conflict {
                    StatusCode::CONFLICT
                } else {
                    StatusCode::BAD_REQUEST
                },
                message,
            )),
            Ok(other) => Err((
                StatusCode::BAD_GATEWAY,
                format!("PC 节点返回了非文档响应：{other:?}"),
            )),
            Err(error) => Err((StatusCode::BAD_GATEWAY, error.to_string())),
        };
    }
    let workspace =
        state.resolve_project_workspace(&access.workspace_key, access.workspace_path.as_deref());
    write_project_document_file(&workspace, document_path, content, expected_revision)
        .map_err(write_service_error)
}

fn bound_pc_workspace(access: &ProjectAccess) -> Option<(&str, &str)> {
    Some((
        access
            .node_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())?,
        access
            .workspace_path
            .as_deref()
            .filter(|value| !value.trim().is_empty())?,
    ))
}

fn write_service_error(error: ProjectDocumentWriteError) -> ProjectDocumentGatewayError {
    (
        if error.conflict {
            StatusCode::CONFLICT
        } else {
            StatusCode::BAD_REQUEST
        },
        error.message,
    )
}

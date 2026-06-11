/// project_docs.rs — read project Markdown documents for project space.
use anyhow::anyhow;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use homecli_proto::ProjectDocumentEntry;
use serde::Serialize;
use std::{collections::HashMap, path::Path as FsPath, sync::Arc};

use crate::{
    project_auth::{auth_from_headers, json_error, project_access},
    project_docs_scan::collect_project_documents,
    project_mobile::ensure_mobile_project,
    store::ProjectAccess,
    types::AppState,
};

#[derive(Clone, Serialize)]
struct ProjectDocument {
    title: String,
    path: String,
    content: String,
    size_bytes: u64,
    truncated: bool,
}

impl From<ProjectDocumentEntry> for ProjectDocument {
    fn from(entry: ProjectDocumentEntry) -> Self {
        Self {
            title: entry.title,
            path: entry.path,
            content: entry.content,
            size_bytes: entry.byte_len,
            truncated: entry.truncated,
        }
    }
}

pub async fn get_project_document(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let access = match project_access(&state, &user.id, &project_id) {
        Ok(access) => access,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    project_document_response(state, access)
}

pub async fn get_user_project_document(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((user_id, project_id)): Path<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let access = match ensure_user_project_for_document(
        &state,
        &headers,
        &user_id,
        &project_id,
        query.get("title").map(String::as_str),
    ) {
        Ok(access) => access,
        Err(response) => return response,
    };
    project_document_response(state, access)
}

fn project_document_response(state: Arc<AppState>, access: ProjectAccess) -> Response {
    let workspace =
        state.resolve_project_workspace(&access.workspace_key, access.workspace_path.as_deref());
    match load_project_documents(&workspace) {
        Ok((workspace_path, document, documents, warnings)) => Json(serde_json::json!({
            "project_id": access.id,
            "workspace": workspace_path,
            "document": document,
            "documents": documents,
            "warnings": warnings,
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::NOT_FOUND, e.to_string()),
    }
}

fn ensure_user_project_for_document(
    state: &AppState,
    headers: &HeaderMap,
    user_id: &str,
    project_id: &str,
    project_title: Option<&str>,
) -> Result<ProjectAccess, Response> {
    let effective_user_id = if state.require_login {
        match auth_from_headers(state, headers) {
            Ok(user) => user.id,
            Err(e) => {
                return Err(json_error(
                    StatusCode::UNAUTHORIZED,
                    format!("请先登录后再使用（{}）", e),
                ));
            }
        }
    } else {
        user_id.to_string()
    };
    ensure_mobile_project(state, &effective_user_id, project_id, project_title)
        .map(|(_, project)| project)
        .map_err(|e| json_error(StatusCode::BAD_REQUEST, e.to_string()))
}

fn load_project_documents(
    workspace: &FsPath,
) -> anyhow::Result<(String, ProjectDocument, Vec<ProjectDocument>, Vec<String>)> {
    let snapshot = collect_project_documents(workspace)?;
    let documents = snapshot
        .documents
        .into_iter()
        .map(ProjectDocument::from)
        .collect::<Vec<_>>();
    let document = documents
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("未找到可展示的项目文档"))?;
    Ok((
        snapshot.workspace_path,
        document,
        documents,
        snapshot.warnings,
    ))
}

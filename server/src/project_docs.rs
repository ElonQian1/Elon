/// project_docs.rs — read project Markdown documents for project space.
use anyhow::anyhow;
use axum::{
    extract::{Path, Query, State},
    http::{
        header::{ETAG, IF_NONE_MATCH},
        HeaderMap, HeaderValue, StatusCode,
    },
    response::{IntoResponse, Response},
    Json,
};
use homecli_proto::ProjectDocumentEntry;
use serde::Serialize;
use std::{collections::HashMap, sync::Arc};

use crate::{
    project_auth::{auth_from_headers, json_error, project_access},
    project_docs_snapshot::load_project_documents_snapshot,
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
    source: String,
}

impl From<ProjectDocumentEntry> for ProjectDocument {
    fn from(entry: ProjectDocumentEntry) -> Self {
        Self {
            title: entry.title,
            path: entry.path,
            content: entry.content,
            size_bytes: entry.byte_len,
            truncated: entry.truncated,
            source: entry.source,
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
    project_document_response(state, headers, access).await
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
    project_document_response(state, headers, access).await
}

async fn project_document_response(
    state: Arc<AppState>,
    headers: HeaderMap,
    access: ProjectAccess,
) -> Response {
    match load_project_documents(&state, &access).await {
        Ok((workspace_path, revision, source, generated_at_ms, document, documents, warnings)) => {
            if etag_matches(&headers, &revision) {
                let mut response = StatusCode::NOT_MODIFIED.into_response();
                if let Ok(value) = HeaderValue::from_str(&format!("\"{revision}\"")) {
                    response.headers_mut().insert(ETAG, value);
                }
                return response;
            }
            let mut response = Json(serde_json::json!({
            "project_id": access.id,
            "workspace": workspace_path,
            "revision": revision.clone(),
            "source": source,
            "generated_at_ms": generated_at_ms,
            "document": document,
            "documents": documents,
            "warnings": warnings,
            }))
            .into_response();
            if let Ok(value) = HeaderValue::from_str(&format!("\"{revision}\"")) {
                response.headers_mut().insert(ETAG, value);
            }
            response
        }
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

async fn load_project_documents(
    state: &AppState,
    access: &ProjectAccess,
) -> anyhow::Result<(
    String,
    String,
    String,
    u64,
    ProjectDocument,
    Vec<ProjectDocument>,
    Vec<String>,
)> {
    let snapshot = load_project_documents_snapshot(state, access).await;
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
        snapshot.revision,
        snapshot.source,
        snapshot.generated_at_ms,
        document,
        documents,
        snapshot.warnings,
    ))
}

fn etag_matches(headers: &HeaderMap, revision: &str) -> bool {
    if revision.is_empty() {
        return false;
    }
    headers
        .get(IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            value
                .split(',')
                .map(|part| part.trim().trim_matches('"'))
                .any(|etag| etag == revision || etag == "*")
        })
        .unwrap_or(false)
}

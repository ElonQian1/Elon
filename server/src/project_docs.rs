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
use homecli_proto::{ProjectDocumentEntry, ProjectDocumentMetadata};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashMap, sync::Arc};

use crate::{
    project_auth::{auth_from_headers, can_edit, json_error, project_access},
    project_docs_snapshot::{
        load_project_documents_catalog_snapshot, load_project_documents_snapshot,
    },
    project_document_gateway::{read_project_file, write_project_file},
    project_mobile::ensure_mobile_project,
    store::{ProjectAccess, PublicUser},
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
    metadata: ProjectDocumentMetadata,
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
            metadata: entry.metadata,
        }
    }
}

#[derive(Deserialize)]
pub struct ProjectDocumentPathQuery {
    path: String,
}

#[derive(Deserialize)]
pub struct ProjectDocumentFederationQuery {
    parent_id: Option<String>,
    query: Option<String>,
    offset: Option<usize>,
    limit: Option<usize>,
    cursor: Option<String>,
}

#[derive(Deserialize)]
pub struct SaveProjectDocumentRequest {
    path: String,
    content: String,
    #[serde(default)]
    expected_revision: Option<String>,
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

pub async fn get_project_document_catalog(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let (user, access) = match authorized_project(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let snapshot = load_project_documents_catalog_snapshot(&state, &access).await;
    let estimated_tokens = snapshot
        .documents
        .iter()
        .map(|document| document.metadata.token_estimate)
        .sum::<u64>();
    let default_tokens = snapshot
        .documents
        .iter()
        .filter(|document| document.metadata.default_retrieval)
        .map(|document| document.metadata.token_estimate)
        .sum::<u64>();
    let ambiguous_count = snapshot
        .documents
        .iter()
        .filter(|document| document.metadata.ambiguous)
        .count();
    let excluded_count = snapshot
        .documents
        .iter()
        .filter(|document| !document.metadata.default_retrieval)
        .count();
    let degraded = snapshot.source.starts_with("server_fallback:");
    let writable = can_edit(&access.role) && !degraded;
    let body_readable = !snapshot.documents.is_empty();
    let access_mode = if degraded {
        "server_fallback_read_only"
    } else if snapshot.source.starts_with("pc_node") {
        "pc_node"
    } else {
        "server_workspace"
    };
    Json(serde_json::json!({
        "project_id": access.id,
        "workspace": snapshot.workspace_path,
        "revision": snapshot.revision,
        "source": snapshot.source,
        "generated_at_ms": snapshot.generated_at_ms,
        "documents": snapshot.documents.into_iter().map(ProjectDocument::from).collect::<Vec<_>>(),
        "warnings": snapshot.warnings,
        "analysis": snapshot.analysis,
        "can_edit": writable,
        "access": {
            "mode": access_mode,
            "degraded": degraded,
            "body_readable": body_readable,
            "writable": writable,
        },
        "requested_by": user.id,
        "budget": {
            "classification_model_tokens": 0,
            "estimated_full_read_tokens": estimated_tokens,
            "estimated_default_retrieval_tokens": default_tokens,
            "estimated_tokens_avoided": estimated_tokens.saturating_sub(default_tokens),
            "ambiguous_documents": ambiguous_count,
            "excluded_by_default": excluded_count,
        }
    }))
    .into_response()
}

pub async fn get_project_document_federation(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Query(query): Query<ProjectDocumentFederationQuery>,
) -> Response {
    let (_, access) = match authorized_project(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let arguments = json!({
        "projection": "page",
        "offset": query.offset.unwrap_or_default(),
        "limit": query.limit.unwrap_or(8),
        "cursor": query.cursor,
    });
    let request =
        match crate::project_document_response::ProjectionRequest::from_arguments(&arguments) {
            Ok(request) => request,
            Err(error) => return json_error(StatusCode::BAD_REQUEST, error.to_string()),
        };
    if let (Some(node_id), Some(workspace_path)) = (
        access
            .node_id
            .as_deref()
            .filter(|value| !value.trim().is_empty()),
        access
            .workspace_path
            .as_deref()
            .filter(|value| !value.trim().is_empty()),
    ) {
        return match state
            .agent_manager
            .dispatch_project_document_federation_read(
                node_id,
                workspace_path.to_string(),
                query.parent_id,
                query.query,
                request.offset,
                request.limit,
                arguments["cursor"].as_str().map(str::to_string),
            )
            .await
        {
            Ok(homecli_proto::AgentToServer::ProjectDocumentFederationRead { page, .. }) => {
                Json(page).into_response()
            }
            Ok(homecli_proto::AgentToServer::ProjectDocumentFederationReadError {
                message,
                ..
            }) => json_error(StatusCode::BAD_GATEWAY, message),
            Ok(other) => json_error(
                StatusCode::BAD_GATEWAY,
                format!("PC 节点返回了非联邦分页响应：{other:?}"),
            ),
            Err(error) => json_error(StatusCode::SERVICE_UNAVAILABLE, error.to_string()),
        };
    }
    let workspace =
        state.resolve_project_workspace(&access.workspace_key, access.workspace_path.as_deref());
    match crate::project_document_federation_service::get_federation_index(
        &workspace,
        query.parent_id.as_deref(),
        query.query.as_deref(),
        &request,
    ) {
        Ok(page) => Json(page).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}

pub async fn get_project_document_file(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Query(query): Query<ProjectDocumentPathQuery>,
) -> Response {
    let (_, access) = match authorized_project(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match read_project_file(&state, &access, &query.path).await {
        Ok(document) => Json(serde_json::json!({
            "project_id": access.id,
            "path": document.file.path,
            "content": document.file.content,
            "revision": document.file.revision,
            "byte_len": document.file.byte_len,
            "source": document.source,
            "warnings": document.warnings,
            "can_edit": can_edit(&access.role) && document.source != "server_fallback",
        }))
        .into_response(),
        Err((status, message)) => json_error(status, message),
    }
}

pub async fn put_project_document_file(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<SaveProjectDocumentRequest>,
) -> Response {
    let (_, access) = match authorized_project(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if !can_edit(&access.role) {
        return json_error(StatusCode::FORBIDDEN, "当前项目角色不能编辑文档");
    }
    match write_project_file(
        &state,
        &access,
        &request.path,
        &request.content,
        request.expected_revision.as_deref(),
    )
    .await
    {
        Ok(document) => Json(serde_json::json!({
            "ok": true,
            "project_id": access.id,
            "path": document.path,
            "revision": document.revision,
            "byte_len": document.byte_len,
        }))
        .into_response(),
        Err((status, message)) => json_error(status, message),
    }
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

fn authorized_project(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
) -> Result<(PublicUser, ProjectAccess), Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error.to_string()))?;
    let access = project_access(state, &user.id, project_id)
        .map_err(|error| json_error(StatusCode::FORBIDDEN, error.to_string()))?;
    Ok((user, access))
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

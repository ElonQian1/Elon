/// project_docs.rs — read project README / Markdown documents for project space.
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::{
    collections::HashMap,
    fs,
    io::Read,
    path::{Path as FsPath, PathBuf},
    sync::Arc,
};

use crate::{
    project_auth::{auth_from_headers, json_error, project_access},
    project_mobile::ensure_mobile_project,
    store::ProjectAccess,
    types::AppState,
};

const MAX_DOCUMENT_BYTES: u64 = 128 * 1024;

#[derive(Serialize)]
struct ProjectDocument {
    title: String,
    path: String,
    content: String,
    size_bytes: u64,
    truncated: bool,
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
    match load_project_document(&workspace) {
        Ok(document) => Json(serde_json::json!({
            "project_id": access.id,
            "workspace": workspace.to_string_lossy(),
            "document": document,
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

fn load_project_document(workspace: &FsPath) -> anyhow::Result<ProjectDocument> {
    if !workspace.is_dir() {
        anyhow::bail!("项目工作区不存在，无法读取项目文档");
    }
    let candidate = find_document_candidate(workspace)?
        .ok_or_else(|| anyhow::anyhow!("未找到 README.md 或仓库根目录 Markdown 文档"))?;
    let (content, size_bytes, truncated) = read_limited_utf8(&candidate.path)?;
    Ok(ProjectDocument {
        title: candidate.title,
        path: candidate.relative_path,
        content,
        size_bytes,
        truncated,
    })
}

struct DocumentCandidate {
    title: String,
    relative_path: String,
    path: PathBuf,
}

fn find_document_candidate(workspace: &FsPath) -> anyhow::Result<Option<DocumentCandidate>> {
    let preferred = [
        "README.md",
        "README.MD",
        "Readme.md",
        "readme.md",
        "README.markdown",
        "README",
    ];
    for name in preferred {
        let path = workspace.join(name);
        if path.is_file() {
            return Ok(Some(candidate(name.to_string(), path)));
        }
    }

    let mut markdown = markdown_files_in(workspace, "")?;
    if markdown.is_empty() {
        let docs_dir = workspace.join("docs");
        if docs_dir.is_dir() {
            markdown = markdown_files_in(&docs_dir, "docs/")?;
        }
    }
    Ok(markdown.into_iter().next())
}

fn markdown_files_in(dir: &FsPath, prefix: &str) -> anyhow::Result<Vec<DocumentCandidate>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() || !is_markdown_file(&path) {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        files.push(candidate(format!("{prefix}{file_name}"), path));
    }
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(files)
}

fn candidate(relative_path: String, path: PathBuf) -> DocumentCandidate {
    DocumentCandidate {
        title: relative_path.clone(),
        relative_path,
        path,
    }
}

fn is_markdown_file(path: &FsPath) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "md" | "markdown" | "mdown"
            )
        })
        .unwrap_or(false)
}

fn read_limited_utf8(path: &FsPath) -> anyhow::Result<(String, u64, bool)> {
    let mut file = fs::File::open(path)?;
    let size_bytes = file.metadata()?.len();
    let limit = size_bytes.min(MAX_DOCUMENT_BYTES) as usize;
    let mut bytes = vec![0; limit];
    if limit > 0 {
        file.read_exact(&mut bytes)?;
    }
    let truncated = size_bytes > MAX_DOCUMENT_BYTES;
    let mut content = String::from_utf8_lossy(&bytes).into_owned();
    if truncated {
        content.push_str("\n\n……\n\n（文档超过 128 KB，已截断显示。）");
    }
    Ok((content, size_bytes, truncated))
}

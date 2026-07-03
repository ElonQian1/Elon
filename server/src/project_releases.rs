use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, Path as AxumPath, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use uuid::Uuid;

use crate::{
    project_auth::{
        auth_from_headers, auth_from_headers_or_query, can_edit, json_error, project_access,
    },
    project_downloads, tools,
    types::AppState,
};

pub const MAX_PROJECT_RELEASE_APK_BYTES: usize = 160 * 1024 * 1024;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/user/:user_id/projects/:project_id/download/:filename",
            get(project_downloads::download_user_project_apk),
        )
        .route(
            "/api/projects/:project_id/download/:filename",
            get(project_downloads::download_project_apk),
        )
        .route(
            "/api/projects/:project_id/releases",
            get(list_project_releases)
                .post(upload_project_release)
                .layer(DefaultBodyLimit::max(MAX_PROJECT_RELEASE_APK_BYTES)),
        )
        .route(
            "/api/projects/:project_id/releases/:release_id/download.apk",
            get(download_project_release_apk),
        )
}

#[derive(Deserialize)]
pub struct ReleaseUploadQuery {
    pub file_name: Option<String>,
    pub version_name: Option<String>,
    pub changelog: Option<String>,
    pub channel: Option<String>,
    pub task_id: Option<String>,
}

pub async fn list_project_releases(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    AxumPath(project_id): AxumPath<String>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    if let Err(e) = project_access(&state, &user.id, &project_id) {
        return json_error(StatusCode::FORBIDDEN, e.to_string());
    }
    match state.store.list_project_releases(&project_id, 50) {
        Ok(releases) => Json(serde_json::json!({ "releases": releases })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn upload_project_release(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    AxumPath(project_id): AxumPath<String>,
    Query(query): Query<ReleaseUploadQuery>,
    body: Bytes,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match project_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    if !can_edit(&project.role) {
        return json_error(
            StatusCode::FORBIDDEN,
            "only project editors can publish APK releases",
        );
    }
    if body.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "APK body cannot be empty");
    }

    let release_id = format!("rel_{}", Uuid::new_v4().simple());
    let file_name = safe_apk_file_name(query.file_name.as_deref().unwrap_or("app-release.apk"));
    let release_dir = state
        .data_dir
        .join("project-releases")
        .join(safe_path_part(&project_id))
        .join(&release_id);
    let file_path = release_dir.join(&file_name);
    if let Err(e) = tokio::fs::create_dir_all(&release_dir).await {
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }
    if let Err(e) = tokio::fs::write(&file_path, &body).await {
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }

    let sha256 = format!("{:x}", Sha256::digest(&body));
    let download_base = format!("{}/api/projects/{}/download", state.public_url, project_id);
    let apk_url = tools::stable_apk_url(&download_base);
    match state
        .store
        .create_project_release(crate::store::project_releases::ProjectReleaseWrite {
            id: Some(&release_id),
            project_id: &project_id,
            task_id: query.task_id.as_deref(),
            uploaded_by: Some(&user.id),
            version_name: query.version_name.as_deref(),
            channel: query.channel.as_deref(),
            status: Some("published"),
            apk_url: &apk_url,
            file_name: &file_name,
            file_path: Some(&file_path.to_string_lossy()),
            sha256: Some(&sha256),
            size_bytes: Some(body.len() as i64),
            changelog: query.changelog.as_deref(),
        }) {
        Ok(release) => Json(serde_json::json!({
            "release": release,
            "latest_download_url": apk_url,
            "release_download_url": format!(
                "{}/api/projects/{}/releases/{}/download.apk",
                state.public_url, project_id, release_id
            )
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn download_project_release_apk(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    AxumPath((project_id, release_id)): AxumPath<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let user = match auth_from_headers_or_query(&state, &headers, &query) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    if let Err(e) = project_access(&state, &user.id, &project_id) {
        return json_error(StatusCode::FORBIDDEN, e.to_string());
    }
    let release = match state.store.project_release(&release_id) {
        Ok(release) if release.project_id == project_id => release,
        Ok(_) => return json_error(StatusCode::NOT_FOUND, "release does not belong to project"),
        Err(_) => return json_error(StatusCode::NOT_FOUND, "release not found"),
    };
    let Some(file_path) = release.file_path.map(PathBuf::from) else {
        return json_error(
            StatusCode::NOT_FOUND,
            "release APK file is not stored on this server",
        );
    };
    let data = match tokio::fs::read(&file_path).await {
        Ok(data) => data,
        Err(e) => return json_error(StatusCode::NOT_FOUND, e.to_string()),
    };
    project_downloads::apk_response(data, &release.file_name)
}

fn safe_apk_file_name(raw: &str) -> String {
    let safe = raw
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(raw)
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
        .collect::<String>();
    if safe.to_ascii_lowercase().ends_with(".apk") && !safe.is_empty() {
        safe
    } else {
        "app-release.apk".to_string()
    }
}

fn safe_path_part(raw: &str) -> String {
    let safe = raw
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        .take(96)
        .collect::<String>();
    if safe.is_empty() {
        "project".to_string()
    } else {
        safe
    }
}

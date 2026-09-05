use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, Path as AxumPath, Query, State},
    http::{header, header::CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use uuid::Uuid;

pub(crate) mod admission;

#[cfg(test)]
#[path = "project_releases_tests.rs"]
mod tests;

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
            "/api/agent/scripts/pc-apk-sync.ps1",
            get(pc_apk_sync_script),
        )
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
    pub package_name: Option<String>,
    pub version_code: Option<String>,
    pub changelog: Option<String>,
    pub channel: Option<String>,
    pub task_id: Option<String>,
    pub build_started_at: Option<String>,
    pub source_git_sha: Option<String>,
    pub source_worktree: Option<String>,
}

#[derive(Deserialize)]
pub struct PcApkSyncScriptQuery {
    pub fresh_after_unix_secs: Option<u64>,
    pub build_if_missing: Option<bool>,
}

pub async fn pc_apk_sync_script(Query(query): Query<PcApkSyncScriptQuery>) -> Response {
    let script = crate::ai_cli::ai_cli_apk_build_script::pc_apk_sync_script(
        query.fresh_after_unix_secs,
        query.build_if_missing.unwrap_or(false),
    );
    ([(CONTENT_TYPE, "text/plain; charset=utf-8")], script).into_response()
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
        Ok(releases) => {
            let official_quant = admission::is_official_quant_project(&project_id);
            let releases = releases
                .into_iter()
                .filter(|release| {
                    !official_quant
                        || crate::store::project_releases::official_quant_release_is_installable(
                            release,
                        )
                })
                .map(release_list_item)
                .collect::<Vec<_>>();
            Json(serde_json::json!({ "releases": releases })).into_response()
        }
        Err(e) => {
            tracing::warn!(project_id = %project_id, error = %e, "failed to list project releases");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "project release history is unavailable",
            )
        }
    }
}

fn release_list_item(release: crate::store::project_releases::ProjectRelease) -> serde_json::Value {
    let installable = release.status == "published"
        && release
            .file_path
            .as_deref()
            .is_some_and(|path| !path.trim().is_empty())
        && (!admission::is_official_quant_project(&release.project_id)
            || crate::store::project_releases::official_quant_release_is_installable(&release));
    let mut value = serde_json::to_value(release).unwrap_or_else(|_| serde_json::json!({}));
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "installable".to_string(),
            serde_json::Value::Bool(installable),
        );
    }
    value
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
    let official_quant = admission::is_official_quant_project(&project_id);
    if body.is_empty() {
        let status = if official_quant {
            StatusCode::UNPROCESSABLE_ENTITY
        } else {
            StatusCode::BAD_REQUEST
        };
        return json_error(status, "APK body cannot be empty");
    }
    let version_code =
        match parse_release_version_code(query.version_code.as_deref(), official_quant) {
            Ok(version_code) => version_code,
            Err(status) => return json_error(status, "version_code must be an integer"),
        };

    let official_apk = if official_quant {
        let declaration = admission::OfficialQuantReleaseDeclaration {
            project_id: &project_id,
            package_name: query.package_name.as_deref(),
            version_code,
            version_name: query.version_name.as_deref(),
            channel: query.channel.as_deref(),
            source_git_sha: query.source_git_sha.as_deref(),
        };
        if admission::validate_release_declaration(declaration).is_err() {
            return json_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "official quant APK does not satisfy the current release contract",
            );
        }
        match admission::validate_apk_payload(&body) {
            Ok(validated) => Some(validated),
            Err(_) => {
                return json_error(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "official quant APK does not satisfy the current release contract",
                )
            }
        }
    } else {
        None
    };

    let sha256 = official_apk
        .as_ref()
        .map(|validated| validated.sha256().to_string())
        .unwrap_or_else(|| format!("{:x}", Sha256::digest(&body)));
    let size_bytes = official_apk
        .as_ref()
        .map(|validated| validated.size_bytes())
        .unwrap_or(body.len() as i64);
    let release_id = format!("rel_{}", Uuid::new_v4().simple());
    let file_name = safe_apk_file_name(query.file_name.as_deref().unwrap_or("app-release.apk"));
    let release_dir = state
        .data_dir
        .join("project-releases")
        .join(safe_path_part(&project_id))
        .join(&release_id);
    let file_path = release_dir.join(&file_name);
    if let Err(e) = tokio::fs::create_dir_all(&release_dir).await {
        tracing::error!(project_id = %project_id, error = %e, "failed to prepare project release directory");
        return json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "project release storage is unavailable",
        );
    }
    if let Err(e) = tokio::fs::write(&file_path, &body).await {
        tracing::error!(project_id = %project_id, error = %e, "failed to persist project release payload");
        cleanup_release_directory(&release_dir).await;
        return json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "project release storage is unavailable",
        );
    }

    let download_base = format!("{}/api/projects/{}/download", state.public_url, project_id);
    let apk_url = tools::stable_apk_url(&download_base);
    match state.store.create_project_release_with_admission(
        crate::store::project_releases::ProjectReleaseWrite {
            id: Some(&release_id),
            project_id: &project_id,
            task_id: query.task_id.as_deref(),
            uploaded_by: persisted_release_uploader(&user.id),
            version_name: query.version_name.as_deref(),
            package_name: query.package_name.as_deref(),
            version_code,
            channel: query.channel.as_deref(),
            status: Some("published"),
            apk_url: &apk_url,
            file_name: &file_name,
            file_path: Some(&file_path.to_string_lossy()),
            sha256: Some(&sha256),
            size_bytes: Some(size_bytes),
            changelog: query.changelog.as_deref(),
            build_started_at: query.build_started_at.as_deref(),
            source_git_sha: query.source_git_sha.as_deref(),
            source_worktree: query.source_worktree.as_deref(),
            metadata_json: None,
        },
        official_apk.as_ref(),
    ) {
        Ok(outcome) => {
            let artifact_repaired = if official_quant && outcome.idempotent_replay {
                match restore_idempotent_official_quant_artifact(
                    &state,
                    &project_id,
                    &outcome.release,
                    &body,
                )
                .await
                {
                    Ok(repaired) => repaired,
                    Err(e) => {
                        cleanup_release_directory(&release_dir).await;
                        tracing::error!(
                            project_id = %project_id,
                            release_id = %outcome.release.id,
                            error = %e,
                            "failed to restore idempotent official quant release artifact"
                        );
                        return json_error(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "project release artifact could not be restored",
                        );
                    }
                }
            } else {
                false
            };
            if outcome.idempotent_replay {
                cleanup_release_directory(&release_dir).await;
            }
            let release_id = outcome.release.id.clone();
            Json(serde_json::json!({
                "release": outcome.release,
                "idempotent_replay": outcome.idempotent_replay,
                "artifact_repaired": artifact_repaired,
                "latest_download_url": apk_url,
                "release_download_url": format!(
                    "{}/api/projects/{}/releases/{}/download.apk",
                    state.public_url, project_id, release_id
                )
            }))
            .into_response()
        }
        Err(e) => {
            cleanup_release_directory(&release_dir).await;
            if let Some(admission_error) = e.downcast_ref::<admission::OfficialQuantReleaseError>()
            {
                let status = if admission_error.is_conflict() {
                    StatusCode::CONFLICT
                } else {
                    StatusCode::UNPROCESSABLE_ENTITY
                };
                return json_error(status, admission_error.to_string());
            }
            tracing::error!(project_id = %project_id, error = %e, "failed to register project release");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "project release registration failed",
            )
        }
    }
}

pub(crate) fn persisted_release_uploader(user_id: &str) -> Option<&str> {
    (user_id != "local-owner").then_some(user_id)
}

fn parse_release_version_code(
    raw: Option<&str>,
    official_quant: bool,
) -> Result<Option<i64>, StatusCode> {
    raw.map(str::parse::<i64>).transpose().map_err(|_| {
        if official_quant {
            StatusCode::UNPROCESSABLE_ENTITY
        } else {
            StatusCode::BAD_REQUEST
        }
    })
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
        Ok(release)
            if release.project_id == project_id
                && release.status == "published"
                && (!admission::is_official_quant_project(&project_id)
                    || crate::store::project_releases::official_quant_release_is_installable(
                        &release,
                    )) =>
        {
            release
        }
        Ok(_) => return json_error(StatusCode::NOT_FOUND, "release does not belong to project"),
        Err(_) => return json_error(StatusCode::NOT_FOUND, "release not found"),
    };
    let Some(file_path) = release.file_path.map(PathBuf::from) else {
        return json_error(
            StatusCode::NOT_FOUND,
            "release APK file is not stored on this server",
        );
    };
    let official_quant = admission::is_official_quant_project(&project_id);
    let file_path = if official_quant {
        let managed_root = state.data_dir.join("project-releases");
        match (
            tokio::fs::canonicalize(&managed_root).await,
            tokio::fs::canonicalize(&file_path).await,
        ) {
            (Ok(root), Ok(file)) if file.starts_with(&root) => file,
            _ => {
                tracing::warn!(project_id = %project_id, release_id = %release.id, "rejected unavailable or unmanaged official quant release path");
                return json_error(StatusCode::NOT_FOUND, "release APK is unavailable");
            }
        }
    } else {
        file_path
    };
    let data = match tokio::fs::read(&file_path).await {
        Ok(data) => data,
        Err(e) => {
            tracing::warn!(project_id = %project_id, release_id = %release.id, error = %e, "failed to read managed release APK");
            return json_error(StatusCode::NOT_FOUND, "release APK is unavailable");
        }
    };
    let sha256 = if official_quant {
        match crate::project_store::apk::verify_release_payload(
            &data,
            release.size_bytes,
            release.sha256.as_deref(),
        ) {
            Ok(sha256) => Some(sha256),
            Err(e) => {
                tracing::error!(project_id = %project_id, release_id = %release.id, error = %e, "release APK integrity verification failed");
                return json_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "release APK integrity verification failed",
                );
            }
        }
    } else {
        None
    };
    let mut response = project_downloads::apk_response(data, &release.file_name);
    if !official_quant {
        return response;
    }
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    if let Some(sha256) = sha256 {
        if let Ok(value) = HeaderValue::from_str(&sha256) {
            response
                .headers_mut()
                .insert(HeaderName::from_static("x-apk-sha256"), value);
        }
    }
    response
}

async fn cleanup_release_directory(release_dir: &std::path::Path) {
    if let Err(cleanup_error) = tokio::fs::remove_dir_all(release_dir).await {
        if cleanup_error.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(
                path = %release_dir.display(),
                %cleanup_error,
                "failed to clean rejected project release files"
            );
        }
    }
}

async fn restore_idempotent_official_quant_artifact(
    state: &AppState,
    project_id: &str,
    release: &crate::store::project_releases::ProjectRelease,
    payload: &[u8],
) -> anyhow::Result<bool> {
    let stored_path = release
        .file_path
        .as_deref()
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("admitted release has no managed file path"))?;
    let expected_path = official_quant_release_file_path(
        &state.data_dir,
        project_id,
        &release.id,
        &release.file_name,
    );
    if stored_path != expected_path {
        anyhow::bail!("admitted release file path does not match managed storage identity");
    }
    let replay_sha256 = format!("{:x}", Sha256::digest(payload));
    if release.sha256.as_deref() != Some(replay_sha256.as_str())
        || release.size_bytes != Some(payload.len() as i64)
    {
        anyhow::bail!("replayed payload does not match admitted release identity");
    }

    let managed_root = state.data_dir.join("project-releases");
    let canonical_root = tokio::fs::canonicalize(&managed_root).await?;
    let project_root = managed_root.join(safe_path_part(project_id));
    let canonical_project_root = tokio::fs::canonicalize(&project_root).await?;
    if !canonical_project_root.starts_with(&canonical_root) {
        anyhow::bail!("official quant project release root escaped managed storage");
    }
    let release_dir = expected_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("official quant release path has no parent"))?;
    tokio::fs::create_dir_all(release_dir).await?;
    let release_dir_metadata = tokio::fs::symlink_metadata(release_dir).await?;
    if release_dir_metadata.file_type().is_symlink()
        || !tokio::fs::canonicalize(release_dir)
            .await?
            .starts_with(&canonical_project_root)
    {
        anyhow::bail!("official quant release directory is not managed storage");
    }

    match tokio::fs::symlink_metadata(&expected_path).await {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            anyhow::bail!("official quant release artifact cannot be a symlink")
        }
        Ok(_) => {
            if let Ok(existing) = tokio::fs::read(&expected_path).await {
                if crate::project_store::apk::verify_release_payload(
                    &existing,
                    release.size_bytes,
                    release.sha256.as_deref(),
                )
                .is_ok()
                {
                    return Ok(false);
                }
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }

    tokio::fs::write(&expected_path, payload).await?;
    let restored = tokio::fs::read(&expected_path).await?;
    crate::project_store::apk::verify_release_payload(
        &restored,
        release.size_bytes,
        release.sha256.as_deref(),
    )?;
    Ok(true)
}

fn official_quant_release_file_path(
    data_dir: &std::path::Path,
    project_id: &str,
    release_id: &str,
    file_name: &str,
) -> PathBuf {
    data_dir
        .join("project-releases")
        .join(safe_path_part(project_id))
        .join(safe_path_part(release_id))
        .join(safe_apk_file_name(file_name))
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

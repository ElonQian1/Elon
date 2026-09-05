use axum::{
    body::Body,
    extract::{Path as AxumPath, Query, State},
    http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::Response,
};
use std::{collections::HashMap, sync::Arc};

use crate::{
    project_auth::{auth_from_headers_or_query, json_error, project_access},
    store::ProjectAccess,
    tools,
    types::AppState,
};

pub async fn download_project_apk(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    AxumPath((project_id, filename)): AxumPath<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let user = match auth_from_headers_or_query(&state, &headers, &query) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match project_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };

    serve_project_apk(&state, &project, &filename).await
}

pub async fn download_user_project_apk(
    State(state): State<Arc<AppState>>,
    AxumPath((user_id, project_id, filename)): AxumPath<(String, String, String)>,
) -> Response {
    let user = match state.store.ensure_device_user(&user_id) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let project = match project_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    serve_project_apk(&state, &project, &filename).await
}

async fn serve_project_apk(state: &AppState, project: &ProjectAccess, filename: &str) -> Response {
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return json_error(StatusCode::BAD_REQUEST, "invalid filename");
    }
    if !filename.ends_with(".apk") {
        return json_error(StatusCode::BAD_REQUEST, "only APK downloads are allowed");
    }
    if crate::project_releases::admission::is_official_quant_project(&project.id) {
        return serve_official_quant_apk(state, &project.id, filename).await;
    }
    let release_path = match state
        .store
        .project_release_for_download(&project.id, filename)
    {
        Ok(Some(release)) => release.file_path.map(std::path::PathBuf::from),
        Ok(None) => None,
        Err(error) => {
            tracing::warn!(project_id = %project.id, %error, "read project release download path failed");
            None
        }
    };
    let release_path = release_path.filter(|path| path.is_file());
    let apk_path = if filename == tools::STABLE_APK_FILENAME {
        release_path
    } else {
        let workspace = state
            .resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
        let managed_workspace = state.get_project_workspace(&project.workspace_key);
        release_path
            .or_else(|| tools::find_download_apk(&managed_workspace, filename))
            .or_else(|| tools::find_download_apk(&workspace, filename))
    };
    let Some(apk_path) = apk_path else {
        return json_error(StatusCode::NOT_FOUND, "APK 文件不存在");
    };
    let download_name = apk_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(filename);
    let data = match tokio::fs::read(&apk_path).await {
        Ok(data) => data,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    apk_response(data, download_name)
}

async fn serve_official_quant_apk(state: &AppState, project_id: &str, filename: &str) -> Response {
    let release = match state.store.latest_project_release(project_id) {
        Ok(Some(release))
            if crate::store::project_releases::official_quant_release_is_installable(&release) =>
        {
            release
        }
        Ok(_) => return json_error(StatusCode::NOT_FOUND, "APK 文件不存在"),
        Err(error) => {
            tracing::warn!(%project_id, %error, "read admitted official quant release failed");
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "APK 下载入口暂时不可用");
        }
    };
    if !official_quant_filename_matches(filename, &release.file_name) {
        return json_error(StatusCode::NOT_FOUND, "APK 文件不存在");
    }
    let Some(file_path) = release.file_path.as_deref() else {
        return json_error(StatusCode::NOT_FOUND, "APK 文件不存在");
    };
    let managed_root = state.data_dir.join("project-releases");
    let file_path = match (
        tokio::fs::canonicalize(&managed_root).await,
        tokio::fs::canonicalize(file_path).await,
    ) {
        (Ok(root), Ok(file)) if file.starts_with(&root) => file,
        (root, file) => {
            tracing::warn!(
                %project_id,
                release_id = %release.id,
                root_ready = root.is_ok(),
                file_ready = file.is_ok(),
                "rejected unavailable or unmanaged official quant release path"
            );
            return json_error(StatusCode::NOT_FOUND, "APK 文件不存在");
        }
    };
    let data = match tokio::fs::read(&file_path).await {
        Ok(data) => data,
        Err(error) => {
            tracing::warn!(%project_id, release_id = %release.id, %error, "read official quant release failed");
            return json_error(StatusCode::NOT_FOUND, "APK 文件不存在");
        }
    };
    let sha256 = match crate::project_store::apk::verify_release_payload(
        &data,
        release.size_bytes,
        release.sha256.as_deref(),
    ) {
        Ok(sha256) => sha256,
        Err(error) => {
            tracing::error!(%project_id, release_id = %release.id, %error, "official quant release integrity verification failed");
            return json_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "APK 完整性校验失败，已停止下载",
            );
        }
    };
    let mut response = apk_response(data, &release.file_name);
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    if let Ok(value) = HeaderValue::from_str(&sha256) {
        response
            .headers_mut()
            .insert(HeaderName::from_static("x-apk-sha256"), value);
    }
    response
}

fn official_quant_filename_matches(requested: &str, latest_file_name: &str) -> bool {
    requested == tools::STABLE_APK_FILENAME || requested == latest_file_name
}

pub fn apk_response(data: Vec<u8>, download_name: &str) -> Response {
    axum::response::Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            "application/vnd.android.package-archive",
        )
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", download_name),
        )
        .body(Body::from(data))
        .unwrap_or_else(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_quant_filename_only_selects_the_latest_admitted_release() {
        assert!(official_quant_filename_matches(
            tools::STABLE_APK_FILENAME,
            "yilong-quant-0.5.0.apk"
        ));
        assert!(official_quant_filename_matches(
            "yilong-quant-0.5.0.apk",
            "yilong-quant-0.5.0.apk"
        ));
        assert!(!official_quant_filename_matches(
            "yilong-quant-0.2.0.apk",
            "yilong-quant-0.5.0.apk"
        ));
    }
}

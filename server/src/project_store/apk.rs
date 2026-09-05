use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, path::Path as StdPath, sync::Arc};

use crate::{
    project_attachment_paths::percent_encode_path_segment,
    project_auth::{auth_from_headers_or_query, json_error, project_access},
    store::PublicProjectItem,
    types::AppState,
};

pub(crate) fn decorate_public_projects(state: &AppState, projects: &mut [PublicProjectItem]) {
    decorate_projects(state, projects, true);
}

pub(crate) fn decorate_joined_projects(state: &AppState, projects: &mut [PublicProjectItem]) {
    decorate_projects(state, projects, false);
}

fn decorate_projects(state: &AppState, projects: &mut [PublicProjectItem], public_only: bool) {
    for project in projects {
        let official_quant =
            crate::project_releases::admission::is_official_quant_project(&project.id);
        if !official_quant && project.latest_apk_url.is_some() {
            continue;
        }
        let download = if public_only {
            state.store.public_project_android_download(&project.id)
        } else {
            state.store.project_android_download(&project.id)
        };
        match download {
            Ok(Some(_)) => {
                project.latest_apk_url =
                    Some(android_download_route(&state.public_url, &project.id));
            }
            Ok(None) => {
                if official_quant {
                    project.latest_apk_url = None;
                }
            }
            Err(error) => {
                if official_quant {
                    project.latest_apk_url = None;
                }
                tracing::warn!(
                    project_id = %project.id,
                    error = %error,
                    "读取项目 Android 下载入口失败"
                );
            }
        }
    }
}

pub(crate) fn android_download_route(public_url: &str, project_id: &str) -> String {
    format!(
        "{}/api/store/projects/{}/downloads/android",
        public_url.trim_end_matches('/'),
        percent_encode_path_segment(project_id)
    )
}

pub(crate) async fn download_project_android(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    match state.store.public_project_android_download(&project_id) {
        Ok(Some((target, _))) => {
            if let Some(response) = serve_managed_public_release(&state, &project_id, &target).await
            {
                return response;
            }
            return redirect_without_credentials(&target);
        }
        Ok(None) => {}
        Err(error) => {
            tracing::warn!(project_id = %project_id, error = %error, "读取公开项目 Android 下载失败");
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "APK 下载入口暂时不可用");
        }
    }

    let user = match auth_from_headers_or_query(&state, &headers, &query) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    if let Err(error) = project_access(&state, &user.id, &project_id) {
        return json_error(StatusCode::FORBIDDEN, error.to_string());
    }
    match state.store.project_android_download(&project_id) {
        Ok(Some((target, _))) => redirect_without_credentials(&target),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "这个项目还没有可安装 APK"),
        Err(error) => {
            tracing::warn!(project_id = %project_id, error = %error, "读取成员项目 Android 下载失败");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "APK 下载入口暂时不可用")
        }
    }
}

async fn serve_managed_public_release(
    state: &AppState,
    project_id: &str,
    target: &str,
) -> Option<Response> {
    let release = match state.store.latest_project_release(project_id) {
        Ok(Some(release)) if release.apk_url.trim() == target.trim() => release,
        Ok(_) => return None,
        Err(error) => {
            tracing::warn!(project_id = %project_id, %error, "读取主服务器项目 APK 发布失败");
            return Some(json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "APK 下载入口暂时不可用",
            ));
        }
    };
    let Some(file_path) = release.file_path.as_deref().map(StdPath::new) else {
        return Some(json_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "APK 发布文件暂时不可用",
        ));
    };
    let managed_root = state.data_dir.join("project-releases");
    let (managed_root, file_path) = match (
        tokio::fs::canonicalize(&managed_root).await,
        tokio::fs::canonicalize(file_path).await,
    ) {
        (Ok(root), Ok(file)) if file.starts_with(&root) => (root, file),
        (Ok(_), Ok(_)) => {
            tracing::warn!(project_id = %project_id, release_id = %release.id, "拒绝公开非主服务器托管的项目 APK");
            return None;
        }
        (root, file) => {
            tracing::warn!(
                project_id = %project_id,
                release_id = %release.id,
                root_ready = root.is_ok(),
                file_ready = file.is_ok(),
                "主服务器项目 APK 文件不存在"
            );
            return Some(json_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "APK 发布文件暂时不可用",
            ));
        }
    };
    debug_assert!(file_path.starts_with(&managed_root));
    let data = match tokio::fs::read(&file_path).await {
        Ok(data) => data,
        Err(error) => {
            tracing::warn!(project_id = %project_id, release_id = %release.id, %error, "读取主服务器项目 APK 文件失败");
            return Some(json_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "APK 发布文件暂时不可用",
            ));
        }
    };
    let sha256 = match verify_release_payload(&data, release.size_bytes, release.sha256.as_deref())
    {
        Ok(sha256) => sha256,
        Err(error) => {
            tracing::error!(project_id = %project_id, release_id = %release.id, %error, "主服务器项目 APK 完整性校验失败");
            return Some(json_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "APK 完整性校验失败，已停止下载",
            ));
        }
    };
    let mut response = crate::project_downloads::apk_response(data, &release.file_name);
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    if let Ok(value) = HeaderValue::from_str(&sha256) {
        response
            .headers_mut()
            .insert(HeaderName::from_static("x-apk-sha256"), value);
    }
    Some(response)
}

pub(crate) fn verify_release_payload(
    data: &[u8],
    expected_size: Option<i64>,
    expected_sha256: Option<&str>,
) -> anyhow::Result<String> {
    let expected_size = expected_size
        .filter(|value| *value >= 0)
        .ok_or_else(|| anyhow::anyhow!("release size is missing"))?;
    if data.len() as i64 != expected_size {
        anyhow::bail!("release size mismatch");
    }
    let expected_sha256 = expected_sha256
        .map(str::trim)
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(|| anyhow::anyhow!("release sha256 is missing or invalid"))?;
    let actual_sha256 = format!("{:x}", Sha256::digest(data));
    if !actual_sha256.eq_ignore_ascii_case(expected_sha256) {
        anyhow::bail!("release sha256 mismatch");
    }
    Ok(actual_sha256)
}

fn redirect_without_credentials(target: &str) -> Response {
    let mut response = Redirect::temporary(target).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("no-referrer"),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_route_is_same_origin_and_encodes_project_id() {
        assert_eq!(
            android_download_route("https://main.example/", "merchant project/one"),
            "https://main.example/api/store/projects/merchant%20project%2Fone/downloads/android"
        );
    }

    #[test]
    fn redirect_does_not_forward_main_account_token() {
        let target = "https://merchant.example/downloads/app.apk?release=1";
        let response = redirect_without_credentials(target);

        assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
        assert_eq!(
            response
                .headers()
                .get(header::LOCATION)
                .and_then(|v| v.to_str().ok()),
            Some(target)
        );
        assert!(!response
            .headers()
            .get(header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .contains("token="));
        assert_eq!(
            response
                .headers()
                .get(header::CACHE_CONTROL)
                .and_then(|value| value.to_str().ok()),
            Some("no-store")
        );
        assert_eq!(
            response
                .headers()
                .get("referrer-policy")
                .and_then(|value| value.to_str().ok()),
            Some("no-referrer")
        );
    }

    #[test]
    fn yilong_quant_android_managed_release_payload_requires_exact_size_and_sha256() {
        let data = b"signed-apk-fixture";
        let sha256 = format!("{:x}", Sha256::digest(data));
        assert_eq!(
            verify_release_payload(data, Some(data.len() as i64), Some(&sha256)).unwrap(),
            sha256
        );
        assert!(verify_release_payload(data, Some(data.len() as i64 + 1), Some(&sha256)).is_err());
        assert!(
            verify_release_payload(data, Some(data.len() as i64), Some(&"0".repeat(64))).is_err()
        );
        assert!(verify_release_payload(data, Some(data.len() as i64), None).is_err());
    }
}

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use std::{collections::HashMap, sync::Arc};

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
        if project.latest_apk_url.is_some() {
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
            Ok(None) => {}
            Err(error) => tracing::warn!(
                project_id = %project.id,
                error = %error,
                "读取项目 Android 下载入口失败"
            ),
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
        Ok(Some((target, _))) => return redirect_without_credentials(&target),
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
}

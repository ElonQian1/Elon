use axum::{
    body::Body,
    extract::{Path as AxumPath, Query, State},
    http::{HeaderMap, StatusCode, header},
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
    let workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
    let Some(apk_path) = tools::find_download_apk(&workspace, filename) else {
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

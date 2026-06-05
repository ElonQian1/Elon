use axum::{
    body::{Body, Bytes},
    extract::{Path as AxumPath, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    project_attachment_paths::{
        content_type_for_file, percent_encode_path_segment, safe_project_path_part,
        unique_project_attachment_name,
    },
    project_auth::{auth_from_headers, json_error, project_access},
    project_mobile::ensure_mobile_project,
    project_ws_protocol::ProjectAttachmentRef,
    store::ProjectAccess,
    types::AppState,
};

pub const MAX_PROJECT_ATTACHMENTS_PER_MESSAGE: usize = 6;
pub const MAX_PROJECT_ATTACHMENT_BYTES: usize = 12 * 1024 * 1024;

pub async fn upload_project_attachment(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    AxumPath(project_id): AxumPath<String>,
    Query(query): Query<HashMap<String, String>>,
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

    upload_project_attachment_impl(
        state,
        user.id,
        project,
        query.get("conversation_id").map(String::as_str),
        query.get("conversation_title").map(String::as_str),
        query.get("kind").map(String::as_str),
        query.get("display_name").map(String::as_str),
        query.get("file_name").map(String::as_str),
        query.get("mime_type").map(String::as_str),
        query
            .get("image_width")
            .and_then(|value| parse_positive_u32(value)),
        query
            .get("image_height")
            .and_then(|value| parse_positive_u32(value)),
        body,
        true,
    )
    .await
}

pub async fn upload_user_project_attachment(
    State(state): State<Arc<AppState>>,
    AxumPath((user_id, project_id)): AxumPath<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
    body: Bytes,
) -> Response {
    let (user, project) = match ensure_mobile_project(
        &state,
        &user_id,
        &project_id,
        query.get("title").map(String::as_str),
    ) {
        Ok(pair) => pair,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };

    upload_project_attachment_impl(
        state,
        user.id,
        project,
        query.get("conversation_id").map(String::as_str),
        query.get("conversation_title").map(String::as_str),
        query.get("kind").map(String::as_str),
        query.get("display_name").map(String::as_str),
        query.get("file_name").map(String::as_str),
        query.get("mime_type").map(String::as_str),
        query
            .get("image_width")
            .and_then(|value| parse_positive_u32(value)),
        query
            .get("image_height")
            .and_then(|value| parse_positive_u32(value)),
        body,
        false,
    )
    .await
}

async fn upload_project_attachment_impl(
    state: Arc<AppState>,
    user_id: String,
    project: ProjectAccess,
    conversation_id_hint: Option<&str>,
    conversation_title_hint: Option<&str>,
    kind_hint: Option<&str>,
    display_name_hint: Option<&str>,
    file_name_hint: Option<&str>,
    mime_type_hint: Option<&str>,
    image_width_hint: Option<u32>,
    image_height_hint: Option<u32>,
    body: Bytes,
    include_project_api_url: bool,
) -> Response {
    if body.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "attachment body is empty");
    }
    if body.len() > MAX_PROJECT_ATTACHMENT_BYTES {
        return json_error(StatusCode::PAYLOAD_TOO_LARGE, "attachment is too large");
    }
    let sha256 = hex::encode(Sha256::digest(body.as_ref()));
    let attachment_id = format!("att_{}_{}", &sha256[..16], uuid::Uuid::new_v4().simple());

    let conversation_id = match state.store.ensure_conversation(
        &project.id,
        &user_id,
        conversation_id_hint,
        conversation_title_hint,
    ) {
        Ok(id) => id,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
    let attachments_dir = workspace
        .join("attachments")
        .join(safe_project_path_part(&conversation_id, 80));
    if let Err(error) = tokio::fs::create_dir_all(&attachments_dir).await {
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string());
    }

    let display_name = display_name_hint
        .or(file_name_hint)
        .unwrap_or("attachment.bin");
    let original_name = file_name_hint.unwrap_or(display_name);
    let file_name = unique_project_attachment_name(&attachments_dir, original_name);
    let path = attachments_dir.join(&file_name);
    if let Err(error) = tokio::fs::write(&path, body.as_ref()).await {
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string());
    }

    let mut urls = vec![format!(
        "{}/api/user/{}/projects/{}/attachments/{}/{}",
        state.public_url.trim_end_matches('/'),
        percent_encode_path_segment(&user_id),
        percent_encode_path_segment(&project.id),
        percent_encode_path_segment(&conversation_id),
        percent_encode_path_segment(&file_name)
    )];
    if include_project_api_url {
        urls.push(format!(
            "{}/api/projects/{}/attachments/{}/{}",
            state.public_url.trim_end_matches('/'),
            percent_encode_path_segment(&project.id),
            percent_encode_path_segment(&conversation_id),
            percent_encode_path_segment(&file_name)
        ));
    }

    let mut attachment = serde_json::json!({
        "attachment_id": attachment_id,
        "kind": kind_hint.unwrap_or("attachment"),
        "display_name": display_name,
        "file_name": file_name,
        "mime_type": mime_type_hint.unwrap_or("application/octet-stream"),
        "path": path.to_string_lossy(),
        "url": urls[0],
        "urls": urls,
        "sha256": sha256,
        "size_bytes": body.len(),
    });
    if let Some(width) = image_width_hint {
        attachment["image_width"] = serde_json::json!(width);
    }
    if let Some(height) = image_height_hint {
        attachment["image_height"] = serde_json::json!(height);
    }
    Json(serde_json::json!({
        "status": "uploaded",
        "project_id": project.id,
        "conversation_id": conversation_id,
        "attachment": attachment,
    }))
    .into_response()
}

fn parse_positive_u32(value: &str) -> Option<u32> {
    value.trim().parse::<u32>().ok().filter(|value| *value > 0)
}

pub async fn download_user_project_attachment(
    State(state): State<Arc<AppState>>,
    AxumPath((user_id, project_id, conversation_id, filename)): AxumPath<(
        String,
        String,
        String,
        String,
    )>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return json_error(StatusCode::BAD_REQUEST, "invalid filename");
    }

    let (_user, project) = match ensure_mobile_project(
        &state,
        &user_id,
        &project_id,
        query.get("title").map(String::as_str),
    ) {
        Ok(pair) => pair,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    download_project_attachment_impl(&state, &project, &conversation_id, &filename).await
}

pub async fn download_project_attachment(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    AxumPath((project_id, conversation_id, filename)): AxumPath<(String, String, String)>,
) -> Response {
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return json_error(StatusCode::BAD_REQUEST, "invalid filename");
    }

    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match project_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    download_project_attachment_impl(&state, &project, &conversation_id, &filename).await
}

async fn download_project_attachment_impl(
    state: &AppState,
    project: &ProjectAccess,
    conversation_id: &str,
    filename: &str,
) -> Response {
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return json_error(StatusCode::BAD_REQUEST, "invalid filename");
    }

    let workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
    let attachments_dir = workspace
        .join("attachments")
        .join(safe_project_path_part(conversation_id, 80));
    let path = attachments_dir.join(filename);
    let valid_path = std::fs::canonicalize(&attachments_dir)
        .ok()
        .and_then(|root| {
            std::fs::canonicalize(&path)
                .ok()
                .filter(|canonical| canonical.starts_with(root))
        })
        .is_some();
    if !valid_path {
        return json_error(StatusCode::NOT_FOUND, "attachment not found");
    }

    let data = match tokio::fs::read(&path).await {
        Ok(data) => data,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    let content_type = content_type_for_file(filename);
    axum::response::Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{}\"", filename),
        )
        .body(Body::from(data))
        .unwrap_or_else(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

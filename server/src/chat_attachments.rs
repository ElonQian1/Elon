use axum::{
    body::{Body, Bytes},
    extract::{Path as AxumPath, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, path::PathBuf, sync::Arc};

use crate::{
    project_attachment_paths::{
        content_type_for_file, percent_encode_path_segment, safe_project_path_part,
        unique_project_attachment_name,
    },
    project_auth::json_error,
    types::AppState,
};

pub const MAX_CHAT_ATTACHMENT_BYTES: usize = 12 * 1024 * 1024;

pub async fn upload_user_chat_attachment(
    State(state): State<Arc<AppState>>,
    AxumPath(user_id): AxumPath<String>,
    Query(query): Query<HashMap<String, String>>,
    body: Bytes,
) -> Response {
    let user = match state.store.ensure_device_user(&user_id) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    upload_chat_attachment_impl(state, user.id, query, body).await
}

async fn upload_chat_attachment_impl(
    state: Arc<AppState>,
    user_id: String,
    query: HashMap<String, String>,
    body: Bytes,
) -> Response {
    if body.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "attachment body is empty");
    }
    if body.len() > MAX_CHAT_ATTACHMENT_BYTES {
        return json_error(StatusCode::PAYLOAD_TOO_LARGE, "attachment is too large");
    }

    let conversation_id = query
        .get("conversation_id")
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("chat");
    let conversation_key = safe_project_path_part(conversation_id, 80);
    let attachments_dir = chat_attachment_dir(&state.workspace_root, &user_id, &conversation_key);
    if let Err(error) = tokio::fs::create_dir_all(&attachments_dir).await {
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string());
    }

    let display_name = query
        .get("display_name")
        .or_else(|| query.get("file_name"))
        .map(String::as_str)
        .unwrap_or("attachment.bin");
    let original_name = query
        .get("file_name")
        .map(String::as_str)
        .unwrap_or(display_name);
    let file_name = unique_project_attachment_name(&attachments_dir, original_name);
    let path = attachments_dir.join(&file_name);
    if let Err(error) = tokio::fs::write(&path, body.as_ref()).await {
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string());
    }

    let sha256 = hex::encode(Sha256::digest(body.as_ref()));
    let attachment_id = format!(
        "chat_att_{}_{}",
        &sha256[..16],
        uuid::Uuid::new_v4().simple()
    );
    let url = format!(
        "{}/api/user/{}/chat-attachments/{}/{}",
        state.public_url.trim_end_matches('/'),
        percent_encode_path_segment(&user_id),
        percent_encode_path_segment(&conversation_key),
        percent_encode_path_segment(&file_name)
    );

    let mut attachment = serde_json::json!({
        "attachment_id": attachment_id,
        "kind": query.get("kind").map(String::as_str).unwrap_or("attachment"),
        "display_name": display_name,
        "file_name": file_name,
        "mime_type": query
            .get("mime_type")
            .map(String::as_str)
            .unwrap_or("application/octet-stream"),
        "path": path.to_string_lossy(),
        "url": url,
        "sha256": sha256,
        "size_bytes": body.len(),
    });
    if let Some(width) = query
        .get("image_width")
        .and_then(|value| parse_positive_u32(value))
    {
        attachment["image_width"] = serde_json::json!(width);
    }
    if let Some(height) = query
        .get("image_height")
        .and_then(|value| parse_positive_u32(value))
    {
        attachment["image_height"] = serde_json::json!(height);
    }

    Json(serde_json::json!({
        "status": "uploaded",
        "conversation_id": conversation_key,
        "attachment": attachment,
    }))
    .into_response()
}

pub async fn download_user_chat_attachment(
    State(state): State<Arc<AppState>>,
    AxumPath((user_id, conversation_id, filename)): AxumPath<(String, String, String)>,
) -> Response {
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return json_error(StatusCode::BAD_REQUEST, "invalid filename");
    }

    let attachments_dir = chat_attachment_dir(&state.workspace_root, &user_id, &conversation_id);
    let path = attachments_dir.join(&filename);
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
    axum::response::Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type_for_file(&filename))
        .header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{}\"", filename),
        )
        .body(Body::from(data))
        .unwrap_or_else(|e| json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

fn chat_attachment_dir(workspace_root: &str, user_id: &str, conversation_id: &str) -> PathBuf {
    PathBuf::from(workspace_root)
        .join("chat_attachments")
        .join(safe_project_path_part(user_id, 128))
        .join(safe_project_path_part(conversation_id, 80))
}

fn parse_positive_u32(value: &str) -> Option<u32> {
    value.trim().parse::<u32>().ok().filter(|value| *value > 0)
}

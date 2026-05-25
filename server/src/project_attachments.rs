use axum::{
    Json,
    body::{Body, Bytes},
    extract::{Path as AxumPath, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
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
    body: Bytes,
    include_project_api_url: bool,
) -> Response {
    if body.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "attachment body is empty");
    }
    if body.len() > MAX_PROJECT_ATTACHMENT_BYTES {
        return json_error(StatusCode::PAYLOAD_TOO_LARGE, "attachment is too large");
    }

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

    let attachment = serde_json::json!({
        "kind": kind_hint.unwrap_or("attachment"),
        "display_name": display_name,
        "file_name": file_name,
        "mime_type": mime_type_hint.unwrap_or("application/octet-stream"),
        "path": path.to_string_lossy(),
        "url": urls[0],
        "urls": urls,
        "size_bytes": body.len(),
    });
    Json(serde_json::json!({
        "status": "uploaded",
        "project_id": project.id,
        "conversation_id": conversation_id,
        "attachment": attachment,
    }))
    .into_response()
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

pub fn append_project_attachment_notes(
    state: &AppState,
    project: &ProjectAccess,
    conversation_id: &str,
    message: String,
    attachments: Option<&[ProjectAttachmentRef]>,
) -> String {
    let Some(attachments) = attachments.filter(|items| !items.is_empty()) else {
        return message;
    };
    let workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
    let attachments_root = workspace.join("attachments");
    let canonical_root = std::fs::canonicalize(&attachments_root).ok();
    let mut notes = Vec::new();
    for attachment in attachments.iter().take(MAX_PROJECT_ATTACHMENTS_PER_MESSAGE) {
        let Some(path_text) = attachment
            .path
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        else {
            continue;
        };
        let path = PathBuf::from(path_text);
        let valid_path = canonical_root.as_ref().is_some_and(|root| {
            std::fs::canonicalize(&path)
                .map(|canonical| canonical.starts_with(root))
                .unwrap_or(false)
        });
        if !valid_path {
            notes.push(format!(
                "- {}: attachment path was rejected",
                attachment.display_name.as_deref().unwrap_or("attachment")
            ));
            continue;
        }
        let display_name = attachment
            .display_name
            .as_deref()
            .or(attachment.file_name.as_deref())
            .unwrap_or("attachment");
        let mime_type = attachment
            .mime_type
            .as_deref()
            .unwrap_or("application/octet-stream");
        let mut note = format!(
            "- {} [{}; {}; {} bytes] -> {}",
            display_name,
            attachment.kind.as_deref().unwrap_or("attachment"),
            mime_type,
            attachment.size_bytes.unwrap_or(0),
            path.display()
        );
        if let Some(url) = attachment
            .url
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            note.push_str(&format!(" (url: {})", url));
        }
        if mime_type.starts_with("image/") {
            note.push_str(
                "\n  Image context: this is an actual uploaded chat image. Open/view the local file path above when answering image questions; do not answer from the file name alone.",
            );
        }
        notes.push(note);
    }
    if attachments.len() > MAX_PROJECT_ATTACHMENTS_PER_MESSAGE {
        notes.push(format!(
            "- {} extra attachments were ignored by the message limit.",
            attachments.len() - MAX_PROJECT_ATTACHMENTS_PER_MESSAGE
        ));
    }
    if notes.is_empty() {
        return message;
    }
    format!(
        "{}\n\nUser uploaded real chat attachments for this project conversation (conversation_id={}):\n{}\nThese attachments are part of the current message context, like images/files in a normal chat app. If the user asks about an uploaded image, inspect the exact local path listed above before answering.",
        message,
        conversation_id,
        notes.join("\n")
    )
}

pub fn content_type_for_file(filename: &str) -> &'static str {
    match filename
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "txt" | "md" | "log" => "text/plain; charset=utf-8",
        "json" => "application/json",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

pub fn percent_encode_path_segment(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*byte as char)
            }
            other => encoded.push_str(&format!("%{:02X}", other)),
        }
    }
    encoded
}

pub fn unique_project_attachment_name(dir: &Path, original: &str) -> String {
    let safe = safe_project_file_name(original);
    let stamp = chrono::Utc::now().timestamp_millis();
    let mut candidate = format!("{}_{}", stamp, safe);
    let mut suffix = 1;
    while dir.join(&candidate).exists() {
        candidate = format!("{}_{}_{}", stamp, suffix, safe);
        suffix += 1;
    }
    candidate
}

pub fn safe_project_path_part(value: &str, max_len: usize) -> String {
    let safe = value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        .take(max_len)
        .collect::<String>();
    if safe.is_empty() {
        "default".into()
    } else {
        safe
    }
}

fn safe_project_file_name(original: &str) -> String {
    let mut safe = original
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                Some(ch)
            } else if ch.is_whitespace() {
                Some('_')
            } else {
                None
            }
        })
        .take(120)
        .collect::<String>();
    if safe.is_empty() || safe.trim_matches('.').is_empty() {
        safe = "attachment.bin".into();
    }
    safe
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_project_attachment_file_names() {
        assert_eq!(safe_project_file_name("../my file.png"), "..my_file.png");
        assert_eq!(safe_project_file_name("../../"), "attachment.bin");
        assert_eq!(safe_project_file_name(""), "attachment.bin");
        assert_eq!(
            safe_project_path_part("../conversation id!", 80),
            "conversationid"
        );
    }

    #[test]
    fn encodes_attachment_url_path_segments() {
        assert_eq!(
            percent_encode_path_segment("project 1/图.png"),
            "project%201%2F%E5%9B%BE.png"
        );
    }
}

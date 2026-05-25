use axum::{
    Json,
    body::{Body, Bytes},
    extract::{Path as AxumPath, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    project_attachment_paths::{
        attachment_display_name, content_type_for_file, percent_encode_path_segment,
        safe_attachment_artifact_id, safe_project_file_name, safe_project_path_part,
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

    let attachment = serde_json::json!({
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
    Json(serde_json::json!({
        "status": "uploaded",
        "project_id": project.id,
        "conversation_id": conversation_id,
        "attachment": attachment,
    }))
    .into_response()
}

pub async fn append_project_cli_attachment_artifacts(
    state: &AppState,
    project: &ProjectAccess,
    conversation_id: &str,
    message: String,
    attachments: Option<&[ProjectAttachmentRef]>,
    execution_workspace: &Path,
) -> String {
    let Some(attachments) = attachments.filter(|items| !items.is_empty()) else {
        return message;
    };

    let base_workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
    let attachments_root = base_workspace.join("attachments");
    let canonical_root = std::fs::canonicalize(&attachments_root).ok();
    let artifact_root = execution_workspace
        .join(".elon")
        .join("attachments")
        .join(safe_project_path_part(conversation_id, 80));

    let mut notes = Vec::new();
    let mut manifest_entries = Vec::new();
    if let Err(error) = tokio::fs::create_dir_all(&artifact_root).await {
        notes.push(format!(
            "- attachment artifact directory could not be created: {}",
            error
        ));
    }

    for (index, attachment) in attachments
        .iter()
        .take(MAX_PROJECT_ATTACHMENTS_PER_MESSAGE)
        .enumerate()
    {
        let display_name = attachment_display_name(attachment);
        let Some(path_text) = attachment
            .path
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        else {
            notes.push(format!(
                "- {}: missing server attachment path",
                display_name
            ));
            continue;
        };
        let Some(root) = canonical_root.as_ref() else {
            notes.push(format!(
                "- {}: server attachment root was unavailable",
                display_name
            ));
            continue;
        };
        let source_path = PathBuf::from(path_text);
        let canonical_source = match std::fs::canonicalize(&source_path) {
            Ok(path) if path.starts_with(root) => path,
            _ => {
                notes.push(format!(
                    "- {}: server attachment path was rejected",
                    display_name
                ));
                continue;
            }
        };

        let artifact_id = safe_attachment_artifact_id(
            attachment.attachment_id.as_deref(),
            attachment.sha256.as_deref(),
            index,
        );
        let artifact_file_name = format!(
            "{}_{}",
            artifact_id,
            safe_project_file_name(
                attachment
                    .file_name
                    .as_deref()
                    .unwrap_or(display_name.as_str())
            )
        );
        let artifact_path = artifact_root.join(artifact_file_name);
        if let Err(error) = tokio::fs::copy(&canonical_source, &artifact_path).await {
            notes.push(format!(
                "- {} [{}]: failed to copy into CLI workspace: {}",
                display_name, artifact_id, error
            ));
            continue;
        }

        let sha256 = match attachment
            .sha256
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some(value) => value.to_string(),
            None => match tokio::fs::read(&artifact_path).await {
                Ok(data) => hex::encode(Sha256::digest(&data)),
                Err(_) => String::new(),
            },
        };
        let size_bytes = attachment
            .size_bytes
            .or_else(|| {
                std::fs::metadata(&artifact_path)
                    .ok()
                    .map(|metadata| metadata.len())
            })
            .unwrap_or(0);
        let kind = attachment.kind.as_deref().unwrap_or("attachment");
        let mime_type = attachment
            .mime_type
            .as_deref()
            .unwrap_or("application/octet-stream");
        let url = attachment
            .url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());

        let mut note = format!(
            "- {} [{}; {}; {} bytes]\n  artifact_id: {}\n  cli_workspace_path: {}\n  source_server_path: {}",
            display_name,
            kind,
            mime_type,
            size_bytes,
            artifact_id,
            artifact_path.display(),
            canonical_source.display()
        );
        if !sha256.is_empty() {
            note.push_str(&format!("\n  sha256: {}", sha256));
        }
        if let Some(url) = url {
            note.push_str(&format!("\n  url: {}", url));
        }
        if mime_type.starts_with("image/") {
            note.push_str("\n  Image context: inspect cli_workspace_path directly when this message asks about the image.");
        }
        notes.push(note);

        manifest_entries.push(serde_json::json!({
            "attachment_id": artifact_id,
            "display_name": display_name,
            "file_name": attachment.file_name.as_deref().unwrap_or("attachment"),
            "kind": kind,
            "mime_type": mime_type,
            "size_bytes": size_bytes,
            "sha256": sha256,
            "cli_workspace_path": artifact_path.to_string_lossy(),
            "source_server_path": canonical_source.to_string_lossy(),
            "url": url,
        }));
    }

    if attachments.len() > MAX_PROJECT_ATTACHMENTS_PER_MESSAGE {
        notes.push(format!(
            "- {} extra attachments were ignored by the message limit.",
            attachments.len() - MAX_PROJECT_ATTACHMENTS_PER_MESSAGE
        ));
    }

    if !manifest_entries.is_empty() {
        let manifest_path = artifact_root.join("attachments.json");
        let manifest = serde_json::json!({
            "conversation_id": conversation_id,
            "artifact_root": artifact_root.to_string_lossy(),
            "attachments": manifest_entries,
        });
        match serde_json::to_vec_pretty(&manifest) {
            Ok(bytes) => {
                if tokio::fs::write(&manifest_path, bytes).await.is_ok() {
                    notes.push(format!("- manifest: {}", manifest_path.display()));
                }
            }
            Err(error) => notes.push(format!("- manifest serialization failed: {}", error)),
        }
    }

    if notes.is_empty() {
        return message;
    }
    format!(
        "{}\n\nCLI attachment artifacts prepared for this project task (conversation_id={}):\n{}\nPrefer cli_workspace_path when inspecting uploaded images/files in Codex CLI. The source_server_path is a fallback for server-side diagnostics.",
        message,
        conversation_id,
        notes.join("\n")
    )
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

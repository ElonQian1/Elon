use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::{
    project_attachment_paths::{
        attachment_display_name, safe_attachment_artifact_id, safe_project_file_name,
        safe_project_path_part,
    },
    project_attachments::MAX_PROJECT_ATTACHMENTS_PER_MESSAGE,
    project_ws_protocol::ProjectAttachmentRef,
    store::ProjectAccess,
    types::AppState,
};

pub(crate) fn append_project_attachment_notes(
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
            if let Some(dimensions) = attachment_image_dimensions(attachment) {
                note.push_str(&format!("\n  image_dimensions: {}", dimensions));
            }
            if let Some(annotation_notes) = attachment_annotations_summary(attachment) {
                note.push_str(&format!("\n  image_annotations:\n{}", annotation_notes));
            }
            note.push_str(
                "\n  Image context: this image has been passed via --attachment; Copilot can view it directly. Do NOT try to open the local path above (it may not exist); use the image content that was already loaded.",
            );
        } else if mime_type.starts_with("audio/") || attachment.kind.as_deref() == Some("audio") {
            note.push_str(
                "\n  Voice context: this is the user's raw uploaded voice input. Prefer the audio file over any placeholder message text when the runtime can inspect or transcribe audio.",
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

fn attachment_image_dimensions(attachment: &ProjectAttachmentRef) -> Option<String> {
    Some(format!(
        "{}x{}",
        attachment.image_width?, attachment.image_height?
    ))
}

fn attachment_annotations_summary(attachment: &ProjectAttachmentRef) -> Option<String> {
    if attachment.annotations.is_empty() {
        return None;
    }
    let lines = attachment
        .annotations
        .iter()
        .enumerate()
        .filter_map(|(index, annotation)| {
            let note = annotation.note.trim();
            if note.is_empty() {
                return None;
            }
            Some(format!(
                "  - #{} at x={:.3}, y={:.3}, width={:.3}, height={:.3}: {}",
                index + 1,
                annotation.x,
                annotation.y,
                annotation.width,
                annotation.height,
                note.chars().take(500).collect::<String>()
            ))
        })
        .collect::<Vec<_>>();
    (!lines.is_empty()).then(|| lines.join("\n"))
}

/// CLI 任务执行时把附件文件复制到工作区，并生成 manifest 和提示注释。
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
        let dimensions = attachment_image_dimensions(attachment);

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
            if let Some(dimensions) = dimensions.as_deref() {
                note.push_str(&format!("\n  image_dimensions: {}", dimensions));
            }
            if let Some(annotation_notes) = attachment_annotations_summary(attachment) {
                note.push_str(&format!("\n  image_annotations:\n{}", annotation_notes));
            }
            note.push_str("\n  Image context: inspect cli_workspace_path directly when this message asks about the image.");
        } else if mime_type.starts_with("audio/") || kind == "audio" {
            note.push_str(
                "\n  Voice context: this is the user's raw uploaded voice input. Prefer cli_workspace_path over any placeholder message text when the runtime can inspect or transcribe audio.",
            );
        }
        notes.push(note);

        manifest_entries.push(serde_json::json!({
            "attachment_id": artifact_id,
            "display_name": display_name,
            "file_name": attachment.file_name.as_deref().unwrap_or("attachment"),
            "kind": kind,
            "mime_type": mime_type,
            "size_bytes": size_bytes,
            "image_width": attachment.image_width,
            "image_height": attachment.image_height,
            "annotations": &attachment.annotations,
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

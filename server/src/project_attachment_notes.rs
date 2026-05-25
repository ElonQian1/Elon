use std::path::PathBuf;

use crate::{
    project_attachments::MAX_PROJECT_ATTACHMENTS_PER_MESSAGE,
    project_ws_protocol::ProjectAttachmentRef, store::ProjectAccess, types::AppState,
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

fn attachment_image_dimensions(attachment: &ProjectAttachmentRef) -> Option<String> {
    Some(format!(
        "{}x{}",
        attachment.image_width?,
        attachment.image_height?
    ))
}

use std::path::{Path, PathBuf};

use crate::{project_ws_protocol::ProjectAttachmentRef, store::ProjectAccess, types::AppState};

pub const MAX_PROJECT_ATTACHMENTS_PER_MESSAGE: usize = 6;
pub const MAX_PROJECT_ATTACHMENT_BYTES: usize = 12 * 1024 * 1024;

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

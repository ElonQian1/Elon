use std::path::Path;

use crate::project_ws_protocol::ProjectAttachmentRef;

pub(crate) fn content_type_for_file(filename: &str) -> &'static str {
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

pub(crate) fn percent_encode_path_segment(value: &str) -> String {
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

pub(crate) fn unique_project_attachment_name(dir: &Path, original: &str) -> String {
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

pub(crate) fn safe_project_path_part(value: &str, max_len: usize) -> String {
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

pub(crate) fn safe_project_file_name(original: &str) -> String {
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

pub(crate) fn attachment_display_name(attachment: &ProjectAttachmentRef) -> String {
    attachment
        .display_name
        .as_deref()
        .or(attachment.file_name.as_deref())
        .unwrap_or("attachment")
        .to_string()
}

pub(crate) fn safe_attachment_artifact_id(
    attachment_id: Option<&str>,
    sha256: Option<&str>,
    index: usize,
) -> String {
    let raw = attachment_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            sha256
                .map(str::trim)
                .filter(|value| value.len() >= 16)
                .map(|value| format!("att_{}", &value[..16]))
        })
        .unwrap_or_else(|| format!("att_{}", index + 1));
    safe_project_path_part(&raw, 96)
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

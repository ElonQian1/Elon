use crate::project_ws_protocol::{ProjectAttachmentAnnotation, ProjectAttachmentRef};

pub(crate) fn append_to_message_content(
    content: &str,
    attachments: &[ProjectAttachmentRef],
) -> String {
    let content = content.trim();
    let Some(attachment_context) = attachment_context_summary(attachments) else {
        return content.to_string();
    };
    if content.is_empty() {
        attachment_context
    } else {
        format!("{content}\n{attachment_context}")
    }
}

fn attachment_context_summary(attachments: &[ProjectAttachmentRef]) -> Option<String> {
    if attachments.is_empty() {
        return None;
    }
    let mut lines = vec![
        "Attached media context for AI. Treat image annotations below as user-provided marked content:"
            .to_string(),
    ];
    for (index, attachment) in attachments.iter().enumerate() {
        lines.extend(format_attachment(index, attachment));
    }
    (lines.len() > 1).then(|| lines.join("\n"))
}

fn format_attachment(index: usize, attachment: &ProjectAttachmentRef) -> Vec<String> {
    let display_name = attachment_display_name(attachment);
    let kind = attachment.kind.as_deref().unwrap_or("attachment");
    let mime_type = attachment
        .mime_type
        .as_deref()
        .unwrap_or("application/octet-stream");
    let mut lines = vec![format!(
        "- attachment #{}: {} (kind={}, mime={})",
        index + 1,
        display_name,
        kind,
        mime_type
    )];

    if let (Some(width), Some(height)) = (attachment.image_width, attachment.image_height) {
        lines.push(format!("  image_dimensions: {}x{}", width, height));
    }

    if is_image_attachment(attachment) {
        let annotations = format_image_annotations(&attachment.annotations);
        if annotations.is_empty() {
            lines.push("  image_annotations: none".to_string());
        } else {
            lines.push("  image_annotations in marker order:".to_string());
            lines.extend(annotations);
        }
    }

    if let Some(transcription) = attachment
        .transcription
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        lines.push(format!(
            "  audio_transcription: {}",
            truncate_chars(&single_line(transcription), 600)
        ));
    }

    lines
}

fn format_image_annotations(annotations: &[ProjectAttachmentAnnotation]) -> Vec<String> {
    annotations
        .iter()
        .enumerate()
        .filter_map(|(index, annotation)| {
            let note = single_line(annotation.note.trim());
            if note.is_empty() {
                return None;
            }
            let icon = match (
                annotation.icon_x,
                annotation.icon_y,
                annotation.icon_width,
                annotation.icon_height,
            ) {
                (Some(x), Some(y), Some(width), Some(height)) => format!(
                    "; icon x={}, y={}, width={}, height={}",
                    coord(x),
                    coord(y),
                    coord(width),
                    coord(height)
                ),
                _ => String::new(),
            };
            Some(format!(
                "  - image_annotation #{}: region x={}, y={}, width={}, height={}{}; note: {}",
                index + 1,
                coord(annotation.x),
                coord(annotation.y),
                coord(annotation.width),
                coord(annotation.height),
                icon,
                truncate_chars(&note, 600)
            ))
        })
        .collect()
}

fn attachment_display_name(attachment: &ProjectAttachmentRef) -> String {
    attachment
        .display_name
        .as_deref()
        .or(attachment.file_name.as_deref())
        .or(attachment.attachment_id.as_deref())
        .unwrap_or("attachment")
        .trim()
        .chars()
        .take(120)
        .collect()
}

fn is_image_attachment(attachment: &ProjectAttachmentRef) -> bool {
    attachment_field_matches(&attachment.kind, &["image", "photo"])
        || attachment_mime_starts_with(&attachment.mime_type, "image/")
        || attachment_has_extension(attachment, &["jpg", "jpeg", "png", "gif", "webp", "bmp"])
}

fn attachment_field_matches(value: &Option<String>, choices: &[&str]) -> bool {
    value.as_deref().map(str::trim).is_some_and(|value| {
        choices
            .iter()
            .any(|choice| value.eq_ignore_ascii_case(choice))
    })
}

fn attachment_mime_starts_with(value: &Option<String>, prefix: &str) -> bool {
    value
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| value.to_ascii_lowercase().starts_with(prefix))
}

fn attachment_has_extension(attachment: &ProjectAttachmentRef, extensions: &[&str]) -> bool {
    [
        &attachment.file_name,
        &attachment.display_name,
        &attachment.path,
        &attachment.url,
    ]
    .into_iter()
    .filter_map(|value| value.as_deref())
    .filter_map(|value| value.rsplit(['/', '\\']).next())
    .filter_map(|value| value.split('?').next())
    .filter_map(|value| value.rsplit_once('.').map(|(_, extension)| extension))
    .any(|extension| {
        extensions
            .iter()
            .any(|choice| extension.eq_ignore_ascii_case(choice))
    })
}

fn single_line(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn coord(value: f32) -> String {
    format!("{value:.3}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annotated_image_context_keeps_marker_order_and_notes() {
        let text = append_to_message_content(
            "",
            &[ProjectAttachmentRef {
                attachment_id: Some("att_1".to_string()),
                kind: Some("image".to_string()),
                display_name: Some("marked.jpg".to_string()),
                file_name: Some("marked.jpg".to_string()),
                mime_type: Some("image/jpeg".to_string()),
                path: None,
                url: None,
                sha256: None,
                size_bytes: Some(2048),
                image_width: Some(1080),
                image_height: Some(720),
                duration_seconds: None,
                transcription: None,
                annotations: vec![
                    ProjectAttachmentAnnotation {
                        x: 0.1,
                        y: 0.2,
                        width: 0.3,
                        height: 0.4,
                        note: "first marked note".to_string(),
                        icon_x: Some(0.45),
                        icon_y: Some(0.56),
                        icon_width: Some(0.06),
                        icon_height: Some(0.07),
                    },
                    ProjectAttachmentAnnotation {
                        x: 0.5,
                        y: 0.6,
                        width: 0.2,
                        height: 0.1,
                        note: "second marked note".to_string(),
                        icon_x: None,
                        icon_y: None,
                        icon_width: None,
                        icon_height: None,
                    },
                ],
            }],
        );

        assert!(text.contains("image_annotation #1"));
        assert!(text.contains("first marked note"));
        assert!(text.contains("image_annotation #2"));
        assert!(text.contains("second marked note"));
        assert!(text.contains("image_dimensions: 1080x720"));
    }
}

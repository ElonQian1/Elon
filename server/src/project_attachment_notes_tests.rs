    use super::{attachment_annotations_summary, project_message_with_attachment_fallback};
    use crate::project_ws_protocol::{ProjectAttachmentAnnotation, ProjectAttachmentRef};

    #[test]
    fn project_attachment_annotations_are_summarized_for_ai_context() {
        let attachment = ProjectAttachmentRef {
            attachment_id: Some("att_marked".to_string()),
            kind: Some("image".to_string()),
            display_name: Some("marked.jpg".to_string()),
            file_name: Some("marked.jpg".to_string()),
            mime_type: Some("image/jpeg".to_string()),
            path: Some("/workspace/attachments/marked.jpg".to_string()),
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
                    note: "first marked requirement".to_string(),
                    icon_x: Some(0.41),
                    icon_y: Some(0.58),
                    icon_width: Some(0.06),
                    icon_height: Some(0.08),
                },
                ProjectAttachmentAnnotation {
                    x: 0.5,
                    y: 0.6,
                    width: 0.2,
                    height: 0.1,
                    note: "second marked requirement".to_string(),
                    icon_x: None,
                    icon_y: None,
                    icon_width: None,
                    icon_height: None,
                },
            ],
        };

        let summary =
            attachment_annotations_summary(&attachment).expect("summary should be generated");

        assert!(summary.contains("#1"));
        assert!(summary.contains("first marked requirement"));
        assert!(summary.contains("#2"));
        assert!(summary.contains("second marked requirement"));
        assert!(summary.contains("x=0.100"));
    }

    #[test]
    fn empty_project_message_uses_image_annotation_fallback() {
        let attachment = ProjectAttachmentRef {
            attachment_id: Some("att_marked".to_string()),
            kind: Some("image".to_string()),
            display_name: Some("marked.jpg".to_string()),
            file_name: Some("marked.jpg".to_string()),
            mime_type: Some("image/jpeg".to_string()),
            path: Some("/workspace/attachments/marked.jpg".to_string()),
            url: None,
            sha256: None,
            size_bytes: Some(2048),
            image_width: Some(1080),
            image_height: Some(720),
            duration_seconds: None,
            transcription: None,
            annotations: vec![ProjectAttachmentAnnotation {
                x: 0.1,
                y: 0.2,
                width: 0.3,
                height: 0.4,
                note: "add a yellow button named 魔王".to_string(),
                icon_x: None,
                icon_y: None,
                icon_width: None,
                icon_height: None,
            }],
        };

        let message =
            project_message_with_attachment_fallback("   ".to_string(), Some(&[attachment]));

        assert!(message.contains("uploaded attachments"));
        assert!(message.contains("annotation #1"));
        assert!(message.contains("add a yellow button named 魔王"));
    }

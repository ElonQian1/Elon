use super::*;

#[test]
fn parses_mobile_camel_case_ui_design_task() {
    let task: UiDesignTaskInput = serde_json::from_str(
        r#"{
            "taskId":"ui_1",
            "mode":"CREATE_NEW",
            "attachmentIntent":"TARGET_DESIGN",
            "screenId":"checkout",
            "targetDesignAttachmentId":"att_target",
            "renderTarget":{
                "kind":"PREVIEW_HOST",
                "fontScale":1.3,
                "localeTag":"zh-CN"
            }
        }"#,
    )
    .expect("task should parse");

    assert_eq!(task.mode, UiDesignTaskMode::CreateNew);
    assert_eq!(task.attachment_intent, UiDesignAttachmentIntent::TargetDesign);
    assert_eq!(task.render_target.kind, UiDesignRenderTargetKind::PreviewHost);
    assert!(task.execution_policy.require_build_verification);
}

#[test]
fn auto_mode_uses_available_project_evidence() {
    let task = UiDesignTaskInput::default();

    assert_eq!(
        task.resolved_mode(UiDesignTaskEvidence::default()),
        UiDesignTaskMode::CreateNew
    );
    assert_eq!(
        task.resolved_mode(UiDesignTaskEvidence {
            has_source_candidate: true,
            ..UiDesignTaskEvidence::default()
        }),
        UiDesignTaskMode::ModifyExisting
    );
    assert_eq!(
        task.resolved_mode(UiDesignTaskEvidence {
            has_source_candidate: true,
            requires_structure_extension: true,
            ..UiDesignTaskEvidence::default()
        }),
        UiDesignTaskMode::ExtendExisting
    );
}

#[test]
fn validation_rejects_unsafe_or_unbounded_values() {
    let mut task = UiDesignTaskInput::default();
    task.render_target.font_scale = Some(5.0);
    assert!(task.validate().is_err());

    task.render_target.font_scale = Some(1.0);
    task.task_id = Some("bad\nvalue".into());
    assert!(task.validate().is_err());
}

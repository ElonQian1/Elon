use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) enum UiDesignTaskMode {
    #[default]
    #[serde(rename = "AUTO", alias = "auto")]
    Auto,
    #[serde(rename = "MODIFY_EXISTING", alias = "modify_existing")]
    ModifyExisting,
    #[serde(rename = "EXTEND_EXISTING", alias = "extend_existing")]
    ExtendExisting,
    #[serde(rename = "CREATE_NEW", alias = "create_new")]
    CreateNew,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) enum UiDesignAttachmentIntent {
    #[default]
    #[serde(rename = "AUTO", alias = "auto")]
    Auto,
    #[serde(rename = "TARGET_DESIGN", alias = "target_design")]
    TargetDesign,
    #[serde(
        rename = "ANNOTATED_CHANGE_REQUEST",
        alias = "annotated_change_request"
    )]
    AnnotatedChangeRequest,
    #[serde(rename = "REFERENCE_STYLE", alias = "reference_style")]
    ReferenceStyle,
    #[serde(rename = "CURRENT_SCREENSHOT", alias = "current_screenshot")]
    CurrentScreenshot,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) enum UiDesignRenderTargetKind {
    #[default]
    #[serde(rename = "AUTO", alias = "auto")]
    Auto,
    #[serde(rename = "PREVIEW_HOST", alias = "preview_host")]
    PreviewHost,
    #[serde(rename = "EMULATOR", alias = "emulator")]
    Emulator,
    #[serde(rename = "CONNECTED_DEVICE", alias = "connected_device")]
    ConnectedDevice,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub(crate) struct UiDesignRenderTarget {
    #[serde(default)]
    pub(crate) kind: UiDesignRenderTargetKind,
    #[serde(default, alias = "deviceSerial")]
    pub(crate) device_serial: Option<String>,
    #[serde(default, alias = "screenId")]
    pub(crate) screen_id: Option<String>,
    #[serde(default)]
    pub(crate) scenario: Option<String>,
    #[serde(default)]
    pub(crate) theme: Option<String>,
    #[serde(default, alias = "fontScale")]
    pub(crate) font_scale: Option<f32>,
    #[serde(default, alias = "localeTag")]
    pub(crate) locale_tag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct UiDesignExecutionPolicy {
    #[serde(default = "default_true", alias = "allowLivePatch")]
    pub(crate) allow_live_patch: bool,
    #[serde(default = "default_true", alias = "allowDeterministicCommit")]
    pub(crate) allow_deterministic_commit: bool,
    #[serde(default = "default_true", alias = "allowSourceEdit")]
    pub(crate) allow_source_edit: bool,
    #[serde(default = "default_true", alias = "requireBuildVerification")]
    pub(crate) require_build_verification: bool,
}

impl Default for UiDesignExecutionPolicy {
    fn default() -> Self {
        Self {
            allow_live_patch: true,
            allow_deterministic_commit: true,
            allow_source_edit: true,
            require_build_verification: true,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub(crate) struct UiDesignTaskInput {
    #[serde(default, alias = "taskId")]
    pub(crate) task_id: Option<String>,
    #[serde(default)]
    pub(crate) mode: UiDesignTaskMode,
    #[serde(default, alias = "attachmentIntent")]
    pub(crate) attachment_intent: UiDesignAttachmentIntent,
    #[serde(default, alias = "screenId")]
    pub(crate) screen_id: Option<String>,
    #[serde(default, alias = "entryPoint")]
    pub(crate) entry_point: Option<String>,
    #[serde(default, alias = "targetNodeId")]
    pub(crate) target_node_id: Option<String>,
    #[serde(default, alias = "definitionId")]
    pub(crate) definition_id: Option<String>,
    #[serde(default, alias = "behaviorNotes")]
    pub(crate) behavior_notes: Vec<String>,
    #[serde(default, alias = "originalAttachmentId")]
    pub(crate) original_attachment_id: Option<String>,
    #[serde(default, alias = "annotatedPreviewAttachmentId")]
    pub(crate) annotated_preview_attachment_id: Option<String>,
    #[serde(default, alias = "targetDesignAttachmentId")]
    pub(crate) target_design_attachment_id: Option<String>,
    #[serde(default, alias = "referenceAttachmentIds")]
    pub(crate) reference_attachment_ids: Vec<String>,
    #[serde(default, alias = "renderTarget")]
    pub(crate) render_target: UiDesignRenderTarget,
    #[serde(default, alias = "executionPolicy")]
    pub(crate) execution_policy: UiDesignExecutionPolicy,
    #[serde(default, alias = "routeLearningId")]
    pub(crate) route_learning_id: Option<String>,
    #[serde(default, alias = "routeLearningOrigin")]
    pub(crate) route_learning_origin: Option<String>,
    #[serde(default, alias = "routeLearningPhrase")]
    pub(crate) route_learning_phrase: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct UiDesignTaskEvidence {
    pub(crate) has_source_candidate: bool,
    pub(crate) has_runtime_node: bool,
    pub(crate) requires_structure_extension: bool,
}

impl UiDesignTaskInput {
    pub(crate) fn resolved_mode(&self, evidence: UiDesignTaskEvidence) -> UiDesignTaskMode {
        if self.mode != UiDesignTaskMode::Auto {
            return self.mode;
        }
        if !evidence.has_source_candidate && !evidence.has_runtime_node {
            return UiDesignTaskMode::CreateNew;
        }
        if evidence.requires_structure_extension {
            return UiDesignTaskMode::ExtendExisting;
        }
        UiDesignTaskMode::ModifyExisting
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        validate_optional_id("taskId", self.task_id.as_deref())?;
        validate_optional_id("screenId", self.screen_id.as_deref())?;
        validate_optional_id("targetNodeId", self.target_node_id.as_deref())?;
        validate_optional_id("definitionId", self.definition_id.as_deref())?;
        validate_optional_id("routeLearningId", self.route_learning_id.as_deref())?;
        if self
            .route_learning_phrase
            .as_deref()
            .is_some_and(|value| value.chars().count() > 2_000)
        {
            return Err("routeLearningPhrase 不能超过 2000 个字符".into());
        }
        validate_optional_id(
            "originalAttachmentId",
            self.original_attachment_id.as_deref(),
        )?;
        validate_optional_id(
            "annotatedPreviewAttachmentId",
            self.annotated_preview_attachment_id.as_deref(),
        )?;
        validate_optional_id(
            "targetDesignAttachmentId",
            self.target_design_attachment_id.as_deref(),
        )?;
        if self.behavior_notes.len() > 32 {
            return Err("behaviorNotes 最多允许 32 条".into());
        }
        if self
            .behavior_notes
            .iter()
            .any(|note| note.chars().count() > 2_000)
        {
            return Err("单条 behaviorNotes 不能超过 2000 个字符".into());
        }
        if self.reference_attachment_ids.len() > 16 {
            return Err("referenceAttachmentIds 最多允许 16 个附件".into());
        }
        if let Some(font_scale) = self.render_target.font_scale {
            if !(0.5..=3.0).contains(&font_scale) {
                return Err("fontScale 必须在 0.5 到 3.0 之间".into());
            }
        }
        Ok(())
    }
}

fn validate_optional_id(field: &str, value: Option<&str>) -> Result<(), String> {
    let Some(value) = value else {
        return Ok(());
    };
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{field} 不能为空字符串"));
    }
    if value.chars().count() > 200 {
        return Err(format!("{field} 不能超过 200 个字符"));
    }
    if value.chars().any(char::is_control) {
        return Err(format!("{field} 不能包含控制字符"));
    }
    Ok(())
}

const fn default_true() -> bool {
    true
}

use super::{UiDesignAttachmentIntent, UiDesignTaskInput, UiDesignTaskMode};
use crate::project_ws_protocol::ProjectAttachmentRef;
use serde::{Deserialize, Serialize};

const TASK_MARKER_BEGIN: &str = "<elon-ui-design-task version=\"1\">";
const TASK_MARKER_END: &str = "</elon-ui-design-task>";

/// 把客户端提交的强类型 UI 任务追加为机器可识别的执行契约。
///
/// 这里只负责跨服务器/PC 节点兼容传输；节点侧会在启动 Codex 前读取该契约，
/// 准备附件、项目 UI Profile 和可用的 Live Runtime MCP。
pub(crate) fn append_ui_design_task_context(
    message: String,
    task: Option<&UiDesignTaskInput>,
    attachments: Option<&[ProjectAttachmentRef]>,
) -> Result<String, String> {
    let Some(task) = task else {
        return Ok(message);
    };
    task.validate()?;
    let envelope = UiDesignTaskEnvelope {
        task: task.clone(),
        attachments: design_attachments(attachments),
    };
    let task_json = serde_json::to_string(&envelope)
        .map_err(|error| format!("UI 设计任务序列化失败: {error}"))?;
    let mode_contract = mode_contract(task.mode);
    let image_contract = attachment_contract(task.attachment_intent);
    Ok(format!(
        "{message}\n\n{TASK_MARKER_BEGIN}\n{task_json}\n{TASK_MARKER_END}\n\
         This is a structured UI design development task supplied by the trusted Elon task router.\n\
         Execution contract:\n\
         - {mode_contract}\n\
         - {image_contract}\n\
         - Prefer project UI profile, component catalog, design tokens and targeted source bundles over scanning the whole repository.\n\
         - Do not stop after writing a skeleton or source file. For CREATE_NEW/EXTEND_EXISTING, compile and call ui_prepare_debug_runtime so the real Android Renderer becomes the authority.\n\
         - Bind only a clean TARGET_DESIGN with ui_bind_target_design. Map annotated requests with ui_map_annotations_to_nodes instead of comparing annotation pixels.\n\
         - Once a real Preview/Runtime node exists, start a persistent ui_start_fit_run for each target region. Let its local solver perform numeric trials without model tokens.\n\
         - If a FitRun reaches AWAITING_CODEX, use its compact handoff to make the smallest structural/source change, report it with ui_control_fit_run, and let the run continue.\n\
         - If a FitRun reaches CANDIDATE_READY, confirm it with ACCEPT_BEST so deterministic write-back, patch-free build verification and reusable learning can finish.\n\
         - Runtime patches are previews only; never report completion until source write-back and patch-free build verification pass when requireBuildVerification is true.\n\
         - Finish with a concise Chinese result containing created/changed files, FitRun outcomes, final visual loss, verification status, APK result and any remaining human decision."
    ))
}

/// 从服务器生成的任务契约中取出远程节点可下载的图片 URL。
/// 仅允许当前一龙服务器自己的 `/api/` 附件地址，防止把节点变成任意 URL 下载器。
pub(crate) fn ui_design_image_attachment_urls(
    prompt: &str,
    public_url: &str,
) -> Vec<String> {
    let Some(envelope) = parse_task_envelope(prompt) else {
        return Vec::new();
    };
    let allowed_prefix = format!("{}/api/", public_url.trim_end_matches('/'));
    envelope
        .attachments
        .into_iter()
        .filter(|attachment| attachment.mime_type.starts_with("image/"))
        .filter_map(|attachment| attachment.url)
        .filter(|url| url.starts_with(&allowed_prefix))
        .take(8)
        .collect()
}

fn parse_task_envelope(prompt: &str) -> Option<UiDesignTaskEnvelope> {
    let (_, rest) = prompt.rsplit_once(TASK_MARKER_BEGIN)?;
    let (json, _) = rest.split_once(TASK_MARKER_END)?;
    serde_json::from_str(json.trim()).ok()
}

fn design_attachments(attachments: Option<&[ProjectAttachmentRef]>) -> Vec<UiDesignAttachment> {
    attachments
        .unwrap_or_default()
        .iter()
        .take(16)
        .map(|attachment| UiDesignAttachment {
            attachment_id: attachment.attachment_id.clone(),
            display_name: attachment.display_name.clone(),
            mime_type: attachment.mime_type.clone().unwrap_or_default(),
            url: attachment.url.clone(),
            sha256: attachment.sha256.clone(),
            image_width: attachment.image_width,
            image_height: attachment.image_height,
            annotations: attachment
                .annotations
                .iter()
                .map(|annotation| UiDesignAnnotation {
                    x: annotation.x,
                    y: annotation.y,
                    width: annotation.width,
                    height: annotation.height,
                    note: annotation.note.clone(),
                })
                .collect(),
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UiDesignTaskEnvelope {
    task: UiDesignTaskInput,
    #[serde(default)]
    attachments: Vec<UiDesignAttachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UiDesignAttachment {
    attachment_id: Option<String>,
    display_name: Option<String>,
    #[serde(default)]
    mime_type: String,
    url: Option<String>,
    sha256: Option<String>,
    image_width: Option<u32>,
    image_height: Option<u32>,
    #[serde(default)]
    annotations: Vec<UiDesignAnnotation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UiDesignAnnotation {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    note: String,
}

fn mode_contract(mode: UiDesignTaskMode) -> &'static str {
    match mode {
        UiDesignTaskMode::Auto => {
            "Resolve AUTO as MODIFY_EXISTING, EXTEND_EXISTING or CREATE_NEW from project/runtime evidence before editing."
        }
        UiDesignTaskMode::ModifyExisting => {
            "MODIFY_EXISTING: locate the existing screen/node and prefer LIVE style tuning before source edits."
        }
        UiDesignTaskMode::ExtendExisting => {
            "EXTEND_EXISTING: add only the missing structure, build once, then continue through Runtime fitting."
        }
        UiDesignTaskMode::CreateNew => {
            "CREATE_NEW: the target screen may not exist. Create a Preview-first screen skeleton with mock scenarios, stable node IDs and style bindings; compile it before starting visual fitting."
        }
    }
}

fn attachment_contract(intent: UiDesignAttachmentIntent) -> &'static str {
    match intent {
        UiDesignAttachmentIntent::Auto => {
            "Determine whether each image is a clean target, an annotated change request, a current screenshot or style reference; never assume annotation overlays are target pixels."
        }
        UiDesignAttachmentIntent::TargetDesign => {
            "TARGET_DESIGN is a clean visual target and may participate in geometric/pixel comparison."
        }
        UiDesignAttachmentIntent::AnnotatedChangeRequest => {
            "ANNOTATED_CHANGE_REQUEST annotations are semantic instructions. Exclude arrows, labels and drawing overlays from pixel fitting."
        }
        UiDesignAttachmentIntent::ReferenceStyle => {
            "REFERENCE_STYLE is inspirational evidence only and must not be treated as a pixel-perfect target."
        }
        UiDesignAttachmentIntent::CurrentScreenshot => {
            "CURRENT_SCREENSHOT describes the existing result and must not be treated as the desired target."
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_new_contract_requires_preview_bootstrap_before_fit() {
        let task = UiDesignTaskInput {
            mode: UiDesignTaskMode::CreateNew,
            attachment_intent: UiDesignAttachmentIntent::AnnotatedChangeRequest,
            ..UiDesignTaskInput::default()
        };

        let prompt = append_ui_design_task_context("创建页面".into(), Some(&task), None)
            .expect("context should append");

        assert!(prompt.contains(TASK_MARKER_BEGIN));
        assert!(prompt.contains("Preview-first screen skeleton"));
        assert!(prompt.contains("Exclude arrows, labels and drawing overlays"));
    }

    #[test]
    fn only_returns_image_urls_from_current_server() {
        let task = UiDesignTaskInput::default();
        let attachments = vec![ProjectAttachmentRef {
            attachment_id: Some("att_1".into()),
            kind: Some("image".into()),
            display_name: Some("target.png".into()),
            file_name: Some("target.png".into()),
            mime_type: Some("image/png".into()),
            path: None,
            url: Some("https://elon.test/api/user/u/projects/p/attachments/a.png".into()),
            sha256: Some("abc".into()),
            size_bytes: Some(3),
            image_width: Some(100),
            image_height: Some(200),
            duration_seconds: None,
            transcription: None,
            annotations: Vec::new(),
        }];
        let prompt = append_ui_design_task_context(
            "创建页面".into(),
            Some(&task),
            Some(&attachments),
        )
        .expect("context should append");

        assert_eq!(
            ui_design_image_attachment_urls(&prompt, "https://elon.test"),
            vec!["https://elon.test/api/user/u/projects/p/attachments/a.png"]
        );
        assert!(ui_design_image_attachment_urls(&prompt, "https://other.test").is_empty());
    }
}

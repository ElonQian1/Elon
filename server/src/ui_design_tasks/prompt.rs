use super::{UiDesignAttachmentIntent, UiDesignTaskInput, UiDesignTaskMode};

const TASK_MARKER_BEGIN: &str = "<elon-ui-design-task version=\"1\">";
const TASK_MARKER_END: &str = "</elon-ui-design-task>";

/// 把客户端提交的强类型 UI 任务追加为机器可识别的执行契约。
///
/// 这里只负责跨服务器/PC 节点兼容传输；节点侧会在启动 Codex 前读取该契约，
/// 准备附件、项目 UI Profile 和可用的 Live Runtime MCP。
pub(crate) fn append_ui_design_task_context(
    message: String,
    task: Option<&UiDesignTaskInput>,
) -> Result<String, String> {
    let Some(task) = task else {
        return Ok(message);
    };
    task.validate()?;
    let task_json = serde_json::to_string(task)
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
         - Once a real Preview/Runtime node exists, use yilong-ui-live tools for numeric fitting and deterministic style commits.\n\
         - Runtime patches are previews only; completion requires source write-back and a patch-free build verification when requireBuildVerification is true."
    ))
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

        let prompt = append_ui_design_task_context("创建页面".into(), Some(&task))
            .expect("context should append");

        assert!(prompt.contains(TASK_MARKER_BEGIN));
        assert!(prompt.contains("Preview-first screen skeleton"));
        assert!(prompt.contains("Exclude arrows, labels and drawing overlays"));
    }
}

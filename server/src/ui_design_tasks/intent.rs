use super::{UiDesignAttachmentIntent, UiDesignTaskInput, UiDesignTaskMode};
use crate::project_ws_protocol::ProjectAttachmentRef;

/// 兼容没有 `uiDesignTask` 字段的旧 APK、PC 页面和普通文本入口。
///
/// 这里只做高置信度、确定性的 UI 意图识别，避免调用模型做前置分类。
pub(super) fn infer_ui_design_task(
    message: &str,
    attachments: Option<&[ProjectAttachmentRef]>,
) -> Option<UiDesignTaskInput> {
    let normalized = message.to_lowercase();
    let images: Vec<&ProjectAttachmentRef> = attachments
        .unwrap_or_default()
        .iter()
        .filter(|item| {
            item.kind.as_deref() == Some("image")
                || item
                    .mime_type
                    .as_deref()
                    .is_some_and(|mime| mime.starts_with("image/"))
        })
        .collect();
    let has_annotations = images.iter().any(|item| !item.annotations.is_empty());

    if !has_annotations && !looks_like_ui_request(&normalized, !images.is_empty()) {
        return None;
    }

    let mode = infer_mode(&normalized);
    let attachment_intent =
        infer_attachment_intent(&normalized, has_annotations, !images.is_empty());
    let primary_id = images.iter().find_map(|item| item.attachment_id.clone());
    let mut task = UiDesignTaskInput {
        mode,
        attachment_intent,
        ..UiDesignTaskInput::default()
    };
    match attachment_intent {
        UiDesignAttachmentIntent::TargetDesign => task.target_design_attachment_id = primary_id,
        UiDesignAttachmentIntent::AnnotatedChangeRequest => {
            task.annotated_preview_attachment_id = primary_id
        }
        UiDesignAttachmentIntent::ReferenceStyle => {
            task.reference_attachment_ids = images
                .iter()
                .filter_map(|item| item.attachment_id.clone())
                .take(16)
                .collect()
        }
        UiDesignAttachmentIntent::CurrentScreenshot => task.original_attachment_id = primary_id,
        UiDesignAttachmentIntent::Auto => {}
    }
    Some(task)
}

fn looks_like_ui_request(text: &str, has_image: bool) -> bool {
    if contains_any(text, HIGH_CONFIDENCE_UI_MARKERS) {
        return true;
    }
    if has_image && contains_any(text, IMAGE_ACTION_MARKERS) {
        return true;
    }
    let has_property = contains_any(text, UI_PROPERTY_MARKERS);
    let has_action = contains_any(text, UI_ACTION_MARKERS);
    let has_subject = contains_any(text, UI_SUBJECT_MARKERS);
    (has_property && has_action)
        || (has_subject && contains_any(text, UI_SEMANTIC_MARKERS))
        || is_create_request(text)
        || contains_any(text, EXTEND_MARKERS)
        || contains_any(text, MODIFY_MARKERS)
}

fn infer_mode(text: &str) -> UiDesignTaskMode {
    if is_create_request(text) {
        UiDesignTaskMode::CreateNew
    } else if contains_any(text, EXTEND_MARKERS) {
        UiDesignTaskMode::ExtendExisting
    } else if contains_any(text, MODIFY_MARKERS)
        || (contains_any(text, UI_PROPERTY_MARKERS) && contains_any(text, UI_ACTION_MARKERS))
    {
        UiDesignTaskMode::ModifyExisting
    } else {
        UiDesignTaskMode::Auto
    }
}

fn infer_attachment_intent(
    text: &str,
    has_annotations: bool,
    has_image: bool,
) -> UiDesignAttachmentIntent {
    if !has_image {
        return UiDesignAttachmentIntent::Auto;
    }
    if has_annotations {
        return UiDesignAttachmentIntent::AnnotatedChangeRequest;
    }
    if contains_any(text, CURRENT_MARKERS) {
        UiDesignAttachmentIntent::CurrentScreenshot
    } else if contains_any(text, REFERENCE_MARKERS) {
        UiDesignAttachmentIntent::ReferenceStyle
    } else if contains_any(text, TARGET_MARKERS) || contains_any(text, IMAGE_ACTION_MARKERS) {
        UiDesignAttachmentIntent::TargetDesign
    } else {
        UiDesignAttachmentIntent::Auto
    }
}

fn is_create_request(text: &str) -> bool {
    contains_any(text, CREATE_MARKERS)
        || (contains_any(text, CREATE_ACTIONS) && contains_any(text, CREATE_SUBJECTS))
}

fn contains_any(text: &str, markers: &[&str]) -> bool {
    markers.iter().any(|marker| text.contains(marker))
}

const HIGH_CONFIDENCE_UI_MARKERS: &[&str] = &[
    "设计稿",
    "设计图",
    "草稿图",
    "页面样式",
    "组件样式",
    "像素级",
    "1:1",
    "ui拟合",
];
const UI_SUBJECT_MARKERS: &[&str] = &[
    "页面", "按钮", "卡片", "文本", "文字", "标题", "图标", "图片", "导航", "弹窗", "列表", "组件",
];
const UI_PROPERTY_MARKERS: &[&str] = &[
    "颜色",
    "圆角",
    "间距",
    "边距",
    "内边距",
    "外边距",
    "宽度",
    "高度",
    "宽高",
    "字号",
    "字体",
    "字重",
    "行高",
    "透明度",
    "对齐",
    "布局",
    "阴影",
    "边框",
    "背景",
    "padding",
    "margin",
    "radius",
    "width",
    "height",
    "font",
    "color",
    "opacity",
    "alignment",
    "spacing",
];
const UI_ACTION_MARKERS: &[&str] = &[
    "修改", "调整", "优化", "改成", "变成", "缩小", "放大", "增大", "减小", "加大", "减少", "增加",
    "去掉", "换成", "统一", "对齐", "还原", "匹配", "change", "update", "make", "resize", "align",
];
const UI_SEMANTIC_MARKERS: &[&str] = &[
    "更紧凑",
    "更突出",
    "更明显",
    "更好看",
    "更协调",
    "更圆",
    "更小",
    "更大",
    "太松",
    "太挤",
    "太宽",
    "太窄",
    "太高",
    "太矮",
    "样式",
    "视觉",
    "美化",
    "美观",
];
const CREATE_MARKERS: &[&str] = &[
    "全新页面",
    "新建页面",
    "从零开始",
    "还没有源码",
    "没有相关源码",
    "create new screen",
];
const CREATE_ACTIONS: &[&str] = &[
    "创建",
    "新建",
    "生成",
    "做一个",
    "实现一个",
    "build",
    "create",
];
const CREATE_SUBJECTS: &[&str] = &["页面", "界面", "屏幕", "screen", "page"];
const EXTEND_MARKERS: &[&str] = &[
    "扩展页面",
    "增加区域",
    "新增区域",
    "添加组件",
    "新增组件",
    "extend existing",
];
const MODIFY_MARKERS: &[&str] = &[
    "修改现有",
    "调整现有",
    "还原设计稿",
    "按图修改",
    "修改样式",
    "modify existing",
];
const TARGET_MARKERS: &[&str] = &["设计稿", "设计图", "目标图", "1:1", "像素级", "按图还原"];
const REFERENCE_MARKERS: &[&str] = &["风格参考", "参考风格", "参考这张", "灵感图"];
const CURRENT_MARKERS: &[&str] = &["当前截图", "现状截图", "真机截图", "现在的页面"];
const IMAGE_ACTION_MARKERS: &[&str] = &[
    "照着做",
    "按这个做",
    "按图做",
    "还原这张",
    "做成这样",
    "匹配这张",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_text_only_style_request() {
        let task = infer_ui_design_task("把支付按钮的圆角改小，间距更紧凑", None)
            .expect("UI request should be inferred");
        assert_eq!(task.mode, UiDesignTaskMode::ModifyExisting);
        assert_eq!(task.attachment_intent, UiDesignAttachmentIntent::Auto);
    }

    #[test]
    fn does_not_misroute_button_behavior_bug() {
        assert!(infer_ui_design_task("修复按钮点击后没有反应的问题", None).is_none());
    }

    #[test]
    fn recognizes_new_screen_without_existing_source() {
        let task = infer_ui_design_task("创建一个新的结算页面", None)
            .expect("new screen should be inferred");
        assert_eq!(task.mode, UiDesignTaskMode::CreateNew);
    }
}

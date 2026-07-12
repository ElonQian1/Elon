use super::{UiDesignAttachmentIntent, UiDesignTaskInput, UiDesignTaskMode};
use crate::project_ws_protocol::ProjectAttachmentRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UiRouteClass {
    ConfirmedUi,
    ConfirmedNonUi,
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct UiRouteDecision {
    pub(crate) class: UiRouteClass,
    pub(crate) score: f64,
    pub(crate) mode: UiDesignTaskMode,
    pub(crate) reasons: Vec<&'static str>,
}

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
    let decision = classify_normalized_ui_route(&normalized, !images.is_empty(), has_annotations);
    if decision.class != UiRouteClass::ConfirmedUi {
        return None;
    }

    Some(build_ui_design_task(
        &normalized,
        &images,
        has_annotations,
        decision.mode,
    ))
}

pub(crate) fn force_ui_design_task(
    message: &str,
    attachments: Option<&[ProjectAttachmentRef]>,
) -> UiDesignTaskInput {
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
    let mode = infer_mode(&normalized);
    build_ui_design_task(&normalized, &images, has_annotations, mode)
}

fn build_ui_design_task(
    normalized: &str,
    images: &[&ProjectAttachmentRef],
    has_annotations: bool,
    mode: UiDesignTaskMode,
) -> UiDesignTaskInput {
    let attachment_intent =
        infer_attachment_intent(normalized, has_annotations, !images.is_empty());
    let primary_id = images.iter().find_map(|item| item.attachment_id.clone());
    let mut task = UiDesignTaskInput {
        task_id: Some(format!("design_auto_{}", uuid::Uuid::new_v4().simple())),
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
    task
}

pub(crate) fn classify_ui_route(
    message: &str,
    attachments: Option<&[ProjectAttachmentRef]>,
) -> UiRouteDecision {
    let normalized = message.to_lowercase();
    let images = attachments.unwrap_or_default().iter().filter(|item| {
        item.kind.as_deref() == Some("image")
            || item
                .mime_type
                .as_deref()
                .is_some_and(|mime| mime.starts_with("image/"))
    });
    let mut has_image = false;
    let mut has_annotations = false;
    for image in images {
        has_image = true;
        has_annotations |= !image.annotations.is_empty();
    }
    classify_normalized_ui_route(&normalized, has_image, has_annotations)
}

fn classify_normalized_ui_route(
    text: &str,
    has_image: bool,
    has_annotations: bool,
) -> UiRouteDecision {
    let mut score = 0.0_f64;
    let mut reasons = Vec::new();
    let has_property = contains_any(text, UI_PROPERTY_MARKERS);
    let has_action = contains_any(text, UI_ACTION_MARKERS);
    let has_subject = contains_any(text, UI_SUBJECT_MARKERS);
    let has_semantic = contains_any(text, UI_SEMANTIC_MARKERS);
    let has_behavior = contains_any(text, BEHAVIOR_MARKERS);
    let mode = infer_mode(text);

    if has_annotations {
        score = score.max(0.98);
        reasons.push("图片包含结构化标注");
    }
    if is_create_request(text) {
        score = score.max(0.96);
        reasons.push("明确创建新页面");
    }
    if is_extend_request(text) {
        score = score.max(0.94);
        reasons.push("明确扩展视觉结构");
    }
    if contains_any(text, MODIFY_MARKERS) {
        score = score.max(0.92);
        reasons.push("明确修改现有样式");
    }
    if contains_any(text, HIGH_CONFIDENCE_UI_MARKERS) {
        score = score.max(0.9);
        reasons.push("命中高置信度 UI 表达");
    }
    if has_image && contains_any(text, IMAGE_ACTION_MARKERS) {
        score = score.max(0.88);
        reasons.push("图片与还原动作同时出现");
    }
    if has_property && has_action {
        score = score.max(0.86);
        reasons.push("样式属性与修改动作同时出现");
    }
    if has_subject && has_semantic {
        score = score.max(0.82);
        reasons.push("视觉对象与审美描述同时出现");
    }
    if contains_any(text, UI_SURFACE_MARKERS)
        && contains_any(text, UI_SURFACE_ACTION_MARKERS)
    {
        score = score.max(0.84);
        reasons.push("页面对象与美化动作同时出现");
    }
    if contains_any(text, AMBIGUOUS_VISUAL_MARKERS) {
        score = score.max(if has_subject || contains_any(text, AMBIGUOUS_REGION_MARKERS) {
            0.62
        } else {
            0.45
        });
        reasons.push("出现模糊视觉描述");
    }
    if has_subject && has_action {
        score = score.max(0.48);
        reasons.push("视觉对象与通用调整动作同时出现");
    } else if has_subject || has_property || has_semantic {
        score = score.max(0.32);
        reasons.push("仅出现部分 UI 线索");
    }
    if has_behavior && !has_property && !has_semantic && !has_annotations {
        score = score.min(0.08);
        reasons.push("功能或交互逻辑证据占主导");
    }

    let class = if score >= 0.75 {
        UiRouteClass::ConfirmedUi
    } else if score <= 0.2 {
        UiRouteClass::ConfirmedNonUi
    } else {
        UiRouteClass::Ambiguous
    };
    UiRouteDecision {
        class,
        score,
        mode,
        reasons,
    }
}

fn infer_mode(text: &str) -> UiDesignTaskMode {
    if is_create_request(text) {
        UiDesignTaskMode::CreateNew
    } else if is_extend_request(text) {
        UiDesignTaskMode::ExtendExisting
    } else if contains_any(text, MODIFY_MARKERS)
        || (contains_any(text, UI_PROPERTY_MARKERS)
            && (contains_any(text, UI_ACTION_MARKERS) || contains_any(text, UI_SEMANTIC_MARKERS)))
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

fn is_extend_request(text: &str) -> bool {
    contains_any(text, EXTEND_MARKERS)
        || (contains_any(text, EXTEND_ACTION_MARKERS)
            && contains_any(text, EXTEND_SUBJECT_MARKERS)
            && !contains_any(text, BEHAVIOR_MARKERS))
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
    "优化界面",
    "美化页面",
    "ui样式",
    "ui设计",
];
const UI_SUBJECT_MARKERS: &[&str] = &[
    "页面", "按钮", "卡片", "文本", "文字", "标题", "图标", "图片", "导航", "弹窗", "列表", "组件",
];
const UI_SURFACE_MARKERS: &[&str] = &["页面", "界面", "screen", "page"];
const UI_SURFACE_ACTION_MARKERS: &[&str] = &["优化", "美化", "调整样式", "polish", "restyle"];
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
    "去掉", "换成", "统一", "对齐", "还原", "匹配", "改小", "改大", "change", "update", "make",
    "resize", "align",
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
    "太大",
    "太小",
    "改大",
    "改小",
    "样式",
    "视觉",
    "美化",
    "美观",
];
const AMBIGUOUS_VISUAL_MARKERS: &[&str] = &[
    "轻一点",
    "重一点",
    "高级一点",
    "更高级",
    "更舒服",
    "有呼吸感",
    "不够高级",
    "不够舒服",
    "抢眼一点",
    "更抢眼",
    "克制一点",
    "更克制",
    "更精致",
    "不够精致",
];
const AMBIGUOUS_REGION_MARKERS: &[&str] = &[
    "底部", "顶部", "主操作", "次操作", "这个区域", "这块", "这里",
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
    "增加按钮",
    "添加按钮",
    "新增按钮",
    "增加卡片",
    "添加卡片",
    "新增卡片",
    "extend existing",
];
const EXTEND_ACTION_MARKERS: &[&str] = &["增加", "添加", "新增", "插入", "add"];
const EXTEND_SUBJECT_MARKERS: &[&str] = &[
    "区域", "组件", "按钮", "卡片", "图标", "图片", "标题", "文本", "列表",
];
const BEHAVIOR_MARKERS: &[&str] = &[
    "点击",
    "逻辑",
    "事件",
    "接口",
    "网络",
    "功能",
    "崩溃",
    "无响应",
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
        assert!(task
            .task_id
            .as_deref()
            .is_some_and(|id| id.starts_with("design_auto_")));
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

    #[test]
    fn recognizes_natural_visual_phrases_and_structure_additions() {
        assert!(infer_ui_design_task("这个按钮太大了，改小一点", None).is_some());
        assert!(infer_ui_design_task("优化一下这个界面", None).is_some());
        assert_eq!(
            infer_ui_design_task("在列表里新增一张卡片", None)
                .expect("extension should route")
                .mode,
            UiDesignTaskMode::ExtendExisting
        );
        assert!(infer_ui_design_task("调整按钮点击逻辑", None).is_none());
    }

    #[test]
    fn exposes_ambiguous_visual_requests_for_second_stage_routing() {
        let decision = classify_ui_route("让底部轻一点，看起来更克制", None);
        assert_eq!(decision.class, UiRouteClass::Ambiguous);
        assert!(decision.score > 0.2 && decision.score < 0.75);
        assert!(!decision.reasons.is_empty());
    }

    #[test]
    fn exposes_confident_non_ui_requests_without_model_work() {
        let decision = classify_ui_route("调整按钮点击逻辑，修复接口无响应", None);
        assert_eq!(decision.class, UiRouteClass::ConfirmedNonUi);
        assert!(decision.score <= 0.2);
    }
}

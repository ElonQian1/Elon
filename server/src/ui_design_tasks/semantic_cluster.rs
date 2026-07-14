/// 受控 UI 近义概念版本。规则变化时必须升级，避免旧经验被静默改义。
pub(crate) const CONTROLLED_UI_CONCEPT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ControlledUiConcept {
    pub(crate) key: &'static str,
    pub(crate) label: &'static str,
    pub(crate) version: u32,
}

/// 将自然语言映射到封闭、可审计的 UI 概念。
///
/// 这里不调用模型，也不接受模型生成别名。只有代码中明确登记的主体、问题和
/// 排除词组合才能产出稳定 key；不确定的表达返回 None，继续走现有二次判断。
pub(crate) fn controlled_ui_route_concept(message: &str) -> Option<ControlledUiConcept> {
    let text = message.trim().to_lowercase();
    if text.is_empty()
        || contains_any(&text, BEHAVIOR_BLOCKERS)
        || contains_any(&text, TYPOGRAPHY_SCOPES)
        || contains_any(&text, BORDER_SCOPES)
        || !contains_any(&text, ACTION_CONTROL_SUBJECTS)
    {
        return None;
    }

    if contains_any(&text, VISUAL_HEAVINESS_REDUCE_MARKERS) {
        return Some(ControlledUiConcept {
            key: "ui.action_control.visual_weight.reduce",
            label: "操作控件显得厚重，需要轻量化",
            version: CONTROLLED_UI_CONCEPT_VERSION,
        });
    }

    None
}

fn contains_any(text: &str, markers: &[&str]) -> bool {
    markers.iter().any(|marker| text.contains(marker))
}

const ACTION_CONTROL_SUBJECTS: &[&str] = &[
    "主操作",
    "主要操作",
    "主按钮",
    "操作按钮",
    "确认按钮",
    "提交按钮",
    "按钮",
    "button",
    "cta",
];

const VISUAL_HEAVINESS_REDUCE_MARKERS: &[&str] = &[
    "太胖",
    "显得胖",
    "有点胖",
    "肥大",
    "臃肿",
    "笨重",
    "厚重",
    "太厚",
    "过厚",
    "太重",
    "不够轻盈",
    "不够轻巧",
];

/// 行为、业务或性能语义优先阻断，防止“按钮响应太重”误复用样式经验。
const BEHAVIOR_BLOCKERS: &[&str] = &[
    "点击逻辑",
    "点击事件",
    "点击无效",
    "点击失败",
    "响应速度",
    "响应时间",
    "回调",
    "onclick",
    "接口",
    "请求",
    "跳转",
    "导航",
    "埋点",
    "业务逻辑",
    "功能逻辑",
];

/// 字体粗细和组件体量是两类概念，不允许共享经验。
const TYPOGRAPHY_SCOPES: &[&str] = &["按钮字", "按钮文字", "文案", "字体", "字重", "字号"];

/// 边框厚度也不能被解释为整个按钮厚重。
const BORDER_SCOPES: &[&str] = &["边框", "描边", "线条"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controlled_synonyms_share_one_visual_weight_concept() {
        let messages = ["按钮太胖", "按钮显得笨重", "主操作太厚重"];
        let keys = messages
            .iter()
            .map(|message| controlled_ui_route_concept(message).unwrap().key)
            .collect::<Vec<_>>();
        assert!(keys.iter().all(|key| *key == keys[0]));
        assert_eq!(keys[0], "ui.action_control.visual_weight.reduce");
    }

    #[test]
    fn behavior_typography_and_border_requests_are_not_clustered() {
        for message in [
            "调整按钮点击逻辑，它现在太笨重",
            "按钮文字太厚重",
            "按钮字体太粗",
            "按钮边框太厚",
            "服务器负担太重",
        ] {
            assert_eq!(controlled_ui_route_concept(message), None, "{message}");
        }
    }

    #[test]
    fn unknown_adjectives_do_not_expand_the_closed_vocabulary() {
        assert_eq!(controlled_ui_route_concept("按钮看起来不够高级"), None);
        assert_eq!(controlled_ui_route_concept("按钮像一颗石头"), None);
    }
}

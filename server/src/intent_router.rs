#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserIntent {
    NormalChat,
    AppDevelopment,
    TextToImage,
    ImageAssetForApp,
    ModelConfig,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityRoute {
    ChatAgent,
    CodeAgent,
    #[allow(dead_code)]
    TextToImage,
    #[allow(dead_code)]
    ImageThenCode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingDecision {
    pub intent: UserIntent,
    pub route: CapabilityRoute,
    pub confidence: u8,
    pub needs_image_generation: bool,
    pub needs_code_change: bool,
    pub allow_user_agent_preference: bool,
    pub reason: &'static str,
}

impl RoutingDecision {
    fn new(
        intent: UserIntent,
        route: CapabilityRoute,
        confidence: u8,
        reason: &'static str,
    ) -> Self {
        Self {
            intent,
            route,
            confidence,
            needs_image_generation: matches!(
                route,
                CapabilityRoute::TextToImage | CapabilityRoute::ImageThenCode
            ),
            needs_code_change: matches!(
                route,
                CapabilityRoute::CodeAgent | CapabilityRoute::ImageThenCode
            ),
            allow_user_agent_preference: matches!(
                route,
                CapabilityRoute::ChatAgent | CapabilityRoute::CodeAgent
            ),
            reason,
        }
    }
}

const IMAGE_OBJECT_TERMS: &[&str] = &[
    "文生图",
    "生图",
    "生成图",
    "图像",
    "图片",
    "照片",
    "头像",
    "壁纸",
    "插画",
    "海报",
    "卡通",
    "山水画",
    "图标",
    "启动图",
    "背景图",
    "素材",
    "封面",
    "配图",
    "logo",
    "icon",
    "image",
    "picture",
    "avatar",
    "poster",
    "wallpaper",
    "photo",
];

const IMAGE_ACTION_TERMS: &[&str] = &[
    "文生图",
    "生图",
    "生成",
    "画",
    "绘制",
    "做一张",
    "来一张",
    "出一张",
    "创作",
    "设计",
    "generate",
    "draw",
    "create",
    "make",
];

const APP_TERMS: &[&str] = &[
    "app",
    "apk",
    "android",
    "应用",
    "功能",
    "页面",
    "界面",
    "按钮",
    "代码",
    "开发",
    "修改",
    "添加",
    "新增",
    "编译",
    "打包",
    "安装",
    "发布",
    "登录",
    "注册",
    "首页",
    "设置",
    "接口",
    "后端",
    "服务端",
    "服务器",
    "数据库",
    "项目",
    "网页",
    "网站",
    "web",
    "前端",
    "win端",
    "windows",
];

const APP_ACTION_TERMS: &[&str] = &[
    "开发",
    "修改",
    "添加",
    "新增",
    "生成",
    "做一个",
    "做个",
    "实现",
    "编译",
    "打包",
    "部署",
    "修复",
    "接入",
    "重构",
    "完善",
    "build",
    "fix",
    "deploy",
    "implement",
];

const ASSET_INTEGRATION_TERMS: &[&str] = &[
    "放进",
    "加入",
    "替换",
    "用于",
    "作为",
    "集成",
    "导入",
    "接入",
    "应用图标",
    "启动图标",
    "资源",
    "res",
    "drawable",
    "mipmap",
];

const MODEL_CONFIG_TERMS: &[&str] = &[
    "切换模型",
    "换模型",
    "选择模型",
    "设置模型",
    "默认模型",
    "use_agent",
    "api key",
    "apikey",
    "代理配置",
    "codex",
    "codex cli",
    "hunyuan",
    "混元",
    "tokenhub",
    "deepseek",
];

const MODEL_CONFIG_ACTION_TERMS: &[&str] = &[
    "切换",
    "换成",
    "改成",
    "默认",
    "使用",
    "设置",
    "配置",
    "选择",
    "为什么不是",
];

const EXPLANATION_QUERY_TERMS: &[&str] = &[
    "什么意思",
    "是什么",
    "是什么原因",
    "为什么",
    "怎么理解",
    "解释一下",
    "解释下",
    "含义",
    "啥意思",
    "what does it mean",
    "what is",
    "why",
];

pub fn classify(message: &str) -> RoutingDecision {
    let normalized = normalize(message);
    if normalized.is_empty() {
        return RoutingDecision::new(
            UserIntent::Unknown,
            CapabilityRoute::ChatAgent,
            50,
            "empty_message",
        );
    }

    let has_image_object = contains_any(&normalized, IMAGE_OBJECT_TERMS);
    let has_image_action = contains_any(&normalized, IMAGE_ACTION_TERMS);
    let has_app_context = contains_any(&normalized, APP_TERMS);
    let has_app_action = contains_any(&normalized, APP_ACTION_TERMS);
    let has_asset_integration = contains_any(&normalized, ASSET_INTEGRATION_TERMS);
    let has_model_config = contains_any(&normalized, MODEL_CONFIG_TERMS)
        && contains_any(&normalized, MODEL_CONFIG_ACTION_TERMS);
    let asks_for_explanation = contains_any(&normalized, EXPLANATION_QUERY_TERMS);

    // 明确要生成新图片并集成到应用（两步：文生图 + 代码集成）
    if has_image_object && has_image_action && has_app_context && has_asset_integration {
        return RoutingDecision::new(
            UserIntent::ImageAssetForApp,
            CapabilityRoute::ImageThenCode,
            88,
            "image_generate_then_integrate_into_app",
        );
    }

    if has_image_object
        && has_app_context
        && (has_image_action || has_asset_integration || is_strong_app_asset(&normalized))
    {
        return RoutingDecision::new(
            UserIntent::ImageAssetForApp,
            CapabilityRoute::CodeAgent,
            86,
            "image_asset_for_app_cli_testing",
        );
    }

    if has_image_object && has_image_action && !has_app_context {
        return RoutingDecision::new(
            UserIntent::TextToImage,
            CapabilityRoute::CodeAgent,
            90,
            "standalone_image_cli_testing",
        );
    }

    if has_model_config {
        return RoutingDecision::new(
            UserIntent::ModelConfig,
            CapabilityRoute::ChatAgent,
            82,
            "model_config_request",
        );
    }

    // 解释/澄清类问题应优先走聊天路径，避免误触发代码工作流。
    if asks_for_explanation && !has_app_action {
        return RoutingDecision::new(
            UserIntent::NormalChat,
            CapabilityRoute::ChatAgent,
            88,
            "explanation_query",
        );
    }

    if has_app_context && (has_app_action || !has_image_object) {
        return RoutingDecision::new(
            UserIntent::AppDevelopment,
            CapabilityRoute::CodeAgent,
            84,
            "app_or_web_development",
        );
    }

    if has_image_object {
        return RoutingDecision::new(
            UserIntent::TextToImage,
            CapabilityRoute::CodeAgent,
            68,
            "weak_image_cli_testing",
        );
    }

    RoutingDecision::new(
        UserIntent::NormalChat,
        CapabilityRoute::ChatAgent,
        60,
        "normal_chat",
    )
}

#[allow(dead_code)]
pub fn image_prompt_from_message(message: &str) -> String {
    let trimmed = message.trim();
    let prefixes = [
        "请帮我生成一张",
        "帮我生成一张",
        "帮我生成一个",
        "帮我生成",
        "请生成一张",
        "请生成一个",
        "请生成",
        "给 App 生成一个",
        "给App生成一个",
        "给应用生成一个",
        "为 App 生成一个",
        "为App生成一个",
        "为应用生成一个",
        "生成一张",
        "生成一个",
        "生成",
        "画一张",
        "画一个",
        "画",
        "绘制一张",
        "绘制一个",
        "绘制",
        "来一张",
        "做一张",
        "做一个",
    ];

    let mut prompt = trimmed;
    for prefix in prefixes {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            prompt = rest.trim_start_matches(['：', ':', '，', ',', ' ']).trim();
            break;
        }
    }

    for marker in [
        "并放进",
        "并加入",
        "并替换",
        "然后放进",
        "再放进",
        "用于",
        "作为",
    ] {
        if let Some((left, _)) = prompt.split_once(marker) {
            let left = left.trim();
            if !left.is_empty() {
                prompt = left;
                break;
            }
        }
    }

    if prompt.is_empty() {
        trimmed.to_string()
    } else {
        prompt.to_string()
    }
}

pub fn looks_like_development_request(message: &str) -> bool {
    classify(message).needs_code_change
}

fn normalize(message: &str) -> String {
    message.trim().to_lowercase()
}

fn contains_any(message: &str, terms: &[&str]) -> bool {
    terms.iter().any(|term| message.contains(term))
}

fn is_strong_app_asset(message: &str) -> bool {
    contains_any(
        message,
        &["图标", "启动图", "背景图", "logo", "icon", "素材"],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_standalone_image_requests_to_codex_cli() {
        let decision = classify("帮我生成一张山水画");
        assert_eq!(decision.intent, UserIntent::TextToImage);
        assert_eq!(decision.route, CapabilityRoute::CodeAgent);
        assert!(!decision.needs_image_generation);
        assert!(decision.needs_code_change);
    }

    #[test]
    fn routes_short_image_prompt() {
        let decision = classify("画一个头像");
        assert_eq!(decision.route, CapabilityRoute::CodeAgent);
    }

    #[test]
    fn routes_app_development() {
        let decision = classify("帮我做一个租房管理 App");
        assert_eq!(decision.intent, UserIntent::AppDevelopment);
        assert_eq!(decision.route, CapabilityRoute::CodeAgent);
    }

    #[test]
    fn routes_web_development() {
        let decision = classify("生成一个登录网页");
        assert_eq!(decision.route, CapabilityRoute::CodeAgent);
    }

    #[test]
    fn routes_image_asset_for_app() {
        let decision = classify("给 App 生成一个猫咪图标并替换启动图标");
        assert_eq!(decision.intent, UserIntent::ImageAssetForApp);
        assert_eq!(decision.route, CapabilityRoute::ImageThenCode);
        assert!(decision.needs_image_generation);
        assert!(decision.needs_code_change);
    }

    #[test]
    fn routes_model_config_to_chat_agent() {
        let decision = classify("为什么默认不是 Codex CLI，还用了混元");
        assert_eq!(decision.intent, UserIntent::ModelConfig);
        assert_eq!(decision.route, CapabilityRoute::ChatAgent);
    }

    #[test]
    fn routes_normal_chat() {
        let decision = classify("今天我们先聊一下产品方向");
        assert_eq!(decision.intent, UserIntent::NormalChat);
        assert_eq!(decision.route, CapabilityRoute::ChatAgent);
    }

    #[test]
    fn routes_explanation_question_to_chat_agent() {
        let decision = classify(
            "这个项目，用户说选了4场比赛，同时选择2串1，3串1，4串1，后台只显示4串1，这是什么意思？",
        );
        assert_eq!(decision.intent, UserIntent::NormalChat);
        assert_eq!(decision.route, CapabilityRoute::ChatAgent);
    }

    #[test]
    fn extracts_image_prompt_from_hybrid_request() {
        let prompt = image_prompt_from_message("给 App 生成一个猫咪图标并替换启动图标");
        assert_eq!(prompt, "猫咪图标");
    }
}

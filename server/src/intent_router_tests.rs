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

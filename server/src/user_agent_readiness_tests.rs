    use super::*;

    #[test]
    fn ready_when_custom_model_is_complete_and_byok_allowed() {
        let cfg = UserAgentConfig {
            api_base: Some("https://api.example.com/v1".into()),
            api_key: Some("sk-test".into()),
            model: Some("tool-model".into()),
            tool_call_ok: Some(true),
            capability: Some("tools_ok".into()),
            capability_checked_at: Some("2026-06-16 00:00:00 UTC".into()),
            ..Default::default()
        };

        let readiness = build_user_agent_rag_readiness(&cfg, true, true);

        assert_eq!(readiness.status, "ready_without_embedding_model");
        assert!(readiness.development_ready);
        assert!(!readiness.semantic_embedding_ready);
        assert!(readiness.tool_call_verified);
        assert_eq!(
            readiness.required_capability,
            "OpenAI tools/function calling"
        );
    }

    #[test]
    fn semantic_embedding_ready_when_embedding_model_is_configured() {
        let cfg = UserAgentConfig {
            api_base: Some("https://api.example.com/v1".into()),
            api_key: Some("sk-test".into()),
            model: Some("tool-model".into()),
            embedding_model: Some("openai:text-embedding-3-small".into()),
            tool_call_ok: Some(true),
            capability: Some("tools_ok".into()),
            capability_checked_at: Some("2026-06-16 00:00:00 UTC".into()),
            ..Default::default()
        };

        let readiness = build_user_agent_rag_readiness(&cfg, true, true);

        assert_eq!(readiness.status, "ready");
        assert!(readiness.development_ready);
        assert!(readiness.semantic_embedding_ready);
        assert_eq!(
            readiness.embedding_model.as_deref(),
            Some("openai:text-embedding-3-small")
        );
    }

    #[test]
    fn blocked_when_codex_only_disallows_byok() {
        let cfg = UserAgentConfig {
            api_base: Some("https://api.example.com/v1".into()),
            api_key: Some("sk-test".into()),
            model: Some("tool-model".into()),
            tool_call_ok: Some(true),
            ..Default::default()
        };

        let readiness = build_user_agent_rag_readiness(&cfg, true, false);

        assert_eq!(readiness.status, "blocked_by_policy");
        assert!(!readiness.development_ready);
    }

    #[test]
    fn complete_config_without_probe_needs_capability_check() {
        let cfg = UserAgentConfig {
            api_base: Some("https://api.example.com/v1".into()),
            api_key: Some("sk-test".into()),
            model: Some("tool-model".into()),
            ..Default::default()
        };

        let readiness = build_user_agent_rag_readiness(&cfg, false, true);

        assert_eq!(readiness.status, "needs_capability_check");
        assert!(!readiness.development_ready);
        assert!(!readiness.tool_call_verified);
    }

    #[test]
    fn failed_probe_is_not_ready() {
        let cfg = UserAgentConfig {
            api_base: Some("https://api.example.com/v1".into()),
            api_key: Some("sk-test".into()),
            model: Some("chat-only".into()),
            tool_call_ok: Some(false),
            capability: Some("chat_only".into()),
            capability_warning: Some("no tool calls".into()),
            ..Default::default()
        };

        let readiness = build_user_agent_rag_readiness(&cfg, false, true);

        assert_eq!(readiness.status, "tool_call_failed");
        assert!(!readiness.development_ready);
        assert_eq!(
            readiness.capability_warning.as_deref(),
            Some("no tool calls")
        );
    }

    #[test]
    fn explains_missing_api_key() {
        let cfg = UserAgentConfig {
            api_base: Some("https://api.example.com/v1".into()),
            model: Some("tool-model".into()),
            ..Default::default()
        };

        let readiness = build_user_agent_rag_readiness(&cfg, false, true);

        assert_eq!(readiness.status, "missing_api_key");
        assert!(!readiness.development_ready);
    }

    #[test]
    fn allows_verified_custom_api_for_development() {
        let cfg = UserAgentConfig {
            api_base: Some("https://api.example.com/v1".into()),
            api_key: Some("sk-test".into()),
            model: Some("tool-model".into()),
            tool_call_ok: Some(true),
            ..Default::default()
        };

        let block = custom_api_development_block_message(&cfg, true, true);

        assert!(block.is_none());
    }

    #[test]
    fn blocks_unverified_custom_api_for_development() {
        let cfg = UserAgentConfig {
            api_base: Some("https://api.example.com/v1".into()),
            api_key: Some("sk-test".into()),
            model: Some("tool-model".into()),
            ..Default::default()
        };

        let block = custom_api_development_block_message(&cfg, false, true)
            .expect("unverified custom API should be blocked");

        assert!(block.contains("需要重新验证模型能力"));
        assert!(block.contains("OpenAI tools/function calling"));
    }

    #[test]
    fn does_not_block_cli_or_global_agent_selection() {
        let cfg = UserAgentConfig {
            use_agent: Some("codex_cli".into()),
            ..Default::default()
        };

        let block = custom_api_development_block_message(&cfg, true, false);

        assert!(block.is_none());
    }

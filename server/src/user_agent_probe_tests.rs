    use super::*;

    #[test]
    fn normalize_api_base_trims_trailing_slashes() {
        assert_eq!(
            normalize_api_base(" https://api.example.com/v1/// ").as_deref(),
            Some("https://api.example.com/v1")
        );
        assert!(normalize_api_base("api.example.com/v1").is_none());
    }

    #[test]
    fn resolve_probe_reuses_saved_key_when_request_key_is_empty() {
        let existing = UserAgentConfig {
            api_base: Some("https://api.example.com/v1".into()),
            api_key: Some("sk-saved".into()),
            model: Some("saved-model".into()),
            ..Default::default()
        };
        let cfg = resolve_probe_config(
            UserAgentProbeRequest {
                api_base: Some("https://api.other.com/v1/".into()),
                api_key: Some(" ".into()),
                model: Some("new-model".into()),
            },
            &existing,
        )
        .expect("probe config should resolve");

        assert_eq!(cfg.api_base, "https://api.other.com/v1");
        assert_eq!(cfg.api_key, "sk-saved");
        assert_eq!(cfg.model, "new-model");
    }

    #[test]
    fn tool_probe_detects_openai_tool_calls() {
        let response = json!({
            "choices": [
                {
                    "message": {
                        "tool_calls": [
                            {
                                "type": "function",
                                "function": {
                                    "name": "elon_probe",
                                    "arguments": "{\"ok\":true}"
                                }
                            }
                        ]
                    }
                }
            ]
        });

        let outcome = tool_probe_outcome_from_response(&response);
        assert!(outcome.ok);
        assert_eq!(outcome.tool_call_name.as_deref(), Some("elon_probe"));
        assert!(outcome.warning.is_none());
    }

    #[test]
    fn tool_probe_warns_when_chat_response_has_no_tool_call() {
        let response = json!({
            "choices": [
                {
                    "message": {
                        "content": "OK"
                    }
                }
            ]
        });

        let outcome = tool_probe_outcome_from_response(&response);
        assert!(!outcome.ok);
        assert_eq!(outcome.tool_call_name, None);
        assert!(outcome
            .warning
            .as_deref()
            .unwrap_or_default()
            .contains("没有返回工具调用"));
    }

    #[test]
    fn development_tool_call_error_explains_save_block() {
        let result = UserAgentProbeResult {
            api_base: "https://api.example.com/v1".into(),
            model: "chat-only".into(),
            latency_ms: 12,
            sample: "OK".into(),
            tool_call_ok: false,
            tool_call_name: None,
            capability: "chat_only".into(),
            warning: Some("模型没有返回工具调用".into()),
        };

        let message = development_tool_call_error(&result);
        assert!(message.contains("不能作为项目开发代理保存"));
        assert!(message.contains("OpenAI tools/function calling"));
    }

    use super::*;

    #[test]
    fn direct_custom_api_resolves_without_global_agent() {
        let cfg = UserAgentConfig {
            api_base: Some("https://api.example.com/v1".into()),
            api_key: Some("sk-user".into()),
            model: Some("example-model".into()),
            ..Default::default()
        };
        let global = AgentsConfig {
            agents: HashMap::new(),
            default_agent: String::new(),
        };

        let resolved = cfg.resolve(&global).expect("custom API should resolve");

        assert_eq!(resolved.name, "user-custom-api");
        assert_eq!(resolved.api_key, "sk-user");
        assert_eq!(resolved.embedding_model, None);
        assert_eq!(resolved.usage_mode(), "user_api_key_proxy");
    }

    #[test]
    fn partial_custom_config_still_overrides_default_agent() {
        let cfg = UserAgentConfig {
            model: Some("user-model".into()),
            ..Default::default()
        };
        let global = AgentsConfig {
            agents: HashMap::from([(
                "default".into(),
                AgentConfig {
                    name: "default".into(),
                    api_base: "https://api.example.com/v1".into(),
                    api_key: "server-key".into(),
                    model: "server-model".into(),
                    embedding_model: Some("openai:text-embedding-3-small".into()),
                    usage_mode: None,
                },
            )]),
            default_agent: "default".into(),
        };

        let resolved = cfg.resolve(&global).expect("default agent should resolve");

        assert_eq!(resolved.api_base, "https://api.example.com/v1");
        assert_eq!(resolved.api_key, "server-key");
        assert_eq!(resolved.model, "user-model");
        assert_eq!(
            resolved.embedding_model.as_deref(),
            Some("openai:text-embedding-3-small")
        );
        assert_eq!(resolved.usage_mode(), "server_api_key");
    }

    #[test]
    fn user_embedding_model_overrides_global_agent() {
        let cfg = UserAgentConfig {
            embedding_model: Some("remote:bge-m3".into()),
            ..Default::default()
        };
        let global = AgentsConfig {
            agents: HashMap::from([(
                "default".into(),
                AgentConfig {
                    name: "default".into(),
                    api_base: "https://api.example.com/v1".into(),
                    api_key: "server-key".into(),
                    model: "server-model".into(),
                    embedding_model: Some("openai:text-embedding-3-small".into()),
                    usage_mode: None,
                },
            )]),
            default_agent: "default".into(),
        };

        let resolved = cfg.resolve(&global).expect("default agent should resolve");

        assert_eq!(resolved.embedding_model.as_deref(), Some("remote:bge-m3"));
    }

    #[test]
    fn saving_user_api_key_writes_encrypted_reference_only() {
        std::env::set_var("USER_API_KEY_SECRET", "test-secret-for-user-api-key");
        let workspace = std::env::temp_dir().join(format!(
            "elon-user-agent-config-test-{}",
            uuid::Uuid::new_v4()
        ));
        let cfg = UserAgentConfig {
            api_base: Some("https://api.example.com/v1".into()),
            api_key: Some("sk-sensitive".into()),
            model: Some("example-model".into()),
            embedding_model: Some("openai:text-embedding-3-small".into()),
            ..Default::default()
        };

        cfg.save(&workspace).expect("save config");
        let raw =
            std::fs::read_to_string(workspace.join("agent_config.json")).expect("read config");
        assert!(!raw.contains("sk-sensitive"));
        assert!(raw.contains("api_key_encrypted"));
        assert!(raw.contains("openai:text-embedding-3-small"));

        let loaded = UserAgentConfig::load(&workspace).expect("load config");
        assert_eq!(loaded.api_key.as_deref(), Some("sk-sensitive"));
        assert_eq!(
            loaded.embedding_model.as_deref(),
            Some("openai:text-embedding-3-small")
        );

        let _ = std::fs::remove_dir_all(workspace);
        std::env::remove_var("USER_API_KEY_SECRET");
    }

    #[test]
    fn capability_probe_metadata_is_persisted_without_plain_api_key() {
        std::env::set_var("USER_API_KEY_SECRET", "test-secret-for-user-api-key");
        let workspace = std::env::temp_dir().join(format!(
            "elon-user-agent-capability-test-{}",
            uuid::Uuid::new_v4()
        ));
        let mut cfg = UserAgentConfig {
            api_base: Some("https://api.example.com/v1".into()),
            api_key: Some("sk-sensitive".into()),
            model: Some("example-model".into()),
            ..Default::default()
        };
        let probe = crate::user_agent_probe::UserAgentProbeResult {
            api_base: "https://api.example.com/v1".into(),
            model: "example-model".into(),
            latency_ms: 12,
            sample: "OK".into(),
            tool_call_ok: true,
            tool_call_name: Some("elon_probe".into()),
            capability: "tools_ok".into(),
            warning: None,
        };

        cfg.remember_capability_probe(&probe, "2026-06-16 00:00:00 UTC".into());
        cfg.save(&workspace).expect("save config");
        let raw =
            std::fs::read_to_string(workspace.join("agent_config.json")).expect("read config");

        assert!(!raw.contains("sk-sensitive"));
        assert!(raw.contains("\"tool_call_ok\": true"));
        assert!(raw.contains("\"capability_checked_at\""));

        let loaded = UserAgentConfig::load(&workspace).expect("load config");
        assert_eq!(loaded.tool_call_ok, Some(true));
        assert_eq!(loaded.capability.as_deref(), Some("tools_ok"));

        let _ = std::fs::remove_dir_all(workspace);
        std::env::remove_var("USER_API_KEY_SECRET");
    }

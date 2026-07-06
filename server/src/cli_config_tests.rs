    use super::*;

    fn cli_option(id: &str, provider: &str, bin: &str) -> AiCliOption {
        AiCliOption {
            id: id.to_string(),
            label: id.to_string(),
            provider: provider.to_string(),
            model: None,
            reasoning_effort: None,
            reasoning_summary: None,
            verbosity: None,
            bin: bin.to_string(),
            args: Vec::new(),
            prompt_mode: CliPromptMode::Arg,
            timeout_secs: 60,
        }
    }

    fn cli_config(options: Vec<AiCliOption>, default_option: Option<&str>) -> AiCliConfig {
        AiCliConfig {
            enabled: true,
            options,
            default_option: default_option.map(ToString::to_string),
            fallback_to_api: false,
            codex_cli_only: false,
            fallback_cli_option: None,
        }
    }

    #[test]
    fn find_codex_option_keeps_explicit_codex_choice() {
        let config = cli_config(
            vec![
                cli_option("copilot:gpt-4o", "copilot", "copilot"),
                cli_option("codex:gpt-5.4:high", "codex", "codex"),
                cli_option("codex:gpt-5.5:xhigh", "codex", "codex"),
            ],
            Some("copilot:gpt-4o"),
        );

        let option = config
            .find_codex_option(Some("codex:gpt-5.5:xhigh"))
            .unwrap();

        assert_eq!(option.id, "codex:gpt-5.5:xhigh");
    }

    #[test]
    fn find_codex_option_falls_back_from_non_codex_choice() {
        let config = cli_config(
            vec![
                cli_option("copilot:gpt-4o", "copilot", "copilot"),
                cli_option("codex:gpt-5.4:high", "codex", "codex"),
            ],
            Some("copilot:gpt-4o"),
        );

        let option = config.find_codex_option(Some("copilot:gpt-4o")).unwrap();

        assert_eq!(option.id, "codex:gpt-5.4:high");
    }

    #[test]
    fn find_codex_option_prefers_default_codex_when_no_explicit_choice() {
        let config = cli_config(
            vec![
                cli_option("codex:gpt-5.4:medium", "codex", "codex"),
                cli_option("codex:gpt-5.5:xhigh", "codex", "codex"),
            ],
            Some("codex:gpt-5.5:xhigh"),
        );

        let option = config.find_codex_option(None).unwrap();

        assert_eq!(option.id, "codex:gpt-5.5:xhigh");
    }

    #[test]
    fn find_codex_option_returns_none_without_codex() {
        let config = cli_config(
            vec![cli_option("copilot:gpt-4o", "copilot", "copilot")],
            Some("copilot:gpt-4o"),
        );

        assert!(config.find_codex_option(Some("copilot:gpt-4o")).is_none());
    }

    #[test]
    fn parse_codex_catalog_keeps_all_supported_reasoning_levels() {
        let catalog = r#"{
            "models": [
                {
                    "slug": "gpt-5.4",
                    "supported_reasoning_levels": [
                        { "effort": "low" },
                        { "effort": "medium" },
                        { "effort": "high" },
                        { "effort": "xhigh" }
                    ]
                },
                {
                    "slug": "gpt-5.4-mini",
                    "supported_reasoning_levels": [
                        { "effort": "low" },
                        { "effort": "medium" },
                        { "effort": "medium" }
                    ]
                }
            ]
        }"#;

        let efforts = parse_codex_reasoning_effort_catalog(catalog).unwrap();

        assert_eq!(
            efforts.get("gpt-5.4").unwrap(),
            &vec![
                "low".to_string(),
                "medium".to_string(),
                "high".to_string(),
                "xhigh".to_string()
            ]
        );
        assert_eq!(
            efforts.get("gpt-5.4-mini").unwrap(),
            &vec!["low".to_string(), "medium".to_string()]
        );
    }

    #[test]
    fn fallback_codex_efforts_are_not_two_level_only() {
        assert_eq!(
            fallback_codex_reasoning_efforts("gpt-5.4"),
            vec![
                "low".to_string(),
                "medium".to_string(),
                "high".to_string(),
                "xhigh".to_string()
            ]
        );
    }

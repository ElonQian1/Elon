    use super::{
        choose_cli_for_runtime, cli_name_from_parts, first_available_route_a_cli,
        route_c_runtime_ready, PcRuntimeRoutePreference,
    };
    use homecli_proto::NodeDevRuntimeProfile;
    use serde_json::json;

    #[test]
    fn cli_name_detects_known_providers() {
        assert_eq!(cli_name_from_parts("codex", "x", "x"), "codex");
        assert_eq!(cli_name_from_parts("x", "github-copilot", "x"), "copilot");
        assert_eq!(cli_name_from_parts("x", "x", "claude.exe"), "claude");
        assert_eq!(cli_name_from_parts("x", "api-runtime", "x"), "api-runtime");
        assert_eq!(
            cli_name_from_parts("x", "server-runtime", "x"),
            "server-runtime"
        );
    }

    #[test]
    fn route_a_preference_is_stable() {
        let allowed = vec!["gemini".to_string(), "codex".to_string()];
        assert_eq!(
            first_available_route_a_cli(&allowed).as_deref(),
            Some("codex")
        );
    }

    #[test]
    fn route_b_is_selected_before_server_runtime() {
        let runtime = NodeDevRuntimeProfile {
            api_runtime_ready: true,
            server_runtime_ready: true,
            ..Default::default()
        };
        assert_eq!(
            choose_cli_for_runtime(&[], Some(&runtime), "codex".to_string(), None).unwrap(),
            "api-runtime"
        );
    }

    #[test]
    fn route_c_is_selected_when_no_cli_or_api_runtime_exists() {
        let runtime = NodeDevRuntimeProfile {
            server_runtime_ready: true,
            ..Default::default()
        };
        assert_eq!(
            choose_cli_for_runtime(&[], Some(&runtime), "codex".to_string(), None).unwrap(),
            "server-runtime"
        );
    }

    #[test]
    fn route_c_status_gate_preserves_old_nodes_without_cloud_detail() {
        let runtime = NodeDevRuntimeProfile {
            server_runtime_ready: true,
            server_runtime_status: None,
            ..Default::default()
        };

        assert!(route_c_runtime_ready(Some(&runtime)));
        assert_eq!(
            choose_cli_for_runtime(&[], Some(&runtime), "codex".to_string(), None).unwrap(),
            "server-runtime"
        );
    }

    #[test]
    fn auto_route_skips_route_c_when_admission_is_limited() {
        let runtime = NodeDevRuntimeProfile {
            route_a_ready: false,
            server_runtime_ready: true,
            server_runtime_status: Some(json!({
                "ready": true,
                "status": "ready",
                "admissionAvailability": {
                    "ready": false,
                    "reason": "rate_limited",
                    "retryAfterSecs": 17
                }
            })),
            ..Default::default()
        };

        assert!(!route_c_runtime_ready(Some(&runtime)));
        assert_eq!(
            choose_cli_for_runtime(&[], Some(&runtime), "codex".to_string(), None).unwrap(),
            "codex"
        );
    }

    #[test]
    fn auto_route_skips_route_c_when_budget_is_exhausted() {
        let runtime = NodeDevRuntimeProfile {
            route_a_ready: false,
            server_runtime_ready: true,
            server_runtime_status: Some(json!({
                "ready": true,
                "status": "ready",
                "budget": {
                    "status": "user_exhausted",
                    "remainingCallsTodayForUser": 0
                }
            })),
            ..Default::default()
        };

        assert!(!route_c_runtime_ready(Some(&runtime)));
        assert_eq!(
            choose_cli_for_runtime(&[], Some(&runtime), "codex".to_string(), None).unwrap(),
            "codex"
        );
    }

    #[test]
    fn auto_route_skips_route_c_when_blocking_reasons_are_reported() {
        let runtime = NodeDevRuntimeProfile {
            route_a_ready: false,
            server_runtime_ready: true,
            server_runtime_status: Some(json!({
                "ready": true,
                "status": "ready",
                "blockingReasons": [{
                    "code": "platform_budget_exhausted",
                    "scope": "budget",
                    "message": "平台AI今日平台预算已用完"
                }]
            })),
            ..Default::default()
        };

        assert!(!route_c_runtime_ready(Some(&runtime)));
        assert_eq!(
            choose_cli_for_runtime(&[], Some(&runtime), "codex".to_string(), None).unwrap(),
            "codex"
        );
    }

    #[test]
    fn auto_route_skips_route_c_when_agent_policy_blocks_selection() {
        let runtime = NodeDevRuntimeProfile {
            route_a_ready: false,
            server_runtime_ready: true,
            server_runtime_status: Some(json!({
                "ready": true,
                "status": "ready",
                "agentPolicy": {
                    "ready": false,
                    "reason": "no_server_api_key_agent"
                }
            })),
            ..Default::default()
        };

        assert!(!route_c_runtime_ready(Some(&runtime)));
        assert_eq!(
            choose_cli_for_runtime(&[], Some(&runtime), "codex".to_string(), None).unwrap(),
            "codex"
        );
    }

    #[test]
    fn auto_route_skips_detected_route_a_when_profile_probe_failed() {
        let runtime = NodeDevRuntimeProfile {
            route_a_ready: false,
            server_runtime_ready: true,
            ..Default::default()
        };
        let allowed = vec!["codex".to_string()];
        assert_eq!(
            choose_cli_for_runtime(&allowed, Some(&runtime), "codex".to_string(), None).unwrap(),
            "server-runtime"
        );
    }

    #[test]
    fn auto_route_keeps_route_a_when_profile_probe_is_ready() {
        let runtime = NodeDevRuntimeProfile {
            route_a_ready: true,
            server_runtime_ready: true,
            ..Default::default()
        };
        let allowed = vec!["codex".to_string()];
        assert_eq!(
            choose_cli_for_runtime(&allowed, Some(&runtime), "codex".to_string(), None).unwrap(),
            "codex"
        );
    }

    #[test]
    fn forced_route_b_skips_available_route_a() {
        let runtime = NodeDevRuntimeProfile {
            api_runtime_ready: true,
            server_runtime_ready: true,
            ..Default::default()
        };
        let allowed = vec!["codex".to_string()];
        assert_eq!(
            choose_cli_for_runtime(
                &allowed,
                Some(&runtime),
                "codex".to_string(),
                Some(PcRuntimeRoutePreference::RouteB),
            )
            .unwrap(),
            "api-runtime"
        );
    }

    #[test]
    fn route_c1_alias_maps_to_server_runtime() {
        assert_eq!(
            PcRuntimeRoutePreference::from_request("route_c1").unwrap(),
            Some(PcRuntimeRoutePreference::RouteC)
        );
        assert_eq!(
            PcRuntimeRoutePreference::from_request("c1").unwrap(),
            Some(PcRuntimeRoutePreference::RouteC)
        );
    }

    #[test]
    fn route_c2_and_c3_aliases_are_open() {
        assert_eq!(
            PcRuntimeRoutePreference::from_request("route_c2").unwrap(),
            Some(PcRuntimeRoutePreference::RouteC2)
        );
        assert_eq!(
            PcRuntimeRoutePreference::from_request("remote-api-runtime").unwrap(),
            Some(PcRuntimeRoutePreference::RouteC2)
        );
        assert_eq!(
            PcRuntimeRoutePreference::from_request("route_c3").unwrap(),
            Some(PcRuntimeRoutePreference::RouteC3)
        );
        assert_eq!(
            PcRuntimeRoutePreference::from_request("remote-cli-runtime").unwrap(),
            Some(PcRuntimeRoutePreference::RouteC3)
        );
    }

    #[test]
    fn forced_route_c2_selects_remote_api_runtime() {
        let runtime = NodeDevRuntimeProfile {
            api_runtime_ready: true,
            route_a_ready: true,
            ..Default::default()
        };
        assert_eq!(
            choose_cli_for_runtime(
                &["codex".to_string()],
                Some(&runtime),
                "codex".to_string(),
                Some(PcRuntimeRoutePreference::RouteC2),
            )
            .unwrap(),
            "api-runtime"
        );
    }

    #[test]
    fn forced_route_c2_requires_api_runtime() {
        let err = choose_cli_for_runtime(
            &["codex".to_string()],
            Some(&NodeDevRuntimeProfile {
                route_a_ready: true,
                ..Default::default()
            }),
            "codex".to_string(),
            Some(PcRuntimeRoutePreference::RouteC2),
        )
        .expect_err("remote AI should require API key readiness");
        assert!(err.contains("远程AI"));
        assert!(err.contains("API key"));
    }

    #[test]
    fn forced_route_c3_selects_remote_cli() {
        let runtime = NodeDevRuntimeProfile {
            route_a_ready: true,
            api_runtime_ready: true,
            ..Default::default()
        };
        assert_eq!(
            choose_cli_for_runtime(
                &["copilot".to_string()],
                Some(&runtime),
                "codex".to_string(),
                Some(PcRuntimeRoutePreference::RouteC3),
            )
            .unwrap(),
            "copilot"
        );
    }

    #[test]
    fn forced_route_c3_requires_cli_probe() {
        let err = choose_cli_for_runtime(
            &["codex".to_string()],
            Some(&NodeDevRuntimeProfile {
                route_a_ready: false,
                api_runtime_ready: true,
                ..Default::default()
            }),
            "codex".to_string(),
            Some(PcRuntimeRoutePreference::RouteC3),
        )
        .expect_err("remote Codex should require remote AI tool readiness");
        assert!(err.contains("远程Codex"));
        assert!(err.contains("AI 工具"));
    }

    #[test]
    fn forced_unavailable_route_returns_actionable_error() {
        let err = choose_cli_for_runtime(
            &["codex".to_string()],
            Some(&NodeDevRuntimeProfile::default()),
            "codex".to_string(),
            Some(PcRuntimeRoutePreference::RouteC),
        )
        .expect_err("platform AI should not be selected when server runtime is not ready");
        assert!(err.contains("平台AI"));
        assert!(err.contains("暂时不可用"));
    }

    #[test]
    fn forced_route_c_reports_operational_protection_block() {
        let runtime = NodeDevRuntimeProfile {
            server_runtime_ready: true,
            server_runtime_status: Some(json!({
                "ready": true,
                "status": "ready",
                "admissionAvailability": {
                    "ready": false,
                    "reason": "user_concurrency_limited"
                }
            })),
            ..Default::default()
        };

        let err = choose_cli_for_runtime(
            &[],
            Some(&runtime),
            "codex".to_string(),
            Some(PcRuntimeRoutePreference::RouteC),
        )
        .expect_err("platform AI should not bypass cloud admission protection");
        assert!(err.contains("平台AI"));
        assert!(err.contains("限流"));
        assert!(err.contains("预算"));
    }

    #[test]
    fn forced_route_c_does_not_bypass_blocking_reasons() {
        let runtime = NodeDevRuntimeProfile {
            server_runtime_ready: true,
            server_runtime_status: Some(json!({
                "ready": true,
                "status": "ready",
                "blocking_reasons": [{
                    "code": "agent_policy_blocked",
                    "scope": "agent_policy"
                }]
            })),
            ..Default::default()
        };

        let err = choose_cli_for_runtime(
            &[],
            Some(&runtime),
            "codex".to_string(),
            Some(PcRuntimeRoutePreference::RouteC),
        )
        .expect_err("platform AI should not bypass cloud blocking reasons");
        assert!(err.contains("平台AI"));
        assert!(err.contains("限流"));
        assert!(err.contains("预算"));
    }

    #[test]
    fn forced_route_a_requires_successful_runtime_probe() {
        let runtime = NodeDevRuntimeProfile {
            route_a_ready: false,
            server_runtime_ready: true,
            ..Default::default()
        };
        let err = choose_cli_for_runtime(
            &["codex".to_string()],
            Some(&runtime),
            "codex".to_string(),
            Some(PcRuntimeRoutePreference::RouteA),
        )
        .expect_err("local AI should not be selected when tool probe failed");
        assert!(err.contains("本机AI"));
        assert!(err.contains("未通过"));
    }

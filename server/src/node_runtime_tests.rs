    use super::*;

    fn runtime(
        node_id: &str,
        device_name: Option<&str>,
        install_id: Option<&str>,
        online: bool,
        project_count: i64,
        connected_at: u64,
    ) -> NodeRuntime {
        NodeRuntime {
            node_id: node_id.to_string(),
            owner_user_id: "usr_test".to_string(),
            label: String::new(),
            device_name: device_name.map(ToOwned::to_owned),
            install_id: install_id.map(ToOwned::to_owned),
            public_dev_enabled: false,
            public_dev_allowed_clis: Vec::new(),
            public_dev_permission_level: "project_write".to_string(),
            last_handshake_at: None,
            last_handshake_agent_version: None,
            last_handshake_allowed_clis: Vec::new(),
            last_handshake_route_a_ready: false,
            last_handshake_api_runtime_ready: false,
            last_handshake_server_runtime_ready: false,
            last_handshake_ai_cli_ready: false,
            hardware: None,
            storage: None,
            dev_runtime: None,
            lifecycle: None,
            display_name: device_name.unwrap_or(node_id).to_string(),
            short_id: node_id.to_string(),
            models: Vec::new(),
            allowed_clis: Vec::new(),
            allowed_cwds: Vec::new(),
            agent_version: None,
            connected_at,
            created_at: format!("2026-07-05T00:00:{connected_at:02}Z"),
            online,
            registry_online: online,
            cli_connected: false,
            project_count,
        }
    }

    #[test]
    fn dedupe_prefers_online_node_for_same_install_id() {
        let nodes = dedupe_node_runtimes(vec![
            runtime("node-old", Some("ELONQIAN"), Some("ins_same"), false, 0, 1),
            runtime("node-live", Some("ELONQIAN"), Some("ins_same"), true, 0, 2),
        ]);

        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].node_id, "node-live");
    }

    #[test]
    fn dedupe_keeps_project_bound_legacy_node() {
        let nodes = dedupe_node_runtimes(vec![
            runtime("node-live", Some("ELONQIAN"), None, true, 0, 2),
            runtime("node-project", Some("ELONQIAN"), None, false, 1, 1),
            runtime("node-stale", Some("ELONQIAN"), None, false, 0, 0),
        ]);
        let ids = nodes
            .into_iter()
            .map(|node| node.node_id)
            .collect::<Vec<_>>();

        assert_eq!(ids, vec!["node-live", "node-project"]);
    }
    #[test]
    fn project_cli_support_accepts_codex_or_copilot_case_insensitive() {
        assert!(supports_project_cli(&["Codex".to_string()]));
        assert!(supports_project_cli(&["copilot".to_string()]));
        assert!(!supports_project_cli(&["node".to_string()]));
    }

    #[test]
    fn workspace_provision_ready_prefers_dev_runtime_profile() {
        let mut runtime = test_runtime(vec!["codex".to_string()]);
        runtime.dev_runtime = Some(NodeDevRuntimeProfile {
            workspace_provision_ready: false,
            ..Default::default()
        });

        assert!(!runtime.workspace_provision_ready());
    }

    #[test]
    fn workspace_provision_ready_falls_back_for_legacy_ai_cli_nodes() {
        let runtime = test_runtime(vec!["codex".to_string()]);

        assert!(runtime.workspace_provision_ready());
    }

    #[test]
    fn short_node_id_keeps_tail_for_long_ids() {
        assert_eq!(short_node_id("node-short"), "node-short");
        assert_eq!(
            short_node_id("node-user-1234567890abcdef"),
            "...34567890abcdef"
        );
    }

    fn test_runtime(allowed_clis: Vec<String>) -> NodeRuntime {
        NodeRuntime {
            node_id: "node-a".to_string(),
            owner_user_id: "user-a".to_string(),
            label: "PC-A".to_string(),
            device_name: Some("PC-A".to_string()),
            install_id: None,
            public_dev_enabled: false,
            public_dev_allowed_clis: Vec::new(),
            public_dev_permission_level: "project_write".to_string(),
            last_handshake_at: None,
            last_handshake_agent_version: None,
            last_handshake_allowed_clis: Vec::new(),
            last_handshake_route_a_ready: false,
            last_handshake_api_runtime_ready: false,
            last_handshake_server_runtime_ready: false,
            last_handshake_ai_cli_ready: false,
            hardware: None,
            storage: None,
            dev_runtime: None,
            lifecycle: None,
            display_name: "PC-A".to_string(),
            short_id: "node-a".to_string(),
            models: Vec::new(),
            allowed_clis,
            allowed_cwds: Vec::new(),
            agent_version: None,
            connected_at: 1,
            created_at: String::new(),
            online: true,
            registry_online: true,
            cli_connected: true,
            project_count: 0,
        }
    }

    use super::*;
    use homecli_proto::NodeDevRuntimeProfile;

    #[test]
    fn waits_only_for_initial_lightweight_node_registration() {
        let initial = test_runtime(None, Vec::new());
        assert!(should_wait_for_workspace_capability_profile(&initial));

        let legacy_ready = test_runtime(None, vec!["codex".to_string()]);
        assert!(!should_wait_for_workspace_capability_profile(&legacy_ready));

        let scanned_not_ready = test_runtime(
            Some(NodeDevRuntimeProfile {
                workspace_provision_ready: false,
                ..Default::default()
            }),
            Vec::new(),
        );
        assert!(!should_wait_for_workspace_capability_profile(
            &scanned_not_ready
        ));
    }

    #[test]
    fn runtime_owner_match_requires_current_user() {
        let runtime = test_runtime(None, vec!["codex".to_string()]);

        assert!(runtime_owned_by_user(&runtime, "user-a"));
        assert!(runtime_owned_by_user(&runtime, " user-a "));
        assert!(!runtime_owned_by_user(&runtime, "user-b"));
        assert!(!runtime_owned_by_user(&runtime, ""));
    }

    fn test_runtime(
        dev_runtime: Option<NodeDevRuntimeProfile>,
        allowed_clis: Vec<String>,
    ) -> NodeRuntime {
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
            dev_runtime,
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

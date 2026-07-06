    use super::*;

    #[test]
    fn encoded_query_escapes_local_admin_url() {
        assert_eq!(
            encode_query_component("http://127.0.0.1:7799/?a=1&b=2"),
            "http%3A%2F%2F127.0.0.1%3A7799%2F%3Fa%3D1%26b%3D2"
        );
    }

    #[test]
    fn admin_health_requires_node_status_marker() {
        assert!(admin_status_response_healthy(
            "HTTP/1.1 200 OK\r\n\r\n{\"local_admin_token_header\":\"X-Elon-Local-Admin-Token\"}"
        ));
        assert!(!admin_status_response_healthy(
            "HTTP/1.1 200 OK\r\n\r\n<html>not our service</html>"
        ));
        assert!(!admin_status_response_healthy(
            "HTTP/1.1 404 Not Found\r\n\r\n{\"local_admin_token_header\":\"x\"}"
        ));
    }

    #[test]
    fn admin_health_accepts_large_status_response() {
        let response = format!(
            "HTTP/1.1 200 OK\r\n\r\n{{\"padding\":\"{}\",\"local_admin_token_header\":\"x\"}}",
            "x".repeat(8 * 1024)
        );

        assert!(response.len() < ADMIN_HEALTH_READ_LIMIT);
        assert!(admin_status_response_healthy(&response));
    }

    #[test]
    fn open_target_defaults_to_pc_workspace() {
        let env_values = HashMap::new();

        assert_eq!(
            open_target_from_env_values(&env_values),
            OpenTarget::SmartWorkbench
        );
        assert!(!open_target_from_env_values(&env_values).requires_admin_ready());
    }

    #[test]
    fn local_workbench_open_target_requires_ready_admin_api() {
        let mut env_values = HashMap::new();
        env_values.insert(
            "NODE_AGENT_OPEN_TARGET".to_string(),
            " local_workbench ".to_string(),
        );

        assert_eq!(
            open_target_from_env_values(&env_values),
            OpenTarget::LocalWorkbench
        );
        assert!(open_target_from_env_values(&env_values).requires_admin_ready());
    }

    #[test]
    fn cloud_pc_url_carries_selected_local_admin_port() {
        let mut env_values = HashMap::new();
        env_values.insert(
            "NODE_AGENT_WEB_BASE_URL".to_string(),
            "http://cloud.example/".to_string(),
        );

        assert_eq!(
            cloud_pc_url(7803, &env_values),
            "http://cloud.example/pc?node_admin=http%3A%2F%2F127.0.0.1%3A7803%2F"
        );
    }

    #[test]
    fn cloud_probe_timeout_is_bounded() {
        let mut env_values = HashMap::new();
        env_values.insert(
            "NODE_AGENT_CLOUD_PROBE_TIMEOUT_MS".to_string(),
            "10".to_string(),
        );
        assert_eq!(cloud_probe_timeout(&env_values), Duration::from_millis(300));

        env_values.insert(
            "NODE_AGENT_CLOUD_PROBE_TIMEOUT_MS".to_string(),
            "50000".to_string(),
        );
        assert_eq!(
            cloud_probe_timeout(&env_values),
            Duration::from_millis(10_000)
        );
    }

    #[test]
    fn admin_port_defaults_and_accepts_configured_value() {
        let env_values = HashMap::new();
        assert_eq!(admin_port_from_env_values(&env_values), DEFAULT_ADMIN_PORT);

        let mut env_values = HashMap::new();
        env_values.insert("NODE_ADMIN_PORT".to_string(), "7801".to_string());
        assert_eq!(admin_port_from_env_values(&env_values), 7801);

        env_values.insert("NODE_ADMIN_PORT".to_string(), "not-a-port".to_string());
        assert_eq!(admin_port_from_env_values(&env_values), DEFAULT_ADMIN_PORT);
    }

    #[test]
    fn local_admin_open_target_requires_ready_admin_api() {
        let mut env_values = HashMap::new();
        env_values.insert(
            "NODE_AGENT_OPEN_TARGET".to_string(),
            " local_ADMIN ".to_string(),
        );

        assert_eq!(
            open_target_from_env_values(&env_values),
            OpenTarget::LocalAdmin
        );
        assert!(open_target_from_env_values(&env_values).requires_admin_ready());
    }

    #[test]
    fn cloud_pc_open_target_keeps_legacy_cloud_entry() {
        let mut env_values = HashMap::new();
        env_values.insert("NODE_AGENT_OPEN_TARGET".to_string(), "cloud_pc".to_string());

        assert_eq!(
            open_target_from_env_values(&env_values),
            OpenTarget::CloudPc
        );
        assert!(!open_target_from_env_values(&env_values).requires_admin_ready());
    }

    #[test]
    fn running_runtime_keeps_configured_admin_port() {
        assert_eq!(select_admin_port_for_runtime(7799, true), 7799);
    }

    #[cfg(windows)]
    #[test]
    fn runtime_query_matches_current_client_only() {
        let script = agent_runtime_query_script(Path::new(r"C:\ElonNode\一龙PC节点.exe"));

        assert!(script.contains("--agent-runtime"));
        assert!(script.contains(r"C:\ElonNode\一龙PC节点.exe"));
        assert!(script.contains("and $exeMatch"));
        assert!(!script.contains("lineMatch"));
        assert!(!script.contains("elon-node-agent.exe"));
    }

    #[cfg(windows)]
    #[test]
    fn runtime_spawn_script_uses_start_process_and_overrides_runtime_env() {
        let mut env_values = HashMap::new();
        env_values.insert("NODE_AUTO_OPEN_ADMIN".to_string(), "1".to_string());
        env_values.insert(
            "NODE_AGENT_WEB_BASE_URL".to_string(),
            "http://example.test".to_string(),
        );
        env_values.insert("QUOTED".to_string(), "O'Hara".to_string());

        let script = spawn_agent_runtime_script(
            Path::new(r"C:\ElonNode\client.exe"),
            Path::new(r"C:\ElonNode"),
            7801,
            &env_values,
            Path::new(r"C:\ElonNode\_internal\logs\runtime-spawn.pid"),
        );

        assert!(script.contains("Start-Process -FilePath $client -ArgumentList '--agent-runtime'"));
        assert!(script.contains("Set-Content -LiteralPath $pidFile"));
        assert!(!script.contains("Write-Output $process.Id"));
        assert!(script.contains(
            "[Environment]::SetEnvironmentVariable('NODE_ADMIN_PORT', '7801', 'Process')"
        ));
        assert!(script
            .contains("[Environment]::SetEnvironmentVariable('QUOTED', 'O''Hara', 'Process')"));
        assert!(script.contains(r"$client = 'C:\ElonNode\client.exe'"));
        assert!(script.contains(r"$pidFile = 'C:\ElonNode\_internal\logs\runtime-spawn.pid'"));

        let inherited_auto_open = script
            .find("[Environment]::SetEnvironmentVariable('NODE_AUTO_OPEN_ADMIN', '1', 'Process')")
            .unwrap();
        let launcher_auto_open = script
            .rfind("[Environment]::SetEnvironmentVariable('NODE_AUTO_OPEN_ADMIN', '0', 'Process')")
            .unwrap();
        assert!(launcher_auto_open > inherited_auto_open);
    }

    #[cfg(windows)]
    #[test]
    fn stop_installed_client_processes_excludes_current_pid() {
        let script = stop_installed_client_processes_script(
            Path::new(r"C:\ElonNode\一龙开发平台.exe"),
            1234,
        );

        assert!(script.contains(r"C:\ElonNode\一龙开发平台.exe"));
        assert!(script.contains("$currentPid = 1234"));
        assert!(script.contains("ProcessId -ne"));
        assert!(script.contains("elon-node-agent.exe"));
        assert!(script.contains("Terminate"));
    }

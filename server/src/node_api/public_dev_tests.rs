    use super::*;

    fn ready_runtime() -> NodeDevRuntimeProfile {
        NodeDevRuntimeProfile {
            route_a_ready: true,
            ai_cli_ready: true,
            workspace_provision_ready: true,
            ..NodeDevRuntimeProfile::default()
        }
    }

    #[test]
    fn public_dev_handshake_reports_ready_for_matching_online_codex_node() {
        let (ready, status) = public_dev_handshake_state_from_parts(
            true,
            &["codex".to_string()],
            Some("2026-07-06T00:00:00Z"),
            Some("0.3.70"),
            &["codex".to_string()],
            true,
            Some("0.3.70"),
            &["Codex".to_string()],
            Some(&ready_runtime()),
        );

        assert!(ready);
        assert_eq!(status, "ready");
    }

    #[test]
    fn public_dev_handshake_explains_common_pending_states() {
        let (_, status) = public_dev_handshake_state_from_parts(
            true,
            &["codex".to_string()],
            Some("2026-07-06T00:00:00Z"),
            Some("0.3.69"),
            &["codex".to_string()],
            true,
            Some("0.3.70"),
            &["codex".to_string()],
            Some(&ready_runtime()),
        );
        assert_eq!(status, "version_reconnected_waiting_capabilities");

        let (_, status) = public_dev_handshake_state_from_parts(
            true,
            &["codex".to_string()],
            Some("2026-07-06T00:00:00Z"),
            Some("0.3.70"),
            &["copilot".to_string()],
            true,
            Some("0.3.70"),
            &["copilot".to_string()],
            Some(&ready_runtime()),
        );
        assert_eq!(status, "no_allowed_cli");

        let (_, status) = public_dev_handshake_state_from_parts(
            true,
            &["codex".to_string()],
            Some("2026-07-06T00:00:00Z"),
            Some("0.3.70"),
            &["codex".to_string()],
            true,
            Some("0.3.70"),
            &["codex".to_string()],
            Some(&NodeDevRuntimeProfile::default()),
        );
        assert_eq!(status, "runtime_not_ready");
    }

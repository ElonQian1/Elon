    use super::{diagnostic_payload, DiagnosticPaths};
    use serde_json::json;
    use std::{fs, path::PathBuf};

    #[test]
    fn diagnostic_payload_redacts_secret_values_and_raw_output() {
        let dir = unique_test_dir("redaction");
        let state_file = dir.join("node.json");
        let journal_dir = dir.join("task-journal");
        fs::create_dir_all(&journal_dir).expect("journal dir");
        fs::write(
            journal_dir.join("registry.json"),
            serde_json::to_string_pretty(&json!({
                "req-1": {
                    "req_id": "req-1",
                    "cli_name": "server-runtime",
                    "route": "route_c_server_runtime",
                    "run_handle_id": "req-1",
                    "cwd": "D:/demo",
                    "runtime_permission": "project_write",
                    "status": "running",
                    "os_pid": 42,
                    "started_at_ms": 1,
                    "updated_at_ms": 2,
                    "codex_session_id": "secret-session-id"
                }
            }))
            .unwrap(),
        )
        .expect("registry");
        fs::write(
            journal_dir.join("events.jsonl"),
            "{\"type\":\"started\",\"req_id\":\"req-1\"}\n{\"type\":\"cli_chunk\",\"text\":\"secret output\"}\n",
        )
        .expect("events");
        let logs_dir = dir.join("logs");
        fs::create_dir_all(&logs_dir).expect("logs dir");
        fs::write(
            logs_dir.join("client-maintenance.jsonl"),
            "{\"at_ms\":3,\"action\":\"open_target\",\"ok\":false,\"detail\":\"secret maintenance detail\"}\n",
        )
        .expect("maintenance log");
        let paths = DiagnosticPaths::from_state_file(state_file);
        let payload = diagnostic_payload(&paths);
        let text = serde_json::to_string(&payload).expect("payload json");

        assert!(payload["privacy"]["raw_cli_output_exported"] == false);
        assert!(payload["privacy"]["maintenance_log_contents_exported"] == false);
        assert!(payload["privacy"]["maintenance_log_details_exported"] == false);
        assert!(payload["privacy"]["api_key_values_exported"] == false);
        assert!(payload["paths"]["logs_dir"].as_str().is_some());
        assert!(payload
            .get("paths")
            .unwrap()
            .get("launcher_logs_dir")
            .is_some());
        assert!(payload
            .get("paths")
            .unwrap()
            .get("launcher_log_file")
            .is_some());
        assert!(payload["files"]["maintenance_log"]["path"]
            .as_str()
            .unwrap()
            .contains("client-maintenance.jsonl"));
        assert!(payload["files"].get("launcher_log").is_some());
        assert!(text.contains("\"codex_session_present\":true"));
        assert!(!text.contains("secret-session-id"));
        assert!(!text.contains("secret output"));
        assert!(!text.contains("secret maintenance detail"));
        assert_eq!(payload["logs"]["maintenance"]["line_count"], 1);
        assert_eq!(
            payload["logs"]["maintenance"]["actions"]["open_target"]["failed"],
            1
        );
        assert_eq!(payload["tasks"]["events"]["line_count"], 2);
        assert_eq!(payload["tasks"]["events"]["types"]["started"], 1);
        assert_eq!(payload["tasks"]["events"]["types"]["cli_chunk"], 1);

        let _ = fs::remove_dir_all(dir);
    }

    fn unique_test_dir(suffix: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "elon-client-diagnostics-test-{}-{}",
            std::process::id(),
            suffix
        ))
    }

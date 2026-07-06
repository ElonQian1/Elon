    use super::diagnostic_log_summary;
    use std::{fs, path::PathBuf};

    #[test]
    fn summarizes_maintenance_and_launcher_logs_without_detail_values() {
        let dir = unique_test_dir("summary");
        fs::create_dir_all(&dir).unwrap();
        let maintenance = dir.join("client-maintenance.jsonl");
        let launcher = dir.join("client-launcher.jsonl");
        fs::write(
            &maintenance,
            concat!(
                "{\"at_ms\":1,\"action\":\"open_target\",\"ok\":true,\"detail\":\"secret-token\"}\n",
                "{\"at_ms\":2,\"action\":\"update\",\"ok\":false,\"detail\":\"api-key-value\"}\n",
                "not-json\n"
            ),
        )
        .unwrap();
        fs::write(
            &launcher,
            "{\"at_ms\":3,\"action\":\"install\",\"ok\":true,\"detail\":\"private path\",\"pid\":42}\n",
        )
        .unwrap();

        let summary = diagnostic_log_summary(&maintenance, Some(&launcher));
        let text = serde_json::to_string(&summary).unwrap();

        assert_eq!(summary["maintenance"]["line_count"], 3);
        assert_eq!(summary["maintenance"]["parse_errors"], 1);
        assert_eq!(summary["maintenance"]["actions"]["open_target"]["ok"], 1);
        assert_eq!(summary["maintenance"]["actions"]["update"]["failed"], 1);
        assert_eq!(summary["launcher"]["actions"]["install"]["ok"], 1);
        assert_eq!(summary["launcher"]["recent_events"][0]["pid"], 42);
        assert!(!text.contains("secret-token"));
        assert!(!text.contains("api-key-value"));
        assert!(!text.contains("private path"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_launcher_path_is_reported_without_failing_export() {
        let dir = unique_test_dir("missing");
        let summary = diagnostic_log_summary(&dir.join("missing.jsonl"), None);

        assert_eq!(summary["maintenance"]["exists"], false);
        assert_eq!(summary["launcher"]["exists"], false);
        assert_eq!(
            summary["launcher"]["reason"],
            "launcher_log_path_unavailable"
        );

        let _ = fs::remove_dir_all(dir);
    }

    fn unique_test_dir(suffix: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "elon-client-diagnostic-log-test-{}-{}",
            std::process::id(),
            suffix
        ))
    }

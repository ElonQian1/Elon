#[cfg(test)]
mod tests {
    use super::{
        install_dir_from_local_app_data, maintenance_actions, maintenance_overview,
        maintenance_target, primary_maintenance_action, recent_maintenance_events, status_payload,
        truncate_chars,
    };
    use serde_json::json;
    use std::{fs, path::PathBuf};

    #[test]
    fn install_dir_is_under_local_app_data_elon_node() {
        assert_eq!(
            install_dir_from_local_app_data(Some(r"C:\Users\ELon\AppData\Local")).unwrap(),
            PathBuf::from(r"C:\Users\ELon\AppData\Local").join("ElonNode")
        );
        assert!(install_dir_from_local_app_data(Some(" ")).is_none());
        assert!(install_dir_from_local_app_data(None).is_none());
    }

    #[cfg(windows)]
    #[test]
    fn autostart_command_decode_preserves_unicode_path() {
        use base64::{engine::general_purpose::STANDARD as B64, Engine as _};

        let command = r#""C:\Users\ELon\AppData\Local\ElonNode\一龙开发平台.exe""#;
        let encoded = B64.encode(command.as_bytes());

        assert_eq!(
            super::decode_autostart_command(&encoded)
                .unwrap()
                .as_deref(),
            Some(command)
        );
    }

    #[test]
    fn only_fixed_open_targets_are_supported() {
        assert!(maintenance_target("task_journal").is_ok());
        assert!(maintenance_target("logs").is_ok());
        assert!(maintenance_target("maintenance_log").is_ok());
        assert!(maintenance_target("launcher_logs").is_ok());
        assert!(maintenance_target("diagnostics_dir").is_ok());
        assert!(maintenance_target("config_dir").is_ok());
        assert!(maintenance_target("state_file").is_ok());
        assert!(maintenance_target(r"C:\Windows").is_err());
    }

    #[test]
    fn status_exposes_productized_maintenance_targets() {
        let status = status_payload();
        assert!(status["logs_dir"].as_str().is_some());
        assert!(status["maintenance_log_file"].as_str().is_some());
        assert!(status.get("launcher_logs_dir").is_some());
        assert!(status.get("launcher_log_file").is_some());
        assert!(status["diagnostics_dir"].as_str().is_some());
        assert!(status["maintenance_recent_events"].as_array().is_some());
        assert!(status["maintenance_targets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|target| target["target"].as_str() == Some("logs")));
        assert!(status["maintenance_targets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|target| target["target"].as_str() == Some("launcher_logs")));
        assert!(status["maintenance_targets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|target| target["target"].as_str() == Some("diagnostics_dir")));
        assert!(status["client_care_summary"]
            .as_str()
            .unwrap()
            .contains("运行日志"));
        assert!(status["maintenance_actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| {
                action["id"].as_str() == Some("open_client_logs")
                    && action["target"].as_str() == Some("logs")
            }));
        assert!(status["maintenance_actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| {
                action["id"].as_str() == Some("open_launcher_logs")
                    && action["target"].as_str() == Some("launcher_logs")
            }));
        assert!(status["maintenance_actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| {
                action["id"].as_str() == Some("open_config_dir")
                    && action["target"].as_str() == Some("config_dir")
            }));
        assert!(status["maintenance_actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| {
                action["id"].as_str() == Some("open_state_file")
                    && action["target"].as_str() == Some("state_file")
            }));
        assert!(status["maintenance_actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| {
                action["id"].as_str() == Some("repair_client")
                    && action["kind"].as_str() == Some("repair")
            }));
        assert!(status["maintenance_actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| action["kind"].as_str() == Some("export_diagnostics")));
        assert!(status["maintenance_actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| {
                action["kind"].as_str() == Some("uninstall")
                    && action["confirmation"].as_str().is_some()
            }));
        assert!(status["product_status"]["summary"].as_str().is_some());
        assert!(status["primary_maintenance_action"]["id"]
            .as_str()
            .is_some());
        assert_eq!(
            status["primary_maintenance_action"]["recommended"].as_bool(),
            Some(true)
        );
        assert!(status["primary_maintenance_action"]["recommendation"]
            .as_str()
            .is_some());
        assert_eq!(
            status["maintenance_overview"]["safe_to_share_diagnostics"].as_bool(),
            Some(true)
        );
        assert!(status["maintenance_overview"]["title"].as_str().is_some());
        assert!(status["maintenance_overview"]["primary_action_id"]
            .as_str()
            .is_some());
        assert_eq!(
            status["product_status"]["primary_entry_name"].as_str(),
            Some(crate::node_client_launcher::CLIENT_EXE_NAME)
        );
    }

    #[test]
    fn maintenance_actions_are_ui_renderable_contracts() {
        let status = status_payload();
        let actions = status["maintenance_actions"]
            .as_array()
            .expect("maintenance actions should be an array");
        assert!(actions.len() >= 9);

        for action in actions {
            let kind = action["kind"]
                .as_str()
                .expect("maintenance action kind should be string");
            assert!(
                !action["id"].as_str().unwrap_or_default().trim().is_empty(),
                "maintenance action id should be renderable"
            );
            assert!(
                !action["label"]
                    .as_str()
                    .unwrap_or_default()
                    .trim()
                    .is_empty(),
                "maintenance action label should be renderable"
            );
            assert!(
                !action["description"]
                    .as_str()
                    .unwrap_or_default()
                    .trim()
                    .is_empty(),
                "maintenance action description should be renderable"
            );
            assert!(action["enabled"].as_bool().is_some());
            assert!(matches!(
                action["tone"].as_str(),
                Some("primary" | "neutral" | "danger")
            ));
            if kind == "open_target" {
                assert!(
                    !action["target"]
                        .as_str()
                        .unwrap_or_default()
                        .trim()
                        .is_empty(),
                    "open_target action should include a maintenance target"
                );
            }
            if kind == "uninstall" {
                assert!(
                    action["confirmation"]
                        .as_str()
                        .unwrap_or_default()
                        .contains("卸载"),
                    "uninstall action should carry its confirmation copy"
                );
            }
        }
    }

    #[test]
    fn maintenance_actions_mark_repair_as_primary_when_layout_needs_cleanup() {
        let actions = maintenance_actions(&json!({
            "supported": true,
            "installed": true,
            "layout_status": "legacy_files_present",
            "product_status": { "status": "cleanup_recommended" }
        }));
        let primary = primary_maintenance_action(&actions);

        assert_eq!(primary["id"].as_str(), Some("repair_client"));
        assert_eq!(primary["recommended"].as_bool(), Some(true));
        assert!(primary["recommendation"]
            .as_str()
            .unwrap_or_default()
            .contains("修复客户端入口"));
        assert_eq!(
            actions
                .as_array()
                .unwrap()
                .iter()
                .filter(|action| action["recommended"].as_bool() == Some(true))
                .count(),
            1
        );
    }

    #[test]
    fn maintenance_actions_mark_update_as_primary_when_client_is_ready() {
        let actions = maintenance_actions(&json!({
            "supported": true,
            "installed": true,
            "layout_status": "clean",
            "product_status": { "status": "ready" }
        }));
        let primary = primary_maintenance_action(&actions);

        assert_eq!(primary["id"].as_str(), Some("check_update"));
        assert_eq!(primary["recommended"].as_bool(), Some(true));
        assert!(primary["recommendation"]
            .as_str()
            .unwrap_or_default()
            .contains("检查"));
    }

    #[test]
    fn maintenance_overview_marks_ready_client_as_ok() {
        let actions = maintenance_actions(&json!({
            "supported": true,
            "installed": true,
            "layout_status": "clean",
            "product_status": { "status": "ready", "summary": "客户端入口正常" }
        }));
        let primary = primary_maintenance_action(&actions);
        let overview = maintenance_overview(
            &json!({
                "supported": true,
                "installed": true,
                "layout_status": "clean",
                "product_status": { "status": "ready", "summary": "客户端入口正常" }
            }),
            &primary,
            &json!([]),
        );

        assert_eq!(overview["status"].as_str(), Some("ready"));
        assert_eq!(overview["severity"].as_str(), Some("ok"));
        assert_eq!(overview["primary_action_id"].as_str(), Some("check_update"));
        assert_eq!(overview["recent_failure_count"].as_u64(), Some(0));
    }

    #[test]
    fn maintenance_overview_surfaces_recent_failures() {
        let actions = maintenance_actions(&json!({
            "supported": true,
            "installed": true,
            "layout_status": "clean",
            "product_status": { "status": "ready", "summary": "客户端入口正常" }
        }));
        let primary = primary_maintenance_action(&actions);
        let overview = maintenance_overview(
            &json!({
                "supported": true,
                "installed": true,
                "layout_status": "clean",
                "product_status": { "status": "ready", "summary": "客户端入口正常" }
            }),
            &primary,
            &json!([
                { "action": "repair", "ok": false, "detail": "failed" },
                { "action": "update", "ok": true, "detail": "scheduled" }
            ]),
        );

        assert_eq!(overview["status"].as_str(), Some("attention"));
        assert_eq!(overview["severity"].as_str(), Some("warning"));
        assert_eq!(overview["recent_failure_count"].as_u64(), Some(1));
        assert_eq!(overview["latest_failure_action"].as_str(), Some("repair"));
        assert!(!overview.to_string().contains("failed"));
    }

    #[test]
    fn maintenance_log_details_are_bounded() {
        let long = "x".repeat(700);
        assert_eq!(truncate_chars(&long, 500).chars().count(), 500);
        assert_eq!(truncate_chars("  ok  ", 500), "ok");
    }

    #[test]
    fn recent_maintenance_events_are_newest_first_and_bounded() {
        let path = std::env::temp_dir().join(format!(
            "elon-client-maintenance-events-{}.jsonl",
            std::process::id()
        ));
        let long_detail = "x".repeat(220);
        fs::write(
            &path,
            format!(
                "not-json\n\
                 {{\"at_ms\":1,\"action\":\"open_target\",\"ok\":true,\"detail\":\"{long_detail}\"}}\n\
                 {{\"at_ms\":2,\"action\":\"\",\"ok\":true,\"detail\":\"ignored\"}}\n\
                 {{\"at_ms\":3,\"action\":\"update\",\"ok\":false,\"detail\":\"failed\"}}\n\
                 {{\"at_ms\":4,\"action\":\"uninstall\",\"ok\":true,\"detail\":\"scheduled\"}}\n"
            ),
        )
        .expect("maintenance event fixture should write");

        let events = recent_maintenance_events(&path, 3);
        let items = events.as_array().expect("events should be an array");
        assert_eq!(items.len(), 3);
        assert_eq!(items[0]["action"].as_str(), Some("uninstall"));
        assert_eq!(items[1]["action"].as_str(), Some("update"));
        assert_eq!(items[2]["action"].as_str(), Some("open_target"));
        assert!(items[2]["detail"].as_str().unwrap().chars().count() <= 180);

        let _ = fs::remove_file(path);
    }
}

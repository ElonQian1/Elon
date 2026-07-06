    use super::*;

    #[test]
    fn keeps_existing_done_apk_url() {
        let raw = r#"{"type":"done","message":"ok","apk_url":"https://example.test/app.apk"}"#;
        let (updated, apk_url) =
            ensure_done_event_has_project_apk_url(raw.into(), "https://download.test", &[]);

        assert_eq!(updated, raw);
        assert_eq!(apk_url.as_deref(), Some("https://example.test/app.apk"));
    }

    #[test]
    fn leaves_non_done_events_unchanged() {
        let raw = r#"{"type":"progress","message":"working"}"#;
        let (updated, apk_url) =
            ensure_done_event_has_project_apk_url(raw.into(), "https://download.test", &[]);

        assert_eq!(updated, raw);
        assert!(apk_url.is_none());
    }

    #[test]
    fn fills_missing_done_apk_url_when_artifact_exists() {
        let root = std::env::temp_dir().join(format!(
            "elon_project_completion_test_{}",
            std::process::id()
        ));
        let apk_dir = root.join("android/app/build/outputs/apk/release");
        std::fs::create_dir_all(&apk_dir).unwrap();
        std::fs::write(apk_dir.join("app-release.apk"), b"apk").unwrap();

        let raw = r#"{"type":"done","message":"ok","apk_url":null}"#;
        let (updated, apk_url) = ensure_done_event_has_project_apk_url(
            raw.into(),
            "https://download.test/project",
            &[&root],
        );
        let value: Value = serde_json::from_str(&updated).unwrap();

        assert_eq!(
            apk_url.as_deref(),
            Some("https://download.test/project/latest.apk")
        );
        assert_eq!(
            value.get("apk_url").and_then(Value::as_str),
            Some("https://download.test/project/latest.apk")
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn removes_no_remote_push_notice_from_done_message() {
        let raw = r#"{"type":"done","message":"已完成并提交：9179ecf Build simple role selection app。\n\n当前仓库没有配置远端，所以无法 push。\n\n下载新 APK：https://example.test/latest.apk","apk_url":"https://example.test/latest.apk"}"#;
        let (updated, apk_url) =
            ensure_done_event_has_project_apk_url(raw.into(), "https://download.test", &[]);
        let value: Value = serde_json::from_str(&updated).unwrap();
        let message = value.get("message").and_then(Value::as_str).unwrap();

        assert_eq!(apk_url.as_deref(), Some("https://example.test/latest.apk"));
        assert!(message.contains("已完成并提交"));
        assert!(message.contains("下载新 APK"));
        assert!(!message.contains("无法 push"));
        assert!(!message.contains("没有配置远端"));
    }

    #[test]
    fn keeps_commit_summary_when_removing_parenthetical_remote_noise() {
        let raw = r#"{"type":"done","message":"已完成并提交：9179ecf（无远程或 push 失败，仅本地提交）","apk_url":"https://example.test/latest.apk"}"#;
        let (updated, _) =
            ensure_done_event_has_project_apk_url(raw.into(), "https://download.test", &[]);
        let value: Value = serde_json::from_str(&updated).unwrap();
        let message = value.get("message").and_then(Value::as_str).unwrap();

        assert_eq!(message, "已完成并提交：9179ecf");
    }

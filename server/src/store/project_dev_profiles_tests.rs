    use super::*;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon_store_project_dev_profile_{}.db",
            Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn upsert_project_dev_profile_round_trips_detected_commands() {
        let store = temp_store();
        let user = store
            .create_user("project-profile-owner@example.com", "secret1", None, None)
            .expect("user should be created");
        let project = store
            .register_external_project(
                &user.id,
                None,
                "Profiled Project",
                None,
                r"D:\rust\active-projects\profiled",
                Some("node-a"),
                Some("https://example.com/profiled.git"),
                Some("main"),
            )
            .expect("project should register");

        let saved = store
            .upsert_project_dev_profile(
                &user.id,
                &project.project.id,
                &ProjectDevProfile {
                    project_type: Some("Node.js".to_string()),
                    package_manager: Some("pnpm".to_string()),
                    run_command: Some("pnpm dev".to_string()),
                    test_command: Some("pnpm test".to_string()),
                    build_command: Some("pnpm build".to_string()),
                    detected_files: vec!["package.json".to_string(), "pnpm-lock.yaml".to_string()],
                    source: None,
                    updated_at: None,
                },
            )
            .expect("profile should save")
            .expect("profile should be non-empty");

        assert_eq!(saved.project_type.as_deref(), Some("Node.js"));
        assert_eq!(saved.package_manager.as_deref(), Some("pnpm"));
        assert_eq!(saved.test_command.as_deref(), Some("pnpm test"));
        assert_eq!(saved.source.as_deref(), Some("node_agent_project_picker"));
        assert!(saved.updated_at.is_some());
        assert_eq!(saved.detected_files.len(), 2);
    }

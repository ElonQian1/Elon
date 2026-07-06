    use super::*;

    #[test]
    fn known_project_branding_matches_windows_workspace_paths() {
        assert_eq!(
            default_display_name_for_project(
                "bb64a",
                "pc_managed",
                Some(r"D:\rust\active-projects\bb64a"),
                None,
                None
            )
            .as_deref(),
            Some(BB64A_DISPLAY_NAME)
        );
        assert_eq!(
            default_display_name_for_project(
                "一龙项目",
                "pc_managed",
                Some(r"D:\rust\active-projects\elon cli"),
                None,
                None
            )
            .as_deref(),
            Some(ELON_SELF_DISPLAY_NAME)
        );
        assert_eq!(
            default_display_name_for_project(
                "fb2",
                "pc_managed",
                Some(r"D:\rust\active-projects\fb2"),
                None,
                None
            )
            .as_deref(),
            Some(FB2_DISPLAY_NAME)
        );
        assert_eq!(
            default_display_name_for_project(
                "NanchangJiAnChamber",
                "local_path",
                Some(r"D:\rust\active-projects\江西吉安商会\NanchangJiAnChamber"),
                None,
                None
            )
            .as_deref(),
            Some(JIANGXI_JIAN_CHAMBER_DISPLAY_NAME)
        );
    }

    #[test]
    fn known_project_branding_preserves_manual_icon() {
        let icon = branded_icon_data_url(
            Some("data:image/png;base64,manual".to_string()),
            "fb2",
            "pc_managed",
            Some(r"D:\rust\active-projects\fb2"),
            None,
            None,
        );
        assert_eq!(icon.as_deref(), Some("data:image/png;base64,manual"));
    }

    #[test]
    fn public_project_branding_compacts_large_known_project_icons() {
        let small_icon = "data:image/png;base64,manual".to_string();
        let mut small_project = public_project("fb2", Some(small_icon.clone()));
        apply_public_project_branding(
            &mut small_project,
            "pc_managed",
            Some(r"D:\rust\active-projects\fb2"),
        );
        assert_eq!(
            small_project.icon_data_url.as_deref(),
            Some(small_icon.as_str())
        );

        let large_icon = format!(
            "data:image/png;base64,{}",
            "a".repeat(PUBLIC_PROJECT_ICON_DATA_URL_SOFT_LIMIT + 1)
        );
        let mut large_project = public_project("fb2", Some(large_icon));
        apply_public_project_branding(
            &mut large_project,
            "pc_managed",
            Some(r"D:\rust\active-projects\fb2"),
        );
        let icon = large_project.icon_data_url.expect("compact default icon");
        assert!(icon.starts_with("data:image/png;base64,"));
        assert!(icon.len() <= PUBLIC_PROJECT_ICON_DATA_URL_SOFT_LIMIT);
    }

    #[test]
    fn public_project_branding_replaces_elon_self_generic_description() {
        let mut project = public_project("一龙项目", None);
        project.description = Some("本地 Git 项目".to_string());

        apply_public_project_branding(
            &mut project,
            "pc_managed",
            Some(r"D:\rust\active-projects\elon cli"),
        );

        let description = project.description.expect("branded description");
        assert_eq!(description, ELON_SELF_PUBLIC_DESCRIPTION);
        assert!(description.chars().count() > 24);
    }

    #[test]
    fn public_project_branding_preserves_specific_description() {
        let mut project = public_project("fb2", None);
        project.description = Some("多冠体育赛事应用".to_string());

        apply_public_project_branding(
            &mut project,
            "pc_managed",
            Some(r"D:\rust\active-projects\fb2"),
        );

        assert_eq!(project.description.as_deref(), Some("多冠体育赛事应用"));
    }

    #[test]
    fn known_project_branding_supplies_default_icons() {
        let icon = branded_icon_data_url(
            None,
            "elon-self",
            "pc_managed",
            Some(r"D:\rust\active-projects\elon cli"),
            None,
            None,
        )
        .expect("default icon");
        assert!(icon.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn configured_display_name_overrides_bb64a_default() {
        let mut project = ProjectSummary {
            id: "prj-test".to_string(),
            name: "bb64a".to_string(),
            display_name: Some("自定义加速器".to_string()),
            description: None,
            workspace_key: "prj-test".to_string(),
            template: "local".to_string(),
            source_type: "pc_managed".to_string(),
            repo_url: None,
            branch: None,
            workspace_path: Some(r"D:\rust\active-projects\bb64a".to_string()),
            node_id: None,
            storage_node_id: None,
            storage_repo_path: None,
            storage_repo_url: None,
            storage_worktree_path: None,
            storage_status: "none".to_string(),
            status: "active".to_string(),
            role: "owner".to_string(),
            member_count: 1,
            is_public: false,
            join_mode: "invite".to_string(),
            runtime_permission: "project_write".to_string(),
            last_task_status: None,
            last_apk_url: None,
            icon_data_url: None,
            updated_at: "now".to_string(),
        };
        apply_project_summary_branding(&mut project);
        assert_eq!(project.display_name.as_deref(), Some("自定义加速器"));
    }

    fn public_project(name: &str, icon_data_url: Option<String>) -> PublicProjectItem {
        PublicProjectItem {
            id: format!("prj-{name}"),
            name: name.to_string(),
            display_name: None,
            description: None,
            template: "android".to_string(),
            owner_account: "owner".to_string(),
            owner_id: "owner-id".to_string(),
            member_count: 1,
            is_public: true,
            join_mode: "approval".to_string(),
            viewer_role: None,
            last_task_status: None,
            latest_apk_url: None,
            icon_data_url,
            created_at: "now".to_string(),
            updated_at: "now".to_string(),
        }
    }

    use super::{
        pc_cli_chat_requested, pc_cli_chat_route_label, project_chat_should_use_pc_cli,
        project_cli_runtime_permission, project_cli_runtime_permission_fallback,
        project_fields_require_pc_workspace, route_a_full_access_grant_error,
        should_attempt_pc_apk_sync,
    };
    use crate::pc_agent_runtime_choice::PcRuntimeRoutePreference;
    use crate::store::ProjectAccess;

    #[test]
    fn explicit_pc_cli_routes_are_cli_first_for_chat() {
        assert!(pc_cli_chat_requested(Some(
            PcRuntimeRoutePreference::RouteA
        )));
        assert!(pc_cli_chat_requested(Some(
            PcRuntimeRoutePreference::RouteC3
        )));
        assert!(!pc_cli_chat_requested(Some(
            PcRuntimeRoutePreference::RouteC
        )));
        assert!(!pc_cli_chat_requested(None));

        assert!(!project_chat_should_use_pc_cli(None, None, false));
        assert!(project_chat_should_use_pc_cli(
            Some(PcRuntimeRoutePreference::RouteA),
            None,
            false
        ));
        assert!(project_chat_should_use_pc_cli(None, Some("codex"), true));
        assert!(!project_chat_should_use_pc_cli(
            Some(PcRuntimeRoutePreference::RouteC),
            None,
            false
        ));
        assert!(!project_chat_should_use_pc_cli(None, Some("api"), false));
    }

    #[test]
    fn pc_cli_chat_labels_match_user_selected_route() {
        assert!(pc_cli_chat_route_label(Some(PcRuntimeRoutePreference::RouteA)).contains("AI"));
        assert!(pc_cli_chat_route_label(Some(PcRuntimeRoutePreference::RouteC3)).contains("Codex"));
    }

    #[test]
    fn android_template_pc_project_attempts_apk_sync_for_ui_changes() {
        let project = pc_project("prj_android", "android_kotlin", Some("node-local"));

        assert!(should_attempt_pc_apk_sync(&project, "新增一个绿色按钮"));
    }

    #[test]
    fn pc_managed_development_request_attempts_apk_sync_without_apk_keyword() {
        let mut project = pc_project("prj_pc", "blank", Some("node-local"));
        project.template = "custom".into();

        assert!(should_attempt_pc_apk_sync(
            &project,
            "现在新增一个按钮 金色 叫 角色"
        ));
    }

    #[test]
    fn pc_managed_casual_chat_does_not_force_apk_sync() {
        let project = pc_project("prj_pc", "blank", Some("node-local"));

        assert!(!should_attempt_pc_apk_sync(&project, "你好，先聊一下想法"));
    }

    #[test]
    fn pc_managed_projects_use_full_access_for_cli() {
        let project = pc_project("prj_pc", "android_kotlin", Some("node-local"));

        assert_eq!(project_cli_runtime_permission(&project), "full_access");
    }

    #[test]
    fn route_a_full_access_grant_errors_can_fallback_to_project_write() {
        let message = "PC CLI 执行失败: Route A 完全访问尚未在本机授权：请在 PC 工作台设置中重新选择该项目目录并确认完全访问。";

        assert!(route_a_full_access_grant_error(message));
        assert_eq!(
            project_cli_runtime_permission_fallback("full_access", message),
            Some("project_write")
        );
        assert_eq!(
            project_cli_runtime_permission_fallback("project_write", message),
            None
        );
    }

    #[test]
    fn pc_managed_projects_require_pc_workspace_route() {
        assert!(project_fields_require_pc_workspace(
            "pc_managed",
            None,
            Some("/srv/elon/project")
        ));
    }

    #[test]
    fn bound_node_projects_require_pc_workspace_route() {
        assert!(project_fields_require_pc_workspace(
            "local_path",
            Some("node-local"),
            Some("/srv/elon/project")
        ));
    }

    #[test]
    fn windows_local_paths_require_pc_workspace_route() {
        assert!(project_fields_require_pc_workspace(
            "local_path",
            None,
            Some(r"D:\rust\active-projects\elon cli")
        ));
        assert!(project_fields_require_pc_workspace(
            "local_path",
            None,
            Some("D:/rust/active-projects/elon cli")
        ));
    }

    #[test]
    fn unc_paths_require_pc_workspace_route() {
        assert!(project_fields_require_pc_workspace(
            "local_path",
            None,
            Some(r"\\workstation\repos\elon")
        ));
        assert!(project_fields_require_pc_workspace(
            "local_path",
            None,
            Some("//workstation/repos/elon")
        ));
    }

    #[test]
    fn server_local_paths_can_still_use_server_git_route() {
        assert!(!project_fields_require_pc_workspace(
            "local_path",
            None,
            Some("/srv/elon/project")
        ));
    }

    fn pc_project(id: &str, template: &str, node_id: Option<&str>) -> ProjectAccess {
        ProjectAccess {
            id: id.into(),
            name: "PC App".into(),
            workspace_key: id.into(),
            template: template.into(),
            source_type: "pc_managed".into(),
            repo_url: None,
            branch: None,
            workspace_path: Some(format!(
                r"C:\Users\Administrator\Elon\workspaces\usr\{id}\repo"
            )),
            node_id: node_id.map(str::to_string),
            storage_node_id: None,
            storage_repo_path: None,
            storage_repo_url: None,
            storage_worktree_path: None,
            storage_status: "none".into(),
            role: "owner".into(),
            status: "active".into(),
            runtime_permission: "project_write".into(),
        }
    }

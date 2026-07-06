    use super::*;
    use uuid::Uuid;

    fn temp_workspace(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "elon_full_access_{label}_{}",
            Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&path).expect("create temp workspace");
        path
    }

    fn grant_file(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "elon_full_access_grants_{label}_{}.json",
            Uuid::new_v4().simple()
        ))
    }

    #[test]
    fn platform_managed_workspace_allows_project_repo_and_conversation_worktree() {
        let root = temp_workspace("managed_root");
        let project_id = "prj_abc123";
        let project_part = safe_path_part(project_id, "project", 80);
        let repo = root.join("usr_1").join(&project_part).join("repo");
        let worktree = root
            .join("conversation-worktrees")
            .join(&project_part)
            .join("conv_1");
        std::fs::create_dir_all(&repo).expect("create managed repo");
        std::fs::create_dir_all(&worktree).expect("create managed worktree");

        assert!(platform_managed_workspace_matches_under(
            project_id,
            repo.to_string_lossy().as_ref(),
            &root
        ));
        assert!(platform_managed_workspace_matches_under(
            project_id,
            worktree.to_string_lossy().as_ref(),
            &root
        ));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn platform_managed_workspace_rejects_other_project_paths() {
        let root = temp_workspace("managed_mismatch");
        let repo = root.join("usr_1").join("prj_other").join("repo");
        std::fs::create_dir_all(&repo).expect("create other repo");

        assert!(!platform_managed_workspace_matches_under(
            "prj_expected",
            repo.to_string_lossy().as_ref(),
            &root
        ));

        let _ = std::fs::remove_dir_all(root);
    }

    fn context(project_id: &str) -> CliProjectContext {
        CliProjectContext {
            project_id: project_id.to_string(),
            conversation_id: "conv".to_string(),
            runtime_permission: Some("full_access".to_string()),
        }
    }

    #[tokio::test]
    async fn grant_and_require_full_access_for_same_project_path() {
        let workspace = temp_workspace("ok");
        let state = FullAccessGrantState::load_from_path(grant_file("ok"));
        state
            .grant_project("project_1", workspace.to_string_lossy().as_ref())
            .await
            .expect("grant project");

        require_route_a_full_access_grant(
            &state,
            "codex",
            Some("full_access"),
            Some(&context("project_1")),
            Some(workspace.to_string_lossy().as_ref()),
        )
        .await
        .expect("grant should authorize matching project path");
    }

    #[tokio::test]
    async fn route_a_full_access_requires_local_grant() {
        let workspace = temp_workspace("missing");
        let state = FullAccessGrantState::load_from_path(grant_file("missing"));
        let error = require_route_a_full_access_grant(
            &state,
            "codex",
            Some("full_access"),
            Some(&context("project_1")),
            Some(workspace.to_string_lossy().as_ref()),
        )
        .await
        .expect_err("missing grant should reject full access");

        assert!(
            error.to_string().contains("完全访问尚未在本机授权"),
            "unexpected error: {error}"
        );
    }

    #[tokio::test]
    async fn project_write_and_builtin_runtime_do_not_need_full_access_grant() {
        let workspace = temp_workspace("bypass");
        let state = FullAccessGrantState::load_from_path(grant_file("bypass"));

        require_route_a_full_access_grant(
            &state,
            "codex",
            Some("project_write"),
            Some(&context("project_1")),
            Some(workspace.to_string_lossy().as_ref()),
        )
        .await
        .expect("project_write route A should not require full-access grant");

        require_route_a_full_access_grant(
            &state,
            "api-runtime",
            Some("full_access"),
            Some(&context("project_1")),
            Some(workspace.to_string_lossy().as_ref()),
        )
        .await
        .expect("built-in runtime keeps its own sandbox guard");
    }

    #[test]
    fn runtime_policy_summary_exposes_route_bc_safety_limits() {
        let summary = runtime_policy_summary();

        assert_eq!(summary["schema"], "elon.pc_node.runtime_policy.v1");
        assert_eq!(summary["fullAccess"]["routeAInstalledCliOnly"], true);
        assert_eq!(
            summary["fullAccess"]["routeBCFullAccessEffect"],
            "keeps_workspace_path_checks_command_allowlist_and_tool_approvals"
        );
        assert_eq!(
            summary["fullAccess"]["routeBCDangerFullAccessEffect"],
            "danger_full_access_allows_absolute_paths_arbitrary_shell_and_skips_tool_approvals"
        );
        assert_eq!(
            summary["operatorVisibility"]["policyField"],
            "runtime_policy"
        );

        let approval_tools = summary["routeBC"]["approvalRequiredTools"]
            .as_array()
            .expect("approvalRequiredTools should be an array");
        for tool in ["write_file", "apply_patch", "run_command"] {
            assert!(
                approval_tools
                    .iter()
                    .any(|item| item.as_str() == Some(tool)),
                "missing approval tool {tool}"
            );
        }

        let denied = summary["routeBC"]["highRiskGitPushDenied"]
            .as_array()
            .expect("highRiskGitPushDenied should be an array");
        for arg in ["--force*", "--delete", "--mirror", "+refspec", ":branch"] {
            assert!(
                denied.iter().any(|item| item.as_str() == Some(arg)),
                "missing high-risk git push marker {arg}"
            );
        }
    }

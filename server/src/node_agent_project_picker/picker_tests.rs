#[cfg(test)]
mod tests {
    use super::super::{
        detect_project_identity, inspect_agent_runtime_freshness, local_project_info,
        local_project_registration_readiness, AgentRuntimeFreshness, LocalProjectInfo,
    };
    use homecli_proto::ProjectWorkspaceInspectStatus;
    use serde_json::json;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_project(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "elon-project-picker-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn inspect_status() -> ProjectWorkspaceInspectStatus {
        ProjectWorkspaceInspectStatus {
            workspace_path: "C:\\demo".to_string(),
            path_exists: true,
            is_dir: true,
            is_git_worktree: true,
            git_branch: Some("main".to_string()),
            git_head: Some("abc1234".to_string()),
            git_remote_origin: Some("https://example.com/demo.git".to_string()),
            has_uncommitted_changes: false,
            uncommitted_count: Some(0),
            disk_free_bytes: Some(1024 * 1024 * 1024),
            codex_available: true,
            copilot_available: false,
        }
    }

    fn local_project() -> LocalProjectInfo {
        LocalProjectInfo {
            name: "demo".to_string(),
            workspace_path: "C:\\demo".to_string(),
            description: Some("绑定到本 PC 节点的本地项目: demo".to_string()),
            identity_source: Some("目录名".to_string()),
            repo_url: Some("https://example.com/demo.git".to_string()),
            branch: Some("main".to_string()),
            git_head: Some("abc1234".to_string()),
            is_git_worktree: true,
            has_uncommitted_changes: false,
            uncommitted_count: Some(0),
            project_type: Some("Rust".to_string()),
            package_manager: Some("Cargo".to_string()),
            run_command: Some("cargo run".to_string()),
            test_command: Some("cargo test".to_string()),
            build_command: Some("cargo build".to_string()),
            detected_files: vec!["Cargo.toml".to_string()],
            agent_runtime: current_agent_runtime(),
        }
    }

    fn current_agent_runtime() -> AgentRuntimeFreshness {
        AgentRuntimeFreshness {
            status: "current".to_string(),
            summary: "项目内便携一龙入口已包含命令预算和输出截断保护，默认每轮最多 8 个 run_command；可作为离线或无一龙客户端时的高级入口。"
                .to_string(),
            script_path: "C:\\demo\\scripts\\elon-agent.ps1".to_string(),
            runtime_scope: "project_portable_cli_entry",
            registration_required: false,
            has_elon_agent: true,
            has_command_budget: true,
            has_output_limit: true,
            max_run_commands_default: Some(8),
        }
    }

    #[test]
    fn registration_readiness_reports_ready_project_autofill() {
        let project = local_project();
        let inspect = inspect_status();

        let readiness = local_project_registration_readiness(&project, &inspect);

        assert!(readiness.can_register);
        assert_eq!(readiness.status, "ready");
        assert!(readiness.missing_fields.is_empty());
        assert!(readiness.warnings.is_empty());
        assert!(readiness.autofill_fields.contains(&"Git 远端".to_string()));
        assert!(readiness.autofill_fields.contains(&"构建命令".to_string()));
        assert!(readiness
            .autofill_fields
            .contains(&"便携一龙入口".to_string()));
        assert_eq!(readiness.next_action.kind, "auto_register");
        assert_eq!(readiness.register_payload.name, "demo");
        assert_eq!(
            readiness.register_payload.repo_url.as_deref(),
            Some("https://example.com/demo.git")
        );
        assert_eq!(readiness.register_payload.branch.as_deref(), Some("main"));
        assert_eq!(
            readiness
                .register_payload
                .dev_profile
                .as_ref()
                .and_then(|profile| profile.build_command.as_deref()),
            Some("cargo build")
        );
    }

    #[test]
    fn registration_readiness_stays_ready_when_only_portable_agent_entry_is_missing() {
        let mut project = local_project();
        project.agent_runtime = AgentRuntimeFreshness {
            status: "missing".to_string(),
            summary: "未生成项目内便携一龙入口 scripts\\elon-agent.ps1；不影响 Win 端节点内置开发能力，仅影响离线或无一龙客户端时在项目目录直接运行一龙 agent。"
                .to_string(),
            script_path: "C:\\demo\\scripts\\elon-agent.ps1".to_string(),
            runtime_scope: "project_portable_cli_entry",
            registration_required: false,
            has_elon_agent: false,
            has_command_budget: false,
            has_output_limit: false,
            max_run_commands_default: None,
        };
        let inspect = inspect_status();

        let readiness = local_project_registration_readiness(&project, &inspect);

        assert!(readiness.can_register);
        assert_eq!(readiness.status, "ready");
        assert!(readiness.warnings.is_empty());
        assert!(!readiness
            .autofill_fields
            .contains(&"便携一龙入口".to_string()));
        assert_eq!(readiness.next_action.kind, "auto_register");
    }

    #[test]
    fn registration_readiness_warns_about_gitless_unknown_project() {
        let mut project = local_project();
        project.repo_url = None;
        project.branch = None;
        project.is_git_worktree = false;
        project.project_type = None;
        project.package_manager = None;
        project.run_command = None;
        project.test_command = None;
        project.build_command = None;
        project.detected_files.clear();
        project.agent_runtime = AgentRuntimeFreshness {
            status: "missing".to_string(),
            summary: "未生成项目内便携一龙入口 scripts\\elon-agent.ps1；不影响 Win 端节点内置开发能力，仅影响离线或无一龙客户端时在项目目录直接运行一龙 agent。"
                .to_string(),
            script_path: "C:\\demo\\scripts\\elon-agent.ps1".to_string(),
            runtime_scope: "project_portable_cli_entry",
            registration_required: false,
            has_elon_agent: false,
            has_command_budget: false,
            has_output_limit: false,
            max_run_commands_default: None,
        };

        let mut inspect = inspect_status();
        inspect.is_git_worktree = false;
        inspect.git_remote_origin = None;
        inspect.git_branch = None;
        inspect.codex_available = false;
        inspect.copilot_available = false;

        let readiness = local_project_registration_readiness(&project, &inspect);

        assert!(readiness.can_register);
        assert_eq!(readiness.status, "needs_review");
        assert!(readiness
            .warnings
            .iter()
            .any(|warning| warning.contains("未检测到 Git 工作区")));
        assert!(readiness
            .warnings
            .iter()
            .any(|warning| warning.contains("未检测到 Codex/Copilot")));
        assert!(!readiness
            .warnings
            .iter()
            .any(|warning| warning.contains("elon-agent.ps1")));
        assert!(!readiness.autofill_fields.contains(&"Git 远端".to_string()));
        assert_eq!(readiness.next_action.kind, "review_then_register");
        assert!(readiness.next_action.detail.contains("未检测到 Git 工作区"));
        assert!(readiness.register_payload.repo_url.is_none());
        assert!(readiness.register_payload.dev_profile.is_none());
    }

    #[test]
    fn registration_readiness_blocks_missing_required_payload_fields() {
        let mut project = local_project();
        project.name = " ".to_string();
        project.workspace_path = " ".to_string();
        let inspect = inspect_status();

        let readiness = local_project_registration_readiness(&project, &inspect);

        assert!(!readiness.can_register);
        assert_eq!(readiness.status, "blocked");
        assert_eq!(readiness.next_action.kind, "complete_missing_fields");
        assert!(readiness.next_action.detail.contains("项目目录"));
        assert!(readiness.next_action.detail.contains("项目名称"));
    }

    #[test]
    fn agent_runtime_freshness_detects_missing_stale_and_current_templates() {
        let dir = temp_project("agent-runtime-freshness");

        let missing = inspect_agent_runtime_freshness(&dir);
        assert_eq!(missing.status, "missing");
        assert_eq!(missing.runtime_scope, "project_portable_cli_entry");
        assert!(!missing.registration_required);
        assert!(missing.summary.contains("不影响 Win 端节点内置开发能力"));
        assert!(!missing.has_elon_agent);

        let scripts = dir.join("scripts");
        std::fs::create_dir_all(&scripts).unwrap();
        std::fs::write(
            scripts.join("elon-agent.ps1"),
            "function Invoke-AgentAction {}\n",
        )
        .unwrap();
        let stale = inspect_agent_runtime_freshness(&dir);
        assert_eq!(stale.status, "stale");
        assert!(!stale.registration_required);
        assert!(stale.summary.contains("不影响 Win 端节点内置开发能力"));
        assert!(stale.has_elon_agent);
        assert!(!stale.has_command_budget);
        assert!(!stale.has_output_limit);

        std::fs::write(
            scripts.join("elon-agent.ps1"),
            "[int]$MaxRunCommands = 8\n$AgentCommandOutputMaxChars = 12000\nfunction Use-AgentRunCommandBudget {}\nfunction Limit-AgentText {}\n",
        )
        .unwrap();
        let current = inspect_agent_runtime_freshness(&dir);
        assert_eq!(current.status, "current");
        assert!(!current.registration_required);
        assert_eq!(current.max_run_commands_default, Some(8));
        assert!(current.has_command_budget);
        assert!(current.has_output_limit);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn detects_project_identity_from_landing_manifest() {
        let landing = json!({
            "title": "智能客服工作台",
            "tagline": "给运营团队使用的客服项目"
        });

        let identity =
            detect_project_identity(PathBuf::from("C:\\demo").as_path(), Some(&landing), None);

        assert_eq!(identity.name, "智能客服工作台");
        assert_eq!(
            identity.description.as_deref(),
            Some("给运营团队使用的客服项目")
        );
        assert_eq!(
            identity.source.as_deref(),
            Some(".elon/project-landing.json")
        );
    }

    #[test]
    fn detects_project_identity_from_package_json() {
        let dir = temp_project("identity-node");
        std::fs::write(
            dir.join("package.json"),
            r#"{"name":"agent-desk","description":"本地 AI 工作台"}"#,
        )
        .unwrap();

        let identity = detect_project_identity(&dir, None, None);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "agent-desk");
        assert_eq!(identity.description.as_deref(), Some("本地 AI 工作台"));
        assert_eq!(identity.source.as_deref(), Some("package.json"));
    }

    #[test]
    fn detects_project_identity_from_cargo_manifest() {
        let dir = temp_project("identity-rust");
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"repair-agent\"\ndescription = 'Windows 维修代理'\n",
        )
        .unwrap();

        let identity = detect_project_identity(&dir, None, None);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "repair-agent");
        assert_eq!(identity.description.as_deref(), Some("Windows 维修代理"));
        assert_eq!(identity.source.as_deref(), Some("Cargo.toml"));
    }

    #[test]
    fn detects_project_identity_from_go_mod() {
        let dir = temp_project("identity-go");
        std::fs::write(
            dir.join("go.mod"),
            "module github.com/example/pc-node-runtime/v2\n\ngo 1.22\n",
        )
        .unwrap();

        let identity = detect_project_identity(&dir, None, None);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "pc-node-runtime");
        assert_eq!(
            identity.description.as_deref(),
            Some("绑定到本 PC 节点的本地项目: pc-node-runtime")
        );
        assert_eq!(identity.source.as_deref(), Some("go.mod"));
    }

    #[test]
    fn detects_project_identity_from_shallow_module_manifest() {
        let dir = temp_project("identity-shallow-module");
        std::fs::create_dir_all(dir.join("web")).unwrap();
        std::fs::write(
            dir.join("web").join("package.json"),
            r#"{"name":"desktop-workbench","description":"PC 端项目工作台"}"#,
        )
        .unwrap();
        std::fs::write(dir.join("README.md"), "# 根目录 README\n\n通用仓库说明").unwrap();

        let identity = detect_project_identity(&dir, None, None);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "desktop-workbench");
        assert_eq!(identity.description.as_deref(), Some("PC 端项目工作台"));
        assert_eq!(identity.source.as_deref(), Some("web/package.json"));
    }

    #[test]
    fn detects_project_identity_from_readme_heading_and_intro() {
        let dir = temp_project("identity-readme");
        std::fs::write(
            dir.join("README.md"),
            "# 网络诊断助手\n\n![badge](https://example.com/badge.svg)\n\n帮助用户自动检查代理、DNS 和网卡配置。\n第二句会合并进项目描述。\n\n## 安装\n",
        )
        .unwrap();

        let identity = detect_project_identity(&dir, None, None);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "网络诊断助手");
        assert_eq!(
            identity.description.as_deref(),
            Some("帮助用户自动检查代理、DNS 和网卡配置。 第二句会合并进项目描述。")
        );
        assert_eq!(identity.source.as_deref(), Some("README.md"));
    }

    #[test]
    fn structured_manifest_identity_takes_precedence_over_readme() {
        let dir = temp_project("identity-priority");
        std::fs::write(
            dir.join("package.json"),
            r#"{"name":"package-name","description":"manifest desc"}"#,
        )
        .unwrap();
        std::fs::write(dir.join("README.md"), "# README 名称\n\nREADME 描述").unwrap();

        let identity = detect_project_identity(&dir, None, None);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "package-name");
        assert_eq!(identity.description.as_deref(), Some("manifest desc"));
        assert_eq!(identity.source.as_deref(), Some("package.json"));
    }

    #[test]
    fn detects_project_identity_from_git_remote_when_no_manifest_or_readme() {
        let dir = temp_project("identity-git-remote");

        let identity = detect_project_identity(
            &dir,
            None,
            Some("https://github.com/example/acme-desktop-agent.git"),
        );
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "acme-desktop-agent");
        assert_eq!(
            identity.description.as_deref(),
            Some("绑定到本 PC 节点的本地项目: acme-desktop-agent")
        );
        assert_eq!(identity.source.as_deref(), Some("Git 远端"));
    }

    #[test]
    fn detects_project_identity_from_ssh_git_remote() {
        let dir = temp_project("identity-ssh-git-remote");

        let identity = detect_project_identity(
            &dir,
            None,
            Some("git@github.com:example/win-client-runtime.git"),
        );
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "win-client-runtime");
        assert_eq!(identity.source.as_deref(), Some("Git 远端"));
    }

    #[test]
    fn readme_identity_takes_precedence_over_git_remote() {
        let dir = temp_project("identity-readme-before-git");
        std::fs::write(dir.join("README.md"), "# README 名称\n\nREADME 描述").unwrap();

        let identity = detect_project_identity(
            &dir,
            None,
            Some("https://github.com/example/git-remote-name.git"),
        );
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(identity.name, "README 名称");
        assert_eq!(identity.description.as_deref(), Some("README 描述"));
        assert_eq!(identity.source.as_deref(), Some("README.md"));
    }

    #[test]
    fn local_project_info_uses_landing_identity() {
        let dir = temp_project("landing-info");
        std::fs::create_dir_all(dir.join(".elon")).unwrap();
        std::fs::write(
            dir.join(".elon").join("project-landing.json"),
            r#"{"title":"项目元信息名称","summary":"项目元信息描述"}"#,
        )
        .unwrap();
        let landing = crate::project_landing::load_workspace_landing(&dir);

        let (project, _) = local_project_info(dir.to_string_lossy().as_ref(), landing.as_ref())
            .expect("local project should inspect");
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(project.name, "项目元信息名称");
        assert_eq!(project.description.as_deref(), Some("项目元信息描述"));
        assert_eq!(
            project.identity_source.as_deref(),
            Some(".elon/project-landing.json")
        );
    }
}

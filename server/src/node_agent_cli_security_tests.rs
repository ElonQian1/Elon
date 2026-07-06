    use super::{
        codex_session_scope_key, prepare_cli_base_cwd, resolve_cli_request,
        validate_cli_extra_args, ResolvedCli,
    };

    #[test]
    fn rejects_unknown_cli_even_when_cloud_sends_executable_name() {
        assert!(resolve_cli_request("powershell", &[]).is_err());
        assert!(resolve_cli_request("C:\\Windows\\System32\\cmd.exe", &[]).is_err());
    }

    #[test]
    fn built_in_runtime_does_not_need_local_binary_path() {
        assert_eq!(
            resolve_cli_request("api-runtime", &[]).unwrap(),
            ResolvedCli::BuiltIn {
                name: "api-runtime"
            }
        );
    }

    #[test]
    fn cli_prompt_requires_absolute_cwd_when_provided() {
        assert!(prepare_cli_base_cwd(Some("relative".to_string()), None).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn cli_prompt_cwd_strips_windows_verbatim_prefix() {
        let user_profile = std::env::var("USERPROFILE").expect("USERPROFILE should exist");
        let (cwd, _) = prepare_cli_base_cwd(Some(user_profile), None).unwrap();
        assert!(
            !cwd.to_string_lossy().starts_with(r"\\?\"),
            "cmd.exe cannot start from verbatim cwd {}",
            cwd.display()
        );
    }

    #[test]
    fn extra_args_reject_privilege_escalation_flags() {
        assert!(validate_cli_extra_args("copilot", &["--allow-all".to_string()]).is_err());
        assert!(validate_cli_extra_args(
            "codex",
            &["--dangerously-bypass-approvals-and-sandbox".to_string()]
        )
        .is_err());
        assert!(validate_cli_extra_args(
            "codex",
            &["--sandbox".to_string(), "danger-full-access".to_string()]
        )
        .is_err());
    }

    #[test]
    fn extra_args_allow_expected_model_and_session_flags() {
        assert!(validate_cli_extra_args(
            "codex",
            &[
                "--codex-model=gpt-5.4".to_string(),
                "--codex-effort=medium".to_string(),
                "--session-id=abc-123".to_string()
            ]
        )
        .is_ok());
        assert!(validate_cli_extra_args(
            "copilot",
            &[
                "--session-id=abc-123".to_string(),
                "--model".to_string(),
                "gpt-5.4".to_string()
            ]
        )
        .is_ok());
    }

    #[test]
    fn codex_session_key_includes_permission_and_cwd_scope() {
        let args = vec!["--session-id=thread-1".to_string()];
        let project = codex_session_scope_key(&args, Some("project_write"), Some("C:/repo"));
        let full = codex_session_scope_key(&args, Some("full_access"), Some("C:/repo"));
        assert_ne!(project, full);
        assert!(full.unwrap().contains("perm=full_access"));
    }

    #[cfg(windows)]
    #[test]
    fn batch_wrapper_quotes_cli_shim_path() {
        let (program, args) =
            super::windows_batch_wrapper(r"C:\Users\me\AppData\Roaming\npm\codex.cmd").unwrap();

        assert_eq!(program, "cmd");
        assert_eq!(
            args,
            vec![
                "/D".to_string(),
                "/S".to_string(),
                "/C".to_string(),
                r"C:\Users\me\AppData\Roaming\npm\codex.cmd".to_string()
            ]
        );
    }

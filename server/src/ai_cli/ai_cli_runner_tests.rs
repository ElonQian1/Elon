    use super::*;
    use crate::types::CliPromptMode;

    fn option(id: &str, provider: &str, args: &[&str]) -> AiCliOption {
        AiCliOption {
            id: id.to_string(),
            label: id.to_string(),
            provider: provider.to_string(),
            model: None,
            reasoning_effort: None,
            reasoning_summary: None,
            verbosity: None,
            bin: provider.to_string(),
            args: args.iter().map(|arg| arg.to_string()).collect(),
            prompt_mode: CliPromptMode::Arg,
            timeout_secs: 60,
        }
    }

    #[test]
    fn codex_project_write_keeps_workspace_sandbox() {
        let option = option(
            "codex_cli",
            "codex",
            &[
                "exec",
                "--sandbox",
                "workspace-write",
                "--skip-git-repo-check",
            ],
        );
        let args = cli_args_for_run(&option, None, Some("project_write"));
        assert_eq!(
            args,
            vec![
                "exec",
                "--json",
                "--sandbox",
                "workspace-write",
                "--skip-git-repo-check"
            ]
        );
    }

    #[test]
    fn codex_full_access_replaces_workspace_sandbox() {
        let option = option(
            "codex_cli",
            "codex",
            &[
                "exec",
                "--sandbox",
                "workspace-write",
                "--skip-git-repo-check",
            ],
        );
        let args = cli_args_for_run(&option, None, Some("full_access"));
        assert_eq!(
            args,
            vec![
                "exec",
                "--json",
                "--dangerously-bypass-approvals-and-sandbox",
                "--skip-git-repo-check"
            ]
        );
    }

    #[test]
    fn codex_project_write_resume_keeps_workspace_sandbox() {
        let option = option(
            "codex_cli",
            "codex",
            &[
                "exec",
                "--sandbox",
                "workspace-write",
                "--skip-git-repo-check",
            ],
        );
        let args = cli_args_for_run(&option, Some("thread-1"), Some("project_write"));
        assert_eq!(
            args,
            vec![
                "exec",
                "resume",
                "--sandbox",
                "workspace-write",
                "--skip-git-repo-check",
                "--json",
                "thread-1"
            ]
        );
    }

    #[test]
    fn copilot_full_access_adds_allow_all() {
        let option = option("copilot_cli", "copilot", &["--model", "gpt-5"]);
        let args = cli_args_for_run(&option, Some("conv-1"), Some("full_access"));
        assert_eq!(args, vec!["--continue", "--allow-all", "--model", "gpt-5"]);
    }

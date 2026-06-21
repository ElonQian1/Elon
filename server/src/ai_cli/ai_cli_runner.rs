//! Codex CLI 命令行参数构造、子进程超时清理、thread URI 拼接（从 ai_cli.rs 抽出）。

#[cfg(unix)]
use std::time::Duration;
#[cfg(unix)]
use tokio::process::Command;

use super::ai_cli_process::{supports_codex_sessions, supports_copilot_sessions};
use crate::types::AiCliOption;

pub(crate) fn cli_args_for_run(
    option: &AiCliOption,
    native_session_id: Option<&str>,
    runtime_permission: Option<&str>,
) -> Vec<String> {
    let full_access = crate::store::project_runtime_permission_allows_full_access(
        runtime_permission.unwrap_or_default(),
    );
    if supports_codex_sessions(option) {
        let raw_args = if full_access {
            codex_full_access_raw_args(&option.args)
        } else {
            option.args.clone()
        };
        if let Some(session_id) = native_session_id {
            if let Some(args) = codex_resume_args(&raw_args, session_id) {
                return args;
            }
        }
        return codex_exec_json_args(&raw_args);
    }
    if supports_copilot_sessions(option) {
        return copilot_session_args(&option.args, native_session_id, full_access);
    }
    option.args.clone()
}

/// 构造 CopilotCLI 的会话参数。
///
/// - 有 `native_session_id`（包含 sentinel `conv-*`）→ 在 args 前插入 `--continue`
///   （CopilotCLI 会续接当前工作目录的最近一次会话，与我们的 per-conversation worktree 完全匹配）
/// - 首次运行（`native_session_id` 为 None）→ 直接使用原始 args，行为不变
pub(crate) fn copilot_session_args(
    raw_args: &[String],
    native_session_id: Option<&str>,
    full_access: bool,
) -> Vec<String> {
    let mut args = Vec::new();
    if native_session_id.is_some() {
        // --continue 会在当前工作目录续接最近的 CopilotCLI 会话
        args.push("--continue".into());
    }
    if full_access && !raw_args.iter().any(|arg| arg == "--allow-all") {
        args.push("--allow-all".into());
    }
    args.extend_from_slice(raw_args);
    args
}

/// CopilotCLI 首次成功运行后写入 agent_native_sessions 的 sentinel 格式。
///
/// 不包含真实的 session UUID（CopilotCLI 不在 stdout 输出 session ID），
/// 仅作为"此 conversation 已有历史 session"的标记，让下次请求插入 `--continue`。
pub(crate) fn copilot_session_sentinel(conversation_id: &str) -> String {
    format!("conv-{}", conversation_id)
}

pub(crate) fn codex_exec_json_args(raw_args: &[String]) -> Vec<String> {
    let mut args = raw_args.to_vec();
    if args.iter().any(|arg| arg == "--json") {
        return args;
    }
    if let Some(exec_index) = args.iter().position(|arg| arg == "exec" || arg == "e") {
        args.insert(exec_index + 1, "--json".into());
    }
    args
}

pub(crate) fn codex_full_access_raw_args(raw_args: &[String]) -> Vec<String> {
    let mut args = Vec::with_capacity(raw_args.len() + 1);
    let mut i = 0;
    while i < raw_args.len() {
        match raw_args[i].as_str() {
            "--sandbox" | "-s" => {
                i += 2;
                continue;
            }
            arg if arg.starts_with("--sandbox=") => {
                i += 1;
                continue;
            }
            "--dangerously-bypass-approvals-and-sandbox" => {
                i += 1;
                continue;
            }
            _ => {
                args.push(raw_args[i].clone());
                i += 1;
            }
        }
    }

    let insert_at = args
        .iter()
        .position(|arg| arg == "exec" || arg == "e")
        .map(|index| index + 1)
        .unwrap_or(args.len());
    args.insert(
        insert_at,
        "--dangerously-bypass-approvals-and-sandbox".into(),
    );
    args
}

pub(crate) fn codex_resume_args(raw_args: &[String], session_id: &str) -> Option<Vec<String>> {
    let exec_index = raw_args
        .iter()
        .position(|arg| arg == "exec" || arg == "e")?;
    let mut args = raw_args[..exec_index].to_vec();
    args.push("exec".into());
    args.push("resume".into());

    let mut has_json = false;
    let mut iter = raw_args[exec_index + 1..].iter().peekable();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--json" => {
                has_json = true;
                args.push(arg.clone());
            }
            "--skip-git-repo-check"
            | "--ignore-user-config"
            | "--ignore-rules"
            | "--strict-config"
            | "--dangerously-bypass-approvals-and-sandbox"
            | "--dangerously-bypass-hook-trust" => args.push(arg.clone()),
            arg if arg.starts_with("--sandbox=") => args.push(arg.to_string()),
            "-m" | "--model" | "-c" | "--config" | "-p" | "--profile" | "--profile-v2"
            | "--output-schema" | "-s" | "--sandbox" => {
                args.push(arg.clone());
                if let Some(value) = iter.next() {
                    args.push(value.clone());
                }
            }
            _ => {}
        }
    }
    if !has_json {
        args.push("--json".into());
    }
    args.push(session_id.to_string());
    Some(args)
}

pub(crate) async fn kill_timed_out_child(child: &mut tokio::process::Child) {
    #[cfg(unix)]
    {
        if let Some(pid) = child.id() {
            let process_group = format!("-{}", pid);
            let _ = Command::new("kill")
                .args(["-TERM", &process_group])
                .status()
                .await;
            tokio::time::sleep(Duration::from_millis(250)).await;
            let _ = Command::new("kill")
                .args(["-KILL", &process_group])
                .status()
                .await;
        }
    }
    let _ = child.kill().await;
}

pub(crate) fn codex_thread_uri(session_id: &str) -> String {
    let session_id = session_id.trim();
    if session_id.starts_with("codex://threads/") {
        session_id.to_string()
    } else {
        format!("codex://threads/{session_id}")
    }
}

#[cfg(test)]
mod tests {
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
}

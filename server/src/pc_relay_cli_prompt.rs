use anyhow::{anyhow, Result};
use homecli_proto::{AgentToServer, CliWorkspaceStatus};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};

use crate::ws_client_transport::try_send_json;

pub(crate) async fn handle_cli_prompt(
    req_id: String,
    cli: String,
    extra_args: Vec<String>,
    cwd: Option<String>,
    project_context: Option<homecli_proto::CliProjectContext>,
    prompt: String,
    out: mpsc::UnboundedSender<Message>,
) {
    info!(
        "[relay-client] CliPrompt: cli={} cwd={} req_id={}",
        cli,
        cwd.as_deref().unwrap_or("<default>"),
        req_id
    );
    let requested_runtime_permission = project_context
        .as_ref()
        .and_then(|ctx| ctx.runtime_permission.clone());
    let _ = send_agent_event(
        &out,
        &AgentToServer::CliPromptAccepted {
            req_id: req_id.clone(),
            cli: Some(cli.clone()),
            cwd: cwd.clone(),
            runtime_permission: requested_runtime_permission,
        },
    );

    let resolved_cli = match resolve_relay_cli(&cli) {
        Ok(resolved) => resolved,
        Err(e) => {
            warn!("[relay-client] 拒绝 CLI 请求: {e:#}");
            send_cli_done_error(out, req_id, e.to_string());
            return;
        }
    };
    if let Err(e) =
        crate::node_agent_cli_security::validate_cli_extra_args(resolved_cli.name(), &extra_args)
    {
        warn!("[relay-client] 拒绝 CLI 参数: {e:#}");
        send_cli_done_error(out, req_id, e.to_string());
        return;
    }
    let prepared_cwd = match prepare_cli_cwd(cwd, project_context) {
        Ok(cwd) => cwd,
        Err(e) => {
            warn!("[relay-client] 准备 CLI 工作目录失败: {e:#}");
            send_cli_done_error(out, req_id, e.to_string());
            return;
        }
    };
    let (exit_ok, error) = match run_cli_and_stream(
        &req_id,
        resolved_cli.name(),
        resolved_cli.bin(),
        &extra_args,
        prepared_cwd.cwd.as_deref(),
        &prompt,
        &out,
    )
    .await
    {
        Ok(ok) => (ok, None),
        Err(e) => {
            warn!("[relay-client] CLI 执行失败: {e:#}");
            (false, Some(e.to_string()))
        }
    };
    let (exit_ok, error, workspace_status) =
        finalize_cli_workspace(exit_ok, error, prepared_cwd.conversation_workspace);
    let _ = send_agent_event(
        &out,
        &AgentToServer::CliDone {
            req_id,
            exit_ok,
            error,
            session_id: None,
            prompt_tokens: None,
            cached_input_tokens: None,
            completion_tokens: None,
            reasoning_tokens: None,
            total_tokens: None,
            model: None,
            workspace_status,
        },
    );
}

fn send_cli_done_error(out: mpsc::UnboundedSender<Message>, req_id: String, error: String) {
    let _ = send_agent_event(
        &out,
        &AgentToServer::CliDone {
            req_id,
            exit_ok: false,
            error: Some(error),
            session_id: None,
            prompt_tokens: None,
            cached_input_tokens: None,
            completion_tokens: None,
            reasoning_tokens: None,
            total_tokens: None,
            model: None,
            workspace_status: None,
        },
    );
}

struct PreparedCliCwd {
    cwd: Option<String>,
    conversation_workspace: Option<crate::pc_workspace_provisioner::ConversationWorkspaceResult>,
}

fn prepare_cli_cwd(
    cwd: Option<String>,
    project_context: Option<homecli_proto::CliProjectContext>,
) -> Result<PreparedCliCwd> {
    let (base_cwd, context) =
        crate::node_agent_cli_security::prepare_cli_base_cwd(cwd, project_context)?;
    if relay_cli_read_only(context.runtime_permission.as_deref()) {
        return Ok(PreparedCliCwd {
            cwd: Some(base_cwd.to_string_lossy().to_string()),
            conversation_workspace: None,
        });
    }
    let workspace = crate::pc_workspace_provisioner::prepare_conversation_workspace(
        base_cwd.to_string_lossy().as_ref(),
        &context.project_id,
        &context.conversation_id,
    )?;
    if workspace.isolated {
        info!(
            "[relay-client] project={} conversation={} 使用会话 worktree: {}",
            context.project_id, context.conversation_id, workspace.workspace_path
        );
    }
    Ok(PreparedCliCwd {
        cwd: Some(workspace.workspace_path.clone()),
        conversation_workspace: Some(workspace),
    })
}

fn finalize_cli_workspace(
    exit_ok: bool,
    error: Option<String>,
    workspace: Option<crate::pc_workspace_provisioner::ConversationWorkspaceResult>,
) -> (bool, Option<String>, Option<CliWorkspaceStatus>) {
    let Some(workspace) = workspace else {
        return (exit_ok, error, None);
    };
    if !exit_ok {
        return (
            exit_ok,
            error.clone(),
            Some(workspace_status(&workspace, "skipped", error.as_deref())),
        );
    }
    match crate::pc_workspace_provisioner::merge_conversation_workspace(&workspace) {
        Ok(message)
            if message.starts_with("conversation worktree still")
                || message.starts_with("conversation worktree missing git metadata")
                || message.starts_with("base workspace") =>
        {
            warn!("[relay-client] 会话 worktree 暂未合并: {message}");
            if conversation_workspace_head_landed(&workspace) {
                let cleanup_status = cleanup_failure_status(&message);
                return (
                    true,
                    None,
                    Some(workspace_status(&workspace, cleanup_status, Some(&message))),
                );
            }
            (
                false,
                Some(message.clone()),
                Some(workspace_status(&workspace, "blocked", Some(&message))),
            )
        }
        Ok(message) => {
            info!("[relay-client] 会话 worktree 合并结果: {message}");
            let merge_status = if workspace.isolated {
                "merged"
            } else {
                "shared"
            };
            (
                true,
                None,
                Some(workspace_status(&workspace, merge_status, Some(&message))),
            )
        }
        Err(e) => {
            warn!("[relay-client] 会话 worktree 合并失败: {e:#}");
            let message = format!("会话 worktree 合并失败: {e}");
            if conversation_workspace_head_landed(&workspace) {
                let cleanup_status = cleanup_failure_status(&message);
                return (
                    true,
                    None,
                    Some(workspace_status(&workspace, cleanup_status, Some(&message))),
                );
            }
            (
                false,
                Some(message.clone()),
                Some(workspace_status(&workspace, "failed", Some(&message))),
            )
        }
    }
}

fn conversation_workspace_head_landed(
    workspace: &crate::pc_workspace_provisioner::ConversationWorkspaceResult,
) -> bool {
    crate::pc_workspace_provisioner::conversation_workspace_head_landed(workspace).unwrap_or_else(
        |probe_error| {
            warn!("[relay-client] 检查会话 worktree 是否已进入远端主线失败: {probe_error:#}");
            false
        },
    )
}

fn cleanup_failure_status(message: &str) -> &'static str {
    let lower = message.to_ascii_lowercase();
    if lower.contains("non-fast-forward")
        || lower.contains("ff-only")
        || lower.contains("fast-forward")
        || lower.contains("diverg")
        || lower.contains("fetch first")
    {
        "local_main_diverged"
    } else {
        "cleanup_failed"
    }
}

fn workspace_status(
    workspace: &crate::pc_workspace_provisioner::ConversationWorkspaceResult,
    merge_status: &str,
    merge_message: Option<&str>,
) -> CliWorkspaceStatus {
    CliWorkspaceStatus {
        base_workspace_path: workspace.base_workspace_path.clone(),
        active_workspace_path: workspace.workspace_path.clone(),
        isolated: workspace.isolated,
        branch: workspace.branch.clone(),
        git_head: crate::pc_workspace_provisioner::conversation_workspace_git_head(workspace),
        prepare_status: "prepared".into(),
        merge_status: Some(merge_status.into()),
        merge_message: merge_message.map(ToOwned::to_owned),
    }
}

fn relay_cli_read_only(runtime_permission: Option<&str>) -> bool {
    !matches!(
        runtime_permission,
        Some("project_write" | "full_access" | "danger_full_access")
    )
}

fn resolve_relay_cli(cli: &str) -> Result<crate::node_agent_cli_security::ResolvedCli> {
    let paths = detect_relay_cli_paths();
    let resolved = crate::node_agent_cli_security::resolve_cli_request(cli, &paths)?;
    if matches!(
        resolved,
        crate::node_agent_cli_security::ResolvedCli::BuiltIn { .. }
    ) {
        return Err(anyhow!(
            "legacy relay 模式不支持内置 runtime，请使用一龙 PC 节点客户端。"
        ));
    }
    Ok(resolved)
}

fn detect_relay_cli_paths() -> Vec<(String, String)> {
    ["copilot", "codex", "claude", "gemini"]
        .iter()
        .filter_map(|name| {
            let path = elon_pc_dev_runtime::command_candidates(name)
                .into_iter()
                .next()?;
            Some((name.to_string(), path.to_string_lossy().to_string()))
        })
        .collect()
}

async fn run_cli_and_stream(
    req_id: &str,
    cli_name: &str,
    program: &str,
    extra_args: &[String],
    cwd: Option<&str>,
    prompt: &str,
    out: &mpsc::UnboundedSender<Message>,
) -> Result<bool> {
    use tokio::io::AsyncBufReadExt;
    use tokio::process::Command;

    info!(
        "[relay-client] cli={} 使用可执行路径: {}",
        cli_name, program
    );
    let batch_wrapper = crate::node_agent_cli_security::windows_batch_wrapper(program);
    let mut cmd = if let Some((actual_program, args)) = batch_wrapper.as_ref() {
        let mut c = Command::new(actual_program);
        c.args(args);
        c
    } else {
        Command::new(program)
    };
    for arg in extra_args {
        cmd.arg(arg);
    }
    if let Some(cwd) = cwd.filter(|value| !value.trim().is_empty()) {
        cmd.current_dir(cwd);
    }
    cmd.arg("-p").arg(prompt);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::null());
    cmd.kill_on_drop(true);
    hide_relay_command_window(&mut cmd);

    let mut child = cmd
        .spawn()
        .map_err(|e| anyhow!("启动 {} 失败: {e}", program))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("无法获取 stdout"))?;

    let req_id_s = req_id.to_string();
    let out_clone = out.clone();
    let stream_task = tokio::spawn(async move {
        let mut reader = tokio::io::BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let chunk = AgentToServer::CliChunk {
                req_id: req_id_s.clone(),
                text: format!("{}\n", line),
            };
            if send_agent_event(&out_clone, &chunk).is_err() {
                break;
            }
        }
    });

    let status = child.wait().await?;
    let _ = stream_task.await;
    Ok(status.success())
}

fn send_agent_event(out: &mpsc::UnboundedSender<Message>, event: &AgentToServer) -> Result<()> {
    try_send_json(out, event)
}

fn hide_relay_command_window(_command: &mut tokio::process::Command) {
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        _command.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
    }
}

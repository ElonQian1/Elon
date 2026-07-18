// server/src/node_agent_cli_prompt_direct.rs
//! 直接子进程 CLI 路径

use crate::node_agent_cli_done::{
    cli_done_message_from_output, latest_codex_session_id, persist_and_send_cli_done,
    CliCompletionContext,
};
use crate::node_agent_cli_prompt_runner::{cli_done_error, cli_prompt_timeout_secs, ws_text};
use crate::node_agent_cli_runner::*;
use crate::node_agent_codex_session::CodexSessionPlan;
use crate::node_agent_runtime::NodeRuntime;
use crate::pc_workspace_provisioner;
use crate::{node_agent_codex_auth_switch, node_agent_codex_session};
use homecli_proto::AgentToServer;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};

pub(crate) struct CliDirectRunContext {
    pub cmd: tokio::process::Command,
    pub cli_name_owned: String,
    pub bin_owned: String,
    pub req_id: String,
    pub codex_sessions_file: PathBuf,
    pub codex_plan: CodexSessionPlan,
    pub codex_last_message_path: Option<PathBuf>,
    pub codex_key: Option<String>,
    pub extra_args: Vec<String>,
    pub runtime_permission: Option<String>,
    pub conversation_workspace: Option<pc_workspace_provisioner::ConversationWorkspaceResult>,
    pub codex_vault_switch_attempted: bool,
    pub runtime: Arc<NodeRuntime>,
    pub out_tx: tokio::sync::mpsc::UnboundedSender<Message>,
    pub cancel_rx: watch::Receiver<bool>,
    pub task_journal: crate::node_agent_task_journal::TaskJournal,
    pub cwd: Option<String>,
    pub prompt: String,
    pub server_runtime_config: Option<crate::node_agent_server_runtime::ServerRuntimeConfig>,
    pub approval_state: crate::node_agent_tool_approval::ToolApprovalState,
    pub completion_context: CliCompletionContext,
    pub frozen_codex_home: Option<crate::node_agent_codex_child_env::FrozenCodexHome>,
}

pub(crate) async fn run_cli_direct_process(ctx: CliDirectRunContext) {
    use tokio::io::AsyncBufReadExt;
    let CliDirectRunContext {
        mut cmd,
        cli_name_owned,
        bin_owned,
        req_id,
        codex_sessions_file,
        codex_plan,
        codex_last_message_path,
        codex_key,
        extra_args,
        runtime_permission,
        conversation_workspace,
        codex_vault_switch_attempted,
        runtime,
        out_tx,
        mut cancel_rx,
        task_journal,
        cwd,
        prompt,
        server_runtime_config,
        approval_state,
        completion_context,
        frozen_codex_home,
    } = ctx;
    let cli_name = cli_name_owned.as_str();
    let bin = bin_owned.as_str();
    let timeout_secs = cli_prompt_timeout_secs(
        cli_name,
        runtime_permission.as_deref(),
        completion_context.is_desktop_supervised(),
    );

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            let message = format!("无法启动 {} : {}", bin, e);
            let done = AgentToServer::CliDone {
                req_id,
                exit_ok: false,
                error: Some(message),
                session_id: latest_codex_session_id(cli_name, &codex_plan, &task_journal),
                prompt_tokens: None,
                cached_input_tokens: None,
                completion_tokens: None,
                reasoning_tokens: None,
                total_tokens: None,
                model: None,
                workspace_status: None,
            };
            if let Err(error) = persist_and_send_cli_done(
                &runtime,
                &completion_context,
                cli_name,
                None,
                done,
                &out_tx,
            ) {
                warn!(%error, "failed to persist CLI spawn completion");
            }
            return;
        }
    };
    if let Some(pid) = child.id() {
        runtime.set_cli_prompt_os_pid(&req_id, Some(pid)).await;
        if let Err(error) = task_journal.record_process_started(&req_id, pid) {
            warn!("PC 任务 journal 写入进程 pid 失败: {error}");
        }
    }

    let stdout = child.stdout.take().expect("stdout");
    let stderr = child.stderr.take().expect("stderr");
    let mut stdout_lines = tokio::io::BufReader::new(stdout).lines();
    let (stderr_tx, mut stderr_rx) = tokio::sync::mpsc::unbounded_channel::<Option<String>>();
    {
        let stderr_tx = stderr_tx.clone();
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let mut reader = tokio::io::BufReader::new(stderr);
            let mut buf = Vec::new();
            loop {
                buf.clear();
                match reader.read_until(b'\n', &mut buf).await {
                    Ok(0) | Err(_) => {
                        let _ = stderr_tx.send(None);
                        break;
                    }
                    Ok(_) => {
                        while matches!(buf.last(), Some(&b'\n') | Some(&b'\r')) {
                            buf.pop();
                        }
                        let _ = stderr_tx.send(Some(String::from_utf8_lossy(&buf).into_owned()));
                    }
                }
            }
        });
    }
    let mut stdout_text = String::new();
    let mut stderr_text = String::new();
    let mut stdout_done = false;
    let mut stderr_done = false;

    while !stdout_done || !stderr_done {
        tokio::select! {
            line = stdout_lines.next_line(), if !stdout_done => match line {
                Ok(Some(l)) => {
                    stdout_text.push_str(&l);
                    stdout_text.push('\n');
                    if cli_name == "codex" {
                        if let Some(real_id) =
                            node_agent_codex_session::extract_session_id_from_text(&l)
                        {
                            if let Some(ref key) = codex_key {
                                node_agent_codex_session::persist_session_compat(
                                    &task_journal,
                                    Some(&codex_sessions_file),
                                    &req_id,
                                    key,
                                    &real_id,
                                );
                            }
                            continue;
                        }
                    }
                    send_cli_chunk(&out_tx, &task_journal, &req_id, "stdout", &(l + "\n"));
                }
                Ok(None) => { stdout_done = true; }
                Err(e) => {
                    let message = format!("stdout 读取错误: {e}");
                    warn!("{message}");
                    stdout_text.push_str(&message);
                    stdout_text.push('\n');
                    stdout_done = true;
                }
            },
            opt = stderr_rx.recv(), if !stderr_done => match opt {
                Some(Some(l)) => {
                    stderr_text.push_str(&l);
                    stderr_text.push('\n');
                    if cli_name == "codex" {
                        if let Some(real_id) =
                            node_agent_codex_session::extract_session_id_from_text(&l)
                        {
                            if let Some(ref key) = codex_key {
                                node_agent_codex_session::persist_session_compat(
                                    &task_journal,
                                    Some(&codex_sessions_file),
                                    &req_id,
                                    key,
                                    &real_id,
                                );
                            }
                            continue;
                        }
                        if !l.trim().is_empty() {
                            info!("[codex stderr] {}", l);
                            let _ = task_journal.record_cli_chunk(&req_id, "stderr", &(l + "\n"));
                        }
                    } else {
                        send_cli_chunk(&out_tx, &task_journal, &req_id, "stderr", &(l + "\n"));
                    }
                }
                Some(None) | None => { stderr_done = true; }
            },
            changed = cancel_rx.changed() => {
                if changed.is_ok() && *cancel_rx.borrow() {
                    warn!("[{}] CLI 收到取消请求，强杀进程", cli_name);
                    let _ = child.kill().await;
                    drain_killed_child_output(
                        &mut stdout_lines,
                        &mut stderr_rx,
                        &mut stdout_done,
                        &mut stderr_done,
                        &mut stdout_text,
                        &mut stderr_text,
                    ).await;
                    let message = "用户已停止 PC CLI 任务".to_string();
                    let (exit_ok, error, workspace_status) =
                        finalize_cli_prompt_workspace(false, Some(message), conversation_workspace);
                    let (done, combined_output) = cli_done_message_from_output(
                        req_id,
                        exit_ok,
                        error,
                        &stdout_text,
                        &stderr_text,
                        cli_model_from_args(cli_name, &extra_args),
                        workspace_status,
                        latest_codex_session_id(cli_name, &codex_plan, &task_journal),
                    );
                    if let Err(error) = persist_and_send_cli_done(
                        &runtime,
                        &completion_context,
                        cli_name,
                        Some(&combined_output),
                        done,
                        &out_tx,
                    ) {
                        warn!(%error, "failed to persist canceled CLI completion");
                    }
                    return;
                }
            },
            _ = tokio::time::sleep(std::time::Duration::from_secs(timeout_secs)) => {
                warn!("[{}] CLI 执行超时，强杀进程", cli_name);
                let _ = child.kill().await;
                drain_killed_child_output(
                    &mut stdout_lines,
                    &mut stderr_rx,
                    &mut stdout_done,
                    &mut stderr_done,
                    &mut stdout_text,
                    &mut stderr_text,
                ).await;
                let message = format!("{} 执行超时（超过{}秒），已强制终止",
                    cli_name, timeout_secs);
                let (done, combined_output) = cli_done_message_from_output(
                    req_id,
                    false,
                    Some(message),
                    &stdout_text,
                    &stderr_text,
                    cli_model_from_args(cli_name, &extra_args),
                    None,
                    latest_codex_session_id(cli_name, &codex_plan, &task_journal),
                );
                if let Err(error) = persist_and_send_cli_done(
                    &runtime,
                    &completion_context,
                    cli_name,
                    Some(&combined_output),
                    done,
                    &out_tx,
                ) {
                    warn!(%error, "failed to persist timeout CLI completion");
                }
                return;
            },
        }
    }

    let exit_ok = child.wait().await.map(|s| s.success()).unwrap_or(false);
    if cli_name == "codex" && !contains_codex_reply_marker(&stdout_text) {
        if let Some(text) = codex_last_message_chunk(codex_last_message_path.as_ref()) {
            send_cli_chunk(&out_tx, &task_journal, &req_id, "stdout", &text);
            stdout_text.push_str(&text);
        }
    }
    if !exit_ok && cli_name == "codex" {
        if let Some(fallback_args) =
            crate::node_agent_codex_model_compat::compatibility_fallback_args(
                &extra_args,
                &stdout_text,
                &stderr_text,
            )
        {
            send_cli_chunk(
                &out_tx,
                &task_journal,
                &req_id,
                "stdout",
                "codex\n当前模型需要更高版本 Codex，已自动切换到兼容模型 gpt-5.4 并继续本轮任务。\n",
            );
            Box::pin(crate::run_cli_prompt(crate::CliPromptRun {
                req_id,
                bin: bin_owned,
                cli_name: cli_name_owned,
                extra_args: fallback_args,
                runtime_permission,
                cwd,
                conversation_workspace,
                prompt,
                server_runtime_config,
                approval_state,
                task_journal,
                runtime,
                cancel_rx,
                out_tx,
                codex_vault_switch_attempted,
                completion_context,
                frozen_codex_home,
            }))
            .await;
            return;
        }
    }
    if !exit_ok && cli_name == "codex" && !codex_vault_switch_attempted {
        if let Some(auth_switch) = node_agent_codex_auth_switch::try_after_failure(
            &runtime,
            &req_id,
            &stdout_text,
            &stderr_text,
        )
        .await
        {
            send_cli_chunk(
                &out_tx,
                &task_journal,
                &req_id,
                "stdout",
                &format!("codex\n{}\n", auth_switch.message),
            );
            Box::pin(crate::run_cli_prompt(crate::CliPromptRun {
                req_id,
                bin: bin_owned,
                cli_name: cli_name_owned,
                extra_args,
                runtime_permission,
                cwd,
                conversation_workspace,
                prompt,
                server_runtime_config,
                approval_state,
                task_journal,
                runtime,
                cancel_rx,
                out_tx,
                codex_vault_switch_attempted: true,
                completion_context,
                frozen_codex_home: Some(auth_switch.frozen_codex_home),
            }))
            .await;
            return;
        }
    }
    if !exit_ok
        && cli_name == "codex"
        && codex_plan.is_resume()
        && node_agent_codex_session::stale_resume_failure(&stdout_text, &stderr_text)
    {
        if let Some(scope_key) = codex_plan.scope_key.as_deref() {
            node_agent_codex_session::clear_stale_session(
                &task_journal,
                &codex_sessions_file,
                &req_id,
                scope_key,
            )
            .await;
        }
        send_cli_chunk(
            &out_tx,
            &task_journal,
            &req_id,
            "stdout",
            "codex\n已发现本机 Codex session 失效，正在清理旧 session 并自动重新开始本轮任务。\n",
        );
        Box::pin(crate::run_cli_prompt(crate::CliPromptRun {
            req_id,
            bin: bin_owned,
            cli_name: cli_name_owned,
            extra_args,
            runtime_permission,
            cwd,
            conversation_workspace,
            prompt,
            server_runtime_config,
            approval_state,
            task_journal,
            runtime,
            cancel_rx,
            out_tx,
            codex_vault_switch_attempted,
            completion_context,
            frozen_codex_home,
        }))
        .await;
        return;
    }
    if exit_ok && cli_name == "codex" && !contains_codex_reply_marker(&stdout_text) {
        let diagnostic = if stdout_text.trim().is_empty() {
            "Codex CLI 执行完成，但没有返回可解析输出。请查看 PC 节点日志确认是否已完成文件修改。"
        } else {
            "Codex CLI 执行完成，但输出里没有可解析的 codex 回复段。请查看 PC 节点日志确认是否已完成文件修改。"
        };
        let _ = out_tx.send(ws_text(&AgentToServer::CliChunk {
            req_id: req_id.clone(),
            text: format!("codex\n{diagnostic}\n"),
        }));
        let _ = task_journal.record_cli_chunk(&req_id, "stdout", &format!("codex\n{diagnostic}\n"));
    }
    let error = if exit_ok {
        None
    } else {
        Some(cli_done_error(cli_name, &stdout_text, &stderr_text))
    };
    let (exit_ok, error, workspace_status) =
        finalize_cli_prompt_workspace(exit_ok, error, conversation_workspace);
    let (done, combined_output) = cli_done_message_from_output(
        req_id,
        exit_ok,
        error,
        &stdout_text,
        &stderr_text,
        cli_model_from_args(cli_name, &extra_args),
        workspace_status,
        latest_codex_session_id(cli_name, &codex_plan, &task_journal),
    );
    if let Err(error) = persist_and_send_cli_done(
        &runtime,
        &completion_context,
        cli_name,
        Some(&combined_output),
        done,
        &out_tx,
    ) {
        warn!(%error, "failed to persist direct CLI completion");
    }
}

/// After `kill().await` the process has exited, but pipe-reader tasks may still
/// hold buffered terminal records. Give both streams a short bounded drain so a
/// token-count event emitted before cancellation/timeout reaches the durable
/// completion instead of being mistaken for unknown/zero usage.
async fn drain_killed_child_output(
    stdout_lines: &mut tokio::io::Lines<tokio::io::BufReader<tokio::process::ChildStdout>>,
    stderr_rx: &mut tokio::sync::mpsc::UnboundedReceiver<Option<String>>,
    stdout_done: &mut bool,
    stderr_done: &mut bool,
    stdout_text: &mut String,
    stderr_text: &mut String,
) {
    let drain = async {
        while !*stdout_done || !*stderr_done {
            tokio::select! {
                line = stdout_lines.next_line(), if !*stdout_done => match line {
                    Ok(Some(line)) => {
                        stdout_text.push_str(&line);
                        stdout_text.push('\n');
                    }
                    Ok(None) | Err(_) => *stdout_done = true,
                },
                line = stderr_rx.recv(), if !*stderr_done => match line {
                    Some(Some(line)) => {
                        stderr_text.push_str(&line);
                        stderr_text.push('\n');
                    }
                    Some(None) | None => *stderr_done = true,
                },
            }
        }
    };
    let _ = tokio::time::timeout(std::time::Duration::from_secs(1), drain).await;
}

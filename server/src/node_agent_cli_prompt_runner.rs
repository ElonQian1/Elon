// server/src/node_agent_cli_prompt_runner.rs
//! CLI prompt 运行器：启动 CLI 子进程、流式输出、session 管理、Codex vault 切换。
//! 从 node_agent_main.rs 抽取，保持原有逻辑不变。

use homecli_proto::AgentToServer;
use std::sync::Arc;
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::Message;
use tracing::warn;

use crate::node_agent_cli_done::{persist_and_send_cli_done, CliCompletionContext};
use crate::node_agent_cli_env::apply_env;
use crate::node_agent_cli_prompt_direct::{run_cli_direct_process, CliDirectRunContext};
use crate::node_agent_cli_prompt_sidecar::{run_cli_sidecar_or_fallback, CliSidecarPromptContext};
use crate::node_agent_cli_runner::*;
use crate::node_agent_cli_security;
use crate::node_agent_cli_sidecar_runner;
use crate::node_agent_codex_child_env;
use crate::node_agent_codex_session;
use crate::node_agent_exec::hide_tokio_command_window;
use crate::node_agent_runtime::NodeRuntime;
use crate::node_agent_task_journal;
use crate::node_agent_tool_approval;
use crate::pc_workspace_provisioner;
pub(crate) fn ws_text(msg: &AgentToServer) -> Message {
    Message::Text(serde_json::to_string(msg).unwrap_or_default())
}

fn truncate_cli_text(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let tail: String = trimmed
        .chars()
        .rev()
        .take(max_chars)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("...{}", tail)
}

pub(crate) fn cli_done_error(cli_name: &str, stdout_text: &str, stderr_text: &str) -> String {
    let mut parts = vec![format!("{cli_name} 进程退出失败")];
    let stderr = truncate_cli_text(stderr_text, 2000);
    if !stderr.is_empty() {
        parts.push(format!("stderr:\n{stderr}"));
    }
    let stdout = truncate_cli_text(stdout_text, 1200);
    if !stdout.is_empty() {
        parts.push(format!("stdout:\n{stdout}"));
    }
    parts.join("\n\n")
}

/// 将附件 URL 下载到本地临时文件，并转换成对应 CLI 参数。
pub(crate) async fn resolve_attachment_args(
    args: Vec<String>,
    cli_name: &str,
    user_token: Option<&str>,
) -> Vec<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_default();
    let mut result = Vec::with_capacity(args.len());
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--attachment" {
            if let Some(url) = args.get(i + 1) {
                if url.starts_with("http://") || url.starts_with("https://") {
                    let ext = url
                        .rsplit('.')
                        .next()
                        .filter(|e| e.len() <= 5 && e.chars().all(|c| c.is_alphanumeric()))
                        .unwrap_or("jpg");
                    let tmp_path = std::env::temp_dir().join(format!(
                        "elon_attach_{}.{}",
                        uuid::Uuid::new_v4(),
                        ext
                    ));
                    let mut req = client.get(url.as_str());
                    if let Some(tok) = user_token {
                        req = req.bearer_auth(tok);
                    }
                    match req.send().await {
                        Ok(resp) if resp.status().is_success() => {
                            if let Ok(bytes) = resp.bytes().await {
                                if tokio::fs::write(&tmp_path, &bytes).await.is_ok() {
                                    let local = tmp_path.to_string_lossy().to_string();
                                    if cli_name == "codex" {
                                        // Codex 用 -i 传图片
                                        result.push("-i".to_string());
                                        result.push(local);
                                    } else {
                                        // Copilot 用 --attachment
                                        result.push("--attachment".to_string());
                                        result.push(local);
                                    }
                                    i += 2;
                                    continue;
                                }
                            }
                        }
                        Ok(resp) => {
                            warn!("📎 attachment download failed: status={}", resp.status());
                        }
                        Err(e) => {
                            warn!("📎 attachment download error: {}", e);
                        }
                    }
                    i += 2;
                    continue;
                }
            }
        }
        result.push(args[i].clone());
        i += 1;
    }
    result
}

fn cli_prompt_full_access(runtime_permission: Option<&str>) -> bool {
    matches!(
        runtime_permission.map(str::trim),
        Some("full_access" | "danger_full_access")
    )
}

pub(crate) fn cli_prompt_read_only(runtime_permission: Option<&str>) -> bool {
    !matches!(
        runtime_permission.map(str::trim),
        Some("project_write" | "full_access" | "danger_full_access")
    )
}

pub(crate) fn cli_prompt_timeout_secs(cli_name: &str, runtime_permission: Option<&str>) -> u64 {
    match cli_name.trim().to_ascii_lowercase().as_str() {
        "codex" if cli_prompt_full_access(runtime_permission) => 1200,
        "codex" => 300,
        _ => 180,
    }
}

pub(crate) struct CliPromptRun {
    pub(crate) req_id: String,
    pub(crate) bin: String,
    pub(crate) cli_name: String,
    pub(crate) extra_args: Vec<String>,
    pub(crate) runtime_permission: Option<String>,
    pub(crate) cwd: Option<String>,
    pub(crate) conversation_workspace:
        Option<pc_workspace_provisioner::ConversationWorkspaceResult>,
    pub(crate) prompt: String,
    pub(crate) server_runtime_config: Option<crate::node_agent_server_runtime::ServerRuntimeConfig>,
    pub(crate) approval_state: node_agent_tool_approval::ToolApprovalState,
    pub(crate) task_journal: node_agent_task_journal::TaskJournal,
    pub(crate) runtime: Arc<NodeRuntime>,
    pub(crate) cancel_rx: watch::Receiver<bool>,
    pub(crate) out_tx: tokio::sync::mpsc::UnboundedSender<Message>,
    pub(crate) codex_vault_switch_attempted: bool,
    pub(crate) completion_context: CliCompletionContext,
    pub(crate) frozen_codex_home: Option<node_agent_codex_child_env::FrozenCodexHome>,
}

pub(crate) async fn run_cli_prompt(run: CliPromptRun) {
    let CliPromptRun {
        req_id,
        bin,
        cli_name,
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
    } = run;
    let bin_owned = bin;
    let cli_name_owned = cli_name;
    let bin = bin_owned.as_str();
    let cli_name = cli_name_owned.as_str();
    if let Err(error) =
        node_agent_cli_security::validate_cli_extra_args(cli_name, extra_args.as_slice())
    {
        let message = error.to_string();
        let done = AgentToServer::CliDone {
            req_id,
            exit_ok: false,
            error: Some(message),
            session_id: None,
            prompt_tokens: None,
            cached_input_tokens: None,
            completion_tokens: None,
            reasoning_tokens: None,
            total_tokens: None,
            model: None,
            workspace_status: None,
        };
        if let Err(error) =
            persist_and_send_cli_done(&runtime, &completion_context, cli_name, None, done, &out_tx)
        {
            warn!(%error, "failed to persist invalid-arguments CLI completion");
        }
        return;
    }
    if cli_name == "api-runtime" {
        let result = crate::node_agent_server_runtime::run_api_runtime_prompt(
            crate::node_agent_server_runtime::RuntimePromptOptions {
                req_id: &req_id,
                cwd: cwd.as_deref(),
                runtime_permission: runtime_permission.as_deref(),
                prompt: &prompt,
                approval_state: Some(approval_state.clone()),
                cancel_rx,
                out_tx: out_tx.clone(),
                task_journal: Some(task_journal.clone()),
            },
        )
        .await;
        let (exit_ok, error, workspace_status) =
            finalize_cli_prompt_workspace(result.exit_ok, result.error, conversation_workspace);
        let done = AgentToServer::CliDone {
            req_id,
            exit_ok,
            error,
            session_id: None,
            prompt_tokens: result.prompt_tokens,
            cached_input_tokens: None,
            completion_tokens: result.completion_tokens,
            reasoning_tokens: None,
            total_tokens: result.total_tokens,
            model: result.model,
            workspace_status,
        };
        if let Err(error) =
            persist_and_send_cli_done(&runtime, &completion_context, cli_name, None, done, &out_tx)
        {
            warn!(%error, "failed to persist api-runtime completion");
        }
        return;
    }
    if cli_name == "server-runtime" {
        let result = match server_runtime_config {
            Some(config) => {
                crate::node_agent_server_runtime::run_server_runtime_prompt(
                    config,
                    crate::node_agent_server_runtime::RuntimePromptOptions {
                        req_id: &req_id,
                        cwd: cwd.as_deref(),
                        runtime_permission: runtime_permission.as_deref(),
                        prompt: &prompt,
                        approval_state: Some(approval_state.clone()),
                        cancel_rx,
                        out_tx: out_tx.clone(),
                        task_journal: Some(task_journal.clone()),
                    },
                )
                .await
            }
            None => crate::node_agent_server_runtime::ServerRuntimeRunResult {
                exit_ok: false,
                error: Some("server-runtime 缺少节点登录上下文".to_string()),
                model: Some("server-runtime".to_string()),
                prompt_tokens: None,
                completion_tokens: None,
                total_tokens: None,
            },
        };
        let (exit_ok, error, workspace_status) =
            finalize_cli_prompt_workspace(result.exit_ok, result.error, conversation_workspace);
        let done = AgentToServer::CliDone {
            req_id,
            exit_ok,
            error,
            session_id: None,
            prompt_tokens: result.prompt_tokens,
            cached_input_tokens: None,
            completion_tokens: result.completion_tokens,
            reasoning_tokens: None,
            total_tokens: result.total_tokens,
            model: result.model,
            workspace_status,
        };
        if let Err(error) =
            persist_and_send_cli_done(&runtime, &completion_context, cli_name, None, done, &out_tx)
        {
            warn!(%error, "failed to persist server-runtime completion");
        }
        return;
    }
    let batch_wrapper = node_agent_cli_security::windows_batch_wrapper(bin);
    let actual_bin = batch_wrapper
        .as_ref()
        .map(|(program, _)| *program)
        .unwrap_or(bin);
    let full_access = cli_prompt_full_access(runtime_permission.as_deref());
    let codex_sessions_file = std::env::temp_dir().join("elon_codex_sessions.json");
    let codex_scope_key = if cli_name == "codex" {
        node_agent_cli_security::codex_session_scope_key(
            &extra_args,
            runtime_permission.as_deref(),
            cwd.as_deref(),
        )
    } else {
        None
    };
    let codex_plan = if cli_name == "codex" {
        node_agent_codex_session::load_session_plan(
            &task_journal,
            &codex_sessions_file,
            codex_scope_key.clone(),
        )
    } else {
        node_agent_codex_session::CodexSessionPlan {
            scope_key: None,
            session_id: None,
        }
    };
    let mut cmd = tokio::process::Command::new(actual_bin);
    let mut sidecar_args = Vec::new();
    let mut sidecar_env = Vec::new();
    let codex_last_message_path = if cli_name == "codex" {
        let safe_req_id: String = req_id
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                    ch
                } else {
                    '_'
                }
            })
            .collect();
        let path = std::env::temp_dir().join(format!("elon_codex_last_message_{safe_req_id}.txt"));
        let _ = std::fs::remove_file(&path);
        Some(path)
    } else {
        None
    };
    if let Some((_, args)) = batch_wrapper.as_ref() {
        cmd.args(args);
        sidecar_args.extend(args.iter().map(|arg| arg.to_string()));
    }
    if cli_name == "codex" {
        if let Some(args) = crate::node_agent_cli_mcp::codex_mcp_config_args_for_runtime(
            &prompt,
            cwd.as_deref(),
            runtime.as_ref(),
        )
        .await
        {
            for arg in args {
                push_tracked_arg(&mut cmd, &mut sidecar_args, arg);
            }
        }
        for a in &extra_args {
            if let Some(model) = a.strip_prefix("--codex-model=") {
                push_tracked_arg(&mut cmd, &mut sidecar_args, "-m");
                push_tracked_arg(&mut cmd, &mut sidecar_args, model);
            } else if let Some(effort) = a.strip_prefix("--codex-effort=") {
                let effort =
                    crate::node_agent_codex_effort::normalize_codex_reasoning_effort(effort);
                push_tracked_arg(&mut cmd, &mut sidecar_args, "-c");
                push_tracked_arg(
                    &mut cmd,
                    &mut sidecar_args,
                    format!("model_reasoning_effort=\"{}\"", effort),
                );
            }
        }
        push_tracked_arg(&mut cmd, &mut sidecar_args, "exec");
        if let Some(ref real_sid) = codex_plan.session_id {
            push_tracked_arg(&mut cmd, &mut sidecar_args, "resume");
            push_tracked_arg(&mut cmd, &mut sidecar_args, "--json");
            if let Some(path) = codex_last_message_path.as_ref() {
                push_tracked_arg(&mut cmd, &mut sidecar_args, "--output-last-message");
                push_tracked_arg(
                    &mut cmd,
                    &mut sidecar_args,
                    path.to_string_lossy().to_string(),
                );
            }
            if full_access {
                push_tracked_arg(
                    &mut cmd,
                    &mut sidecar_args,
                    "--dangerously-bypass-approvals-and-sandbox",
                );
            } else {
                push_tracked_arg(&mut cmd, &mut sidecar_args, "--skip-git-repo-check");
            }
            push_tracked_arg(&mut cmd, &mut sidecar_args, real_sid);
        } else {
            push_tracked_arg(&mut cmd, &mut sidecar_args, "--json");
            if let Some(path) = codex_last_message_path.as_ref() {
                push_tracked_arg(&mut cmd, &mut sidecar_args, "--output-last-message");
                push_tracked_arg(
                    &mut cmd,
                    &mut sidecar_args,
                    path.to_string_lossy().to_string(),
                );
            }
            if full_access {
                push_tracked_arg(
                    &mut cmd,
                    &mut sidecar_args,
                    "--dangerously-bypass-approvals-and-sandbox",
                );
            } else if cli_prompt_read_only(runtime_permission.as_deref()) {
                push_tracked_arg(&mut cmd, &mut sidecar_args, "--sandbox");
                push_tracked_arg(&mut cmd, &mut sidecar_args, "read-only");
                push_tracked_arg(&mut cmd, &mut sidecar_args, "--skip-git-repo-check");
            } else {
                push_tracked_arg(&mut cmd, &mut sidecar_args, "--sandbox");
                push_tracked_arg(&mut cmd, &mut sidecar_args, "workspace-write");
                push_tracked_arg(&mut cmd, &mut sidecar_args, "--skip-git-repo-check");
            }
        }
        for a in &extra_args {
            if !a.starts_with("--session-id=")
                && !a.starts_with("--codex-model=")
                && !a.starts_with("--codex-effort=")
                && (full_access || a != "--dangerously-bypass-approvals-and-sandbox")
            {
                push_tracked_arg(&mut cmd, &mut sidecar_args, a);
            }
        }
    } else if cli_name == "copilot" {
        if full_access {
            push_tracked_arg(&mut cmd, &mut sidecar_args, "--allow-all");
        }
        for a in &extra_args {
            push_tracked_arg(&mut cmd, &mut sidecar_args, a);
        }
    } else {
        for a in &extra_args {
            push_tracked_arg(&mut cmd, &mut sidecar_args, a);
        }
    }
    if cli_name == "codex" {
        push_tracked_arg(&mut cmd, &mut sidecar_args, &prompt);
    } else if cli_name == "copilot" || cli_name == "claude" || cli_name == "gemini" {
        push_tracked_arg(&mut cmd, &mut sidecar_args, "-p");
        push_tracked_arg(&mut cmd, &mut sidecar_args, &prompt);
    } else {
        push_tracked_arg(&mut cmd, &mut sidecar_args, &prompt);
    }
    if let Some(dir) = &cwd {
        cmd.current_dir(dir);
    }
    apply_env(
        &mut cmd,
        &mut sidecar_env,
        cli_name,
        actual_bin,
        cwd.as_deref(),
    );
    if cli_name == "codex" {
        let Some(home) = frozen_codex_home.as_ref() else {
            let done = AgentToServer::CliDone {
                req_id,
                exit_ok: false,
                error: Some("Codex 任务缺少冻结的 CODEX_HOME，已拒绝启动。".to_string()),
                session_id: None,
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
                warn!(%error, "failed to persist missing CODEX_HOME completion");
            }
            return;
        };
        let local_offline =
            completion_context.origin == crate::node_agent_completion_outbox::LOCAL_OFFLINE_ORIGIN;
        if let Err(error) =
            home.validate_for_task(local_offline, runtime.is_cloud_connected().await)
        {
            let done = AgentToServer::CliDone {
                req_id,
                exit_ok: false,
                error: Some(error.to_string()),
                session_id: None,
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
                warn!(%error, "failed to persist CODEX_HOME validation completion");
            }
            return;
        }
        let home = home.path().to_string();
        cmd.env("CODEX_HOME", &home);
        sidecar_env.push(("CODEX_HOME".to_string(), home));
    }
    let stdin_piped_empty = cli_name == "copilot" || cli_name == "claude" || cli_name == "gemini";
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .stdin(if stdin_piped_empty {
            std::process::Stdio::piped()
        } else {
            std::process::Stdio::null()
        });
    hide_tokio_command_window(&mut cmd);
    let codex_key = codex_plan.scope_key.clone();

    let sidecar_program = actual_bin.to_string();
    let direct = CliDirectRunContext {
        cmd,
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
        cancel_rx,
        task_journal,
        cwd,
        prompt,
        server_runtime_config,
        approval_state,
        completion_context,
        frozen_codex_home,
    };
    let direct = if node_agent_cli_sidecar_runner::sidecar_enabled_for_cli(&direct.cli_name_owned) {
        match run_cli_sidecar_or_fallback(CliSidecarPromptContext {
            direct,
            program: sidecar_program,
            args: sidecar_args,
            env: sidecar_env,
            stdin_piped_empty,
        })
        .await
        {
            Some(direct) => direct,
            None => return,
        }
    } else {
        direct
    };
    run_cli_direct_process(direct).await;
}

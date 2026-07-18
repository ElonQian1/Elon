//! Sidecar-backed CLI prompt execution and terminal handling.

use homecli_proto::AgentToServer;
use tracing::{info, warn};

use crate::node_agent_cli_done::{
    cli_done_message_from_output, latest_codex_session_id, persist_and_send_cli_done,
};
use crate::node_agent_cli_prompt_direct::CliDirectRunContext;
use crate::node_agent_cli_prompt_runner::{
    cli_done_error, cli_runtime_policy, run_cli_prompt, ws_text, CliPromptRun,
};
use crate::node_agent_cli_runner::*;
use crate::{
    node_agent_active_task, node_agent_cli_pty, node_agent_cli_sidecar_runner,
    node_agent_codex_auth_switch, node_agent_codex_session,
};

pub(crate) struct CliSidecarPromptContext {
    pub(crate) direct: CliDirectRunContext,
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) env: Vec<(String, String)>,
    pub(crate) stdin_piped_empty: bool,
}

/// Run through the durable sidecar when it can be launched. A launch failure
/// returns the untouched direct-process context so the caller can fall back.
pub(crate) async fn run_cli_sidecar_or_fallback(
    context: CliSidecarPromptContext,
) -> Option<CliDirectRunContext> {
    let CliSidecarPromptContext {
        direct,
        program,
        args,
        env,
        stdin_piped_empty,
    } = context;
    let sidecar_registry = direct.runtime.sidecar_registry();
    let session_id = node_agent_cli_sidecar_runner::session_id_for_task(&direct.req_id);
    let output_path = sidecar_registry.output_path(&direct.req_id, &session_id);
    let runtime_policy = cli_runtime_policy(
        &direct.cli_name_owned,
        direct.runtime_permission.as_deref(),
        direct.completion_context.is_desktop_supervised(),
    );
    let _ = direct
        .task_journal
        .configure_runtime_policy(&direct.req_id, &runtime_policy);
    let launch_config = node_agent_cli_sidecar_runner::CliSidecarLaunchConfig {
        session_id,
        task_id: direct.req_id.clone(),
        cli_name: direct.cli_name_owned.clone(),
        route: node_agent_active_task::route_for_cli(&direct.cli_name_owned).to_string(),
        program,
        args,
        cwd: direct.cwd.clone(),
        runtime_permission: direct.runtime_permission.clone(),
        env,
        output_path,
        registry_dir: sidecar_registry.dir(),
        task_journal_dir: None,
        worker_path: None,
        worker_release: None,
        worker_sha256: None,
        codex_session_scope_key: direct.codex_key.clone(),
        legacy_codex_sessions_file: Some(direct.codex_sessions_file.clone()),
        timeout_secs: runtime_policy.total_timeout_secs,
        runtime_policy: Some(runtime_policy),
        stdin_payload: direct.stdin_payload.clone(),
        stdin_piped_empty,
        initial_cols: node_agent_cli_pty::default_cols(),
        initial_rows: node_agent_cli_pty::default_rows(),
    };
    let launch = match node_agent_cli_sidecar_runner::spawn_sidecar(launch_config).await {
        Ok(launch) => launch,
        Err(error) => {
            warn!("启动 CLI sidecar 失败，回落到直接子进程: {error:#}");
            return Some(direct);
        }
    };

    let CliDirectRunContext {
        cmd: _,
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
        stdin_payload: _,
        server_runtime_config,
        approval_state,
        completion_context,
        frozen_codex_home,
    } = direct;
    let cli_name = cli_name_owned.as_str();

    if let Some(pid) = launch.sidecar_pid {
        runtime.set_cli_prompt_os_pid(&req_id, Some(pid)).await;
        if let Err(error) = task_journal.record_process_started(&req_id, pid) {
            warn!("PC 任务 journal 写入 sidecar pid 失败: {error}");
        }
    }
    let mut journal_aggregate =
        crate::node_agent_cli_output_aggregate::CliOutputJournalAggregate::default();
    let result = node_agent_cli_sidecar_runner::follow_sidecar_output(
        &sidecar_registry,
        &req_id,
        &launch.output_path,
        &mut cancel_rx,
        |event| match event {
            node_agent_cli_sidecar_runner::CliSidecarOutputEvent::Stdout(text) => {
                if cli_name == "codex" {
                    let (session_id, visible_text) =
                        node_agent_codex_session::strip_session_id_lines(&text);
                    if let (Some(ref key), Some(real_id)) =
                        (codex_key.as_ref(), session_id.as_deref())
                    {
                        node_agent_codex_session::persist_session_compat(
                            &task_journal,
                            Some(&codex_sessions_file),
                            &req_id,
                            key,
                            real_id,
                        );
                    }
                    if visible_text.is_empty() {
                        return;
                    }
                    let observation =
                        journal_aggregate.observe(&task_journal, &req_id, "stdout", &visible_text);
                    if observation.progress {
                        let _ = task_journal.record_runtime_progress(
                            &req_id,
                            observation.phase.as_deref().unwrap_or("reasoning"),
                            observation.current_command.as_deref(),
                        );
                    }
                    send_cli_chunk_message(&out_tx, &req_id, &visible_text);
                } else {
                    send_cli_chunk_message(&out_tx, &req_id, &text);
                }
            }
            node_agent_cli_sidecar_runner::CliSidecarOutputEvent::Stderr(text) => {
                if cli_name == "codex" {
                    if !text.trim().is_empty() {
                        info!("[codex stderr] {}", text.trim_end());
                        let observation =
                            journal_aggregate.observe(&task_journal, &req_id, "stderr", &text);
                        if observation.progress {
                            let _ = task_journal.record_runtime_progress(
                                &req_id,
                                observation.phase.as_deref().unwrap_or("reasoning"),
                                observation.current_command.as_deref(),
                            );
                        }
                    }
                } else {
                    send_cli_chunk_message(&out_tx, &req_id, &text);
                }
            }
            node_agent_cli_sidecar_runner::CliSidecarOutputEvent::ChildStarted(pid) => {
                if let Err(error) = task_journal.record_process_started(&req_id, pid) {
                    warn!("PC 任务 journal 写入 sidecar child pid 失败: {error}");
                }
            }
            node_agent_cli_sidecar_runner::CliSidecarOutputEvent::Heartbeat => {
                let _ = task_journal.record_runtime_heartbeat(&req_id);
            }
        },
    )
    .await;
    journal_aggregate.flush(&task_journal, &req_id);
    let mut result = match result {
        Ok(result) => result,
        Err(error) => {
            let message = format!("sidecar 输出跟随失败: {error}");
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
                warn!(%error, "failed to persist sidecar follow completion");
            }
            return None;
        }
    };

    if result.canceled || result.terminal_error.is_some() {
        let message = result
            .terminal_error
            .clone()
            .unwrap_or_else(|| "用户已停止 PC CLI 任务".to_string());
        let (exit_ok, error, workspace_status) =
            finalize_cli_prompt_workspace(false, Some(message), conversation_workspace);
        let (done, combined_output) = cli_done_message_from_output(
            req_id,
            exit_ok,
            error,
            &result.stdout_text,
            &result.stderr_text,
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
            warn!(%error, "failed to persist canceled sidecar completion");
        }
        return None;
    }

    if !result.exit_ok && cli_name == "codex" {
        if let Some(fallback_args) =
            crate::node_agent_codex_model_compat::compatibility_fallback_args(
                &extra_args,
                &result.stdout_text,
                &result.stderr_text,
            )
        {
            send_cli_chunk(
                &out_tx,
                &task_journal,
                &req_id,
                "stdout",
                "codex\n当前模型需要更高版本 Codex，已自动切换到兼容模型 gpt-5.4 并继续本轮任务。\n",
            );
            Box::pin(run_cli_prompt(CliPromptRun {
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
            return None;
        }
    }

    if !result.exit_ok && cli_name == "codex" && !codex_vault_switch_attempted {
        if let Some(auth_switch) = node_agent_codex_auth_switch::try_after_failure(
            &runtime,
            &req_id,
            &result.stdout_text,
            &result.stderr_text,
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
            Box::pin(run_cli_prompt(CliPromptRun {
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
            return None;
        }
    }

    if !result.exit_ok
        && cli_name == "codex"
        && codex_plan.is_resume()
        && node_agent_codex_session::stale_resume_failure(&result.stdout_text, &result.stderr_text)
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
        Box::pin(run_cli_prompt(CliPromptRun {
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
        return None;
    }

    if cli_name == "codex" && !contains_codex_reply_marker(&result.stdout_text) {
        if let Some(text) = codex_last_message_chunk(codex_last_message_path.as_ref()) {
            send_cli_chunk(&out_tx, &task_journal, &req_id, "stdout", &text);
            result.stdout_text.push_str(&text);
        }
    }
    if result.exit_ok && cli_name == "codex" && !contains_codex_reply_marker(&result.stdout_text) {
        let diagnostic = if result.stdout_text.trim().is_empty() {
            "Codex CLI 执行完成，但没有返回可解析输出。请查看 PC 节点日志确认是否已完成文件修改。"
        } else {
            "Codex CLI 执行完成，但输出里没有可解析的 codex 回复段。请查看 PC 节点日志确认是否已完成文件修改。"
        };
        let text = format!("codex\n{diagnostic}\n");
        let _ = out_tx.send(ws_text(&AgentToServer::CliChunk {
            req_id: req_id.clone(),
            text: text.clone(),
        }));
        let _ = task_journal.record_cli_chunk(&req_id, "stdout", &text);
    }
    let error = if result.exit_ok {
        None
    } else {
        Some(cli_done_error(
            cli_name,
            &result.stdout_text,
            &result.stderr_text,
        ))
    };
    let (exit_ok, error, workspace_status) =
        finalize_cli_prompt_workspace(result.exit_ok, error, conversation_workspace);
    let (done, combined_output) = cli_done_message_from_output(
        req_id,
        exit_ok,
        error,
        &result.stdout_text,
        &result.stderr_text,
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
        warn!(%error, "failed to persist sidecar CLI completion");
    }
    None
}

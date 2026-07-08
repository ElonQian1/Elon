// server/src/ai_cli/ai_cli_pc_run.rs

use super::*;
use anyhow::Result;
use std::{path::Path, sync::Arc};
use tokio::sync::mpsc::UnboundedSender;

enum PcAgentRunOutcome {
    Completed,
    NoReadableLightweightReply { diagnostic: Option<String> },
}
const PC_PROJECT_NO_CHANGES_ERROR: &str =
    "开发助手已经结束，但项目工作区没有产生新提交；本轮需求没有实际修改项目。请重新发送需求，或切换可用 PC 节点后再试。";

async fn run_via_pc_agent(
    agent_id: &str,
    user_id: &str,
    cwd: Option<&str>,
    user_message: &str,
    preflight_note: Option<&str>,
    request_mode: AiCliRequestMode,
    native_session_scope: Option<NativeSessionScope>,
    download_base: Option<&str>,
    artifact_workspace: Option<&Path>,
    attempt_apk_sync: bool,
    cli_name: &str,
    copilot_model: Option<&str>,
    codex_reasoning_effort: Option<&str>,
    model_label: Option<&str>,
    state: &Arc<AppState>,
    tx: &UnboundedSender<String>,
) -> Result<PcAgentRunOutcome> {
    let raw_pc_passthrough = request_mode.is_passthrough();
    let lightweight_pc_chat = !request_mode.is_plan() && cwd.is_none();
    let apk_sync_probe_since = pc_apk_probe_since(request_mode, cwd);
    let effective_codex_reasoning_effort = if lightweight_pc_chat {
        pc_lightweight_chat_reasoning_effort(cli_name, codex_reasoning_effort)
    } else {
        pc_project_reasoning_effort(cli_name, codex_reasoning_effort, request_mode)
    };
    // Route A CLI 会话锚点：服务器只下发稳定 scope，本机节点再按权限 + cwd 分桶。
    let native_cli_session_uuid = native_session_scope
        .as_ref()
        .map(|scope| native_session_uuid(cli_name, scope));
    let pc_development_prompt =
        !lightweight_pc_chat && !request_mode.is_plan() && !raw_pc_passthrough;
    let pc_prompt_bootstrapped = if pc_development_prompt {
        pc_route_a_prompt_bootstrapped(
            state,
            native_session_scope.as_ref(),
            cli_name,
            agent_id,
            cwd,
            native_cli_session_uuid.as_deref(),
            true,
        )
    } else {
        false
    };
    // prompt 构造
    let prompt = if raw_pc_passthrough {
        pc_project_passthrough_prompt(user_message)
    } else if lightweight_pc_chat {
        pc_lightweight_chat_prompt(user_message, cli_name, model_label.or(copilot_model))
    } else if request_mode.is_plan() {
        match preflight_note {
            Some(note) => format!(
                "当前是 Plan 模式：只生成开发计划，不改文件、不运行命令、不提交、不打包。\n\n注意：{}\n\n{}",
                note, user_message
            ),
            None => format!(
                "当前是 Plan 模式：只生成开发计划，不改文件、不运行命令、不提交、不打包。\n\n{}",
                user_message
            ),
        }
    } else {
        pc_project_execution_prompt(
            user_message,
            preflight_note,
            cli_name,
            model_label.or(copilot_model),
            pc_prompt_bootstrapped,
        )
    };

    // extra_args：Copilot/Codex 用 --session-id 绑定会话；Codex model/effort 由节点翻译成 exec 参数。
    let extra_args = pc_route_a_extra_args(
        cli_name,
        native_cli_session_uuid.as_deref(),
        copilot_model,
        effective_codex_reasoning_effort.as_deref(),
    );

    // dispatch 时节点可能刚好掉线重连；dispatch 成功后仍要等本机 ACK，避免假在线连接吞请求。
    let accepted_dispatch = dispatch_pc_cli_prompt_until_accepted(PcCliPromptDispatchRequest {
        state,
        tx,
        agent_id,
        cli_name,
        extra_args: &extra_args,
        cwd,
        prompt: &prompt,
        request_mode,
        native_session_scope: native_session_scope.as_ref(),
        lightweight_pc_chat,
    })
    .await?;
    let pc_req_id = accepted_dispatch.pc_req_id;
    let mut rx = accepted_dispatch.rx;
    let cancel_handle = accepted_dispatch.cancel_handle;
    let mut first_cli_event = accepted_dispatch.first_cli_event;
    let mut pc_cancel_guard = PcCliCancelOnDrop::armed(cancel_handle);
    let pc_cli_feature = if request_mode.is_plan() {
        "pc_agent_cli_plan"
    } else if raw_pc_passthrough {
        "pc_agent_cli_direct"
    } else if cwd.is_some() {
        "pc_agent_cli_dev"
    } else {
        "pc_agent_cli_chat"
    };
    let pc_accounting_key = format!("pc_agent_cli:{pc_req_id}");
    let pc_reserve_fen = billing::configured_reservation_fen(
        &state.store,
        if cwd.is_some() && !raw_pc_passthrough {
            "billing_cli_dev_reservation_fen"
        } else {
            "billing_cli_chat_reservation_fen"
        },
        if cwd.is_some() && !raw_pc_passthrough {
            100
        } else {
            10
        },
    );
    let (mut pc_billing_call, mut pc_billing_context) = reserve_pc_cli_billing_call(
        state.as_ref(),
        user_id,
        agent_id,
        &pc_accounting_key,
        pc_cli_feature,
        model_label.or(copilot_model).or(Some(cli_name)),
        pc_reserve_fen,
        cli_name,
    )
    .map_err(|msg| anyhow!(msg))?;
    let display_model = pc_display_model_label(
        cli_name,
        model_label.or(copilot_model),
        effective_codex_reasoning_effort.as_deref(),
        lightweight_pc_chat,
        cli_name,
    );
    start_pc_node_compute_run(
        state,
        user_id,
        agent_id,
        &pc_accounting_key,
        pc_cli_feature,
        Some(&display_model),
    );
    record_pc_execution_started(
        state,
        native_session_scope.as_ref(),
        agent_id,
        &pc_req_id,
        cwd,
        model_label.or(copilot_model),
    );
    let mut pc_execution_guard = PcExecutionFinishOnDrop::armed(
        state.clone(),
        native_session_scope.clone(),
        pc_req_id.clone(),
        Some(display_model.clone()),
    );
    let node_progress_name = pc_node_progress_name(state.as_ref(), agent_id).await;
    let _ = tx.send(pc_dispatch_started_event(
        &pc_req_id,
        agent_id,
        &node_progress_name,
        cli_name,
        cwd,
        native_session_scope.as_ref(),
        request_mode,
    ));

    let mut full_text = String::new();
    let stream_id = Uuid::new_v4().to_string();
    let mut stream_started = false;
    let is_codex = cli_name == "codex";
    let mut codex_passthrough_line_buffer = String::new();
    let mut lightweight_streamed_reply = String::new();
    let mut lightweight_received_event = false;
    let mut last_codex_progress_hint: Option<(&'static str, std::time::Instant)> = None;
    let mut pending_first_cli_event = first_cli_event.take();
    let project_recv_timeout_secs =
        pc_agent_cli_recv_timeout_secs(cli_name, request_mode, native_session_scope.as_ref());

    // 进度心跳：开发/规划每 5s 发一次；轻量聊天只回流真实文本，不刷内部状态。
    let progress_tx = tx.clone();
    let cli_label = pc_cli_progress_label(cli_name);
    let disp_model_clone = pc_cli_heartbeat_subject(&display_model, &node_progress_name, agent_id);
    let mut progress_handle = if lightweight_pc_chat {
        None
    } else {
        Some(tokio::spawn(async move {
            let mut elapsed: u64 = 0;
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                elapsed += 5;
                let _ = progress_tx.send(
                    WsMessage::progress(format!(
                        "{} ({}) 正在处理中…（已等待 {}s）",
                        cli_label, disp_model_clone, elapsed
                    ))
                    .to_json(),
                );
            }
        }))
    };

    loop {
        let event = if let Some(event) = pending_first_cli_event.take() {
            event
        } else if lightweight_pc_chat {
            let recv_timeout_secs = if lightweight_received_event {
                PC_LIGHTWEIGHT_CHAT_RECV_TIMEOUT_SECS
            } else {
                PC_LIGHTWEIGHT_CHAT_FIRST_EVENT_TIMEOUT_SECS
            };
            match tokio::time::timeout(std::time::Duration::from_secs(recv_timeout_secs), rx.recv())
                .await
            {
                Ok(Some(event)) => event,
                Ok(None) => break,
                Err(_) => {
                    abort_pc_progress(&mut progress_handle);
                    if !lightweight_received_event {
                        let message = pc_lightweight_no_node_event_diagnostic(
                            cli_name,
                            &node_progress_name,
                            recv_timeout_secs,
                        );
                        let _ = state
                            .agent_manager
                            .close_agent_session(
                                agent_id,
                                "lightweight CLI prompt did not receive any node event",
                            )
                            .await;
                        pc_billing_call.release_no_usage();
                        finish_pc_node_compute_run(
                            state,
                            &pc_accounting_key,
                            "released_no_usage",
                            None,
                            None,
                            None,
                            Some(&message),
                        );
                        record_pc_execution_without_cli_done(
                            state,
                            native_session_scope.as_ref(),
                            &pc_req_id,
                            false,
                            Some(&message),
                            Some(display_model.as_str()),
                        );
                        pc_execution_guard.disarm();
                        return Ok(PcAgentRunOutcome::NoReadableLightweightReply {
                            diagnostic: Some(message),
                        });
                    }
                    if stream_started && !lightweight_streamed_reply.trim().is_empty() {
                        let reply = lightweight_streamed_reply.trim().to_string();
                        let _ = tx.send(
                            WsMessage::Done {
                                message: reply,
                                apk_url: None,
                                image_url: None,
                                model_used: Some(display_model.clone()),
                                node_id: Some(agent_id.to_string()),
                            }
                            .to_json(),
                        );
                        pc_billing_call.release_no_usage();
                        finish_pc_node_compute_run(
                            state,
                            &pc_accounting_key,
                            "released_no_usage",
                            None,
                            None,
                            None,
                            Some(
                                "Lightweight PC chat timed out after streamed readable reply was delivered",
                            ),
                        );
                        record_pc_execution_without_cli_done(
                            state,
                            native_session_scope.as_ref(),
                            &pc_req_id,
                            true,
                            None,
                            Some(display_model.as_str()),
                        );
                        pc_execution_guard.disarm();
                        return Ok(PcAgentRunOutcome::Completed);
                    }
                    if let Some(reply) =
                        extract_lightweight_pc_chat_timeout_reply(&full_text, is_codex)
                    {
                        let _ = tx.send(
                            WsMessage::AssistantMessage {
                                text: reply.clone(),
                                model_used: Some(display_model.clone()),
                                stream_id: None,
                                node_id: Some(agent_id.to_string()),
                            }
                            .to_json(),
                        );
                        let _ = tx.send(
                            WsMessage::Done {
                                message: reply,
                                apk_url: None,
                                image_url: None,
                                model_used: Some(display_model.clone()),
                                node_id: Some(agent_id.to_string()),
                            }
                            .to_json(),
                        );
                        pc_billing_call.release_no_usage();
                        finish_pc_node_compute_run(
                            state,
                            &pc_accounting_key,
                            "released_no_usage",
                            None,
                            None,
                            None,
                            Some(
                                "Lightweight PC chat timed out after partial readable reply was delivered",
                            ),
                        );
                        record_pc_execution_without_cli_done(
                            state,
                            native_session_scope.as_ref(),
                            &pc_req_id,
                            true,
                            None,
                            Some(display_model.as_str()),
                        );
                        pc_execution_guard.disarm();
                        return Ok(PcAgentRunOutcome::Completed);
                    }
                    pc_billing_call.release_no_usage();
                    finish_pc_node_compute_run(
                        state,
                        &pc_accounting_key,
                        "released_no_usage",
                        None,
                        None,
                        None,
                        Some(
                            "Lightweight PC chat timed out before CliDone; fallback to normal chat",
                        ),
                    );
                    record_pc_execution_without_cli_done(
                        state,
                        native_session_scope.as_ref(),
                        &pc_req_id,
                        false,
                        Some(
                            "Lightweight PC chat timed out before CliDone; fallback to normal chat",
                        ),
                        Some(display_model.as_str()),
                    );
                    pc_execution_guard.disarm();
                    return Ok(no_readable_lightweight_reply(&full_text, cli_name));
                }
            }
        } else {
            match tokio::time::timeout(
                std::time::Duration::from_secs(project_recv_timeout_secs),
                rx.recv(),
            )
            .await
            {
                Ok(Some(event)) => event,
                Ok(None) => break,
                Err(_) => {
                    abort_pc_progress(&mut progress_handle);
                    let message = format!(
                        "PC agent CLI 等待终态超时（{}s），已取消本机任务",
                        project_recv_timeout_secs
                    );
                    let _ = state
                        .agent_manager
                        .close_agent_session(
                            agent_id,
                            "project CLI prompt timed out before terminal event",
                        )
                        .await;
                    finish_pc_node_compute_run(
                        state,
                        &pc_accounting_key,
                        "failed",
                        None,
                        None,
                        None,
                        Some(&message),
                    );
                    record_pc_execution_without_cli_done(
                        state,
                        native_session_scope.as_ref(),
                        &pc_req_id,
                        false,
                        Some(&message),
                        Some(display_model.as_str()),
                    );
                    pc_execution_guard.disarm();
                    pc_billing_call.release_error();
                    return Err(anyhow!(message));
                }
            }
        };

        if lightweight_pc_chat {
            lightweight_received_event = true;
        }

        match event {
            AgentToServer::CliPromptAccepted { .. } => {
                continue;
            }
            AgentToServer::CliChunk { text, .. } => {
                if lightweight_pc_chat {
                    full_text.push_str(&text);
                    if let Some(delta) = lightweight_pc_reply_delta(
                        &full_text,
                        is_codex,
                        &mut lightweight_streamed_reply,
                    ) {
                        if !stream_started {
                            stream_started = true;
                            let _ = tx.send(
                                WsMessage::AssistantMessage {
                                    text: delta,
                                    model_used: Some(display_model.clone()),
                                    stream_id: Some(stream_id.clone()),
                                    node_id: Some(agent_id.to_string()),
                                }
                                .to_json(),
                            );
                        } else {
                            let _ = tx.send(
                                WsMessage::AssistantChunk {
                                    stream_id: stream_id.clone(),
                                    text: delta,
                                }
                                .to_json(),
                            );
                        }
                    }
                    continue;
                }
                if is_codex {
                    full_text.push_str(&text);
                    let events = pc_cli_passthrough_events_from_chunk(
                        &mut codex_passthrough_line_buffer,
                        &text,
                        Some(display_model.as_str()),
                    );
                    for event in events {
                        let _ = tx.send(event);
                    }
                    if let Some((hint_key, message)) = pc_codex_progress_hint(&text, &display_model)
                    {
                        let should_send = match last_codex_progress_hint {
                            Some((last_key, last_at))
                                if last_key == hint_key
                                    && last_at.elapsed()
                                        < std::time::Duration::from_secs(
                                            PC_CODEX_PROGRESS_HINT_COOLDOWN_SECS,
                                        ) =>
                            {
                                false
                            }
                            _ => true,
                        };
                        if should_send {
                            last_codex_progress_hint = Some((hint_key, std::time::Instant::now()));
                            let _ = tx.send(WsMessage::progress(message).to_json());
                        }
                    }
                    continue;
                }
                if let Some(event) = pc_cli_passthrough_event(&text) {
                    abort_pc_progress(&mut progress_handle);
                    let _ = tx.send(event);
                    continue;
                }
                if text.trim().is_empty() {
                    full_text.push_str(&text);
                    continue;
                }
                if !stream_started {
                    stream_started = true;
                    abort_pc_progress(&mut progress_handle);
                    let _ = tx.send(
                        WsMessage::AssistantMessage {
                            text: text.clone(),
                            model_used: Some(display_model.clone()),
                            stream_id: Some(stream_id.clone()),
                            node_id: Some(agent_id.to_string()),
                        }
                        .to_json(),
                    );
                } else {
                    let _ = tx.send(
                        WsMessage::AssistantChunk {
                            stream_id: stream_id.clone(),
                            text: text.clone(),
                        }
                        .to_json(),
                    );
                }
                full_text.push_str(&text);
            }
            AgentToServer::CliDone {
                exit_ok,
                error,
                prompt_tokens,
                cached_input_tokens,
                completion_tokens,
                reasoning_tokens,
                total_tokens,
                model,
                workspace_status,
                session_id,
                ..
            } => {
                if is_codex {
                    let events = pc_cli_passthrough_events_flush(
                        &mut codex_passthrough_line_buffer,
                        Some(display_model.as_str()),
                    );
                    for event in events {
                        let _ = tx.send(event);
                    }
                }
                abort_pc_progress(&mut progress_handle); // 停止心跳
                pc_cancel_guard.disarm();
                let mut cli_usage = None;
                let mut accounting_result = None;
                let mut node_transaction = None;
                if let Some(usage) = crate::cli_usage::usage_from_optional_parts(
                    prompt_tokens,
                    cached_input_tokens,
                    completion_tokens,
                    reasoning_tokens,
                    total_tokens,
                    model.clone().or_else(|| Some(display_model.clone())),
                ) {
                    pc_billing_context.refresh(state.as_ref(), user_id, agent_id, cli_name);
                    accounting_result = record_pc_cli_trusted_usage(
                        &state.store,
                        user_id,
                        pc_cli_feature,
                        model.as_deref().or(Some(display_model.as_str())),
                        &usage,
                        &pc_accounting_key,
                        &pc_billing_context,
                    );
                    node_transaction = settle_pc_cli_node_usage(
                        state,
                        user_id,
                        agent_id,
                        pc_cli_feature,
                        model.as_deref().or(Some(display_model.as_str())),
                        &usage,
                        accounting_result.as_ref(),
                    );
                    if accounting_result.is_some() {
                        pc_billing_call.mark_settled();
                    }
                    cli_usage = Some(usage);
                }
                let no_project_changes = pc_project_execution_had_no_changes(
                    request_mode,
                    lightweight_pc_chat,
                    workspace_status.as_ref(),
                    attempt_apk_sync || looks_like_android_task(user_message),
                );
                let effective_exit_ok = exit_ok && !no_project_changes;
                let effective_error = if no_project_changes {
                    Some(PC_PROJECT_NO_CHANGES_ERROR.to_string())
                } else {
                    error.clone()
                };
                if is_codex {
                    record_pc_codex_thread_id(
                        state,
                        native_session_scope.as_ref(),
                        agent_id,
                        cwd,
                        workspace_status.as_ref(),
                        session_id.as_deref(),
                    );
                }
                record_pc_execution_finished(
                    state,
                    native_session_scope.as_ref(),
                    &pc_req_id,
                    effective_exit_ok,
                    effective_error.as_deref(),
                    model.as_deref().or(Some(display_model.as_str())),
                    workspace_status.as_ref(),
                    cli_usage.as_ref(),
                    accounting_result.as_ref(),
                );
                if effective_exit_ok && pc_development_prompt {
                    mark_pc_route_a_prompt_bootstrapped(
                        state,
                        native_session_scope.as_ref(),
                        cli_name,
                        agent_id,
                        cwd,
                        session_id.as_deref().or(native_cli_session_uuid.as_deref()),
                        true,
                    );
                }
                pc_execution_guard.disarm();
                let readable_output = pc_cli_readable_output(
                    is_codex,
                    lightweight_pc_chat,
                    stream_started,
                    &full_text,
                );
                let allow_codex_output_despite_error = pc_codex_error_output_can_complete(
                    is_codex,
                    readable_output.has_success_output,
                    no_project_changes,
                    effective_error.as_deref(),
                    &full_text,
                );
                if effective_exit_ok || allow_codex_output_despite_error {
                    let reply = if lightweight_pc_chat {
                        extract_lightweight_pc_chat_reply(&full_text, is_codex)
                    } else if is_codex {
                        readable_output.codex_final_reply.clone()
                    } else if stream_started {
                        String::new() // 已流式完毕，Done 不重复发
                    } else {
                        full_text.trim().to_string()
                    };
                    if lightweight_pc_chat && stream_started && !reply.is_empty() {
                        if let Some(delta) =
                            lightweight_reply_text_delta(&reply, &mut lightweight_streamed_reply)
                        {
                            let _ = tx.send(
                                WsMessage::AssistantChunk {
                                    stream_id: stream_id.clone(),
                                    text: delta,
                                }
                                .to_json(),
                            );
                        }
                    }
                    if lightweight_pc_chat && reply.is_empty() {
                        if cli_usage.is_none() {
                            pc_billing_call.release_no_usage();
                            finish_pc_node_compute_run(
                                state,
                                &pc_accounting_key,
                                "released_no_usage",
                                None,
                                None,
                                None,
                                Some(
                                    "Lightweight PC chat completed without readable reply; fallback to normal chat",
                                ),
                            );
                        } else {
                            finish_pc_node_compute_run(
                                state,
                                &pc_accounting_key,
                                "settled",
                                cli_usage.as_ref(),
                                accounting_result.as_ref(),
                                node_transaction.as_ref(),
                                Some(
                                    "Lightweight PC chat used tokens but returned no readable reply; fallback to normal chat",
                                ),
                            );
                        }
                        return Ok(no_readable_lightweight_reply(&full_text, cli_name));
                    }
                    let apk_url = sync_pc_agent_apk_after_success(
                        state,
                        agent_id,
                        ai_cli_apk_sync::pc_apk_sync_workspace(
                            cwd,
                            workspace_status
                                .as_ref()
                                .map(|status| status.active_workspace_path.as_str()),
                        ),
                        user_message,
                        request_mode,
                        attempt_apk_sync,
                        apk_sync_probe_since,
                        download_base,
                        artifact_workspace,
                        tx,
                    )
                    .await;
                    let reply = if lightweight_pc_chat || raw_pc_passthrough {
                        reply
                    } else if stream_started && reply.trim().is_empty() && apk_url.is_none() {
                        String::new()
                    } else {
                        sanitize_pc_development_reply(&reply, apk_url.as_deref())
                    };
                    let reply = if raw_pc_passthrough
                        && reply.trim().is_empty()
                        && apk_url.is_none()
                    {
                        pc_passthrough_empty_reply_diagnostic(&full_text, cli_name, &display_model)
                    } else {
                        reply
                    };
                    if lightweight_pc_chat && !reply.is_empty() && !stream_started {
                        let _ = tx.send(
                            WsMessage::AssistantMessage {
                                text: reply.clone(),
                                model_used: Some(display_model.clone()),
                                stream_id: None,
                                node_id: Some(agent_id.to_string()),
                            }
                            .to_json(),
                        );
                    }
                    if cli_usage.is_none() {
                        pc_billing_call.release_no_usage();
                        finish_pc_node_compute_run(
                            state,
                            &pc_accounting_key,
                            "released_no_usage",
                            None,
                            None,
                            None,
                            Some("CLI completed without token usage"),
                        );
                    } else {
                        finish_pc_node_compute_run(
                            state,
                            &pc_accounting_key,
                            "settled",
                            cli_usage.as_ref(),
                            accounting_result.as_ref(),
                            node_transaction.as_ref(),
                            None,
                        );
                    }
                    let _ = tx.send(
                        WsMessage::Done {
                            message: reply,
                            apk_url,
                            image_url: None,
                            model_used: Some(display_model.clone()),
                            node_id: Some(agent_id.to_string()),
                        }
                        .to_json(),
                    );
                    return Ok(PcAgentRunOutcome::Completed);
                } else {
                    let error_message = pc_cli_terminal_error_message(
                        cli_name,
                        no_project_changes,
                        effective_error.as_deref(),
                        &full_text,
                    );
                    finish_pc_node_compute_run(
                        state,
                        &pc_accounting_key,
                        "failed",
                        cli_usage.as_ref(),
                        accounting_result.as_ref(),
                        node_transaction.as_ref(),
                        Some(&error_message),
                    );
                    pc_billing_call.release_error();
                    return Err(anyhow!(error_message));
                }
            }
            _ => {}
        }
    }

    abort_pc_progress(&mut progress_handle);
    if lightweight_pc_chat {
        let mut reply = extract_lightweight_pc_chat_reply(&full_text, is_codex);
        if reply.is_empty() && stream_started && !lightweight_streamed_reply.trim().is_empty() {
            reply = lightweight_streamed_reply.trim().to_string();
        }
        if !reply.is_empty() {
            if stream_started {
                if let Some(delta) =
                    lightweight_reply_text_delta(&reply, &mut lightweight_streamed_reply)
                {
                    let _ = tx.send(
                        WsMessage::AssistantChunk {
                            stream_id: stream_id.clone(),
                            text: delta,
                        }
                        .to_json(),
                    );
                }
            } else {
                let _ = tx.send(
                    WsMessage::AssistantMessage {
                        text: reply.clone(),
                        model_used: Some(display_model.clone()),
                        stream_id: None,
                        node_id: Some(agent_id.to_string()),
                    }
                    .to_json(),
                );
            }
            let _ = tx.send(
                WsMessage::Done {
                    message: reply,
                    apk_url: None,
                    image_url: None,
                    model_used: Some(display_model.clone()),
                    node_id: Some(agent_id.to_string()),
                }
                .to_json(),
            );
            record_pc_execution_without_cli_done(
                state,
                native_session_scope.as_ref(),
                &pc_req_id,
                true,
                None,
                Some(display_model.as_str()),
            );
            pc_execution_guard.disarm();
            return Ok(PcAgentRunOutcome::Completed);
        }
        pc_billing_call.release_no_usage();
        finish_pc_node_compute_run(
            state,
            &pc_accounting_key,
            "released_no_usage",
            None,
            None,
            None,
            Some("Lightweight PC chat channel closed before CliDone; fallback to normal chat"),
        );
        record_pc_execution_without_cli_done(
            state,
            native_session_scope.as_ref(),
            &pc_req_id,
            false,
            Some("Lightweight PC chat channel closed before CliDone; fallback to normal chat"),
            Some(display_model.as_str()),
        );
        pc_execution_guard.disarm();
        return Ok(no_readable_lightweight_reply(&full_text, cli_name));
    }

    finish_pc_node_compute_run(
        state,
        &pc_accounting_key,
        "failed",
        None,
        None,
        None,
        Some("PC agent CLI 连接中断（未收到 CliDone）"),
    );
    record_pc_execution_without_cli_done(
        state,
        native_session_scope.as_ref(),
        &pc_req_id,
        false,
        Some("PC agent CLI 连接中断（未收到 CliDone）"),
        Some(display_model.as_str()),
    );
    pc_execution_guard.disarm();
    pc_billing_call.release_error();
    Err(anyhow!("PC agent CLI 连接中断（未收到 CliDone）"))
}


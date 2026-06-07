mod ai_cli_chat;
mod ai_cli_environment;
mod ai_cli_intent_gate;
mod ai_cli_native_session;
mod ai_cli_output;
mod ai_cli_prewarm;
mod ai_cli_process;
mod ai_cli_prompts;
mod ai_cli_runner;
mod ai_cli_streaming;
#[cfg(test)]
mod ai_cli_tests;
mod ai_cli_trace;
mod ai_cli_types;

pub use self::ai_cli_types::{AiCliRequestMode, IntentGateResult, NativeSessionScope};

use anyhow::{anyhow, Result};
use homecli_proto::AgentToServer;
use std::{path::Path, sync::Arc};
use uuid::Uuid;
use tokio::sync::mpsc::UnboundedSender;

pub(crate) use self::ai_cli_output::truncate_chars;
pub use self::ai_cli_prewarm::prewarm_codex_session;
pub(crate) use self::ai_cli_process::{
    cap_option_timeout, configured_timeout_cap, run_cli_command_traced, supports_codex_sessions,
    CliOutput,
};
pub(crate) use self::ai_cli_runner::codex_thread_uri;
#[cfg(test)]
pub(crate) use self::ai_cli_runner::{codex_exec_json_args, codex_resume_args};

use self::{
    ai_cli_chat::{chat_timeout_cap_secs, codex_network_or_timeout_error, is_tiny_chat_message},
    ai_cli_environment::{ensure_git, environment_notes, looks_like_android_task},
    ai_cli_native_session::{
        append_native_session_continuity, native_session_continuity_note,
        retire_native_session_and_schedule_repair, should_retry_without_native_session,
    },
    ai_cli_output::{extract_json_agent_message, extract_thread_id, format_cli_reply},
    ai_cli_prompts::build_cli_prompt,
    ai_cli_trace::{record_cli_retry, record_cli_session_skipped, CliTraceContext},
};
use crate::{
    intent_router, tools,
    types::{AppState, WsMessage},
};

const DEFAULT_CHAT_RESUME_TIMEOUT_CAP_SECS: u64 = 12;
const DEFAULT_CHAT_FRESH_TIMEOUT_CAP_SECS: u64 = 20;

pub use self::ai_cli_intent_gate::confirm_project_intent;

pub async fn run_with_workspace(
    user_id: &str,
    workspace: &Path,
    download_base: &str,
    user_message: &str,
    preflight_note: Option<&str>,
    option_id: Option<&str>,
    route: intent_router::CapabilityRoute,
    require_existing_git: bool,
    native_session_scope: Option<NativeSessionScope>,
    trace_id: Option<&str>,
    state: &Arc<AppState>,
    tx: &UnboundedSender<String>,
) -> Result<()> {
    let started = std::time::Instant::now();
    let request_mode = AiCliRequestMode::Execute;
    run_with_workspace_mode(
        user_id,
        workspace,
        download_base,
        user_message,
        preflight_note,
        option_id,
        route,
        require_existing_git,
        native_session_scope,
        trace_id,
        state,
        tx,
        request_mode,
        started,
    )
    .await
}

pub async fn run_plan_with_workspace(
    user_id: &str,
    workspace: &Path,
    download_base: &str,
    user_message: &str,
    preflight_note: Option<&str>,
    option_id: Option<&str>,
    native_session_scope: Option<NativeSessionScope>,
    trace_id: Option<&str>,
    state: &Arc<AppState>,
    tx: &UnboundedSender<String>,
) -> Result<()> {
    run_with_workspace_mode(
        user_id,
        workspace,
        download_base,
        user_message,
        preflight_note,
        option_id,
        intent_router::CapabilityRoute::CodeAgent,
        false,
        native_session_scope,
        trace_id,
        state,
        tx,
        AiCliRequestMode::Plan,
        std::time::Instant::now(),
    )
    .await
}

async fn run_with_workspace_mode(
    user_id: &str,
    workspace: &Path,
    download_base: &str,
    user_message: &str,
    preflight_note: Option<&str>,
    option_id: Option<&str>,
    route: intent_router::CapabilityRoute,
    require_existing_git: bool,
    native_session_scope: Option<NativeSessionScope>,
    trace_id: Option<&str>,
    state: &Arc<AppState>,
    tx: &UnboundedSender<String>,
    request_mode: AiCliRequestMode,
    started: std::time::Instant,
) -> Result<()> {
    // ── PC agent 委托（优先）──────────────────────────────────────────────────
    // 当云端有 PC agent（elon-pc-1）在线时，把 AI 提示委托给 PC 上的本地 Copilot CLI，
    // 利用 PC 性能处理请求，同时将结果流式返回给 APK。
    // 通过 PC_CLI_RELAY_ENABLED=false 可禁用此功能，回退到云端本地 CLI。
    let pc_relay_enabled = std::env::var("PC_CLI_RELAY_ENABLED")
        .map(|v| v != "false")
        .unwrap_or(true);
    if pc_relay_enabled {
        if let Some(agent_id) = state.agent_manager.any_connected_agent_id().await {
            let _ = tx.send(WsMessage::progress("正在连接 PC Copilot CLI...").to_json());
            match run_via_pc_agent(
                &agent_id,
                None,
                user_message,
                preflight_note,
                request_mode,
                None,
                state,
                tx,
            )
            .await
            {
                Ok(()) => return Ok(()),
                Err(e) => {
                    tracing::warn!("[ai_cli] PC agent CLI 失败，回退本地: {e:#}");
                    let _ = tx.send(
                        WsMessage::progress(format!("PC CLI 暂不可用，切换本地 CLI: {e}"))
                            .to_json(),
                    );
                }
            }
        }
    }
    // ─────────────────────────────────────────────────────────────────────────

    let mut option = state
        .ai_cli
        .find_option(option_id)
        .cloned()
        .ok_or_else(|| anyhow!("未找到可用本地 AI CLI 选项"))?;

    std::fs::create_dir_all(workspace)?;

    let planning_task = request_mode.is_plan();
    let development_task = planning_task
        || route != intent_router::CapabilityRoute::ChatAgent
        || intent_router::looks_like_development_request(user_message);
    let lightweight_chat_task =
        route == intent_router::CapabilityRoute::ChatAgent && !development_task;
    let tiny_chat_task = lightweight_chat_task && is_tiny_chat_message(user_message);
    if lightweight_chat_task {
        cap_option_timeout(&mut option, chat_timeout_cap_secs(tiny_chat_task));
    }
    if development_task && !planning_task {
        ensure_git(workspace, user_id, require_existing_git)?;
    }

    let android_task = development_task && !planning_task && looks_like_android_task(user_message);
    if planning_task {
        let _ =
            tx.send(WsMessage::progress("已开启先规划模式：本轮只生成计划，不改代码。").to_json());
    } else if development_task {
        let _ = tx.send(WsMessage::progress("正在准备项目工作区。").to_json());
        for note in environment_notes(user_message, &option) {
            let _ = tx.send(WsMessage::progress(note).to_json());
        }
        let _ = tx.send(WsMessage::progress("AI 助手正在处理你的请求。").to_json());
    } else {
        let _ = tx.send(WsMessage::progress("正在思考。").to_json());
    }

    let workspace_key = workspace.display().to_string();
    let skip_native_session = tiny_chat_task && supports_codex_sessions(&option);
    if skip_native_session {
        record_cli_session_skipped(state, trace_id, "run_workspace", "tiny_chat_fast_path");
    }
    let use_native_sessions = supports_codex_sessions(&option) && !skip_native_session;
    let session_state = if use_native_sessions {
        native_session_scope.as_ref().and_then(|scope| {
            state
                .store
                .get_native_agent_session_state(
                    &scope.project_id,
                    &scope.user_id,
                    Some(&scope.conversation_id),
                    &option.provider,
                    &option.id,
                    &workspace_key,
                )
                .ok()
                .flatten()
        })
    } else {
        None
    };
    let mut native_session_id = session_state
        .as_ref()
        .map(|state| state.native_session_id.clone());
    let mut prompt_bootstrapped = session_state
        .as_ref()
        .map(|state| {
            if development_task && !planning_task {
                state.dev_bootstrapped
            } else {
                state.chat_bootstrapped
            }
        })
        .unwrap_or(false);
    if lightweight_chat_task && native_session_id.is_some() && !prompt_bootstrapped {
        record_cli_session_skipped(
            state,
            trace_id,
            "run_workspace",
            "unbootstrapped_chat_session",
        );
        native_session_id = None;
        prompt_bootstrapped = false;
    }
    if native_session_id.is_some() {
        let _ = tx.send(
            WsMessage::progress("Restoring Codex CLI context for this conversation.").to_json(),
        );
    }

    let mut prompt = build_cli_prompt(
        workspace,
        user_message,
        preflight_note,
        &option,
        route,
        prompt_bootstrapped,
        request_mode,
    );
    let mut initial_option = option.clone();
    if lightweight_chat_task && native_session_id.is_some() {
        cap_option_timeout(
            &mut initial_option,
            configured_timeout_cap(
                "AI_CLI_CHAT_RESUME_TIMEOUT_SECS",
                DEFAULT_CHAT_RESUME_TIMEOUT_CAP_SECS,
            ),
        );
    }
    let mut output = match run_cli_command_traced(
        &initial_option,
        workspace,
        &prompt,
        native_session_id.as_deref(),
        tx,
        Some(CliTraceContext {
            state,
            trace_id,
            operation: "run_workspace",
            attempt: "initial",
            route: Some(route),
            development_task: Some(development_task),
            prompt_bootstrapped: Some(prompt_bootstrapped),
        }),
    )
    .await
    {
        Ok(output) => output,
        Err(error)
            if lightweight_chat_task
                && codex_network_or_timeout_error(&error)
                && supports_codex_sessions(&option) =>
        {
            return Err(error);
        }
        Err(error) if lightweight_chat_task && native_session_id.is_some() => {
            let stale_session_id = native_session_id.clone();
            record_cli_retry(
                state,
                trace_id,
                "run_workspace",
                stale_session_id.as_deref(),
                "resume_error_frontend_fresh_session",
            );
            retire_native_session_and_schedule_repair(
                state,
                trace_id,
                native_session_scope.as_ref(),
                &option,
                workspace,
                &workspace_key,
                stale_session_id.as_deref(),
                "initial_cli_error",
                &error.to_string(),
            );
            let _ = tx.send(
                WsMessage::progress("旧会话恢复超时，已切到新会话继续；旧上下文会在后台整理。")
                    .to_json(),
            );
            native_session_id = None;
            prompt_bootstrapped = false;
            prompt = build_cli_prompt(
                workspace,
                user_message,
                preflight_note,
                &option,
                route,
                prompt_bootstrapped,
                request_mode,
            );
            let mut fresh_option = option.clone();
            cap_option_timeout(
                &mut fresh_option,
                configured_timeout_cap(
                    "AI_CLI_CHAT_FRESH_TIMEOUT_SECS",
                    DEFAULT_CHAT_FRESH_TIMEOUT_CAP_SECS,
                ),
            );
            match run_cli_command_traced(
                &fresh_option,
                workspace,
                &prompt,
                None,
                tx,
                Some(CliTraceContext {
                    state,
                    trace_id,
                    operation: "run_workspace",
                    attempt: "fresh_after_resume_error",
                    route: Some(route),
                    development_task: Some(development_task),
                    prompt_bootstrapped: Some(prompt_bootstrapped),
                }),
            )
            .await
            {
                Ok(output) => output,
                Err(fresh_error)
                    if codex_network_or_timeout_error(&fresh_error)
                        && supports_codex_sessions(&option) =>
                {
                    return Err(fresh_error);
                }
                Err(fresh_error) => {
                    return Err(fresh_error);
                }
            }
        }
        Err(error)
            if lightweight_chat_task
                && codex_network_or_timeout_error(&error)
                && supports_codex_sessions(&option) =>
        {
            return Err(error);
        }
        Err(error) if lightweight_chat_task => {
            return Err(error);
        }
        Err(error) => return Err(error),
    };
    if should_retry_without_native_session(&initial_option, native_session_id.as_deref(), &output) {
        let stale_session_id = native_session_id.clone();
        record_cli_retry(
            state,
            trace_id,
            "run_workspace",
            stale_session_id.as_deref(),
            "stale_native_session",
        );
        retire_native_session_and_schedule_repair(
            state,
            trace_id,
            native_session_scope.as_ref(),
            &option,
            workspace,
            &workspace_key,
            stale_session_id.as_deref(),
            "stale_native_session",
            "Codex native session reported stale or unavailable",
        );
        let _ = tx.send(
            WsMessage::progress(if lightweight_chat_task {
                "旧会话不可用，已切到新会话继续；旧上下文会在后台整理。"
            } else {
                "Codex CLI session expired; starting a fresh session."
            })
            .to_json(),
        );
        native_session_id = None;
        prompt_bootstrapped = false;
        prompt = build_cli_prompt(
            workspace,
            user_message,
            preflight_note,
            &option,
            route,
            prompt_bootstrapped,
            request_mode,
        );
        if !lightweight_chat_task && !tiny_chat_task {
            if let Some(note) = native_session_continuity_note(
                state,
                native_session_scope.as_ref(),
                stale_session_id.as_deref(),
            ) {
                prompt = append_native_session_continuity(prompt, &note);
            }
        }
        let mut fresh_option = option.clone();
        if lightweight_chat_task {
            cap_option_timeout(
                &mut fresh_option,
                configured_timeout_cap(
                    "AI_CLI_CHAT_FRESH_TIMEOUT_SECS",
                    DEFAULT_CHAT_FRESH_TIMEOUT_CAP_SECS,
                ),
            );
        }
        output = match run_cli_command_traced(
            &fresh_option,
            workspace,
            &prompt,
            None,
            tx,
            Some(CliTraceContext {
                state,
                trace_id,
                operation: "run_workspace",
                attempt: "fresh_after_stale",
                route: Some(route),
                development_task: Some(development_task),
                prompt_bootstrapped: Some(prompt_bootstrapped),
            }),
        )
        .await
        {
            Ok(output) => output,
            Err(error)
                if lightweight_chat_task
                    && codex_network_or_timeout_error(&error)
                    && supports_codex_sessions(&option) =>
            {
                return Err(error);
            }
            Err(error) if lightweight_chat_task => {
                return Err(error);
            }
            Err(error) => return Err(error),
        };
    }

    if supports_codex_sessions(&option) && !output.success {
        let combined = format!("{}\n{}", output.stdout, output.stderr);
        if crate::codex_health::is_codex_network_error_text(&combined) {
            if lightweight_chat_task {
                // 轻量聊天遇到网络/超时错误：如果 CLI 已经流式输出了 agent_message，
                // 则 AssistantMessage 已发给客户端，静默发 Done 结束本轮即可避免红色报错气泡。
                // 否则使用友好降级消息，同样避免红色报错气泡。
                if extract_json_agent_message(&output.stdout).is_some() {
                    // agent_message 已经流式发给客户端，静默发 Done 结束本轮即可
                    let _ = tx.send(
                        WsMessage::Done {
                            message: String::new(),
                            apk_url: None,
                            image_url: None,
                            model_used: None,
                            node_id: None,
                        }
                        .to_json(),
                    );
                    return Ok(());
                }
                // 未流式输出任何内容，回传 Err 让 agent.rs API fallback 接管
                return Err(anyhow!(
                    "Codex CLI network unhealthy: {}",
                    truncate_chars(&combined, 500)
                ));
            }
            return Err(anyhow!(
                "Codex CLI network unhealthy: {}",
                truncate_chars(&combined, 500)
            ));
        }
    }

    let mut stored_session_id = native_session_id.clone();
    if let (Some(scope), Some(thread_id)) = (
        native_session_scope
            .as_ref()
            .filter(|_| use_native_sessions),
        extract_thread_id(&output.stdout),
    ) {
        let _ = state.store.upsert_native_agent_session(
            &scope.project_id,
            &scope.user_id,
            Some(&scope.conversation_id),
            &option.provider,
            &option.id,
            &workspace_key,
            &thread_id,
        );
        stored_session_id = Some(thread_id);
    }
    if output.success && !planning_task {
        if let (Some(scope), Some(session_id)) = (
            native_session_scope
                .as_ref()
                .filter(|_| use_native_sessions),
            stored_session_id.as_deref(),
        ) {
            let _ = state.store.mark_native_agent_session_bootstrapped(
                &scope.project_id,
                &scope.user_id,
                Some(&scope.conversation_id),
                &option.provider,
                &option.id,
                &workspace_key,
                session_id,
                development_task,
            );
        }
    }
    // 从 Codex CLI stdout 解析 token 用量并写入数据库
    let cli_feature = if planning_task {
        "codex_cli_plan"
    } else if development_task {
        "codex_cli_dev"
    } else {
        "codex_cli_chat"
    };
    crate::token_usage_api::record_codex_usage_from_stdout(
        &state.store,
        user_id,
        cli_feature,
        Some(option.id.as_str()),
        &output.stdout,
    );

    let reply = format_cli_reply(&output.stdout, &output.stderr, output.success);
    tracing::info!(
        route = ?route,
        development_task,
        elapsed_ms = started.elapsed().as_millis(),
        "local AI CLI request completed"
    );

    let apk_url = if android_task && output.success {
        let _ = tx.send(WsMessage::progress("AI 已完成处理，正在查找 APK 安装包。").to_json());
        let apk_url =
            tools::find_latest_apk(workspace).map(|_| tools::stable_apk_url(download_base));
        if apk_url.is_none() {
            let _ = tx.send(
                WsMessage::progress(
                    "未找到 APK 安装包；如果刚才是在打包，请检查最终回复里的失败原因。",
                )
                .to_json(),
            );
        }
        apk_url
    } else {
        None
    };

    let _ = tx.send(
        WsMessage::Done {
            message: reply,
            apk_url,
            image_url: None,
            model_used: None,
            node_id: None,
        }
        .to_json(),
    );

    Ok(())
}

// ── PC agent 委托辅助函数 ─────────────────────────────────────────────────────

pub async fn run_with_pc_agent_workspace(
    agent_id: &str,
    workspace_path: &str,
    user_message: &str,
    preflight_note: Option<&str>,
    request_mode: AiCliRequestMode,
    native_session_scope: Option<NativeSessionScope>,
    state: &Arc<AppState>,
    tx: &UnboundedSender<String>,
) -> Result<()> {
    run_via_pc_agent(
        agent_id,
        Some(workspace_path),
        user_message,
        preflight_note,
        request_mode,
        native_session_scope,
        state,
        tx,
    )
    .await
}

/// 把 AI 请求委托给通过 WS 连接的 PC agent（elon-pc-1），在 PC 上执行 Copilot CLI。
/// 结果以流式 CliChunk 形式返回并转发给 APK。
async fn run_via_pc_agent(
    agent_id: &str,
    cwd: Option<&str>,
    user_message: &str,
    preflight_note: Option<&str>,
    request_mode: AiCliRequestMode,
    native_session_scope: Option<NativeSessionScope>,
    state: &Arc<AppState>,
    tx: &UnboundedSender<String>,
) -> Result<()> {
    let prompt = if request_mode.is_plan() {
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
        match preflight_note {
            Some(note) => format!(
                "注意：{}\n\n{}", note, user_message
            ),
            None => user_message.to_string(),
        }
    };

    // 查找该会话是否已有 native session（用于 Copilot --continue 续接上下文）
    let existing_session = native_session_scope.as_ref().and_then(|scope| {
        state.store.latest_native_agent_session_for_conversation(
            &scope.project_id,
            &scope.user_id,
            &scope.conversation_id,
            "copilot",
        ).ok().flatten()
    });
    let mut extra_args: Vec<String> = if existing_session.is_some() {
        vec!["--continue".into()]
    } else {
        vec![]
    };

    // PC 节点项目：从 prompt 里提取图片 URL 并通过 --attachment 传给 Copilot
    // 这样 Copilot 能直接下载并分析图片内容，不依赖本地路径
    for line in prompt.lines() {
        let line = line.trim();
        // 匹配 "url: http://..." 格式（由 append_project_attachment_notes 生成）
        if let Some(url) = line.strip_prefix("(url: ").and_then(|s| s.strip_suffix(')')) {
            if url.starts_with("http") {
                extra_args.push("--attachment".into());
                extra_args.push(url.to_string());
            }
        }
    }

    // dispatch 时节点可能刚好掉线重连：
    // 仅对 "agent not connected" 进行较长等待重试（最多约 75 秒），
    // 避免手机端刚发起任务就撞上节点瞬断。
    let (_, mut rx) = {
        let mut last_err = anyhow::anyhow!("dispatch failed");
        let mut result = Err(last_err);
        const MAX_ATTEMPTS: u32 = 25;
        for attempt in 0..MAX_ATTEMPTS {
            match state.agent_manager.dispatch_cli_prompt_in_cwd(
                agent_id,
                "copilot".into(),
                extra_args.clone(),
                cwd.map(ToOwned::to_owned),
                prompt.clone(),
            ).await {
                Ok(r) => { result = Ok(r); break; }
                Err(e) => {
                    last_err = e;
                    let msg = last_err.to_string();
                    let is_offline = msg.contains("agent not connected");
                    if is_offline && attempt + 1 < MAX_ATTEMPTS {
                        let wait = format!(
                            "PC 节点短暂离线，等待重连（{}/{}）…",
                            attempt + 1,
                            MAX_ATTEMPTS
                        );
                        let _ = tx.send(WsMessage::progress(wait).to_json());
                        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    } else {
                        result = Err(last_err);
                        break;
                    }
                }
            }
        }
        result?
    };

    let mut full_text = String::new();
    let stream_id = Uuid::new_v4().to_string();
    let mut stream_started = false;

    while let Some(event) = rx.recv().await {
        match event {
            AgentToServer::CliChunk { text, .. } => {
                if text.trim().is_empty() {
                    full_text.push_str(&text);
                    continue;
                }
                if !stream_started {
                    // 第一个非空块：创建气泡（AssistantMessage 带 stream_id）
                    stream_started = true;
                    let _ = tx.send(WsMessage::AssistantMessage {
                        text: text.clone(),
                        model_used: Some(agent_id.to_string()),
                        stream_id: Some(stream_id.clone()),
                    }.to_json());
                } else {
                    // 后续块：追加到同一气泡（AssistantChunk）
                    let _ = tx.send(WsMessage::AssistantChunk {
                        stream_id: stream_id.clone(),
                        text: text.clone(),
                    }.to_json());
                }
                full_text.push_str(&text);
            }
            AgentToServer::CliDone { exit_ok, error, .. } => {
                if exit_ok {
                    // 成功后保存 native session sentinel，下次带 --continue 续接上下文
                    if let Some(scope) = &native_session_scope {
                        let sentinel = format!("conv-{}", scope.conversation_id);
                        let workspace = cwd.unwrap_or("");
                        let _ = state.store.upsert_native_agent_session(
                            &scope.project_id,
                            &scope.user_id,
                            Some(&scope.conversation_id),
                            "copilot",
                            agent_id,
                            workspace,
                            &sentinel,
                        );
                    }
                    // 若什么都没发（空回复），补一条消息
                    if !stream_started {
                        let reply = full_text.trim().to_string();
                        if !reply.is_empty() {
                            let _ = tx.send(WsMessage::AssistantMessage {
                                text: reply,
                                model_used: Some(agent_id.to_string()),
                                stream_id: None,
                            }.to_json());
                        }
                    }
                    let _ = tx.send(WsMessage::Done {
                        message: String::new(),
                        apk_url: None,
                        image_url: None,
                        model_used: Some(agent_id.to_string()),
                        node_id: None,
                    }.to_json());
                    return Ok(());
                } else {
                    return Err(anyhow!("PC CLI 执行失败: {}", error.unwrap_or_default()));
                }
            }
            _ => {}
        }
    }

    Err(anyhow!("PC agent CLI 连接中断（未收到 CliDone）"))
}

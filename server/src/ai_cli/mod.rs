// server/src/ai_cli/mod.rs

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
use homecli_proto::{AgentToServer, CliProjectContext, CliWorkspaceStatus};
use std::{path::Path, sync::Arc};
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

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
    billing, intent_router, tools,
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
    if let Err(msg) = billing::check_can_call(&state.store, user_id) {
        return Err(anyhow!(msg));
    }

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
                user_id,
                None,
                user_message,
                preflight_note,
                request_mode,
                None,
                "copilot",
                None,
                None, // codex_reasoning_effort
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
    let cli_feature = if planning_task {
        "codex_cli_plan"
    } else if development_task {
        "codex_cli_dev"
    } else {
        "codex_cli_chat"
    };
    let accounting_key = trace_id
        .map(|trace_id| format!("codex_cli:{cli_feature}:{trace_id}"))
        .unwrap_or_else(|| format!("codex_cli:{cli_feature}:{}", Uuid::new_v4()));
    let reserve_fen = billing::configured_reservation_fen(
        &state.store,
        if development_task {
            "billing_cli_dev_reservation_fen"
        } else {
            "billing_cli_chat_reservation_fen"
        },
        if development_task { 100 } else { 10 },
    );
    let mut billing_call = crate::billing_lifecycle::TrustedBillingCall::reserve(
        &state.store,
        user_id,
        &accounting_key,
        cli_feature,
        "server_codex_cli",
        Some(option.id.as_str()),
        reserve_fen,
    )
    .map_err(|msg| anyhow!(msg))?;
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
    let runtime_permission = native_session_scope
        .as_ref()
        .map(|scope| scope.runtime_permission.as_str());

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
        runtime_permission,
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
                runtime_permission,
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
            runtime_permission,
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
    let usage_text = format!("{}\n{}", output.stdout, output.stderr);
    crate::token_usage_api::record_codex_usage_from_stdout_with_key(
        &state.store,
        user_id,
        cli_feature,
        Some(option.id.as_str()),
        &usage_text,
        Some(&accounting_key),
    );
    billing_call.mark_settled();

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

struct PcCliCancelOnDrop {
    handle: Option<crate::homecli_agent::CliPromptCancelHandle>,
}

impl PcCliCancelOnDrop {
    fn armed(handle: crate::homecli_agent::CliPromptCancelHandle) -> Self {
        Self {
            handle: Some(handle),
        }
    }

    fn disarm(&mut self) {
        self.handle = None;
    }
}

impl Drop for PcCliCancelOnDrop {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let sent = handle.cancel();
            tracing::info!(
                req_id = handle.req_id(),
                sent,
                "PC CLI task dropped; sent cancel to agent"
            );
        }
    }
}

pub async fn run_with_pc_agent_workspace(
    agent_id: &str,
    user_id: &str,
    workspace_path: &str,
    user_message: &str,
    preflight_note: Option<&str>,
    request_mode: AiCliRequestMode,
    native_session_scope: Option<NativeSessionScope>,
    cli_name: Option<&str>,
    copilot_model: Option<&str>,
    codex_reasoning_effort: Option<&str>,
    model_label: Option<&str>,
    state: &Arc<AppState>,
    tx: &UnboundedSender<String>,
) -> Result<()> {
    if let Err(msg) = billing::check_can_call(&state.store, user_id) {
        return Err(anyhow!(msg));
    }

    run_via_pc_agent(
        agent_id,
        user_id,
        Some(workspace_path),
        user_message,
        preflight_note,
        request_mode,
        native_session_scope,
        cli_name.unwrap_or("copilot"),
        copilot_model,
        codex_reasoning_effort,
        model_label,
        state,
        tx,
    )
    .await
}

/// 把 AI 请求委托给通过 WS 连接的 PC agent，在 PC 上执行指定 CLI（copilot 或 codex）。
/// 结果以流式 CliChunk 形式返回并转发给 APK。
async fn run_via_pc_agent(
    agent_id: &str,
    user_id: &str,
    cwd: Option<&str>,
    user_message: &str,
    preflight_note: Option<&str>,
    request_mode: AiCliRequestMode,
    native_session_scope: Option<NativeSessionScope>,
    cli_name: &str,
    copilot_model: Option<&str>,
    codex_reasoning_effort: Option<&str>,
    model_label: Option<&str>,
    state: &Arc<AppState>,
    tx: &UnboundedSender<String>,
) -> Result<()> {
    // prompt 构造
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
            Some(note) => format!("注意：{}\n\n{}", note, user_message),
            None => user_message.to_string(),
        }
    };

    // Copilot CLI 专用：--session-id 保证用户隔离+上下文复用
    let copilot_session_uuid = if cli_name != "copilot" {
        None
    } else {
        native_session_scope.as_ref().map(|scope| {
            use sha2::Digest;
            let seed = format!("copilot-session/{}/{}/{}", scope.project_id, scope.user_id, scope.conversation_id);
            let hash = sha2::Sha256::digest(seed.as_bytes());
            format!(
                "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-4{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                hash[0], hash[1], hash[2], hash[3],
                hash[4], hash[5],
                hash[6] & 0x0f, hash[7],
                (hash[8] & 0x3f) | 0x80, hash[9],
                hash[10], hash[11], hash[12], hash[13], hash[14], hash[15]
            )
        })
    };

    // extra_args：Copilot 用 --session-id + --model，Codex 用 --codex-model + --codex-effort
    let extra_args: Vec<String> = match cli_name {
        "copilot" => {
            let mut args = if let Some(ref sid) = copilot_session_uuid {
                vec![format!("--session-id={}", sid)]
            } else {
                vec![]
            };
            if let Some(model) = copilot_model {
                if !model.is_empty() && model != "auto" {
                    args.push("--model".into());
                    args.push(model.to_string());
                }
            }
            args
        }
        "codex" => {
            // Codex：传模型和 reasoning effort，node-agent 在 `exec` 前插入 `-m`/`-c`
            let mut args = vec![];
            if let Some(model) = copilot_model {
                if !model.is_empty() && model != "auto" {
                    args.push(format!("--codex-model={}", model));
                }
            }
            if let Some(effort) = codex_reasoning_effort {
                if !effort.is_empty() {
                    args.push(format!("--codex-effort={}", effort));
                }
            }
            args
        }
        _ => vec![],
    };

    // dispatch 时节点可能刚好掉线重连
    let (pc_req_id, mut rx, cancel_handle) = {
        let mut last_err = anyhow::anyhow!("dispatch failed");
        let mut result = Err(last_err);
        const MAX_ATTEMPTS: u32 = 25;
        for attempt in 0..MAX_ATTEMPTS {
            let project_context = if request_mode.is_plan() {
                None
            } else {
                native_session_scope
                    .as_ref()
                    .map(|scope| CliProjectContext {
                        project_id: scope.project_id.clone(),
                        conversation_id: scope.conversation_id.clone(),
                        runtime_permission: Some(scope.runtime_permission.clone()),
                    })
            };
            match state
                .agent_manager
                .dispatch_cli_prompt_with_context_control(
                    agent_id,
                    cli_name.to_string(),
                    extra_args.clone(),
                    cwd.map(ToOwned::to_owned),
                    project_context,
                    prompt.clone(),
                )
                .await
            {
                Ok(dispatch) => {
                    result = Ok(dispatch.into_parts());
                    break;
                }
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
    let mut pc_cancel_guard = PcCliCancelOnDrop::armed(cancel_handle);
    let pc_cli_feature = if request_mode.is_plan() {
        "pc_agent_cli_plan"
    } else if cwd.is_some() {
        "pc_agent_cli_dev"
    } else {
        "pc_agent_cli_chat"
    };
    let pc_accounting_key = format!("pc_agent_cli:{pc_req_id}");
    let pc_reserve_fen = billing::configured_reservation_fen(
        &state.store,
        if cwd.is_some() {
            "billing_cli_dev_reservation_fen"
        } else {
            "billing_cli_chat_reservation_fen"
        },
        if cwd.is_some() { 100 } else { 10 },
    );
    let mut pc_billing_call = crate::billing_lifecycle::TrustedBillingCall::reserve(
        &state.store,
        user_id,
        &pc_accounting_key,
        pc_cli_feature,
        "pc_agent_cli",
        model_label.or(copilot_model).or(Some(cli_name)),
        pc_reserve_fen,
    )
    .map_err(|msg| anyhow!(msg))?;
    let display_model = model_label
        .or(copilot_model)
        .map(String::from)
        .unwrap_or_else(|| agent_id.to_string());
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

    let mut full_text = String::new();
    let stream_id = Uuid::new_v4().to_string();
    let mut stream_started = false;
    let is_codex = cli_name == "codex";

    // 进度心跳：每 5s 发一次"正在处理"，避免用户以为卡死（之前 15s 太长）
    let progress_tx = tx.clone();
    let cli_label = pc_cli_progress_label(cli_name);
    let disp_model_clone = display_model.clone();
    let progress_handle = tokio::spawn(async move {
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
    });

    while let Some(event) = rx.recv().await {
        match event {
            AgentToServer::CliChunk { text, .. } => {
                if is_codex {
                    // Codex 0.133+ 直接输出 AI 回复，逐行流式发送给 APK
                    // 过滤掉启动信息行（header 区域），只发有效回复内容
                    let trimmed = text.trim();
                    let is_header = trimmed.is_empty()
                        || trimmed.starts_with("OpenAI Codex")
                        || trimmed.starts_with("--------")
                        || trimmed.starts_with("workdir:")
                        || trimmed.starts_with("model:")
                        || trimmed.starts_with("provider:")
                        || trimmed.starts_with("approval:")
                        || trimmed.starts_with("sandbox:")
                        || trimmed.starts_with("reasoning effort:")
                        || trimmed.starts_with("reasoning summaries:")
                        || trimmed.starts_with("session id:");
                    full_text.push_str(&text);
                    if is_header {
                        continue;
                    }
                    // 有效内容：流式发送
                    progress_handle.abort(); // 收到第一行有效内容时停止心跳
                    if !stream_started {
                        stream_started = true;
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
                    continue;
                }
                if text.trim().is_empty() {
                    full_text.push_str(&text);
                    continue;
                }
                if !stream_started {
                    stream_started = true;
                    progress_handle.abort();
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
                ..
            } => {
                progress_handle.abort(); // 停止心跳
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
                    accounting_result = crate::token_usage_api::record_trusted_usage_with_key(
                        &state.store,
                        user_id,
                        pc_cli_feature,
                        "pc_agent_cli",
                        model.as_deref().or(Some(display_model.as_str())),
                        &usage,
                        Some(&pc_accounting_key),
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
                record_pc_execution_finished(
                    state,
                    native_session_scope.as_ref(),
                    &pc_req_id,
                    exit_ok,
                    error.as_deref(),
                    model.as_deref().or(Some(display_model.as_str())),
                    workspace_status.as_ref(),
                    cli_usage.as_ref(),
                    accounting_result.as_ref(),
                );
                let has_useful_output = !full_text.trim().is_empty();
                if exit_ok
                    || (is_codex
                        && has_useful_output
                        && error
                            .as_deref()
                            .map(|e| {
                                !e.contains("断线")
                                    && !e.contains("超时")
                                    && !e.contains("worktree")
                                    && !e.contains("合并")
                            })
                            .unwrap_or(true))
                {
                    // Copilot/Codex 流式已发，CliDone 只需补发未流式的部分
                    let reply = if stream_started {
                        String::new() // 已流式完毕，Done 不重复发
                    } else if is_codex {
                        extract_codex_reply(&full_text)
                    } else {
                        full_text.trim().to_string()
                    };
                    if !reply.is_empty() {
                        let _ = tx.send(
                            WsMessage::AssistantMessage {
                                text: reply,
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
                            message: String::new(),
                            apk_url: None,
                            image_url: None,
                            model_used: Some(display_model.clone()),
                            node_id: Some(agent_id.to_string()),
                        }
                        .to_json(),
                    );
                    return Ok(());
                } else {
                    let error_message = format!("PC CLI 执行失败: {}", error.unwrap_or_default());
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

    progress_handle.abort();
    finish_pc_node_compute_run(
        state,
        &pc_accounting_key,
        "failed",
        None,
        None,
        None,
        Some("PC agent CLI 连接中断（未收到 CliDone）"),
    );
    pc_billing_call.release_error();
    Err(anyhow!("PC agent CLI 连接中断（未收到 CliDone）"))
}

fn pc_cli_progress_label(cli_name: &str) -> &'static str {
    match cli_name {
        "codex" => "Codex",
        "copilot" => "Copilot",
        "claude" => "Claude",
        "gemini" => "Gemini",
        "api-runtime" => "Route B",
        "server-runtime" => "Route C",
        _ => "PC AI",
    }
}

fn start_pc_node_compute_run(
    state: &AppState,
    consumer_user_id: &str,
    node_id: &str,
    compute_call_id: &str,
    feature: &str,
    model: Option<&str>,
) {
    let provider_user_id = match state.store.get_node_credential_owner(node_id) {
        Ok(owner) => owner,
        Err(e) => {
            tracing::warn!(node_id, error = %e, "查询 PC 节点 owner 失败，执行证明仅记录消费者侧");
            None
        }
    };
    let model_id = pc_cli_model_id(model);
    if let Err(e) = state
        .store
        .start_node_compute_run(crate::store::NodeComputeRunStart {
            compute_call_id,
            consumer_user_id,
            provider_user_id: provider_user_id.as_deref(),
            node_id,
            model_id: Some(&model_id),
            feature,
            usage_mode: "pc_agent_cli",
            route_reason: Some("pc_agent_selected"),
        })
    {
        tracing::warn!(
            consumer_user_id,
            node_id,
            compute_call_id,
            "PC CLI 执行证明 start 记录失败: {e:#}"
        );
    }
}

fn finish_pc_node_compute_run(
    state: &AppState,
    compute_call_id: &str,
    requested_status: &str,
    usage: Option<&crate::cli_usage::CliTokenUsage>,
    accounting_result: Option<&crate::store::TokenUsageAccountingResult>,
    node_transaction: Option<&crate::store::NodeTransaction>,
    error_message: Option<&str>,
) {
    let (prompt_tokens, completion_tokens) = usage.map(pc_cli_usage_tokens).unwrap_or((0, 0));
    let status = if requested_status == "settled" {
        if accounting_result
            .map(|result| result.deduplicated)
            .unwrap_or(false)
        {
            "deduplicated"
        } else if accounting_result.is_none() {
            "settlement_failed"
        } else if node_transaction.is_none() {
            "settlement_skipped"
        } else {
            "settled"
        }
    } else {
        requested_status
    };
    let billed_cost = node_transaction
        .map(|tx| tx.billed_cost_rmb_fen)
        .or_else(|| accounting_result.map(|result| result.cost_rmb_fen))
        .unwrap_or(0);
    let provider_earned = node_transaction
        .map(|tx| tx.provider_earned_fen)
        .unwrap_or(0);
    let settlement_status = node_transaction
        .map(|tx| tx.settlement_status.as_str())
        .or_else(|| accounting_result.map(|result| result.accounting_status.as_str()));
    if let Err(e) = state.store.finish_node_compute_run(
        compute_call_id,
        crate::store::NodeComputeRunFinish {
            status,
            prompt_tokens,
            completion_tokens,
            billed_cost_rmb_fen: billed_cost,
            provider_earned_fen: provider_earned,
            settlement_status,
            error_message,
        },
    ) {
        tracing::warn!(compute_call_id, "PC CLI 执行证明 finish 记录失败: {e:#}");
    }
}

fn record_pc_execution_started(
    state: &AppState,
    scope: Option<&NativeSessionScope>,
    node_id: &str,
    request_id: &str,
    requested_workspace_path: Option<&str>,
    model: Option<&str>,
) {
    let Some(scope) = scope else {
        return;
    };
    if let Err(e) =
        state
            .store
            .record_project_execution_started(crate::store::ProjectExecutionSessionStart {
                project_id: &scope.project_id,
                conversation_id: &scope.conversation_id,
                user_id: &scope.user_id,
                node_id,
                request_id,
                requested_workspace_path,
                model,
            })
    {
        tracing::warn!("record project execution start failed: {e:#}");
    }
}

fn record_pc_execution_finished(
    state: &AppState,
    scope: Option<&NativeSessionScope>,
    request_id: &str,
    exit_ok: bool,
    error: Option<&str>,
    model: Option<&str>,
    workspace_status: Option<&CliWorkspaceStatus>,
    usage: Option<&crate::cli_usage::CliTokenUsage>,
    accounting_result: Option<&crate::store::TokenUsageAccountingResult>,
) {
    if scope.is_none() {
        return;
    }
    let status = if exit_ok { "done" } else { "failed" };
    let merge_status = workspace_status
        .and_then(|status| status.merge_status.as_deref())
        .or(Some("legacy_no_workspace_status"));
    let workspace_message = workspace_status.and_then(|status| status.merge_message.as_deref());
    let last_error = error.or_else(|| (!exit_ok).then_some(workspace_message).flatten());

    if let Err(e) =
        state
            .store
            .record_project_execution_finished(crate::store::ProjectExecutionSessionFinish {
                request_id,
                base_workspace_path: workspace_status
                    .and_then(|status| status.base_workspace_path.as_deref()),
                active_workspace_path: workspace_status
                    .map(|status| status.active_workspace_path.as_str()),
                branch: workspace_status.and_then(|status| status.branch.as_deref()),
                isolated: workspace_status
                    .map(|status| status.isolated)
                    .unwrap_or(false),
                status,
                merge_status,
                last_error,
                model,
                prompt_tokens: usage.map(|usage| usage.input_tokens.max(0)),
                cached_input_tokens: usage.map(|usage| usage.cached_input_tokens.max(0)),
                completion_tokens: usage.map(|usage| usage.output_tokens.max(0)),
                reasoning_tokens: usage.map(|usage| usage.reasoning_tokens.max(0)),
                total_tokens: usage.map(|usage| usage.total_tokens.max(0)),
                token_usage_event_id: accounting_result
                    .map(|result| result.token_usage_event_id.as_str()),
                billing_event_id: accounting_result
                    .and_then(|result| result.billing_event_id.as_deref()),
            })
    {
        tracing::warn!("record project execution finish failed: {e:#}");
    }
}

fn settle_pc_cli_node_usage(
    state: &AppState,
    consumer_user_id: &str,
    node_id: &str,
    feature: &str,
    model: Option<&str>,
    usage: &crate::cli_usage::CliTokenUsage,
    accounting_result: Option<&crate::store::TokenUsageAccountingResult>,
) -> Option<crate::store::NodeTransaction> {
    if accounting_result
        .map(|result| result.deduplicated)
        .unwrap_or(true)
    {
        return None;
    }
    let provider_user_id = match state.store.get_node_credential_owner(node_id) {
        Ok(Some(owner)) if !owner.trim().is_empty() => owner,
        Ok(_) => {
            tracing::warn!(
                node_id,
                "PC CLI 用量已记录，但节点缺少 owner，跳过节点收益流水"
            );
            return None;
        }
        Err(e) => {
            tracing::warn!(node_id, error = %e, "查询 PC 节点 owner 失败，跳过节点收益流水");
            return None;
        }
    };
    let (prompt_tokens_i64, completion_tokens_i64) = pc_cli_usage_tokens(usage);
    let prompt_tokens = clamp_i64_to_u32(prompt_tokens_i64);
    let completion_tokens = clamp_i64_to_u32(completion_tokens_i64);
    if prompt_tokens == 0 && completion_tokens == 0 {
        return None;
    }
    let model_id = pc_cli_model_id(model);
    let params = crate::store::SettleParams {
        consumer_user_id,
        provider_user_id: &provider_user_id,
        node_id,
        model_id: &model_id,
        feature,
        usage_mode: "pc_agent_cli",
        compute_call_id: accounting_result.and_then(|result| result.idempotency_key.as_deref()),
        token_usage_event_id: accounting_result.map(|result| result.token_usage_event_id.as_str()),
        billing_event_id: accounting_result.and_then(|result| result.billing_event_id.as_deref()),
        prompt_tokens,
        completion_tokens,
        price_per_1k_credits: pc_cli_price_per_1k_credits(),
        billed_cost_rmb_fen: accounting_result
            .map(|result| result.cost_rmb_fen)
            .unwrap_or(0),
        accounting_status: accounting_result.map(|result| result.accounting_status.as_str()),
        provider_revenue_share_x1000: crate::node_router::provider_revenue_share_x1000(
            &state.store,
        ),
        platform_fee_rate: 0.2,
    };
    match state.store.settle_node_inference(params) {
        Ok(tx) => {
            tracing::debug!(
                consumer_user_id,
                provider_user_id,
                node_id,
                tokens = prompt_tokens + completion_tokens,
                billed_cost_rmb_fen = tx.billed_cost_rmb_fen,
                provider_earned_fen = tx.provider_earned_fen,
                settlement_status = tx.settlement_status,
                "PC CLI 节点收益流水已记录"
            );
            Some(tx)
        }
        Err(e) => {
            tracing::error!(
                consumer_user_id,
                provider_user_id,
                node_id,
                "PC CLI 节点收益流水记录失败: {e}"
            );
            None
        }
    }
}

fn pc_cli_model_id(model: Option<&str>) -> String {
    model
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("pc-cli/{value}"))
        .unwrap_or_else(|| "pc-cli/unknown".to_string())
}

fn pc_cli_usage_tokens(usage: &crate::cli_usage::CliTokenUsage) -> (i64, i64) {
    let prompt_tokens = usage.input_tokens.max(0);
    let total_tokens = usage
        .total_tokens
        .max(usage.input_tokens.max(0) + usage.output_tokens.max(0));
    let completion_tokens = (total_tokens - usage.input_tokens.max(0)).max(0);
    (prompt_tokens, completion_tokens)
}

fn pc_cli_price_per_1k_credits() -> f64 {
    std::env::var("ELON_PC_CLI_PRICE_PER_1K_CREDITS")
        .ok()
        .and_then(|value| value.trim().parse::<f64>().ok())
        .filter(|value| *value >= 0.0)
        .unwrap_or(0.1)
}

fn clamp_i64_to_u32(value: i64) -> u32 {
    value.clamp(0, u32::MAX as i64) as u32
}

/// 从 Codex exec 的完整输出中提取第一段 AI 回复文本。
/// Codex exec 的输出格式：
///   [启动信息头 ... ---]
///   user
///   <用户消息回放>
///   [可能的错误日志]
///   codex
///   <AI 回复>          ← 只取这段
///   tokens used
///   <AI 回复重复>       ← 丢弃
///   <数字>
///   <AI 回复重复>       ← 丢弃
fn extract_codex_reply(output: &str) -> String {
    let lines: Vec<&str> = output.lines().collect();
    let mut in_codex_reply = false;
    let mut reply_lines: Vec<&str> = Vec::new();

    for line in &lines {
        let trimmed = line.trim();
        if trimmed == "codex" && !in_codex_reply {
            in_codex_reply = true;
            continue;
        }
        if in_codex_reply {
            // tokens used 或纯数字行表示回复结束（后面是重复内容）
            if trimmed == "tokens used" || trimmed.chars().all(|c| c.is_ascii_digit() || c == ',') {
                break;
            }
            reply_lines.push(line);
        }
    }

    reply_lines.join("\n").trim().to_string()
}

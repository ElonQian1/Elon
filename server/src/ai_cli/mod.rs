// server/src/ai_cli/mod.rs

pub(crate) mod ai_cli_apk_build_script;
mod ai_cli_apk_sync;
mod ai_cli_chat;
mod ai_cli_chat_policy;
mod ai_cli_environment;
mod ai_cli_intent_gate;
mod ai_cli_native_session;
mod ai_cli_output;
mod ai_cli_pc_execution;
mod ai_cli_pc_prompt;
mod ai_cli_prewarm;
mod ai_cli_process;
mod ai_cli_prompts;
mod ai_cli_runner;
mod ai_cli_streaming;
#[cfg(test)]
mod ai_cli_tests;
mod ai_cli_trace;
mod ai_cli_types;
mod pc_agent_dispatch;
mod pc_artifact_completion;
mod pc_billing;
mod pc_cli_failure;
mod pc_dispatch_capture;
mod pc_passthrough_events;
mod pc_passthrough_reply;
pub(crate) mod pc_prompt_acceptance;

pub use self::ai_cli_types::{AiCliRequestMode, IntentGateResult, NativeSessionScope};

use anyhow::{anyhow, Result};
use homecli_proto::AgentToServer;
use std::{path::Path, sync::Arc};
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

#[cfg(test)]
pub(crate) use self::ai_cli_apk_sync::safe_pc_apk_filename;
pub(crate) use self::ai_cli_apk_sync::{pc_apk_probe_since, sync_pc_agent_apk_after_success};
pub(crate) use self::ai_cli_chat_policy::project_lightweight_chat_split_enabled;
pub(crate) use self::ai_cli_environment::looks_like_android_task;
pub(crate) use self::ai_cli_output::truncate_chars;
pub use self::ai_cli_prewarm::prewarm_codex_session;
pub(crate) use self::ai_cli_process::{
    cap_option_timeout, configured_timeout_cap, run_cli_command_traced, supports_codex_sessions,
    CliOutput,
};
pub(crate) use self::ai_cli_runner::codex_thread_uri;
#[cfg(test)]
pub(crate) use self::ai_cli_runner::{codex_exec_json_args, codex_resume_args};
use self::pc_artifact_completion::pc_project_execution_had_no_changes;
pub(crate) use self::pc_billing::{pc_cli_request_is_own_codex, requested_pc_cli_looks_like_codex};
use self::pc_billing::{
    record_pc_cli_trusted_usage, reserve_pc_cli_billing_call, settle_pc_cli_node_usage,
};
use self::pc_cli_failure::{
    pc_cli_readable_output, pc_cli_terminal_error_message, pc_codex_error_output_can_complete,
};
pub(crate) use self::pc_dispatch_capture::{
    run_pc_agent_workspace_capture, PcAgentWorkspaceCaptureRequest, PcAgentWorkspaceCaptureResult,
};
pub(crate) use self::pc_passthrough_events::{
    pc_cli_passthrough_event, pc_cli_passthrough_events_flush, pc_cli_passthrough_events_from_chunk,
};
#[cfg(test)]
use self::pc_passthrough_reply::clean_codex_stream_chunk;
use self::pc_passthrough_reply::{
    extract_codex_reply, extract_marker_lightweight_reply, pc_lightweight_no_readable_diagnostic,
    pc_passthrough_empty_reply_diagnostic, sanitize_lightweight_pc_reply,
    strip_terminal_control_sequences,
};
use self::pc_prompt_acceptance::pc_lightweight_no_node_event_diagnostic;

use self::{
    ai_cli_chat::{chat_timeout_cap_secs, codex_network_or_timeout_error, is_tiny_chat_message},
    ai_cli_environment::{ensure_git, environment_notes},
    ai_cli_native_session::{
        append_native_session_continuity, native_session_continuity_note,
        retire_native_session_and_schedule_repair, should_retry_without_native_session,
    },
    ai_cli_output::{extract_json_agent_message, extract_thread_id, format_cli_reply},
    ai_cli_pc_execution::{
        finish_pc_node_compute_run, mark_pc_route_a_prompt_bootstrapped,
        pc_route_a_prompt_bootstrapped, record_pc_codex_thread_id, record_pc_execution_finished,
        record_pc_execution_started, record_pc_execution_without_cli_done,
        start_pc_node_compute_run,
    },
    ai_cli_pc_prompt::{
        pc_cli_progress_label, pc_lightweight_chat_prompt, pc_project_execution_prompt,
        pc_project_passthrough_prompt,
    },
    ai_cli_prompts::build_cli_prompt,
    ai_cli_trace::{record_cli_retry, record_cli_session_skipped, CliTraceContext},
    pc_agent_dispatch::{dispatch_pc_cli_prompt_until_accepted, PcCliPromptDispatchRequest},
};
use crate::{
    agent_routing::quick_casual_reply,
    billing, intent_router,
    pc_node_display::{pc_cli_heartbeat_subject, pc_node_progress_name},
    tools,
    types::{AppState, WsMessage},
};

const DEFAULT_CHAT_RESUME_TIMEOUT_CAP_SECS: u64 = 12;
const DEFAULT_CHAT_FRESH_TIMEOUT_CAP_SECS: u64 = 20;
const PC_LIGHTWEIGHT_CHAT_FIRST_EVENT_TIMEOUT_SECS: u64 = 15;
const PC_LIGHTWEIGHT_CHAT_RECV_TIMEOUT_SECS: u64 = 120;
const PC_CODEX_PROJECT_DEFAULT_REASONING_EFFORT: &str = "medium";
const PC_CODEX_PROGRESS_HINT_COOLDOWN_SECS: u64 = 15;
const PC_AGENT_CLI_RECV_TIMEOUT_ENV: &str = "ELON_PC_AGENT_CLI_RECV_TIMEOUT_SECS";
const PC_AGENT_CLI_RECV_TIMEOUT_GRACE_SECS: u64 = 45;

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
    let lightweight_chat_split_enabled = project_lightweight_chat_split_enabled();
    if lightweight_chat_split_enabled
        && request_mode == AiCliRequestMode::Execute
        && route == intent_router::CapabilityRoute::ChatAgent
        && !intent_router::looks_like_development_request(user_message)
    {
        if let Some(reply) = quick_casual_reply(user_message) {
            let _ = tx.send(
                WsMessage::Done {
                    message: reply.to_string(),
                    apk_url: None,
                    image_url: None,
                    model_used: None,
                    node_id: None,
                }
                .to_json(),
            );
            return Ok(());
        }
    }

    if let Err(msg) = billing::check_can_call(&state.store, user_id) {
        return Err(anyhow!(msg));
    }

    let planning_task = request_mode.is_plan();
    let lightweight_chat_task = ai_cli_chat_policy::should_use_project_lightweight_chat(
        lightweight_chat_split_enabled,
        planning_task,
        route,
        user_message,
    );
    let development_task = planning_task || !lightweight_chat_task;
    let tiny_chat_task = lightweight_chat_task && is_tiny_chat_message(user_message);
    let prompt_route =
        ai_cli_chat_policy::prompt_route_for_project_chat(lightweight_chat_split_enabled, route);

    // ── PC agent 委托（优先）──────────────────────────────────────────────────
    // 当云端有 PC agent（elon-pc-1）在线时，把 AI 提示委托给 PC 上的本地 Copilot CLI，
    // 利用 PC 性能处理项目开发请求，同时将结果流式返回给 APK。
    // 只有显式开启轻量聊天分流时，普通聊天才留在轻量通道；默认项目消息直连 PC/Codex。
    // 通过 PC_CLI_RELAY_ENABLED=false 可禁用此功能，回退到云端本地 CLI。
    let pc_relay_enabled = std::env::var("PC_CLI_RELAY_ENABLED")
        .map(|v| v != "false")
        .unwrap_or(true);
    if pc_relay_enabled && development_task {
        if let Some(agent_id) = state.agent_manager.any_connected_agent_id().await {
            let _ = tx.send(WsMessage::progress("正在连接 PC 开发节点。").to_json());
            match run_via_pc_agent(
                &agent_id,
                user_id,
                None,
                user_message,
                preflight_note,
                request_mode,
                None,
                None,
                None,
                false,
                "copilot",
                None,
                None, // codex_reasoning_effort
                None,
                state,
                tx,
            )
            .await
            {
                Ok(PcAgentRunOutcome::Completed) => return Ok(()),
                Ok(PcAgentRunOutcome::NoReadableLightweightReply { diagnostic }) => {
                    tracing::warn!(
                        diagnostic = diagnostic.as_deref().unwrap_or_default(),
                        "[ai_cli] PC agent CLI 未返回可读内容，回退本地"
                    );
                    let _ = tx.send(WsMessage::progress("已切换到云端开发通道。").to_json());
                }
                Err(e) => {
                    tracing::warn!("[ai_cli] PC agent CLI 失败，回退本地: {e:#}");
                    let _ = tx.send(WsMessage::progress("已切换到云端开发通道。").to_json());
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
        prompt_route,
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
                prompt_route,
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
            prompt_route,
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

struct PcExecutionFinishOnDrop {
    state: Option<Arc<AppState>>,
    scope: Option<NativeSessionScope>,
    request_id: String,
    model: Option<String>,
}

impl PcExecutionFinishOnDrop {
    fn armed(
        state: Arc<AppState>,
        scope: Option<NativeSessionScope>,
        request_id: String,
        model: Option<String>,
    ) -> Self {
        Self {
            state: scope.as_ref().map(|_| state),
            scope,
            request_id,
            model,
        }
    }

    fn disarm(&mut self) {
        self.state = None;
        self.scope = None;
    }
}

impl Drop for PcExecutionFinishOnDrop {
    fn drop(&mut self) {
        let (Some(state), Some(scope)) = (self.state.as_ref(), self.scope.as_ref()) else {
            return;
        };
        record_pc_execution_finished(
            state.as_ref(),
            Some(scope),
            &self.request_id,
            false,
            Some("PC CLI 请求在收到终态前被取消或连接断开"),
            self.model.as_deref(),
            None,
            None,
            None,
        );
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
    download_base: Option<&str>,
    artifact_workspace: Option<&Path>,
    attempt_apk_sync: bool,
    cli_name: Option<&str>,
    copilot_model: Option<&str>,
    codex_reasoning_effort: Option<&str>,
    model_label: Option<&str>,
    state: &Arc<AppState>,
    tx: &UnboundedSender<String>,
) -> Result<()> {
    let cli_name = cli_name.unwrap_or("copilot");
    if !pc_cli_request_is_own_codex(state.as_ref(), user_id, agent_id, Some(cli_name)) {
        if let Err(msg) = billing::check_can_call(&state.store, user_id) {
            return Err(anyhow!(msg));
        }
    }

    let outcome = run_via_pc_agent(
        agent_id,
        user_id,
        Some(workspace_path),
        user_message,
        preflight_note,
        request_mode,
        native_session_scope,
        download_base,
        artifact_workspace,
        attempt_apk_sync,
        cli_name,
        copilot_model,
        codex_reasoning_effort,
        model_label,
        state,
        tx,
    )
    .await?;
    match outcome {
        PcAgentRunOutcome::Completed => Ok(()),
        PcAgentRunOutcome::NoReadableLightweightReply { diagnostic } => Err(anyhow!(
            "{}",
            diagnostic.unwrap_or_else(|| "PC agent CLI 未返回可读内容".to_string())
        )),
    }
}
pub async fn run_with_pc_agent_passthrough_workspace(
    agent_id: &str,
    user_id: &str,
    workspace_path: &str,
    user_message: &str,
    native_session_scope: Option<NativeSessionScope>,
    download_base: Option<&str>,
    artifact_workspace: Option<&Path>,
    attempt_apk_sync: bool,
    cli_name: Option<&str>,
    copilot_model: Option<&str>,
    codex_reasoning_effort: Option<&str>,
    model_label: Option<&str>,
    state: &Arc<AppState>,
    tx: &UnboundedSender<String>,
) -> Result<PcAgentChatOutcome> {
    let cli_name = cli_name.unwrap_or("codex");
    if !pc_cli_request_is_own_codex(state.as_ref(), user_id, agent_id, Some(cli_name)) {
        if let Err(msg) = billing::check_can_call(&state.store, user_id) {
            return Err(anyhow!(msg));
        }
    }
    let outcome = run_via_pc_agent(
        agent_id,
        user_id,
        Some(workspace_path),
        user_message,
        None,
        AiCliRequestMode::Passthrough,
        native_session_scope,
        download_base,
        artifact_workspace,
        attempt_apk_sync,
        cli_name,
        copilot_model,
        codex_reasoning_effort,
        model_label,
        state,
        tx,
    )
    .await?;

    Ok(match outcome {
        PcAgentRunOutcome::Completed => PcAgentChatOutcome::Answered,
        PcAgentRunOutcome::NoReadableLightweightReply { diagnostic } => {
            PcAgentChatOutcome::NoReadableReply { diagnostic }
        }
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PcAgentChatOutcome {
    Answered,
    NoReadableReply { diagnostic: Option<String> },
}

pub async fn run_with_pc_agent_chat(
    agent_id: &str,
    user_id: &str,
    user_message: &str,
    native_session_scope: Option<NativeSessionScope>,
    cli_name: Option<&str>,
    copilot_model: Option<&str>,
    codex_reasoning_effort: Option<&str>,
    model_label: Option<&str>,
    state: &Arc<AppState>,
    tx: &UnboundedSender<String>,
) -> Result<PcAgentChatOutcome> {
    let cli_name = cli_name.unwrap_or("codex");
    if !pc_cli_request_is_own_codex(state.as_ref(), user_id, agent_id, Some(cli_name)) {
        if let Err(msg) = billing::check_can_call(&state.store, user_id) {
            return Err(anyhow!(msg));
        }
    }

    let chat_codex_reasoning_effort =
        pc_lightweight_chat_reasoning_effort(cli_name, codex_reasoning_effort);
    let read_only_scope = if should_skip_pc_chat_native_session(user_message) {
        None
    } else {
        native_session_scope.map(|mut scope| {
            scope.runtime_permission = "read_only".to_string();
            scope
        })
    };

    let outcome = run_via_pc_agent(
        agent_id,
        user_id,
        None,
        user_message,
        None,
        AiCliRequestMode::Execute,
        read_only_scope,
        None,
        None,
        false,
        cli_name,
        copilot_model,
        chat_codex_reasoning_effort.as_deref(),
        model_label,
        state,
        tx,
    )
    .await?;

    Ok(match outcome {
        PcAgentRunOutcome::Completed => PcAgentChatOutcome::Answered,
        PcAgentRunOutcome::NoReadableLightweightReply { diagnostic } => {
            PcAgentChatOutcome::NoReadableReply { diagnostic }
        }
    })
}

fn pc_lightweight_chat_reasoning_effort(
    cli_name: &str,
    requested_effort: Option<&str>,
) -> Option<String> {
    if cli_name != "codex" {
        return None;
    }

    let clean = requested_effort
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("low")
        .to_ascii_lowercase();

    match clean.as_str() {
        "high" | "xhigh" => Some("low".to_string()),
        _ => Some(clean),
    }
}

fn pc_project_reasoning_effort(
    cli_name: &str,
    requested_effort: Option<&str>,
    request_mode: AiCliRequestMode,
) -> Option<String> {
    if cli_name != "codex" {
        return None;
    }

    let clean = requested_effort
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase);

    clean.or_else(|| {
        if request_mode.is_passthrough() {
            None
        } else {
            Some(
                if request_mode.is_plan() {
                    "low"
                } else {
                    PC_CODEX_PROJECT_DEFAULT_REASONING_EFFORT
                }
                .to_string(),
            )
        }
    })
}

fn pc_runtime_full_access(runtime_permission: Option<&str>) -> bool {
    matches!(
        runtime_permission.map(str::trim),
        Some("project_write" | "full_access" | "danger_full_access")
    )
}

fn pc_agent_cli_node_timeout_secs(cli_name: &str, runtime_permission: Option<&str>) -> u64 {
    match cli_name.trim().to_ascii_lowercase().as_str() {
        "codex" if pc_runtime_full_access(runtime_permission) => 1200,
        "codex" => 300,
        _ => 180,
    }
}

fn pc_agent_cli_recv_timeout_secs(
    cli_name: &str,
    request_mode: AiCliRequestMode,
    scope: Option<&NativeSessionScope>,
) -> u64 {
    if let Ok(value) = std::env::var(PC_AGENT_CLI_RECV_TIMEOUT_ENV) {
        if let Ok(parsed) = value.trim().parse::<u64>() {
            return parsed.clamp(60, 3600);
        }
    }
    let runtime_permission = if request_mode.is_plan() {
        Some("read_only")
    } else {
        scope.map(|scope| scope.runtime_permission.as_str())
    };
    pc_agent_cli_node_timeout_secs(cli_name, runtime_permission)
        .saturating_add(PC_AGENT_CLI_RECV_TIMEOUT_GRACE_SECS)
}

fn should_skip_pc_chat_native_session(user_message: &str) -> bool {
    if is_tiny_chat_message(user_message) {
        return true;
    }

    let compact: String = user_message
        .trim()
        .to_lowercase()
        .chars()
        .filter(|ch| {
            !ch.is_whitespace()
                && !matches!(
                    ch,
                    '!' | '！'
                        | '?'
                        | '？'
                        | '.'
                        | '。'
                        | ','
                        | '，'
                        | ';'
                        | '；'
                        | ':'
                        | '：'
                        | '~'
                        | '～'
                )
        })
        .take(32)
        .collect();

    matches!(
        compact.as_str(),
        "我有一个想法"
            | "我有个想法"
            | "有一个想法"
            | "有个想法"
            | "我刚有个想法"
            | "我刚刚有个想法"
            | "我有一个需求"
            | "我有个需求"
            | "有一个需求"
            | "有个需求"
    )
}

fn pc_display_model_label(
    cli_name: &str,
    requested_label: Option<&str>,
    codex_reasoning_effort: Option<&str>,
    lightweight_pc_chat: bool,
    fallback: &str,
) -> String {
    let base = requested_label
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback);

    if lightweight_pc_chat && cli_name == "codex" {
        if let Some(effort) = codex_reasoning_effort
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            let model = base
                .split_once(" · 推理 ")
                .map(|(model, _)| model)
                .unwrap_or(base);
            return format!("{model} · 轻量 {effort}");
        }
    }

    base.to_string()
}

fn native_session_uuid(cli_name: &str, scope: &NativeSessionScope) -> String {
    use sha2::Digest;

    let cli_prefix = match cli_name {
        "copilot" => "copilot-session",
        "codex" => "codex-session",
        other => other,
    };
    let seed = format!(
        "{}/{}/{}/{}",
        cli_prefix, scope.project_id, scope.user_id, scope.conversation_id
    );
    let hash = sha2::Sha256::digest(seed.as_bytes());
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-4{:x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        hash[0],
        hash[1],
        hash[2],
        hash[3],
        hash[4],
        hash[5],
        hash[6] & 0x0f, // 单个十六进制位，用 {:x} 避免零填充为两位
        hash[7],
        (hash[8] & 0x3f) | 0x80,
        hash[9],
        hash[10],
        hash[11],
        hash[12],
        hash[13],
        hash[14],
        hash[15]
    )
}

fn pc_route_a_extra_args(
    cli_name: &str,
    native_session_id: Option<&str>,
    model: Option<&str>,
    codex_reasoning_effort: Option<&str>,
) -> Vec<String> {
    match cli_name {
        "copilot" => {
            let mut args = native_session_id
                .map(|sid| vec![format!("--session-id={}", sid)])
                .unwrap_or_default();
            if let Some(model) = model {
                if !model.is_empty() && model != "auto" {
                    args.push("--model".into());
                    args.push(model.to_string());
                }
            }
            args
        }
        "codex" => {
            let mut args = native_session_id
                .map(|sid| vec![format!("--session-id={}", sid)])
                .unwrap_or_default();
            if let Some(model) = model {
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
    }
}

/// 把 AI 请求委托给通过 WS 连接的 PC agent，在 PC 上执行指定 CLI（copilot 或 codex）。
/// 结果以流式 CliChunk 形式返回并转发给 APK。
#[derive(Debug, Clone, PartialEq, Eq)]
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

fn abort_pc_progress(handle: &mut Option<tokio::task::JoinHandle<()>>) {
    if let Some(handle) = handle.take() {
        handle.abort();
    }
}

fn extract_lightweight_pc_chat_reply(output: &str, is_codex: bool) -> String {
    let clean = strip_terminal_control_sequences(output);
    if let Some(reply) = extract_json_agent_message(&clean) {
        let reply = sanitize_lightweight_pc_reply(&reply);
        if !reply.is_empty() {
            return reply;
        }
    }

    if is_codex {
        let reply = sanitize_lightweight_pc_reply(&extract_codex_reply(&clean));
        if !reply.is_empty() {
            return reply;
        }
    }

    extract_marker_lightweight_reply(&clean)
}

fn no_readable_lightweight_reply(output: &str, cli_name: &str) -> PcAgentRunOutcome {
    PcAgentRunOutcome::NoReadableLightweightReply {
        diagnostic: pc_lightweight_no_readable_diagnostic(output, cli_name),
    }
}

fn lightweight_pc_reply_delta(
    output: &str,
    is_codex: bool,
    streamed_reply: &mut String,
) -> Option<String> {
    let reply = extract_lightweight_pc_chat_reply(output, is_codex);
    lightweight_reply_text_delta(&reply, streamed_reply)
}

fn lightweight_reply_text_delta(reply: &str, streamed_reply: &mut String) -> Option<String> {
    let reply = reply.trim();
    if reply.is_empty() || reply == streamed_reply.trim() {
        return None;
    }

    if streamed_reply.trim().is_empty() {
        streamed_reply.clear();
        streamed_reply.push_str(reply);
        return Some(reply.to_string());
    }

    if let Some(delta) = reply.strip_prefix(streamed_reply.as_str()) {
        if delta.is_empty() {
            return None;
        }
        streamed_reply.push_str(delta);
        return Some(delta.to_string());
    }

    None
}

fn extract_lightweight_pc_chat_timeout_reply(output: &str, is_codex: bool) -> Option<String> {
    let reply = extract_lightweight_pc_chat_reply(output, is_codex);
    (!reply.trim().is_empty()).then_some(reply)
}

fn sanitize_pc_development_reply(reply: &str, apk_url: Option<&str>) -> String {
    let clean = strip_terminal_control_sequences(reply);
    let mut lines = Vec::<String>::new();
    let mut in_code_block = false;

    for raw in clean.lines() {
        let line = raw.trim();
        if line.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }
        if is_pc_development_reply_boundary(line) {
            break;
        }
        if line.is_empty() || is_pc_development_reply_noise_line(line) {
            continue;
        }
        lines.push(sanitize_user_reply_line(line));
        if lines.len() >= 4 {
            break;
        }
    }

    let mut text = lines
        .into_iter()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();
    text = truncate_chars(text.as_str(), 220).trim().to_string();

    if text.is_empty() {
        text = if apk_url.is_some() {
            "新的安装包已生成。".to_string()
        } else {
            "开发助手已结束，但没有返回可展示的总结。请查看进度日志确认结果。".to_string()
        };
    }

    if apk_url.is_some()
        && !text.contains("项目空间")
        && !text.contains("安装按钮")
        && !text.contains("点击「安装」")
    {
        if !text.ends_with('。') && !text.ends_with('！') && !text.ends_with('!') {
            text.push('。');
        }
        text.push_str("\n请到项目空间点击「安装」下载体验。");
    }

    text
}

fn is_pc_development_reply_boundary(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    line.starts_with("diff --git")
        || line.starts_with("```")
        || line.starts_with("安装命令")
        || lower.starts_with("adb ")
        || lower.contains("adb.exe")
        || lower.starts_with("powershell")
        || lower.starts_with("git diff")
}

fn is_pc_development_reply_noise_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("c:\\")
        || lower.contains("c:/")
        || lower.contains("\\users\\")
        || lower.contains("/users/")
        || lower.contains("conversation-worktrees")
        || lower.contains("app/build/outputs/apk")
        || lower.contains("app\\build\\outputs\\apk")
        || lower.contains("platform-tools")
        || lower.contains("adb.exe")
        || lower.contains("diff --git")
        || lower.starts_with("index ")
        || lower.starts_with("--- ")
        || lower.starts_with("+++ ")
        || lower.starts_with("@@")
        || lower.starts_with("apply patch")
        || (line.contains(".apk](") && (lower.contains("c:/") || lower.contains("c:\\")))
        || (line.contains("安装包") && line.contains("这里"))
}

fn sanitize_user_reply_line(line: &str) -> String {
    line.replace('`', "")
        .replace("APK 已重新构建成功", "新的安装包已生成")
        .trim()
        .to_string()
}

fn pc_codex_progress_hint(text: &str, display_model: &str) -> Option<(&'static str, String)> {
    let clean = strip_terminal_control_sequences(text);
    let lower = clean.to_ascii_lowercase();

    if lower.contains("stream disconnected - retrying sampling request")
        || lower.contains("reconnecting...")
    {
        let attempt = extract_codex_reconnect_attempt(&clean)
            .map(|value| format!("（第 {value} 次）"))
            .unwrap_or_default();
        return Some((
            "codex_reconnecting",
            format!("Codex ({display_model}) 流式连接不稳定，正在自动重连{attempt}。"),
        ));
    }

    if lower.contains("falling back to http") {
        return Some((
            "codex_http_fallback",
            format!("Codex ({display_model}) 已切换到 HTTP fallback 继续生成。"),
        ));
    }

    if lower.contains("failed to refresh remote installed plugins cache")
        || lower.contains("curated plugin sync")
        || lower.contains("git sync failed for curated plugin")
    {
        return Some((
            "codex_plugin_cache",
            "Codex 插件远程同步不可达，已继续使用本地缓存。".to_string(),
        ));
    }

    None
}

fn extract_codex_reconnect_attempt(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let marker = "reconnecting...";
    if let Some(index) = lower.find(marker) {
        let rest = &text[index + marker.len()..];
        return extract_retry_fraction(rest);
    }

    let marker = "sampling request (";
    let index = lower.find(marker)?;
    let rest = &text[index + marker.len()..];
    extract_retry_fraction(rest.split(')').next().unwrap_or(rest))
}

fn extract_retry_fraction(text: &str) -> Option<String> {
    text.split(|ch: char| !(ch.is_ascii_digit() || ch == '/'))
        .find(|part| {
            let mut split = part.split('/');
            matches!((split.next(), split.next()), (Some(a), Some(b)) if !a.is_empty() && !b.is_empty())
        })
        .map(str::to_string)
}

fn pc_dispatch_started_event(
    pc_req_id: &str,
    agent_id: &str,
    node_display_name: &str,
    cli_name: &str,
    cwd: Option<&str>,
    native_session_scope: Option<&NativeSessionScope>,
    request_mode: AiCliRequestMode,
) -> String {
    serde_json::json!({
        "type": "pc_dispatch_started",
        "pc_req_id": pc_req_id,
        "req_id": pc_req_id,
        "agent_id": agent_id,
        "node_display_name": node_display_name,
        "cli": cli_name,
        "cwd_configured": cwd.is_some(),
        "project_id": native_session_scope.map(|scope| scope.project_id.as_str()),
        "conversation_id": native_session_scope.map(|scope| scope.conversation_id.as_str()),
        "runtime_permission": native_session_scope.map(|scope| scope.runtime_permission.as_str()),
        "mode": if request_mode.is_plan() {
            "plan"
        } else if request_mode.is_passthrough() {
            "passthrough"
        } else {
            "execute"
        }
    })
    .to_string()
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

#[cfg(test)]
mod pc_cli_passthrough_tests {
    use super::{
        clean_codex_stream_chunk, extract_codex_reply, extract_lightweight_pc_chat_reply,
        extract_lightweight_pc_chat_timeout_reply, lightweight_pc_reply_delta, native_session_uuid,
        pc_codex_progress_hint, pc_dispatch_started_event, pc_display_model_label,
        pc_lightweight_chat_reasoning_effort, pc_lightweight_no_node_event_diagnostic,
        pc_lightweight_no_readable_diagnostic, pc_passthrough_empty_reply_diagnostic,
        pc_project_reasoning_effort, pc_route_a_extra_args, sanitize_pc_development_reply,
        should_skip_pc_chat_native_session, strip_terminal_control_sequences, AiCliRequestMode,
        NativeSessionScope,
    };
    use serde_json::Value;

    fn test_scope(conversation_id: &str) -> NativeSessionScope {
        NativeSessionScope {
            project_id: "project-1".to_string(),
            user_id: "user-1".to_string(),
            conversation_id: conversation_id.to_string(),
            runtime_permission: "project_write".to_string(),
        }
    }

    #[test]
    fn pc_codex_stream_chunk_filters_terminal_noise() {
        let raw = "\u{1b}[m\u{1b}]0;C:\\Windows\\system32\\cmd.EXE\u{7}\u{1b}[?25h\
\u{1b}]0;C:\\WINDOWS\\system32\\cmd.exe \u{7}\
\u{1b}[2m2026-07-01T07:02:50.938044Z\u{1b}[22m  \u{1b}[33mWARN \u{1b}[m\
\u{1b}[2mcodex_core_plugins::manifest:\u{1b}[22m ignoring interface.defaultPrompt[0]\n\
memories startup: error returned from database: (code: 1) no such table: stage1_outputs\n\
mcp_native_chat_ok\n";

        let clean = clean_codex_stream_chunk(raw);

        assert_eq!(clean, "mcp_native_chat_ok\n");
    }

    #[test]
    fn pc_dispatch_started_event_exposes_local_req_id_without_prompt() {
        let event = pc_dispatch_started_event(
            "req-1",
            "agent-1",
            "一龙4060（agent-1）",
            "codex",
            Some("D:/workspace"),
            None,
            AiCliRequestMode::Execute,
        );
        let value: Value = serde_json::from_str(&event).unwrap();
        assert_eq!(value["type"], "pc_dispatch_started");
        assert_eq!(value["pc_req_id"], "req-1");
        assert_eq!(value["req_id"], "req-1");
        assert_eq!(value["node_display_name"], "一龙4060（agent-1）");
        assert!(value.get("prompt").is_none());
        assert!(value.get("api_key").is_none());
    }

    #[test]
    fn native_session_uuid_is_stable_and_cli_scoped() {
        let scope = test_scope("conversation-1");
        let codex = native_session_uuid("codex", &scope);
        let same_codex = native_session_uuid("codex", &scope);
        let copilot = native_session_uuid("copilot", &scope);
        let other_conversation = native_session_uuid("codex", &test_scope("conversation-2"));

        assert_eq!(codex, same_codex);
        assert_ne!(codex, copilot);
        assert_ne!(codex, other_conversation);
        assert!(codex.chars().all(|ch| ch.is_ascii_hexdigit() || ch == '-'));
    }

    #[test]
    fn pc_codex_extra_args_include_stable_session_and_runtime_options() {
        let session_id = native_session_uuid("codex", &test_scope("conversation-1"));
        let args = pc_route_a_extra_args(
            "codex",
            Some(&session_id),
            Some("gpt-5.3-codex"),
            Some("medium"),
        );

        assert_eq!(args[0], format!("--session-id={session_id}"));
        assert!(args.contains(&"--codex-model=gpt-5.3-codex".to_string()));
        assert!(args.contains(&"--codex-effort=medium".to_string()));
    }

    #[test]
    fn pc_lightweight_chat_downgrades_heavy_codex_effort() {
        assert_eq!(
            pc_lightweight_chat_reasoning_effort("codex", Some("xhigh")).as_deref(),
            Some("low")
        );
        assert_eq!(
            pc_lightweight_chat_reasoning_effort("codex", Some("high")).as_deref(),
            Some("low")
        );
        assert_eq!(
            pc_lightweight_chat_reasoning_effort("codex", Some("medium")).as_deref(),
            Some("medium")
        );
        assert_eq!(
            pc_lightweight_chat_reasoning_effort("copilot", Some("xhigh")),
            None
        );
    }

    #[test]
    fn pc_project_codex_defaults_to_medium_effort() {
        assert_eq!(
            pc_project_reasoning_effort("codex", None, AiCliRequestMode::Execute).as_deref(),
            Some("medium")
        );
        assert_eq!(
            pc_project_reasoning_effort("codex", None, AiCliRequestMode::Plan).as_deref(),
            Some("low")
        );
        assert_eq!(
            pc_project_reasoning_effort("codex", Some("high"), AiCliRequestMode::Execute)
                .as_deref(),
            Some("high")
        );
        assert_eq!(
            pc_project_reasoning_effort("copilot", None, AiCliRequestMode::Execute),
            None
        );
    }

    #[test]
    fn pc_direct_passthrough_does_not_force_default_effort() {
        assert_eq!(
            pc_project_reasoning_effort("codex", None, AiCliRequestMode::Passthrough),
            None
        );
        assert_eq!(
            pc_project_reasoning_effort("codex", Some("high"), AiCliRequestMode::Passthrough)
                .as_deref(),
            Some("high")
        );
    }

    #[test]
    fn pc_codex_progress_hint_reports_network_fallbacks() {
        let reconnect = "\u{1b}[31mERROR:\u{1b}[m Reconnecting... 3/5";
        let (_, message) =
            pc_codex_progress_hint(reconnect, "Codex · 推理 medium").expect("reconnect hint");
        assert!(message.contains("自动重连"));
        assert!(message.contains("第 3/5 次"));

        let (_, message) =
            pc_codex_progress_hint("codex_core::client: falling back to HTTP", "Codex")
                .expect("fallback hint");
        assert!(message.contains("HTTP fallback"));
    }

    #[test]
    fn pc_lightweight_chat_skips_native_session_for_short_starters() {
        assert!(should_skip_pc_chat_native_session("你好"));
        assert!(should_skip_pc_chat_native_session("我有一个想法"));
        assert!(should_skip_pc_chat_native_session("有个需求"));
        assert!(!should_skip_pc_chat_native_session(
            "我有一个想法，想做一个可以扫描商品并自动比价的 App"
        ));
    }

    #[test]
    fn pc_lightweight_display_label_reports_effective_low_effort() {
        assert_eq!(
            pc_display_model_label(
                "codex",
                Some("GPT-5.5 · 推理 xhigh"),
                Some("low"),
                true,
                "node-a",
            ),
            "GPT-5.5 · 轻量 low"
        );
        assert_eq!(
            pc_display_model_label("codex", Some("GPT-5.5"), Some("low"), false, "node-a"),
            "GPT-5.5"
        );
    }

    #[test]
    fn pc_copilot_extra_args_keep_session_and_model_flags() {
        let session_id = native_session_uuid("copilot", &test_scope("conversation-1"));
        let args = pc_route_a_extra_args("copilot", Some(&session_id), Some("gpt-5"), None);

        assert_eq!(
            args,
            vec![
                format!("--session-id={session_id}"),
                "--model".to_string(),
                "gpt-5".to_string()
            ]
        );
    }

    #[test]
    fn pc_lightweight_chat_reply_ignores_terminal_noise() {
        let output = "\u{1b}[m\\\\?\\C:\\Users\\ELon\n\
用作为当前目录的以上路径启动了 CMD.EXE。\n\
UNC 路径不受支持。默认值设为 Windows 目录。\n\
]0;C:\\WINDOWS\\system32\\cmd.exe\u{1b}[?25h]0;C:\\WINDOWS\\system32\\cmd.exe\n\
2026-06-30T12:14:19.451149Z WARN sqlx::query: slow statement: execution time exceeded alert threshold db.statement=\"DELETE FROM logs WHERE ts < ?\" rows_affected=10449 rows_returned=0 elapsed=1.54s\n\
codex\n\
你好，我在。\n\
tokens used\n\
1\n";

        assert_eq!(
            extract_lightweight_pc_chat_reply(output, true),
            "你好，我在。"
        );
    }

    #[test]
    fn pc_lightweight_chat_strips_orphan_ansi_fragments() {
        assert_eq!(strip_terminal_control_sequences("[m你好[?25h[22m"), "你好");
    }

    #[test]
    fn pc_codex_reply_extracts_last_summary_block() {
        let output = "\u{1b}[35mcodex\u{1b}[m\n\
规则文件已成功读取。\n\
exec\n\
git status\n\
\u{1b}[35mcodex\u{1b}[m\n\
完成。本次只新增了记录文件。\n\
\n\
结果汇总：\n\
- 读取代码成功。\n\
- Git 可用。\n\
tokens used\n\
44,443\n";

        let reply = extract_codex_reply(output);

        assert!(reply.contains("完成。本次只新增了记录文件。"));
        assert!(reply.contains("结果汇总"));
        assert!(!reply.contains("规则文件已成功读取"));
    }

    #[test]
    fn pc_codex_reply_ignores_false_unparseable_diagnostic() {
        let output = "codex\n\
完成。真实最终回复。\n\
tokens used\n\
1\n\
codex\n\
Codex CLI 执行完成，但输出里没有可解析的 codex 回复段。请查看 PC 节点日志确认是否已完成文件修改。\n";

        assert_eq!(extract_codex_reply(output), "完成。真实最终回复。");
    }

    #[test]
    fn pc_codex_reply_reads_json_agent_message() {
        let output = concat!(
            r#"{"type":"item.completed","item":{"type":"agent_message","text":"用户可见：第一段过程"}}"#,
            "\n",
            r#"{"type":"item.completed","item":{"type":"agent_message","text":"最终回复。已完成授权选择器。"}}"#
        );

        assert_eq!(extract_codex_reply(output), "最终回复。已完成授权选择器。");
    }

    #[test]
    fn pc_passthrough_empty_reply_diagnostic_does_not_claim_success() {
        let diagnostic = pc_passthrough_empty_reply_diagnostic("", "codex", "GPT-5.5 · 推理 xhigh");

        assert!(diagnostic.contains("没有返回可展示的正文"));
        assert!(diagnostic.contains("无法确认完成"));
        assert!(!diagnostic.contains("已完成"));
        assert!(!diagnostic.contains("任务已完成"));
    }

    #[test]
    fn pc_development_reply_hides_paths_commands_and_diff() {
        let reply = "已改好，“开始”按钮现在是绿色，文字是白色，并且 APK 已重新构建成功。\n\
新的安装包仍在这里:\n\
[app-debug.apk](C:/Users/Administrator/Elon/workspaces/conversation-worktrees/prj/app/build/outputs/apk/debug/app-debug.apk)\n\
安装命令不变:\n\
```powershell\n\
C:\\Users\\Administrator\\AppData\\Local\\Android\\Sdk\\platform-tools\\adb.exe install -r app\\build\\outputs\\apk\\debug\\app-debug.apk\n\
```\n\
diff --git a/app/src/main/java/com/dadapao/app/MainActivity.java b/app/src/main/java/com/dadapao/app/MainActivity.java\n";

        let sanitized =
            sanitize_pc_development_reply(reply, Some("https://example.test/latest.apk"));

        assert!(sanitized.contains("已改好"));
        assert!(sanitized.contains("项目空间"));
        assert!(!sanitized.contains("C:/Users"));
        assert!(!sanitized.contains("adb.exe"));
        assert!(!sanitized.contains("diff --git"));
    }

    #[test]
    fn pc_development_empty_reply_does_not_claim_code_was_changed() {
        let sanitized = sanitize_pc_development_reply("", None);

        assert!(sanitized.contains("没有返回可展示的总结"));
        assert!(!sanitized.contains("已改好"));
        assert!(!sanitized.contains("本轮开发任务已完成"));
    }

    #[test]
    fn pc_lightweight_chat_empty_output_stays_empty_for_upstream_fallback() {
        let reply = extract_lightweight_pc_chat_reply("", true);

        assert!(reply.trim().is_empty());
    }

    #[test]
    fn pc_lightweight_chat_timeout_keeps_partial_readable_reply() {
        let output = "OpenAI Codex\nmodel: test\ncodex\nhello, tell me your idea.";

        assert_eq!(
            extract_lightweight_pc_chat_timeout_reply(output, true).as_deref(),
            Some("hello, tell me your idea.")
        );
    }

    #[test]
    fn pc_lightweight_no_readable_diagnostic_exposes_codex_network_timeout() {
        let output = "2026-07-02 WARN stream disconnected - retrying sampling request (5/5)\n\
{\"type\":\"error\",\"message\":\"Reconnecting... 5/5 (request timed out)\"}\n\
2026-07-02 WARN falling back to HTTP";

        let diagnostic = pc_lightweight_no_readable_diagnostic(output, "codex").unwrap();

        assert!(diagnostic.contains("Codex"));
        assert!(diagnostic.contains("网络请求超时"));
        assert!(diagnostic.contains("request timed out"));
        assert!(diagnostic.contains("fallback HTTP"));
    }

    #[test]
    fn pc_lightweight_first_event_timeout_names_node_ack_gap() {
        let diagnostic = pc_lightweight_no_node_event_diagnostic("codex", "一龙4060（node-a）", 15);

        assert!(diagnostic.contains("Codex"));
        assert!(diagnostic.contains("一龙4060（node-a）"));
        assert!(diagnostic.contains("15 秒内没有返回任何 CLI 输出或完成事件"));
        assert!(diagnostic.contains("本轮已停止"));
    }

    #[test]
    fn pc_lightweight_chat_reply_delta_streams_growth_only() {
        let mut streamed = String::new();
        assert_eq!(
            lightweight_pc_reply_delta("OpenAI Codex\nmodel: test\ncodex\n说", true, &mut streamed)
                .as_deref(),
            Some("说")
        );
        assert_eq!(
            lightweight_pc_reply_delta(
                "OpenAI Codex\nmodel: test\ncodex\n说说看。",
                true,
                &mut streamed,
            )
            .as_deref(),
            Some("说看。")
        );
        assert_eq!(
            lightweight_pc_reply_delta(
                "OpenAI Codex\nmodel: test\ncodex\n说说看。",
                true,
                &mut streamed,
            ),
            None
        );
    }
}

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
const PC_CODEX_PROGRESS_HINT_COOLDOWN_SECS: u64 = 15;
pub use self::ai_cli_intent_gate::confirm_project_intent;

mod ai_cli_pc_config;
mod ai_cli_pc_reply_helpers;

use self::ai_cli_pc_config::{
    native_session_uuid, pc_agent_cli_recv_timeout_secs, pc_display_model_label,
    pc_lightweight_chat_reasoning_effort, pc_project_reasoning_effort, pc_route_a_extra_args,
    pc_runtime_full_access, should_skip_pc_chat_native_session,
};
use self::ai_cli_pc_reply_helpers::{
    abort_pc_progress, extract_codex_reconnect_attempt, extract_lightweight_pc_chat_reply,
    extract_lightweight_pc_chat_timeout_reply, extract_retry_fraction,
    is_pc_development_reply_boundary, is_pc_development_reply_noise_line,
    lightweight_pc_reply_delta, lightweight_reply_text_delta, no_readable_lightweight_reply,
    pc_cli_model_id, pc_cli_price_per_1k_credits, pc_cli_usage_tokens, pc_codex_progress_hint,
    pc_dispatch_started_event, sanitize_pc_development_reply, sanitize_user_reply_line,
};

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


#[cfg(test)]
#[path = "pc_cli_passthrough_tests.rs"]
mod pc_cli_passthrough_tests;

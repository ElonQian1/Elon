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
pub(crate) use self::pc_billing::{
    record_pc_cli_trusted_usage, reserve_pc_cli_billing_call, settle_pc_cli_node_usage,
};
pub(crate) use self::pc_cli_failure::{
    pc_cli_readable_output, pc_cli_terminal_error_message, pc_codex_error_output_can_complete,
};
pub(crate) use self::pc_dispatch_capture::{
    run_pc_agent_workspace_capture, PcAgentWorkspaceCaptureRequest, PcAgentWorkspaceCaptureResult,
};
pub(crate) use self::pc_passthrough_events::{
    pc_cli_passthrough_event, pc_cli_passthrough_events_flush, pc_cli_passthrough_events_from_chunk,
};
#[cfg(test)]
pub(crate) use self::pc_passthrough_reply::clean_codex_stream_chunk;
pub(crate) use self::pc_passthrough_reply::{
    extract_codex_reply, extract_marker_lightweight_reply, pc_lightweight_no_readable_diagnostic,
    pc_passthrough_empty_reply_diagnostic, sanitize_lightweight_pc_reply,
    strip_terminal_control_sequences,
};
pub(crate) use self::pc_prompt_acceptance::pc_lightweight_no_node_event_diagnostic;

pub(crate) use self::{
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

pub(crate) const DEFAULT_CHAT_RESUME_TIMEOUT_CAP_SECS: u64 = 12;
pub(crate) const DEFAULT_CHAT_FRESH_TIMEOUT_CAP_SECS: u64 = 20;
pub(crate) const PC_LIGHTWEIGHT_CHAT_FIRST_EVENT_TIMEOUT_SECS: u64 = 15;
pub(crate) const PC_LIGHTWEIGHT_CHAT_RECV_TIMEOUT_SECS: u64 = 120;
pub(crate) const PC_CODEX_PROGRESS_HINT_COOLDOWN_SECS: u64 = 15;
pub use self::ai_cli_intent_gate::confirm_project_intent;

mod ai_cli_pc_config;
mod ai_cli_pc_reply_helpers;

pub(crate) use self::ai_cli_pc_config::{
    native_session_uuid, pc_agent_cli_recv_timeout_secs, pc_display_model_label,
    pc_lightweight_chat_reasoning_effort, pc_project_reasoning_effort, pc_route_a_extra_args,
    pc_runtime_full_access, should_skip_pc_chat_native_session,
};
pub(crate) use self::ai_cli_pc_reply_helpers::{
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

mod workspace_mode;
use self::workspace_mode::run_with_workspace_mode;


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
mod ai_cli_pc_run;
pub(crate) use self::ai_cli_pc_run::run_via_pc_agent;

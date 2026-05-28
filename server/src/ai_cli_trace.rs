use serde_json::{json, Value};
use std::{path::Path, sync::Arc};

use crate::{
    ai_cli::{codex_thread_uri, CliOutput, NativeSessionScope},
    ai_cli_output::{extract_thread_id, truncate_chars},
    intent_router,
    types::{AiCliOption, AppState},
};

#[derive(Clone, Copy)]
pub(crate) struct CliTraceContext<'a> {
    pub(crate) state: &'a Arc<AppState>,
    pub(crate) trace_id: Option<&'a str>,
    pub(crate) operation: &'static str,
    pub(crate) attempt: &'static str,
    pub(crate) route: Option<intent_router::CapabilityRoute>,
    pub(crate) development_task: Option<bool>,
    pub(crate) prompt_bootstrapped: Option<bool>,
}

pub(crate) fn record_codex_network_gate(
    trace: CliTraceContext<'_>,
    option: &AiCliOption,
    status: &'static str,
    error: &str,
) {
    let Some(trace_id) = clean_trace_id_opt(trace.trace_id) else {
        return;
    };
    trace.state.server_traces.record(
        trace_id,
        "codex_network_gate",
        json!({
            "operation": trace.operation,
            "attempt": trace.attempt,
            "option_id": &option.id,
            "provider": &option.provider,
            "status": status,
            "error": truncate_chars(error, 500),
        }),
    );
}

pub(crate) fn record_cli_start(
    trace: CliTraceContext<'_>,
    option: &AiCliOption,
    workspace: &Path,
    prompt: &str,
    native_session_id: Option<&str>,
) {
    let Some(trace_id) = clean_trace_id_opt(trace.trace_id) else {
        return;
    };
    trace.state.server_traces.record(
        trace_id,
        "codex_cli_start",
        json!({
            "operation": trace.operation,
            "attempt": trace.attempt,
            "option_id": &option.id,
            "provider": &option.provider,
            "route": trace.route.map(|route| format!("{route:?}")),
            "development_task": trace.development_task,
            "prompt_bootstrapped": trace.prompt_bootstrapped,
            "prompt_chars": prompt.chars().count(),
            "prompt_bytes": prompt.len(),
            "native_session_hit": native_session_id.is_some(),
            "native_thread_uri": native_session_id.map(codex_thread_uri),
            "workspace": workspace.display().to_string(),
            "timeout_secs": option.timeout_secs,
        }),
    );
}

pub(crate) fn record_cli_done(
    trace: CliTraceContext<'_>,
    option: &AiCliOption,
    native_session_id: Option<&str>,
    output: &CliOutput,
    elapsed_ms: u128,
) {
    let Some(trace_id) = clean_trace_id_opt(trace.trace_id) else {
        return;
    };
    let thread_id = extract_thread_id(&output.stdout);
    trace.state.server_traces.record(
        trace_id,
        "codex_cli_done",
        json!({
            "operation": trace.operation,
            "attempt": trace.attempt,
            "option_id": &option.id,
            "provider": &option.provider,
            "route": trace.route.map(|route| format!("{route:?}")),
            "development_task": trace.development_task,
            "prompt_bootstrapped": trace.prompt_bootstrapped,
            "native_session_hit": native_session_id.is_some(),
            "native_thread_uri": native_session_id.map(codex_thread_uri),
            "new_thread_uri": thread_id.as_deref().map(codex_thread_uri),
            "success": output.success,
            "elapsed_ms": elapsed_ms,
            "stdout_bytes": output.stdout.len(),
            "stderr_bytes": output.stderr.len(),
            "stdout_chars": output.stdout.chars().count(),
            "stderr_chars": output.stderr.chars().count(),
        }),
    );
}

pub(crate) fn record_cli_error(
    trace: CliTraceContext<'_>,
    option: &AiCliOption,
    native_session_id: Option<&str>,
    error: &anyhow::Error,
    elapsed_ms: u128,
) {
    let Some(trace_id) = clean_trace_id_opt(trace.trace_id) else {
        return;
    };
    trace.state.server_traces.record(
        trace_id,
        "codex_cli_error",
        json!({
            "operation": trace.operation,
            "attempt": trace.attempt,
            "option_id": &option.id,
            "provider": &option.provider,
            "route": trace.route.map(|route| format!("{route:?}")),
            "development_task": trace.development_task,
            "prompt_bootstrapped": trace.prompt_bootstrapped,
            "native_session_hit": native_session_id.is_some(),
            "native_thread_uri": native_session_id.map(codex_thread_uri),
            "elapsed_ms": elapsed_ms,
            "error": error.to_string(),
        }),
    );
}

pub(crate) fn record_cli_retry(
    state: &Arc<AppState>,
    trace_id: Option<&str>,
    operation: &'static str,
    stale_session_id: Option<&str>,
    reason: &'static str,
) {
    let Some(trace_id) = clean_trace_id_opt(trace_id) else {
        return;
    };
    state.server_traces.record(
        trace_id,
        "codex_cli_retry",
        json!({
            "operation": operation,
            "reason": reason,
            "stale_thread_uri": stale_session_id.map(codex_thread_uri),
        }),
    );
}

pub(crate) fn record_cli_session_skipped(
    state: &Arc<AppState>,
    trace_id: Option<&str>,
    operation: &'static str,
    reason: &'static str,
) {
    let Some(trace_id) = clean_trace_id_opt(trace_id) else {
        return;
    };
    state.server_traces.record(
        trace_id,
        "codex_cli_session_skipped",
        json!({
            "operation": operation,
            "reason": reason,
        }),
    );
}

pub(crate) fn record_intent_gate_fallback(
    state: &Arc<AppState>,
    trace_id: Option<&str>,
    reason: &'static str,
    error: &str,
) {
    let Some(trace_id) = clean_trace_id_opt(trace_id) else {
        return;
    };
    state.server_traces.record(
        trace_id,
        "codex_intent_gate_fallback",
        json!({
            "reason": reason,
            "error": truncate_chars(error, 500),
        }),
    );
}

pub(crate) fn record_lightweight_chat_fallback(
    state: &Arc<AppState>,
    trace_id: Option<&str>,
    reason: &'static str,
    error: &str,
) {
    let Some(trace_id) = clean_trace_id_opt(trace_id) else {
        return;
    };
    state.server_traces.record(
        trace_id,
        "codex_lightweight_chat_fallback",
        json!({
            "reason": reason,
            "error": truncate_chars(error, 500),
        }),
    );
}

pub(crate) fn record_prewarm_session_hit(
    state: &Arc<AppState>,
    trace_id: Option<&str>,
    scope: &NativeSessionScope,
    workspace_key: &str,
    native_session_id: Option<&str>,
    elapsed_ms: u128,
) {
    let Some(trace_id) = clean_trace_id_opt(trace_id) else {
        return;
    };
    state.server_traces.record(
        trace_id,
        "codex_prewarm_session_hit",
        json!({
            "project_id": &scope.project_id,
            "user_id": &scope.user_id,
            "conversation_id": &scope.conversation_id,
            "workspace": workspace_key,
            "native_thread_uri": native_session_id.map(codex_thread_uri),
            "elapsed_ms": elapsed_ms,
        }),
    );
}

pub(crate) fn record_native_session_repair_event(
    state: &Arc<AppState>,
    trace_id: Option<&str>,
    phase: &'static str,
    details: Value,
) {
    let Some(trace_id) = clean_trace_id_opt(trace_id) else {
        return;
    };
    state.server_traces.record(trace_id, phase, details);
}

pub(crate) fn clean_trace_id_opt(trace_id: Option<&str>) -> Option<&str> {
    trace_id.map(str::trim).filter(|value| !value.is_empty())
}

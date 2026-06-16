//! Codex 原生会话失效检测与后台修复。
//!
//! 从 `ai_cli.rs` 抽出，按职责模块化：
//! - 判断 CLI 输出是否暗示原生 session 已失效
//! - 构造 continuity note，把旧 thread URI + 最近对话作为回退上下文
//! - 让失效 session 退役，并在后台跑一次新 session 安装

use anyhow::anyhow;
use serde_json::json;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use super::{
    ai_cli_output::{extract_thread_id, truncate_chars},
    ai_cli_prompts::build_native_session_repair_prompt,
    ai_cli_trace::{clean_trace_id_opt, record_native_session_repair_event, CliTraceContext},
    cap_option_timeout, codex_thread_uri, configured_timeout_cap, run_cli_command_traced,
    supports_codex_sessions, CliOutput, NativeSessionScope,
};
use crate::{
    intent_router,
    store::ConversationMessage,
    types::{AiCliOption, AppState},
};

pub(crate) const DEFAULT_SESSION_REPAIR_TIMEOUT_CAP_SECS: u64 = 25;
pub(crate) const DEFAULT_SESSION_REPAIR_COOLDOWN_SECS: u64 = 120;

pub(crate) fn should_retry_without_native_session(
    option: &AiCliOption,
    native_session_id: Option<&str>,
    output: &CliOutput,
) -> bool {
    if !supports_codex_sessions(option) || native_session_id.is_none() || output.success {
        return false;
    }
    let combined = format!("{}\n{}", output.stdout, output.stderr).to_ascii_lowercase();
    let mentions_session =
        combined.contains("session") || combined.contains("thread") || combined.contains("resume");
    let looks_stale = combined.contains("not found")
        || combined.contains("no such")
        || combined.contains("invalid")
        || combined.contains("expired")
        || combined.contains("unknown")
        || combined.contains("could not resume")
        || combined.contains("failed to resume");
    mentions_session && looks_stale
}

pub(crate) fn native_session_continuity_note(
    state: &Arc<AppState>,
    scope: Option<&NativeSessionScope>,
    stale_session_id: Option<&str>,
) -> Option<String> {
    let session_id = stale_session_id
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let scope = scope?;
    let recent_messages = state
        .store
        .list_recent_conversation_messages(&scope.project_id, Some(&scope.conversation_id), 8)
        .unwrap_or_default();
    Some(build_native_session_continuity_note(
        session_id,
        &recent_messages,
    ))
}

pub(crate) fn build_native_session_continuity_note(
    stale_session_id: &str,
    recent_messages: &[ConversationMessage],
) -> String {
    let thread_uri = codex_thread_uri(stale_session_id);
    let mut note = format!(
        "Previous Codex native thread became unavailable for direct resume.\nPrevious thread URI: {thread_uri}\nIf your environment can resolve Codex thread URIs, use that thread as continuity. If not, use the backend conversation records below as fallback context."
    );
    if !recent_messages.is_empty() {
        note.push_str("\n\nRecent backend conversation records:");
        for message in recent_messages {
            note.push_str(&format!(
                "\n- {}: {}",
                message.role,
                truncate_chars(message.content.trim(), 900)
            ));
        }
    }
    note
}

pub(crate) fn append_native_session_continuity(
    mut prompt: String,
    continuity_note: &str,
) -> String {
    prompt.push_str("\n\nNative session continuity handoff:\n");
    prompt.push_str(continuity_note);
    prompt
}

pub(crate) fn retire_native_session_and_schedule_repair(
    state: &Arc<AppState>,
    trace_id: Option<&str>,
    scope: Option<&NativeSessionScope>,
    option: &AiCliOption,
    workspace: &Path,
    workspace_key: &str,
    stale_session_id: Option<&str>,
    reason: &'static str,
    error: &str,
) {
    let (Some(scope), Some(session_id)) = (
        scope,
        stale_session_id
            .map(str::trim)
            .filter(|value| !value.is_empty()),
    ) else {
        return;
    };

    let _ = state.store.deactivate_native_agent_session(
        &scope.project_id,
        &scope.user_id,
        Some(&scope.conversation_id),
        &option.provider,
        &option.id,
        workspace_key,
        session_id,
    );

    schedule_background_native_session_repair(
        state,
        trace_id,
        scope.clone(),
        option.clone(),
        workspace.to_path_buf(),
        workspace_key.to_string(),
        session_id.to_string(),
        reason,
        error.to_string(),
    );
}

fn schedule_background_native_session_repair(
    state: &Arc<AppState>,
    trace_id: Option<&str>,
    scope: NativeSessionScope,
    option: AiCliOption,
    workspace: PathBuf,
    workspace_key: String,
    stale_session_id: String,
    reason: &'static str,
    error: String,
) {
    if !supports_codex_sessions(&option) {
        return;
    }
    let repair_key = format!(
        "native-session-repair:{}:{}:{}:{}:{}",
        scope.project_id, scope.user_id, scope.conversation_id, option.id, workspace_key
    );
    let state = state.clone();
    let trace_id = clean_trace_id_opt(trace_id).map(str::to_string);

    tokio::spawn(async move {
        let cooldown = Duration::from_secs(configured_timeout_cap(
            "AI_CLI_SESSION_REPAIR_COOLDOWN_SECS",
            DEFAULT_SESSION_REPAIR_COOLDOWN_SECS,
        ));
        if !state
            .codex_prewarm
            .start_if_allowed(&repair_key, cooldown)
            .await
        {
            record_native_session_repair_event(
                &state,
                trace_id.as_deref(),
                "codex_native_session_repair_skipped",
                json!({
                    "reason": "cooldown_or_active",
                    "stale_thread_uri": codex_thread_uri(&stale_session_id),
                    "trigger": reason,
                }),
            );
            return;
        }

        let result = async {
            record_native_session_repair_event(
                &state,
                trace_id.as_deref(),
                "codex_native_session_repair_start",
                json!({
                    "project_id": &scope.project_id,
                    "user_id": &scope.user_id,
                    "conversation_id": &scope.conversation_id,
                    "workspace": &workspace_key,
                    "stale_thread_uri": codex_thread_uri(&stale_session_id),
                    "trigger": reason,
                    "error": truncate_chars(&error, 500),
                }),
            );

            let recent_messages = state
                .store
                .list_recent_conversation_messages(
                    &scope.project_id,
                    Some(&scope.conversation_id),
                    10,
                )
                .unwrap_or_default();
            let mut repair_option = option.clone();
            cap_option_timeout(
                &mut repair_option,
                configured_timeout_cap(
                    "AI_CLI_SESSION_REPAIR_TIMEOUT_SECS",
                    DEFAULT_SESSION_REPAIR_TIMEOUT_CAP_SECS,
                ),
            );
            let prompt = build_native_session_repair_prompt(
                &workspace,
                &repair_option,
                &stale_session_id,
                &recent_messages,
            );
            let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<String>();
            let output = run_cli_command_traced(
                &repair_option,
                &workspace,
                &prompt,
                None,
                Some(scope.runtime_permission.as_str()),
                &tx,
                Some(CliTraceContext {
                    state: &state,
                    trace_id: trace_id.as_deref(),
                    operation: "native_session_repair",
                    attempt: "background_fresh",
                    route: Some(intent_router::CapabilityRoute::ChatAgent),
                    development_task: Some(false),
                    prompt_bootstrapped: Some(false),
                }),
            )
            .await?;

            let Some(thread_id) = extract_thread_id(&output.stdout) else {
                return Err(anyhow!(
                    "background repair did not return a Codex thread id"
                ));
            };
            let installed = state.store.upsert_native_agent_session_if_no_active(
                &scope.project_id,
                &scope.user_id,
                Some(&scope.conversation_id),
                &option.provider,
                &option.id,
                &workspace_key,
                &thread_id,
            )?;
            record_native_session_repair_event(
                &state,
                trace_id.as_deref(),
                "codex_native_session_repair_done",
                json!({
                    "project_id": &scope.project_id,
                    "user_id": &scope.user_id,
                    "conversation_id": &scope.conversation_id,
                    "workspace": &workspace_key,
                    "stale_thread_uri": codex_thread_uri(&stale_session_id),
                    "new_thread_uri": codex_thread_uri(&thread_id),
                    "installed": installed,
                    "stdout_chars": output.stdout.chars().count(),
                    "stderr_chars": output.stderr.chars().count(),
                }),
            );
            Ok::<(), anyhow::Error>(())
        }
        .await;

        let _ = state.codex_prewarm.finish(&repair_key).await;
        if let Err(error) = result {
            record_native_session_repair_event(
                &state,
                trace_id.as_deref(),
                "codex_native_session_repair_failed",
                json!({
                    "stale_thread_uri": codex_thread_uri(&stale_session_id),
                    "trigger": reason,
                    "error": truncate_chars(&error.to_string(), 500),
                }),
            );
        }
    });
}

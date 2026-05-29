//! Codex 意图门控（intent gate）：在执行前用 Codex 判断用户消息是否需要进入开发流程。
//!
//! 从 `ai_cli.rs` 中拆出来，专注于：
//! 1. 在 workspace 中调用 Codex 跑 `build_intent_gate_prompt`；
//! 2. 处理超时/启动错误/解析错误，所有失败都降级为聊天路由（不阻塞用户对话）；
//! 3. 维护 native session 状态（命中/失效重试/更新最新 thread_id）。

use anyhow::{anyhow, Result};
use std::{path::Path, sync::Arc};

use crate::{
    types::AppState,
};
use super::{
    IntentGateResult, NativeSessionScope,
    ai_cli_chat::intent_gate_fallback_chat_result,
    ai_cli_native_session::should_retry_without_native_session,
    ai_cli_output::{extract_thread_id, parse_intent_gate_result},
    ai_cli_process::{
        cap_option_timeout, configured_timeout_cap, is_cli_timeout_error, run_cli_command_traced,
        supports_codex_sessions,
    },
    ai_cli_prompts::build_intent_gate_prompt,
    ai_cli_trace::{record_cli_retry, record_intent_gate_fallback, CliTraceContext},
};

const DEFAULT_INTENT_GATE_TIMEOUT_CAP_SECS: u64 = 30;

pub async fn confirm_project_intent(
    workspace: &Path,
    user_message: &str,
    option_id: Option<&str>,
    native_session_scope: Option<NativeSessionScope>,
    trace_id: Option<&str>,
    state: &Arc<AppState>,
) -> Result<IntentGateResult> {
    let mut option = state
        .ai_cli
        .find_option(option_id)
        .cloned()
        .ok_or_else(|| anyhow!("未找到可用本地 AI CLI 选项"))?;
    cap_option_timeout(
        &mut option,
        configured_timeout_cap(
            "AI_CLI_INTENT_GATE_TIMEOUT_SECS",
            DEFAULT_INTENT_GATE_TIMEOUT_CAP_SECS,
        ),
    );
    if !supports_codex_sessions(&option) {
        return Err(anyhow!("当前阶段意图确认必须使用 Codex CLI"));
    }

    std::fs::create_dir_all(workspace)?;
    let workspace_key = workspace.display().to_string();
    let native_session_id = native_session_scope.as_ref().and_then(|scope| {
        state
            .store
            .get_native_agent_session(
                &scope.project_id,
                &scope.user_id,
                Some(&scope.conversation_id),
                &option.provider,
                &option.id,
                &workspace_key,
            )
            .ok()
            .flatten()
    });

    let prompt = build_intent_gate_prompt(workspace, user_message, &option);
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let mut output = match run_cli_command_traced(
        &option,
        workspace,
        &prompt,
        native_session_id.as_deref(),
        &tx,
        Some(CliTraceContext {
            state,
            trace_id,
            operation: "intent_gate",
            attempt: "initial",
            route: None,
            development_task: None,
            prompt_bootstrapped: None,
        }),
    )
    .await
    {
        Ok(output) => output,
        Err(error) if is_cli_timeout_error(&error) => {
            record_intent_gate_fallback(state, trace_id, "timeout", &error.to_string());
            return Ok(intent_gate_fallback_chat_result(user_message, "timeout"));
        }
        Err(error) => {
            // Step 6：意图门控不是阻塞条件。CLI 启动/运行任何错误都降级为 chat
            // 路由，让用户能先拿到反馈；错误记入 trace 供运维跟踪。
            record_intent_gate_fallback(state, trace_id, "cli_error", &error.to_string());
            return Ok(intent_gate_fallback_chat_result(user_message, "cli_error"));
        }
    };
    if should_retry_without_native_session(&option, native_session_id.as_deref(), &output) {
        record_cli_retry(
            state,
            trace_id,
            "intent_gate",
            native_session_id.as_deref(),
            "stale_native_session",
        );
        if let (Some(scope), Some(session_id)) =
            (native_session_scope.as_ref(), native_session_id.as_deref())
        {
            let _ = state.store.deactivate_native_agent_session(
                &scope.project_id,
                &scope.user_id,
                Some(&scope.conversation_id),
                &option.provider,
                &option.id,
                &workspace_key,
                session_id,
            );
        }
        output = match run_cli_command_traced(
            &option,
            workspace,
            &prompt,
            None,
            &tx,
            Some(CliTraceContext {
                state,
                trace_id,
                operation: "intent_gate",
                attempt: "fresh_after_stale",
                route: None,
                development_task: None,
                prompt_bootstrapped: None,
            }),
        )
        .await
        {
            Ok(output) => output,
            Err(error) if is_cli_timeout_error(&error) => {
                record_intent_gate_fallback(
                    state,
                    trace_id,
                    "fresh_after_stale_timeout",
                    &error.to_string(),
                );
                return Ok(intent_gate_fallback_chat_result(user_message, "timeout"));
            }
            Err(error) => {
                record_intent_gate_fallback(
                    state,
                    trace_id,
                    "fresh_after_stale_cli_error",
                    &error.to_string(),
                );
                return Ok(intent_gate_fallback_chat_result(user_message, "cli_error"));
            }
        };
    }

    if let (Some(scope), Some(thread_id)) = (
        native_session_scope.as_ref(),
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
    }

    parse_intent_gate_result(&output.stdout).or_else(|error| {
        // 解析失败：可能是 Codex 输出不是预期的 JSON，不要让整个请求崩。
        // 降级为 chat 路由，同时记入 fallback 供 trace 追踪。
        record_intent_gate_fallback(state, trace_id, "parse_error", &error.to_string());
        Ok(intent_gate_fallback_chat_result(
            user_message,
            "parse_error",
        ))
    })
}

use anyhow::{anyhow, Result};
use std::{path::Path, sync::Arc, time::Instant};
use tokio::sync::mpsc::UnboundedSender;

pub(crate) use crate::ai_cli_output::truncate_chars;
pub(crate) use crate::ai_cli_process::{
    cap_option_timeout, configured_timeout_cap, is_cli_timeout_error, run_cli_command_traced,
    supports_codex_sessions, CliOutput,
};
pub(crate) use crate::ai_cli_runner::{codex_exec_json_args, codex_resume_args, codex_thread_uri};

use crate::{
    ai_cli_chat::{
        chat_timeout_cap_secs, codex_network_or_timeout_error, finish_lightweight_chat_fallback,
        intent_gate_fallback_chat_result, is_tiny_chat_message,
    },
    ai_cli_environment::{ensure_git, environment_notes, looks_like_android_task},
    ai_cli_native_session::{
        append_native_session_continuity, native_session_continuity_note,
        retire_native_session_and_schedule_repair, should_retry_without_native_session,
    },
    ai_cli_output::{extract_thread_id, format_cli_reply, parse_intent_gate_result},
    ai_cli_prompts::{build_cli_prompt, build_intent_gate_prompt, build_prewarm_cli_prompt},
    ai_cli_trace::{
        record_cli_retry, record_cli_session_skipped, record_intent_gate_fallback,
        record_prewarm_session_hit, CliTraceContext,
    },
    intent_router, tools,
    types::{AiCliOption, AppState, WsMessage},
};

#[cfg(test)]
use crate::ai_cli_chat::{intent_gate_timeout_chat_result, DEFAULT_TINY_CHAT_TIMEOUT_CAP_SECS};
#[cfg(test)]
use crate::ai_cli_native_session::build_native_session_continuity_note;
#[cfg(test)]
use crate::ai_cli_output::extract_json_agent_message;
#[cfg(test)]
use crate::ai_cli_prompts::build_native_session_repair_prompt;
#[cfg(test)]
use crate::store::ConversationMessage;
#[cfg(test)]
use crate::types::CliPromptMode;

#[derive(Debug, Clone)]
pub struct NativeSessionScope {
    pub project_id: String,
    pub user_id: String,
    pub conversation_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IntentGateResult {
    pub route: intent_router::CapabilityRoute,
    pub confidence: f64,
    pub reason: String,
    pub chat_reply: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrewarmResult {
    pub reused: bool,
    pub thread_id: Option<String>,
    pub elapsed_ms: u128,
}

const DEFAULT_PREWARM_TIMEOUT_CAP_SECS: u64 = 8;
const DEFAULT_INTENT_GATE_TIMEOUT_CAP_SECS: u64 = 30;
const DEFAULT_CHAT_RESUME_TIMEOUT_CAP_SECS: u64 = 12;
const DEFAULT_CHAT_FRESH_TIMEOUT_CAP_SECS: u64 = 20;

impl IntentGateResult {
    pub fn should_enter_development(&self) -> bool {
        self.route == intent_router::CapabilityRoute::CodeAgent && self.confidence >= 0.75
    }
}

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

pub async fn prewarm_codex_session(
    workspace: &Path,
    option_id: Option<&str>,
    native_session_scope: NativeSessionScope,
    trace_id: Option<&str>,
    state: &Arc<AppState>,
) -> Result<PrewarmResult> {
    let started = Instant::now();
    let option = state
        .ai_cli
        .find_option(option_id)
        .cloned()
        .ok_or_else(|| anyhow!("no local AI CLI option is available"))?;
    if !supports_codex_sessions(&option) {
        return Err(anyhow!(
            "Codex CLI session prewarm requires a Codex CLI option"
        ));
    }

    std::fs::create_dir_all(workspace)?;
    let workspace_key = workspace.display().to_string();
    let existing_session_id = state.store.get_native_agent_session(
        &native_session_scope.project_id,
        &native_session_scope.user_id,
        Some(&native_session_scope.conversation_id),
        &option.provider,
        &option.id,
        &workspace_key,
    )?;
    if let Some(thread_id) = existing_session_id {
        record_prewarm_session_hit(
            state,
            trace_id,
            &native_session_scope,
            &workspace_key,
            Some(&thread_id),
            started.elapsed().as_millis(),
        );
        return Ok(PrewarmResult {
            reused: true,
            thread_id: Some(thread_id),
            elapsed_ms: started.elapsed().as_millis(),
        });
    }

    let mut prewarm_option = option.clone();
    cap_option_timeout(
        &mut prewarm_option,
        configured_timeout_cap(
            "AI_CLI_PREWARM_TIMEOUT_SECS",
            DEFAULT_PREWARM_TIMEOUT_CAP_SECS,
        ),
    );
    let prompt = build_prewarm_cli_prompt(workspace, &prewarm_option);
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let output = run_cli_command_traced(
        &prewarm_option,
        workspace,
        &prompt,
        None,
        &tx,
        Some(CliTraceContext {
            state,
            trace_id,
            operation: "prewarm",
            attempt: "initial",
            route: None,
            development_task: None,
            prompt_bootstrapped: None,
        }),
    )
    .await?;
    let thread_id = extract_thread_id(&output.stdout);
    if let Some(thread_id) = thread_id.as_deref() {
        let _ = state.store.upsert_native_agent_session_if_no_active(
            &native_session_scope.project_id,
            &native_session_scope.user_id,
            Some(&native_session_scope.conversation_id),
            &option.provider,
            &option.id,
            &workspace_key,
            thread_id,
        );
    }

    Ok(PrewarmResult {
        reused: false,
        thread_id,
        elapsed_ms: started.elapsed().as_millis(),
    })
}

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
    let mut option = state
        .ai_cli
        .find_option(option_id)
        .cloned()
        .ok_or_else(|| anyhow!("未找到可用本地 AI CLI 选项"))?;

    std::fs::create_dir_all(workspace)?;

    let development_task = route != intent_router::CapabilityRoute::ChatAgent
        || intent_router::looks_like_development_request(user_message);
    let lightweight_chat_task =
        route == intent_router::CapabilityRoute::ChatAgent && !development_task;
    let tiny_chat_task = lightweight_chat_task && is_tiny_chat_message(user_message);
    if lightweight_chat_task {
        cap_option_timeout(&mut option, chat_timeout_cap_secs(tiny_chat_task));
    }
    if development_task {
        ensure_git(workspace, user_id, require_existing_git)?;
    }

    let android_task = development_task && looks_like_android_task(user_message);
    if development_task {
        let _ = tx.send(
            WsMessage::Progress {
                message: "正在准备项目工作区。".into(),
            }
            .to_json(),
        );
        for note in environment_notes(user_message, &option) {
            let _ = tx.send(WsMessage::Progress { message: note }.to_json());
        }
        let _ = tx.send(
            WsMessage::Progress {
                message: "AI 助手正在处理你的请求。".into(),
            }
            .to_json(),
        );
    } else {
        let _ = tx.send(
            WsMessage::Progress {
                message: "正在思考。".into(),
            }
            .to_json(),
        );
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
            if development_task {
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
            WsMessage::Progress {
                message: "Restoring Codex CLI context for this conversation.".into(),
            }
            .to_json(),
        );
    }

    let mut prompt = build_cli_prompt(
        workspace,
        user_message,
        preflight_note,
        &option,
        route,
        prompt_bootstrapped,
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
                WsMessage::Progress {
                    message: "旧会话恢复超时，已切到新会话继续；旧上下文会在后台整理。".into(),
                }
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
                    return finish_lightweight_chat_fallback(
                        state,
                        trace_id,
                        tx,
                        user_message,
                        "fresh_after_resume_error_cli_error",
                        &fresh_error.to_string(),
                    );
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
            return finish_lightweight_chat_fallback(
                state,
                trace_id,
                tx,
                user_message,
                "initial_cli_error",
                &error.to_string(),
            );
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
            WsMessage::Progress {
                message: if lightweight_chat_task {
                    "旧会话不可用，已切到新会话继续；旧上下文会在后台整理。".into()
                } else {
                    "Codex CLI session expired; starting a fresh session.".into()
                },
            }
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
                return finish_lightweight_chat_fallback(
                    state,
                    trace_id,
                    tx,
                    user_message,
                    "fresh_after_stale_cli_error",
                    &error.to_string(),
                );
            }
            Err(error) => return Err(error),
        };
    }

    if supports_codex_sessions(&option) && !output.success {
        let combined = format!("{}\n{}", output.stdout, output.stderr);
        if crate::codex_health::is_codex_network_error_text(&combined) {
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
    if output.success {
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
    let reply = format_cli_reply(&output.stdout, &output.stderr, output.success);
    tracing::info!(
        route = ?route,
        development_task,
        elapsed_ms = started.elapsed().as_millis(),
        "local AI CLI request completed"
    );

    let apk_url = if android_task && output.success {
        let _ = tx.send(
            WsMessage::Progress {
                message: "AI 已完成处理，正在查找 APK 安装包。".into(),
            }
            .to_json(),
        );
        let apk_url =
            tools::find_latest_apk(workspace).map(|_| tools::stable_apk_url(download_base));
        if apk_url.is_none() {
            let _ = tx.send(
                WsMessage::Progress {
                    message: "未找到 APK 安装包；如果刚才是在打包，请检查最终回复里的失败原因。"
                        .into(),
                }
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
        }
        .to_json(),
    );

    Ok(())
}

#[cfg(test)]
#[path = "ai_cli_tests.rs"]
mod tests;

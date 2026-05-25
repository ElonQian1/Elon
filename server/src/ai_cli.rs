use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::{
    path::Path,
    path::PathBuf,
    process::Stdio,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    process::Command,
    sync::mpsc::UnboundedSender,
};

use crate::{
    ai_cli_prompts::{
        build_cli_prompt, build_intent_gate_prompt, build_native_session_repair_prompt,
        build_prewarm_cli_prompt,
    },
    intent_router,
    store::ConversationMessage,
    tools,
    types::{AiCliOption, AppState, CliPromptMode, WsMessage},
};

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
const DEFAULT_CHAT_TIMEOUT_CAP_SECS: u64 = 30;
const DEFAULT_TINY_CHAT_TIMEOUT_CAP_SECS: u64 = 8;
const DEFAULT_CHAT_RESUME_TIMEOUT_CAP_SECS: u64 = 12;
const DEFAULT_CHAT_FRESH_TIMEOUT_CAP_SECS: u64 = 20;
const DEFAULT_SESSION_REPAIR_TIMEOUT_CAP_SECS: u64 = 25;
const DEFAULT_SESSION_REPAIR_COOLDOWN_SECS: u64 = 120;

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

struct CliOutput {
    success: bool,
    stdout: String,
    stderr: String,
}

#[derive(Clone, Copy)]
struct CliTraceContext<'a> {
    state: &'a Arc<AppState>,
    trace_id: Option<&'a str>,
    operation: &'static str,
    attempt: &'static str,
    route: Option<intent_router::CapabilityRoute>,
    development_task: Option<bool>,
    prompt_bootstrapped: Option<bool>,
}

fn supports_codex_sessions(option: &AiCliOption) -> bool {
    option.provider.eq_ignore_ascii_case("codex")
        || option.id.to_ascii_lowercase().contains("codex")
        || option
            .bin
            .rsplit(|c| c == '/' || c == '\\')
            .next()
            .map(|bin| bin.eq_ignore_ascii_case("codex"))
            .unwrap_or(false)
}

fn configured_timeout_cap(env_name: &str, default_secs: u64) -> u64 {
    std::env::var(env_name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|secs| (1..=3600).contains(secs))
        .unwrap_or(default_secs)
}

fn cap_option_timeout(option: &mut AiCliOption, cap_secs: u64) {
    let cap_secs = cap_secs.max(1);
    if option.timeout_secs == 0 || option.timeout_secs > cap_secs {
        option.timeout_secs = cap_secs;
    }
}

fn chat_timeout_cap_secs(tiny_chat_task: bool) -> u64 {
    if tiny_chat_task {
        configured_timeout_cap(
            "AI_CLI_TINY_CHAT_TIMEOUT_SECS",
            DEFAULT_TINY_CHAT_TIMEOUT_CAP_SECS,
        )
    } else {
        configured_timeout_cap("AI_CLI_CHAT_TIMEOUT_SECS", DEFAULT_CHAT_TIMEOUT_CAP_SECS)
    }
}

fn is_tiny_chat_message(user_message: &str) -> bool {
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
    if compact.is_empty() {
        return false;
    }
    let compact_chars = compact.chars().count();
    if compact_chars <= 2 {
        return true;
    }
    matches!(
        compact.as_str(),
        "你好"
            | "您好"
            | "嗨"
            | "哈喽"
            | "哈啰"
            | "在吗"
            | "在嘛"
            | "早"
            | "早上好"
            | "晚上好"
            | "hi"
            | "hello"
            | "hey"
            | "yo"
    ) || (compact_chars <= 4
        && (compact.contains("你好")
            || compact.contains("您好")
            || compact.contains("哈喽")
            || compact.contains("在吗")))
}

fn is_cli_timeout_error(error: &anyhow::Error) -> bool {
    let text = error.to_string().to_ascii_lowercase();
    text.contains("timeout") || text.contains("timed out") || text.contains("执行超时")
}

fn codex_network_or_timeout_error(error: &anyhow::Error) -> bool {
    is_cli_timeout_error(error)
        || crate::codex_health::is_codex_network_error_text(&error.to_string())
}

#[cfg(test)]
fn intent_gate_timeout_chat_result(user_message: &str) -> IntentGateResult {
    intent_gate_fallback_chat_result(user_message, "timeout")
}

/// 意图门控降级到普通聊天路由。
fn intent_gate_fallback_chat_result(user_message: &str, cause: &str) -> IntentGateResult {
    let chat_reply = if is_tiny_chat_message(user_message) {
        "你好，我在。刚才服务端意图确认环节没完成，我先按普通聊天处理，避免误进入慢速开发流程。"
    } else {
        "我先按普通聊天处理，避免误进入慢速开发流程。你可以继续说；如果要我修改代码、编译或发布，请直接告诉我具体任务。"
    };
    IntentGateResult {
        route: intent_router::CapabilityRoute::ChatAgent,
        confidence: 0.5,
        reason: format!("intent_gate_fallback:{}", cause),
        chat_reply: Some(chat_reply.into()),
    }
}

fn finish_lightweight_chat_fallback(
    state: &Arc<AppState>,
    trace_id: Option<&str>,
    tx: &UnboundedSender<String>,
    user_message: &str,
    reason: &'static str,
    error: &str,
) -> Result<()> {
    record_lightweight_chat_fallback(state, trace_id, reason, error);
    let message = if is_tiny_chat_message(user_message) {
        "你好，我在。刚才服务端 Codex CLI 会话响应超过轻量聊天限时，我先结束本轮，避免手机一直卡住；你继续发消息就可以。"
    } else {
        "这次服务端 Codex CLI 没有在轻量聊天限时内返回结果。我已经结束本轮，避免手机一直等待；你可以继续发消息，或直接说要进入开发流程检查原因。"
    };
    let _ = tx.send(
        WsMessage::Done {
            message: message.into(),
            apk_url: None,
            image_url: None,
        }
        .to_json(),
    );
    Ok(())
}

fn cli_args_for_run(option: &AiCliOption, native_session_id: Option<&str>) -> Vec<String> {
    if !supports_codex_sessions(option) {
        return option.args.clone();
    }
    if let Some(session_id) = native_session_id {
        if let Some(args) = codex_resume_args(&option.args, session_id) {
            return args;
        }
    }
    codex_exec_json_args(&option.args)
}

fn codex_exec_json_args(raw_args: &[String]) -> Vec<String> {
    let mut args = raw_args.to_vec();
    if args.iter().any(|arg| arg == "--json") {
        return args;
    }
    if let Some(exec_index) = args.iter().position(|arg| arg == "exec" || arg == "e") {
        args.insert(exec_index + 1, "--json".into());
    }
    args
}

fn codex_resume_args(raw_args: &[String], session_id: &str) -> Option<Vec<String>> {
    let exec_index = raw_args
        .iter()
        .position(|arg| arg == "exec" || arg == "e")?;
    let mut args = raw_args[..exec_index].to_vec();
    args.push("exec".into());
    args.push("resume".into());

    let mut has_json = false;
    let mut iter = raw_args[exec_index + 1..].iter().peekable();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--json" => {
                has_json = true;
                args.push(arg.clone());
            }
            "--skip-git-repo-check"
            | "--ignore-user-config"
            | "--ignore-rules"
            | "--strict-config"
            | "--dangerously-bypass-approvals-and-sandbox"
            | "--dangerously-bypass-hook-trust" => args.push(arg.clone()),
            "-m" | "--model" | "-c" | "--config" | "-p" | "--profile" | "--profile-v2"
            | "--output-schema" => {
                args.push(arg.clone());
                if let Some(value) = iter.next() {
                    args.push(value.clone());
                }
            }
            _ => {}
        }
    }
    if !has_json {
        args.push("--json".into());
    }
    args.push(session_id.to_string());
    Some(args)
}

async fn run_cli_command_traced(
    option: &AiCliOption,
    workspace: &Path,
    prompt: &str,
    native_session_id: Option<&str>,
    tx: &UnboundedSender<String>,
    trace: Option<CliTraceContext<'_>>,
) -> Result<CliOutput> {
    let trace_started = Instant::now();
    if supports_codex_sessions(option) {
        if let Some(trace) = trace {
            if let Err(error) = trace
                .state
                .codex_network
                .ensure_ready(trace.operation)
                .await
            {
                record_codex_network_gate(trace, option, "blocked", &error);
                return Err(anyhow!(error));
            }
        }
    }
    if let Some(trace) = trace {
        record_cli_start(trace, option, workspace, prompt, native_session_id);
    }
    let result = run_cli_command(option, workspace, prompt, native_session_id, tx).await;
    if let Some(trace) = trace {
        match &result {
            Ok(output) => record_cli_done(
                trace,
                option,
                native_session_id,
                output,
                trace_started.elapsed().as_millis(),
            ),
            Err(error) => record_cli_error(
                trace,
                option,
                native_session_id,
                error,
                trace_started.elapsed().as_millis(),
            ),
        }
        if supports_codex_sessions(option) {
            match &result {
                Ok(output) if output.success => {
                    trace
                        .state
                        .codex_network
                        .mark_cli_success("codex_cli_success")
                        .await;
                }
                Ok(output) => {
                    let combined = format!("{}\n{}", output.stdout, output.stderr);
                    if crate::codex_health::is_codex_network_error_text(&combined) {
                        trace
                            .state
                            .codex_network
                            .mark_cli_failure("codex_cli_output", &combined)
                            .await;
                    }
                }
                Err(error) => {
                    let text = error.to_string();
                    if is_cli_timeout_error(error)
                        || crate::codex_health::is_codex_network_error_text(&text)
                    {
                        trace
                            .state
                            .codex_network
                            .mark_cli_failure("codex_cli_error", &text)
                            .await;
                    }
                }
            }
        }
    }
    result
}

fn record_codex_network_gate(
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

fn record_cli_start(
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

fn record_cli_done(
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

fn record_cli_error(
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

fn record_cli_retry(
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

fn record_cli_session_skipped(
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

fn record_intent_gate_fallback(
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

fn record_lightweight_chat_fallback(
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

fn record_prewarm_session_hit(
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

async fn run_cli_command(
    option: &AiCliOption,
    workspace: &Path,
    prompt: &str,
    native_session_id: Option<&str>,
    tx: &UnboundedSender<String>,
) -> Result<CliOutput> {
    let mut cmd = Command::new(&option.bin);
    let args = cli_args_for_run(option, native_session_id);
    cmd.args(&args)
        .current_dir(workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    {
        cmd.process_group(0);
    }

    match option.prompt_mode {
        CliPromptMode::Arg => {
            cmd.stdin(Stdio::null());
            cmd.arg(prompt);
        }
        CliPromptMode::Stdin => {
            cmd.stdin(Stdio::piped());
        }
    }

    let mut child = cmd.spawn().map_err(|e| {
        anyhow!(
            "启动本地 AI CLI 失败: {}。请检查选项 '{}' 的 bin/args 配置",
            e,
            option.id
        )
    })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("无法读取本地 AI CLI stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow!("无法读取本地 AI CLI stderr"))?;

    // last_activity_ms 记录 CLI 最近一次 stdout/stderr 出行的时间戳（毫秒）。
    // 心跳任务依赖这个反馈判断是否 CLI 静默，避免在 CLI 还在
    // 正常输出时发废话。
    let now_ms = current_unix_millis();
    let last_activity_ms = Arc::new(AtomicU64::new(now_ms));

    let stdout_task = tokio::spawn(read_cli_stream(
        stdout,
        Some(last_activity_ms.clone()),
        Some(tx.clone()),
    ));
    let stderr_task = tokio::spawn(read_cli_stream(
        stderr,
        Some(last_activity_ms.clone()),
        None,
    ));
    let heartbeat_task = tokio::spawn(send_cli_heartbeat(tx.clone(), last_activity_ms.clone()));

    if option.prompt_mode == CliPromptMode::Stdin {
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(prompt.as_bytes()).await?;
        }
    }

    let status =
        match tokio::time::timeout(Duration::from_secs(option.timeout_secs), child.wait()).await {
            Ok(result) => result?,
            Err(_) => {
                heartbeat_task.abort();
                stdout_task.abort();
                stderr_task.abort();
                kill_timed_out_child(&mut child).await;
                return Err(anyhow!(
                    "本地 AI CLI 执行超时，请稍后重试或调大对应 TIMEOUT_SECS"
                ));
            }
        };
    heartbeat_task.abort();

    let stdout = stdout_task.await.unwrap_or_default();
    let stderr = stderr_task.await.unwrap_or_default();

    Ok(CliOutput {
        success: status.success(),
        stdout,
        stderr,
    })
}

async fn kill_timed_out_child(child: &mut tokio::process::Child) {
    #[cfg(unix)]
    {
        if let Some(pid) = child.id() {
            let process_group = format!("-{}", pid);
            let _ = Command::new("kill")
                .args(["-TERM", &process_group])
                .status()
                .await;
            tokio::time::sleep(Duration::from_millis(250)).await;
            let _ = Command::new("kill")
                .args(["-KILL", &process_group])
                .status()
                .await;
        }
    }
    let _ = child.kill().await;
}

fn should_retry_without_native_session(
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

fn native_session_continuity_note(
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

fn build_native_session_continuity_note(
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

fn append_native_session_continuity(mut prompt: String, continuity_note: &str) -> String {
    prompt.push_str("\n\nNative session continuity handoff:\n");
    prompt.push_str(continuity_note);
    prompt
}

fn retire_native_session_and_schedule_repair(
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

fn record_native_session_repair_event(
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

pub(crate) fn codex_thread_uri(session_id: &str) -> String {
    let session_id = session_id.trim();
    if session_id.starts_with("codex://threads/") {
        session_id.to_string()
    } else {
        format!("codex://threads/{session_id}")
    }
}

fn clean_trace_id_opt(trace_id: Option<&str>) -> Option<&str> {
    trace_id.map(str::trim).filter(|value| !value.is_empty())
}

async fn send_cli_heartbeat(tx: UnboundedSender<String>, last_activity_ms: Arc<AtomicU64>) {
    // CLI 静默超过这个阈值才发心跳；CLI 还在出 stdout 时不干扰。
    const SILENCE_THRESHOLD: Duration = Duration::from_secs(20);
    // 检查频率：足够频繁到能及时发出心跳，但不过于浪费调度。
    const TICK_INTERVAL: Duration = Duration::from_secs(5);
    let started_at = Instant::now();
    let mut last_heartbeat: Option<Instant> = None;
    loop {
        tokio::time::sleep(TICK_INTERVAL).await;
        let now_ms = current_unix_millis();
        let last_ms = last_activity_ms.load(Ordering::Relaxed);
        let silence = Duration::from_millis(now_ms.saturating_sub(last_ms));
        if silence < SILENCE_THRESHOLD {
            continue;
        }
        // 静默期间最多每 15s 重发一次，避免刷屏。
        if let Some(prev) = last_heartbeat {
            if prev.elapsed() < Duration::from_secs(15) {
                continue;
            }
        }
        let elapsed_secs = started_at.elapsed().as_secs();
        let silence_secs = silence.as_secs();
        let message = if silence_secs < 60 {
            format!("AI 还在思考（已等待 {} 秒）…", elapsed_secs)
        } else {
            format!(
                "AI 还在后台处理（已等待 {} 秒，本轮已静默 {} 秒）…",
                elapsed_secs, silence_secs
            )
        };
        if tx.send(WsMessage::Progress { message }.to_json()).is_err() {
            break;
        }
        last_heartbeat = Some(Instant::now());
    }
}

fn current_unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_exec_args_enable_json_output() {
        let args = vec![
            "exec".to_string(),
            "--sandbox".to_string(),
            "workspace-write".to_string(),
            "--skip-git-repo-check".to_string(),
        ];

        assert_eq!(
            codex_exec_json_args(&args),
            vec![
                "exec",
                "--json",
                "--sandbox",
                "workspace-write",
                "--skip-git-repo-check"
            ]
        );
    }

    #[test]
    fn codex_resume_args_keep_supported_options() {
        let args = vec![
            "-m".to_string(),
            "gpt-5".to_string(),
            "exec".to_string(),
            "--sandbox".to_string(),
            "workspace-write".to_string(),
            "--skip-git-repo-check".to_string(),
        ];

        assert_eq!(
            codex_resume_args(&args, "thread-1").unwrap(),
            vec![
                "-m",
                "gpt-5",
                "exec",
                "resume",
                "--skip-git-repo-check",
                "--json",
                "thread-1"
            ]
        );
    }

    #[test]
    fn extracts_codex_json_thread_and_answer() {
        let stdout = r#"{"type":"thread.started","thread_id":"thread-1"}
{"type":"item.completed","item":{"type":"agent_message","text":"hello"}}"#;

        assert_eq!(extract_thread_id(stdout).as_deref(), Some("thread-1"));
        assert_eq!(extract_json_agent_message(stdout).as_deref(), Some("hello"));
    }

    fn test_option() -> AiCliOption {
        AiCliOption {
            id: "codex_cli".into(),
            label: "Codex CLI".into(),
            provider: "codex".into(),
            model: None,
            bin: "codex".into(),
            args: vec!["exec".into(), "--skip-git-repo-check".into()],
            prompt_mode: CliPromptMode::Arg,
            timeout_secs: 60,
        }
    }

    #[test]
    fn chat_prompt_uses_lightweight_mode() {
        let prompt = build_cli_prompt(
            Path::new("D:/tmp/project"),
            "你好，随便聊聊",
            None,
            &test_option(),
            intent_router::CapabilityRoute::ChatAgent,
            false,
        );

        assert!(prompt.contains("轻量聊天模式"));
        assert!(!prompt.contains("通用项目工作流必须始终执行"));
        assert!(!prompt.contains("git pull --rebase"));
    }

    #[test]
    fn prewarm_prompt_does_not_enter_project_workflow() {
        let prompt = build_prewarm_cli_prompt(Path::new("D:/tmp/project"), &test_option());

        assert!(prompt.contains("prewarming a Codex CLI native session"));
        assert!(prompt.contains("Do not inspect files"));
        assert!(!prompt.contains("git pull --rebase"));
        assert!(!prompt.contains("General project workflow"));
    }

    #[test]
    fn development_prompt_keeps_project_workflow() {
        let prompt = build_cli_prompt(
            Path::new("D:/tmp/project"),
            "帮我修改 APK 并发布新版",
            None,
            &test_option(),
            intent_router::CapabilityRoute::CodeAgent,
            false,
        );

        assert!(prompt.contains("通用项目工作流必须始终执行"));
        assert!(prompt.contains("git pull --rebase"));
        assert!(prompt.contains("scripts/publish-apk.ps1"));
        assert!(prompt.contains("不要 rebase 后继续上传旧 APK"));
        assert!(prompt.contains("服务器为本 APK 会话创建的 worktree/分支"));
        assert!(prompt.contains("服务器会在任务完成后串行合并回项目主分支"));
    }

    #[test]
    fn development_prompt_includes_preflight_note() {
        let prompt = build_cli_prompt(
            Path::new("D:/tmp/project"),
            "继续完成刚才的修改",
            Some("git pull 未成功（error: cannot pull with rebase: You have unstaged changes.）"),
            &test_option(),
            intent_router::CapabilityRoute::CodeAgent,
            false,
        );

        assert!(prompt.contains("项目预检结果"));
        assert!(prompt.contains("这不是最终失败"));
        assert!(prompt.contains("不要反复盲目执行同一个失败命令"));
    }

    #[test]
    fn resumed_chat_prompt_is_short() {
        let prompt = build_cli_prompt(
            Path::new("D:/tmp/project"),
            "继续聊这个思路",
            None,
            &test_option(),
            intent_router::CapabilityRoute::ChatAgent,
            true,
        );

        assert!(prompt.contains("Continue the existing Codex CLI native session"));
        assert!(prompt.contains("lightweight chat"));
        assert!(!prompt.contains("git pull --rebase"));
    }

    #[test]
    fn resumed_development_prompt_reuses_bootstrap_rules() {
        let prompt = build_cli_prompt(
            Path::new("D:/tmp/project"),
            "继续发布新版",
            Some("git status is clean"),
            &test_option(),
            intent_router::CapabilityRoute::CodeAgent,
            true,
        );

        assert!(prompt.contains("full development workflow was already injected"));
        assert!(prompt.contains("git status is clean"));
        assert!(!prompt.contains("开始执行前"));
    }

    #[test]
    fn stale_codex_session_output_triggers_fresh_retry() {
        let output = CliOutput {
            success: false,
            stdout: String::new(),
            stderr: "Error: could not resume session thread-1: not found".into(),
        };

        assert!(should_retry_without_native_session(
            &test_option(),
            Some("thread-1"),
            &output
        ));
        assert!(!should_retry_without_native_session(
            &test_option(),
            None,
            &output
        ));
    }

    #[test]
    fn tiny_chat_messages_use_fast_path() {
        assert!(is_tiny_chat_message("你好"));
        assert!(is_tiny_chat_message("你好！"));
        assert!(is_tiny_chat_message("hello"));
        assert!(is_tiny_chat_message("在吗"));
        assert!(!is_tiny_chat_message("你好，帮我发布新版 APK"));
        assert!(!is_tiny_chat_message("继续修复刚才的构建问题"));
    }

    #[test]
    fn timeout_caps_never_expand_cli_timeout() {
        let mut option = test_option();
        option.timeout_secs = 1800;
        cap_option_timeout(&mut option, DEFAULT_TINY_CHAT_TIMEOUT_CAP_SECS);
        assert_eq!(option.timeout_secs, DEFAULT_TINY_CHAT_TIMEOUT_CAP_SECS);

        let mut short_option = test_option();
        short_option.timeout_secs = 3;
        cap_option_timeout(&mut short_option, DEFAULT_TINY_CHAT_TIMEOUT_CAP_SECS);
        assert_eq!(short_option.timeout_secs, 3);
    }

    #[test]
    fn intent_gate_timeout_defaults_to_chat() {
        let result = intent_gate_timeout_chat_result("你好");

        assert_eq!(result.route, intent_router::CapabilityRoute::ChatAgent);
        assert!(!result.should_enter_development());
        assert!(result.chat_reply.unwrap().contains("普通聊天"));
    }

    #[test]
    fn continuity_note_uses_codex_thread_uri_and_recent_messages() {
        let note = build_native_session_continuity_note(
            "019e55ee-81fb-7c03-98d9-957ba60739ca",
            &[
                ConversationMessage {
                    role: "user".into(),
                    content: "我们刚才在讨论普通聊天加速".into(),
                },
                ConversationMessage {
                    role: "assistant".into(),
                    content: "已经建议 session 预热和短 prompt".into(),
                },
            ],
        );

        assert!(note.contains("codex://threads/019e55ee-81fb-7c03-98d9-957ba60739ca"));
        assert!(note.contains("普通聊天加速"));
        assert!(note.contains("短 prompt"));
    }

    #[test]
    fn repair_prompt_creates_background_summary_without_project_workflow() {
        let prompt = build_native_session_repair_prompt(
            Path::new("D:/tmp/project"),
            &test_option(),
            "thread-1",
            &[ConversationMessage {
                role: "assistant".into(),
                content: "已经完成轻量聊天限时修复，剩余后台恢复摘要接力。".into(),
            }],
        );

        assert!(prompt.contains("background recovery job"));
        assert!(prompt.contains("codex://threads/thread-1"));
        assert!(prompt.contains("compact continuity summary"));
        assert!(prompt.contains("后台恢复摘要接力"));
        assert!(prompt.contains("Do not inspect files"));
        assert!(!prompt.contains("git pull --rebase"));
        assert!(!prompt.contains("通用项目工作流必须始终执行"));
    }

    #[test]
    fn parses_intent_gate_chat_result() {
        let stdout = r#"{"type":"item.completed","item":{"type":"agent_message","text":"{\"route\":\"chat\",\"confidence\":0.93,\"reason\":\"只是询问流程\",\"chat_reply\":\"先聊清楚也可以。\"}"}}"#;
        let result = parse_intent_gate_result(stdout).unwrap();

        assert_eq!(result.route, intent_router::CapabilityRoute::ChatAgent);
        assert_eq!(result.chat_reply.as_deref(), Some("先聊清楚也可以。"));
        assert!(!result.should_enter_development());
    }

    #[test]
    fn parses_intent_gate_development_result() {
        let stdout = r#"{"route":"development","confidence":0.91,"reason":"明确要求修改代码","chat_reply":""}"#;
        let result = parse_intent_gate_result(stdout).unwrap();

        assert_eq!(result.route, intent_router::CapabilityRoute::CodeAgent);
        assert!(result.should_enter_development());
    }
}

async fn read_cli_stream<R>(
    reader: R,
    last_activity_ms: Option<Arc<AtomicU64>>,
    progress_tx: Option<UnboundedSender<String>>,
) -> String
where
    R: AsyncRead + Unpin,
{
    let mut lines = BufReader::new(reader).lines();
    let mut collected = String::new();

    while let Ok(Some(line)) = lines.next_line().await {
        if let Some(ts) = last_activity_ms.as_ref() {
            ts.store(current_unix_millis(), Ordering::Relaxed);
        }
        if let Some(tx) = progress_tx.as_ref() {
            for message in crate::codex_stream::stream_event_to_ws_messages(&line) {
                let _ = tx.send(message);
            }
        }
        collected.push_str(&line);
        collected.push('\n');
    }

    collected
}

fn parse_intent_gate_result(stdout: &str) -> Result<IntentGateResult> {
    let text = extract_json_agent_message(stdout).unwrap_or_else(|| stdout.trim().to_string());
    let value = parse_json_object_from_text(&text)
        .ok_or_else(|| anyhow!("Codex CLI 意图确认没有返回有效 JSON"))?;
    let route_text = value
        .get("route")
        .and_then(Value::as_str)
        .unwrap_or("chat")
        .trim()
        .to_ascii_lowercase();
    let route = match route_text.as_str() {
        "development" | "code" | "codeagent" | "dev" => intent_router::CapabilityRoute::CodeAgent,
        _ => intent_router::CapabilityRoute::ChatAgent,
    };
    let confidence = value
        .get("confidence")
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    let reason = value
        .get("reason")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let chat_reply = value
        .get("chat_reply")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|reply| !reply.is_empty())
        .map(ToOwned::to_owned);

    Ok(IntentGateResult {
        route,
        confidence,
        reason,
        chat_reply,
    })
}

fn parse_json_object_from_text(text: &str) -> Option<Value> {
    serde_json::from_str::<Value>(text).ok().or_else(|| {
        let start = text.find('{')?;
        let end = text.rfind('}')?;
        serde_json::from_str::<Value>(&text[start..=end]).ok()
    })
}

fn ensure_git(workspace: &Path, user_id: &str, require_existing_git: bool) -> Result<()> {
    if workspace.join(".git").exists() && has_origin_remote(workspace) {
        return Ok(());
    }

    if require_existing_git {
        return Err(anyhow!(
            "当前项目被标记为 Git/local_path 项目，但工作目录 {} 不是带 origin 远端的 Git 仓库。请先把它设置成真实 git clone（包含 .git 和 origin/main），再让 AI 修改。",
            workspace.display()
        ));
    }

    let _ = std::process::Command::new("git")
        .args(["init"])
        .current_dir(workspace)
        .output();
    let _ = std::process::Command::new("git")
        .args(["config", "user.email", &format!("{}@elon.app", user_id)])
        .current_dir(workspace)
        .output();
    let _ = std::process::Command::new("git")
        .args(["config", "user.name", user_id])
        .current_dir(workspace)
        .output();

    Ok(())
}

fn has_origin_remote(workspace: &Path) -> bool {
    std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(workspace)
        .output()
        .map(|output| output.status.success() && !output.stdout.is_empty())
        .unwrap_or(false)
}

fn environment_notes(user_message: &str, option: &AiCliOption) -> Vec<String> {
    let mut notes = Vec::new();
    if looks_like_android_task(user_message) {
        if option.bin.contains("codex") && !codex_auth_configured() {
            notes.push("环境提醒：AI CLI 登录状态异常，可能会自动切换备用代理。".into());
        }
        if !command_available("git") {
            notes.push("环境提醒：服务器未检测到 git，项目保存可能失败。".into());
        }
        if !command_available("java") {
            notes.push("环境提醒：服务器未检测到 java，Android Gradle 构建会失败。".into());
        }
        if !android_sdk_configured() {
            notes.push(
                "环境提醒：服务器未检测到 Android SDK，请先安装 SDK 后再稳定打包 APK。".into(),
            );
        }
    }
    notes
}

fn codex_auth_configured() -> bool {
    if std::env::var("OPENAI_API_KEY")
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
    {
        return true;
    }

    let codex_home = std::env::var("CODEX_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|home| PathBuf::from(home).join(".codex"))
        });

    codex_home
        .map(|home| home.join("auth.json").exists())
        .unwrap_or(false)
}

fn command_available(bin: &str) -> bool {
    std::process::Command::new(bin)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn android_sdk_configured() -> bool {
    let candidates = [
        std::env::var("ANDROID_HOME").ok(),
        std::env::var("ANDROID_SDK_ROOT").ok(),
        Some("/root/android-sdk".into()),
        Some("/opt/android-sdk".into()),
    ];

    candidates
        .into_iter()
        .flatten()
        .map(PathBuf::from)
        .any(|path| path.join("platforms").exists() || path.join("cmdline-tools").exists())
}

fn looks_like_android_task(user_message: &str) -> bool {
    let lower = user_message.to_ascii_lowercase();
    lower.contains("apk")
        || lower.contains("android")
        || user_message.contains("安卓")
        || user_message.contains("应用")
        || user_message.contains("打包")
        || user_message.contains("编译")
}

fn format_cli_reply(stdout: &str, stderr: &str, success: bool) -> String {
    let extracted;
    let primary = if stdout.trim().is_empty() {
        extracted = extract_codex_answer(stderr);
        extracted.as_deref().unwrap_or(stderr)
    } else if let Some(answer) = extract_json_agent_message(stdout) {
        extracted = Some(answer);
        extracted.as_deref().unwrap_or(stdout)
    } else {
        stdout
    };
    let clean = truncate_chars(strip_ansi(primary).trim(), 8000);

    if clean.is_empty() {
        if success {
            "本地 AI CLI 已完成处理。".into()
        } else {
            "AI 助手尝试处理这个流程，但没有返回可读的失败原因。请稍后重试；如果问题持续出现，需要人工确认当前 Git 工作区状态后再继续。".into()
        }
    } else if success {
        clean
    } else {
        format!(
            "{}\n\n这次 AI 助手已经尝试自行处理，但流程没有正常完成。请根据上面的原因确认下一步，或稍后重试。",
            clean
        )
    }
}

fn extract_thread_id(stdout: &str) -> Option<String> {
    stdout.lines().find_map(|line| {
        let value: Value = serde_json::from_str(line).ok()?;
        if value.get("type").and_then(Value::as_str) == Some("thread.started") {
            value
                .get("thread_id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        } else {
            None
        }
    })
}

fn extract_json_agent_message(stdout: &str) -> Option<String> {
    let mut latest = None;
    for line in stdout.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if value.get("type").and_then(Value::as_str) != Some("item.completed") {
            continue;
        }
        let Some(item) = value.get("item") else {
            continue;
        };
        if item.get("type").and_then(Value::as_str) != Some("agent_message") {
            continue;
        }
        if let Some(text) = item.get("text").and_then(Value::as_str) {
            latest = Some(text.to_string());
        }
    }
    latest
}

fn extract_codex_answer(stderr: &str) -> Option<String> {
    let clean = strip_ansi(stderr);
    let mut answers = Vec::<String>::new();
    let mut collecting = false;
    let mut current = Vec::<String>::new();

    for raw in clean.lines() {
        let line = raw.trim();
        if line == "codex" {
            if !current.is_empty() {
                answers.push(current.join("\n").trim().to_string());
                current.clear();
            }
            collecting = true;
            continue;
        }

        if collecting && is_codex_block_boundary(line) {
            if !current.is_empty() {
                answers.push(current.join("\n").trim().to_string());
                current.clear();
            }
            collecting = false;
            continue;
        }

        if collecting && !is_noisy_codex_answer_line(line) {
            current.push(line.to_string());
        }
    }

    if !current.is_empty() {
        answers.push(current.join("\n").trim().to_string());
    }

    answers
        .into_iter()
        .rev()
        .find(|answer| !answer.trim().is_empty())
}

fn is_codex_block_boundary(line: &str) -> bool {
    if line.is_empty() {
        return false;
    }
    let lower = line.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "user" | "exec" | "tokens used" | "tool" | "system" | "assistant" | "output:"
    ) || lower.starts_with("openai codex")
        || lower.starts_with("workdir:")
        || lower.starts_with("model:")
        || lower.starts_with("provider:")
        || lower.starts_with("approval:")
        || lower.starts_with("sandbox:")
        || lower.starts_with("reasoning")
        || lower.starts_with("session id:")
        || lower.starts_with("wall time:")
        || lower.starts_with("process exited")
        || lower.starts_with("original token count:")
        || lower.starts_with("/bin/")
        || lower.starts_with("succeeded in")
        || lower.starts_with("failed in")
        || lower.starts_with("error:")
        || lower.starts_with("warn")
        || lower.contains(" event.timestamp=")
        || lower.contains("mcp_server=")
}

fn is_noisy_codex_answer_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    line.is_empty()
        || lower.contains("feedback_tags")
        || lower.contains("model_client.")
        || lower.contains("responses_websocket")
        || lower.contains("event.timestamp=")
        || lower.contains("mcp_server=")
        || lower.contains("auth_header")
}

fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '\u{1b}' {
            out.push(ch);
            continue;
        }

        if chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        }
    }

    out
}

pub(crate) fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut iter = value.chars();
    let truncated: String = iter.by_ref().take(max_chars).collect();
    if iter.next().is_some() {
        format!("{}...\n\n（输出过长，已截断）", truncated)
    } else {
        truncated
    }
}

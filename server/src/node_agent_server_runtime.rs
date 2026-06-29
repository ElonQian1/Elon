// server/src/node_agent_server_runtime.rs

use crate::{
    agent_runtime_error_summary::operational_error_summary,
    node_agent_runtime_approval::{
        requires_tool_approval, wait_for_tool_approval, ApprovalOutcome,
    },
    node_agent_runtime_events::{
        runtime_status_chunk, runtime_summary_chunk, tool_approval_checkpoint,
        tool_approval_decision_chunk, tool_approval_id,
        tool_approval_required_chunk_with_diff_and_checkpoint, tool_call_chunk, tool_name,
        tool_result_chunk,
    },
    node_agent_task_journal::TaskJournal,
    node_agent_tool_approval::ToolApprovalState,
    node_agent_tool_guard::{truncate_chars, ToolGuard},
};
use anyhow::{anyhow, bail, Context, Result};
use futures::StreamExt;
use homecli_proto::AgentToServer;
use serde_json::{json, Value};
use std::{collections::VecDeque, future::Future, path::PathBuf, time::Duration};
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::tungstenite::Message;

const MAX_TURNS: usize = 8;
const MAX_TOOL_RESULT_CHARS: usize = 24_000;
const MAX_RUNTIME_HTTP_BODY_BYTES: usize = 1024 * 1024;

#[derive(Clone)]
pub(crate) struct ServerRuntimeConfig {
    pub server_url: String,
    pub user_token: Option<String>,
}

#[derive(Clone)]
pub(crate) struct ApiRuntimeConfig {
    pub api_base: String,
    pub api_key: String,
    pub model: String,
}

pub(crate) struct ServerRuntimeRunResult {
    pub exit_ok: bool,
    pub error: Option<String>,
    pub model: Option<String>,
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}

pub(crate) struct RuntimePromptOptions<'a> {
    pub req_id: &'a str,
    pub cwd: Option<&'a str>,
    pub runtime_permission: Option<&'a str>,
    pub prompt: &'a str,
    pub approval_state: Option<ToolApprovalState>,
    pub cancel_rx: watch::Receiver<bool>,
    pub out_tx: mpsc::UnboundedSender<Message>,
    pub task_journal: Option<TaskJournal>,
}

pub(crate) async fn run_server_runtime_prompt(
    config: ServerRuntimeConfig,
    options: RuntimePromptOptions<'_>,
) -> ServerRuntimeRunResult {
    match run_server_runtime_inner(config, options).await {
        Ok(result) => result,
        Err(error) => ServerRuntimeRunResult {
            exit_ok: false,
            error: Some(error.to_string()),
            model: Some("server-runtime".to_string()),
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
        },
    }
}

pub(crate) async fn run_api_runtime_prompt(
    options: RuntimePromptOptions<'_>,
) -> ServerRuntimeRunResult {
    let Some(config) = api_runtime_config_from_env() else {
        return ServerRuntimeRunResult {
            exit_ok: false,
            error: Some(
                "api-runtime 缺少本机 API key 或模型；请设置 ELON_AGENT_API_KEY/OPENAI_API_KEY 和 ELON_AGENT_MODEL/OPENAI_MODEL"
                    .to_string(),
            ),
            model: Some("api-runtime".to_string()),
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
        };
    };
    match run_api_runtime_inner(config, options).await {
        Ok(result) => result,
        Err(error) => ServerRuntimeRunResult {
            exit_ok: false,
            error: Some(error.to_string()),
            model: Some("api-runtime".to_string()),
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
        },
    }
}

pub(crate) fn api_runtime_config_from_env() -> Option<ApiRuntimeConfig> {
    api_runtime_config_from_lookup(|name| std::env::var(name).ok())
}

fn api_runtime_config_from_lookup(
    lookup: impl Fn(&str) -> Option<String>,
) -> Option<ApiRuntimeConfig> {
    let api_key = first_value(
        &lookup,
        &["ELON_AGENT_API_KEY", "OPENAI_API_KEY", "HUNYUAN_API_KEY"],
    )?;
    let api_base = first_value(
        &lookup,
        &[
            "ELON_AGENT_API_BASE",
            "OPENAI_API_BASE",
            "OPENAI_BASE_URL",
            "HUNYUAN_API_BASE",
        ],
    )
    .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
    let model = first_value(
        &lookup,
        &["ELON_AGENT_MODEL", "OPENAI_MODEL", "HUNYUAN_MODEL"],
    )?;
    Some(ApiRuntimeConfig {
        api_base: api_base.trim_end_matches('/').to_string(),
        api_key,
        model,
    })
}

async fn run_server_runtime_inner(
    config: ServerRuntimeConfig,
    options: RuntimePromptOptions<'_>,
) -> Result<ServerRuntimeRunResult> {
    let RuntimePromptOptions {
        req_id,
        cwd,
        runtime_permission,
        prompt,
        approval_state,
        cancel_rx,
        out_tx,
        task_journal,
    } = options;
    let token = config
        .user_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("server-runtime 需要先在 Win 客户端登录账号"))?;
    let workspace = resolve_workspace(cwd)?;
    let guard = ToolGuard::new(workspace, runtime_permission);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(150))
        .build()
        .unwrap_or_default();
    let server_url = config.server_url.clone();
    let token = token.to_string();
    run_runtime_loop(
        RuntimeLoopOptions {
            req_id,
            label: "server-runtime",
            guard,
            prompt,
            approval_state,
            cancel_rx,
            out_tx,
            task_journal,
            initial_model: Some("server-runtime".to_string()),
        },
        move |messages| {
            let client = client.clone();
            let server_url = server_url.clone();
            let token = token.clone();
            async move { call_server_runtime(&client, &server_url, &token, &messages).await }
        },
    )
    .await
}

async fn run_api_runtime_inner(
    config: ApiRuntimeConfig,
    options: RuntimePromptOptions<'_>,
) -> Result<ServerRuntimeRunResult> {
    let RuntimePromptOptions {
        req_id,
        cwd,
        runtime_permission,
        prompt,
        approval_state,
        cancel_rx,
        out_tx,
        task_journal,
    } = options;
    let workspace = resolve_workspace(cwd)?;
    let guard = ToolGuard::new(workspace, runtime_permission);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(150))
        .no_proxy()  // 绕过本机代理（代理可能停止或不稳定，混元 API 支持直连）
        .build()
        .unwrap_or_default();
    let initial_model = Some(config.model.clone());
    run_runtime_loop(
        RuntimeLoopOptions {
            req_id,
            label: "api-runtime",
            guard,
            prompt,
            approval_state,
            cancel_rx,
            out_tx,
            task_journal,
            initial_model,
        },
        move |messages| {
            let client = client.clone();
            let config = config.clone();
            async move { call_api_runtime(&client, &config, &messages).await }
        },
    )
    .await
}

struct RuntimeLoopOptions<'a> {
    req_id: &'a str,
    label: &'a str,
    guard: ToolGuard,
    prompt: &'a str,
    approval_state: Option<ToolApprovalState>,
    cancel_rx: watch::Receiver<bool>,
    out_tx: mpsc::UnboundedSender<Message>,
    task_journal: Option<TaskJournal>,
    initial_model: Option<String>,
}

async fn run_runtime_loop<F, Fut>(
    options: RuntimeLoopOptions<'_>,
    mut call_chat: F,
) -> Result<ServerRuntimeRunResult>
where
    F: FnMut(Vec<Value>) -> Fut,
    Fut: Future<Output = Result<Value>>,
{
    let RuntimeLoopOptions {
        req_id,
        label,
        mut guard,
        prompt,
        approval_state,
        mut cancel_rx,
        out_tx,
        task_journal,
        initial_model,
    } = options;
    let mut messages = vec![
        json!({"role": "system", "content": system_prompt(label, guard.read_only(), guard.danger_full_access())}),
        json!({"role": "user", "content": prompt}),
    ];

    let mut usage = RuntimeUsage::default();
    let mut model = initial_model;
    let mut total_tools = 0usize;
    let mut failed_tools = 0usize;
    for turn in 1..=MAX_TURNS {
        if *cancel_rx.borrow() {
            send_runtime_canceled(
                &out_tx,
                task_journal.as_ref(),
                req_id,
                label,
                turn,
                total_tools,
                failed_tools,
            );
            return Ok(canceled_runtime_result(label, model, &usage));
        }
        send_chunk(
            &out_tx,
            task_journal.as_ref(),
            req_id,
            runtime_status_chunk(
                req_id,
                turn,
                label,
                "thinking",
                "正在调用模型生成下一步计划",
            ),
        );
        let call_messages = messages.clone();
        let response_result = tokio::select! {
            result = call_chat(call_messages.clone()) => {
                result
            }
            changed = cancel_rx.changed() => {
                if changed.is_ok() && *cancel_rx.borrow() {
                    send_runtime_canceled(
                        &out_tx,
                        task_journal.as_ref(),
                        req_id,
                        label,
                        turn,
                        total_tools,
                        failed_tools,
                    );
                    return Ok(canceled_runtime_result(label, model, &usage));
                }
                call_chat(call_messages).await
            }
        };
        let response = match response_result.with_context(|| format!("调用 {label} 失败")) {
            Ok(response) => response,
            Err(error) => {
                let error_message = format!("{error:#}");
                send_runtime_failure(
                    &out_tx,
                    task_journal.as_ref(),
                    req_id,
                    label,
                    turn,
                    total_tools,
                    failed_tools,
                    &error_message,
                );
                return Err(error);
            }
        };
        usage.merge(&response);
        if let Some(value) = response
            .get("model")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
        {
            model = Some(value.to_string());
        }
        let content = match extract_assistant_content(&response) {
            Ok(content) => content,
            Err(error) => {
                send_runtime_failure(
                    &out_tx,
                    task_journal.as_ref(),
                    req_id,
                    label,
                    turn,
                    total_tools,
                    failed_tools,
                    &error.to_string(),
                );
                return Err(error);
            }
        };
        messages.push(json!({"role": "assistant", "content": content}));
        // 若模型使用原生 tool_calls，将刚加入的纯文本版本替换为含 tool_calls 字段的完整原始消息
        let used_native_tool_calls = {
            let has_tc = response
                .get("choices")
                .and_then(Value::as_array)
                .and_then(|c| c.first())
                .and_then(|c| c.get("message"))
                .and_then(|m| m.get("tool_calls"))
                .and_then(Value::as_array)
                .map(|tc| !tc.is_empty())
                .unwrap_or(false);
            if has_tc {
                if let Some(orig_msg) = response
                    .get("choices")
                    .and_then(Value::as_array)
                    .and_then(|c| c.first())
                    .and_then(|c| c.get("message"))
                    .cloned()
                {
                    *messages.last_mut().unwrap() = orig_msg;
                }
            }
            has_tc
        };
        let agent = match parse_agent_response(&content) {
            Ok(agent) => agent,
            Err(error) => {
                send_runtime_failure(
                    &out_tx,
                    task_journal.as_ref(),
                    req_id,
                    label,
                    turn,
                    total_tools,
                    failed_tools,
                    &error.to_string(),
                );
                return Err(error);
            }
        };
        if let Some(message) = agent
            .get("message")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
        {
            send_chunk(
                &out_tx,
                task_journal.as_ref(),
                req_id,
                format!("{message}\n"),
            );
        }

        let actions = agent
            .get("actions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if actions.is_empty() {
            send_chunk(
                &out_tx,
                task_journal.as_ref(),
                req_id,
                runtime_status_chunk(
                    req_id,
                    turn,
                    label,
                    "completed",
                    "没有更多工具动作，任务完成",
                ),
            );
            send_runtime_summary(
                &out_tx,
                task_journal.as_ref(),
                req_id,
                label,
                turn,
                "ok",
                total_tools,
                failed_tools,
                "任务完成",
            );
            return Ok(ServerRuntimeRunResult {
                exit_ok: true,
                error: None,
                model,
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
                total_tokens: usage.total_tokens,
            });
        }

        let mut results = Vec::new();
        let mut tool_call_id_results: Vec<(String, String)> = Vec::new();
        for (index, action) in actions.into_iter().enumerate() {
            let action_tool_call_id = action
                .get("tool_call_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let tool_index = index + 1;
            let tool = tool_name(&action);
            let mut approved_approval_diff = None;
            if requires_tool_approval(&guard, &action) {
                let approval_id = tool_approval_id(turn, tool_index);
                let approval_diff = match tool.as_str() {
                    "write_file" => guard.write_file_diff_preview(&action).await,
                    "apply_patch" => guard.apply_patch_diff_preview(&action).await,
                    _ => Ok(None),
                };
                let approval_diff = match approval_diff {
                    Ok(diff) => diff,
                    Err(error) => {
                        let result = format!(
                            "error: {tool} approval preview unavailable: {error}; tool was not executed"
                        );
                        send_chunk(
                            &out_tx,
                            task_journal.as_ref(),
                            req_id,
                            tool_result_chunk(
                                req_id,
                                turn,
                                tool_index,
                                &tool,
                                &result,
                                Some(&action),
                            ),
                        );
                        record_tool_result(
                            &mut results,
                            &mut total_tools,
                            &mut failed_tools,
                            &tool,
                            &result,
                        );
                        tool_call_id_results.push((action_tool_call_id.clone(), truncate_chars(&result, MAX_TOOL_RESULT_CHARS)));
                        continue;
                    }
                };
                let mut waiter = match approval_state.as_ref() {
                    Some(state) => state.register(req_id, &approval_id).await,
                    None => {
                        let result =
                            format!("error: {tool} approval unavailable; tool was not executed");
                        send_chunk(
                            &out_tx,
                            task_journal.as_ref(),
                            req_id,
                            tool_result_chunk(
                                req_id,
                                turn,
                                tool_index,
                                &tool,
                                &result,
                                Some(&action),
                            ),
                        );
                        record_tool_result(
                            &mut results,
                            &mut total_tools,
                            &mut failed_tools,
                            &tool,
                            &result,
                        );
                        tool_call_id_results.push((action_tool_call_id.clone(), truncate_chars(&result, MAX_TOOL_RESULT_CHARS)));
                        continue;
                    }
                };
                send_chunk(
                    &out_tx,
                    task_journal.as_ref(),
                    req_id,
                    runtime_status_chunk(
                        req_id,
                        turn,
                        label,
                        "waiting_approval",
                        "等待用户审批工具调用",
                    ),
                );
                send_chunk(
                    &out_tx,
                    task_journal.as_ref(),
                    req_id,
                    match approval_diff.as_ref() {
                        Some(diff) => tool_approval_required_chunk_with_diff_and_checkpoint(
                            req_id,
                            turn,
                            tool_index,
                            &approval_id,
                            &action,
                            diff.clone(),
                            tool_approval_checkpoint(
                                &action,
                                diff,
                                waiter.registered_at_ms(),
                                waiter.expires_at_ms(),
                            ),
                        ),
                        None => {
                            let diff = Value::Null;
                            tool_approval_required_chunk_with_diff_and_checkpoint(
                                req_id,
                                turn,
                                tool_index,
                                &approval_id,
                                &action,
                                diff.clone(),
                                tool_approval_checkpoint(
                                    &action,
                                    &diff,
                                    waiter.registered_at_ms(),
                                    waiter.expires_at_ms(),
                                ),
                            )
                        }
                    },
                );
                match wait_for_tool_approval(&mut waiter, &mut cancel_rx).await {
                    ApprovalOutcome::Approved => {
                        approved_approval_diff = approval_diff;
                        send_chunk(
                            &out_tx,
                            task_journal.as_ref(),
                            req_id,
                            tool_approval_decision_chunk(
                                req_id,
                                turn,
                                tool_index,
                                &approval_id,
                                &tool,
                                "approve",
                                "approved",
                                Some(&action),
                            ),
                        );
                    }
                    ApprovalOutcome::Denied(reason) => {
                        send_chunk(
                            &out_tx,
                            task_journal.as_ref(),
                            req_id,
                            tool_approval_decision_chunk(
                                req_id,
                                turn,
                                tool_index,
                                &approval_id,
                                &tool,
                                "deny",
                                "denied",
                                Some(&action),
                            ),
                        );
                        let result = format!("error: {tool} denied by user: {reason}");
                        send_chunk(
                            &out_tx,
                            task_journal.as_ref(),
                            req_id,
                            tool_result_chunk(
                                req_id,
                                turn,
                                tool_index,
                                &tool,
                                &result,
                                Some(&action),
                            ),
                        );
                        record_tool_result(
                            &mut results,
                            &mut total_tools,
                            &mut failed_tools,
                            &tool,
                            &result,
                        );
                        tool_call_id_results.push((action_tool_call_id.clone(), truncate_chars(&result, MAX_TOOL_RESULT_CHARS)));
                        continue;
                    }
                    ApprovalOutcome::TimedOut => {
                        send_chunk(
                            &out_tx,
                            task_journal.as_ref(),
                            req_id,
                            tool_approval_decision_chunk(
                                req_id,
                                turn,
                                tool_index,
                                &approval_id,
                                &tool,
                                "timeout",
                                "expired",
                                Some(&action),
                            ),
                        );
                        let result =
                            format!("error: {tool} approval timed out; tool was not executed");
                        send_chunk(
                            &out_tx,
                            task_journal.as_ref(),
                            req_id,
                            tool_result_chunk(
                                req_id,
                                turn,
                                tool_index,
                                &tool,
                                &result,
                                Some(&action),
                            ),
                        );
                        record_tool_result(
                            &mut results,
                            &mut total_tools,
                            &mut failed_tools,
                            &tool,
                            &result,
                        );
                        tool_call_id_results.push((action_tool_call_id.clone(), truncate_chars(&result, MAX_TOOL_RESULT_CHARS)));
                        continue;
                    }
                    ApprovalOutcome::Canceled => {
                        send_chunk(
                            &out_tx,
                            task_journal.as_ref(),
                            req_id,
                            tool_approval_decision_chunk(
                                req_id,
                                turn,
                                tool_index,
                                &approval_id,
                                &tool,
                                "cancel",
                                "canceled",
                                Some(&action),
                            ),
                        );
                        send_runtime_canceled(
                            &out_tx,
                            task_journal.as_ref(),
                            req_id,
                            label,
                            turn,
                            total_tools,
                            failed_tools,
                        );
                        return Ok(canceled_runtime_result(label, model, &usage));
                    }
                }
            }
            if let Some(diff) = approved_approval_diff.as_ref() {
                let preview_verification = match tool.as_str() {
                    "write_file" => {
                        guard
                            .verify_write_file_preview_unchanged(&action, diff)
                            .await
                    }
                    "apply_patch" => {
                        guard
                            .verify_apply_patch_preview_unchanged(&action, diff)
                            .await
                    }
                    _ => Ok(()),
                };
                if let Err(error) = preview_verification {
                    let result = format!(
                        "error: {tool} approval preview is stale: {error}; tool was not executed"
                    );
                    send_chunk(
                        &out_tx,
                        task_journal.as_ref(),
                        req_id,
                        tool_result_chunk(req_id, turn, tool_index, &tool, &result, Some(&action)),
                    );
                    record_tool_result(
                        &mut results,
                        &mut total_tools,
                        &mut failed_tools,
                        &tool,
                        &result,
                    );
                    tool_call_id_results.push((action_tool_call_id.clone(), truncate_chars(&result, MAX_TOOL_RESULT_CHARS)));
                    continue;
                }
            }
            send_chunk(
                &out_tx,
                task_journal.as_ref(),
                req_id,
                tool_call_chunk(req_id, turn, tool_index, &action),
            );
            let result = guard.invoke_action(&action).await;
            send_chunk(
                &out_tx,
                task_journal.as_ref(),
                req_id,
                tool_result_chunk(req_id, turn, tool_index, &tool, &result, Some(&action)),
            );
            tool_call_id_results.push((action_tool_call_id.clone(), truncate_chars(&result, MAX_TOOL_RESULT_CHARS)));
            record_tool_result(
                &mut results,
                &mut total_tools,
                &mut failed_tools,
                &tool,
                &result,
            );
        }
        // 原生 tool_calls 用 role:tool 消息（含 tool_call_id），否则用 role:user JSON
        if used_native_tool_calls && !tool_call_id_results.is_empty() {
            for (tc_id, result_content) in &tool_call_id_results {
                messages.push(json!({
                    "role": "tool",
                    "tool_call_id": tc_id,
                    "content": result_content,
                }));
            }
        } else {
            messages.push(json!({
                "role": "user",
                "content": format!("Tool results JSON:\n{}", serde_json::to_string(&results)?),
            }));
        }

        if agent.get("done").and_then(Value::as_bool).unwrap_or(false) {
            send_chunk(
                &out_tx,
                task_journal.as_ref(),
                req_id,
                runtime_status_chunk(req_id, turn, label, "completed", "工具结果已处理，任务完成"),
            );
            send_runtime_summary(
                &out_tx,
                task_journal.as_ref(),
                req_id,
                label,
                turn,
                if failed_tools == 0 { "ok" } else { "error" },
                total_tools,
                failed_tools,
                "工具结果已处理，任务完成",
            );
            return Ok(ServerRuntimeRunResult {
                exit_ok: true,
                error: None,
                model,
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
                total_tokens: usage.total_tokens,
            });
        }
    }

    send_chunk(
        &out_tx,
        task_journal.as_ref(),
        req_id,
        runtime_status_chunk(req_id, MAX_TURNS, label, "failed", "达到最大轮次仍未完成"),
    );
    send_runtime_summary(
        &out_tx,
        task_journal.as_ref(),
        req_id,
        label,
        MAX_TURNS,
        "error",
        total_tools,
        failed_tools,
        "达到最大轮次仍未完成",
    );

    Ok(ServerRuntimeRunResult {
        exit_ok: false,
        error: Some(format!("{label} 超过 {MAX_TURNS} 轮仍未完成")),
        model: model.or_else(|| Some(label.to_string())),
        prompt_tokens: usage.prompt_tokens,
        completion_tokens: usage.completion_tokens,
        total_tokens: usage.total_tokens,
    })
}

fn record_tool_result(
    results: &mut Vec<Value>,
    total_tools: &mut usize,
    failed_tools: &mut usize,
    tool: &str,
    result: &str,
) {
    *total_tools += 1;
    if is_tool_error(result) {
        *failed_tools += 1;
    }
    results.push(json!({
        "tool": tool,
        "result": truncate_chars(result, MAX_TOOL_RESULT_CHARS),
    }));
}

fn is_tool_error(result: &str) -> bool {
    result
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("error:")
}

fn send_runtime_summary(
    out_tx: &mpsc::UnboundedSender<Message>,
    task_journal: Option<&TaskJournal>,
    req_id: &str,
    label: &str,
    turn: usize,
    status: &str,
    total_tools: usize,
    failed_tools: usize,
    message: &str,
) {
    send_chunk(
        out_tx,
        task_journal,
        req_id,
        runtime_summary_chunk(
            req_id,
            label,
            turn,
            status,
            total_tools,
            failed_tools,
            message,
        ),
    );
}

fn send_runtime_canceled(
    out_tx: &mpsc::UnboundedSender<Message>,
    task_journal: Option<&TaskJournal>,
    req_id: &str,
    label: &str,
    turn: usize,
    total_tools: usize,
    failed_tools: usize,
) {
    send_chunk(
        out_tx,
        task_journal,
        req_id,
        runtime_status_chunk(req_id, turn, label, "canceled", "用户已停止运行时任务"),
    );
    send_runtime_summary(
        out_tx,
        task_journal,
        req_id,
        label,
        turn,
        "canceled",
        total_tools,
        failed_tools,
        "用户已停止运行时任务",
    );
}

fn send_runtime_failure(
    out_tx: &mpsc::UnboundedSender<Message>,
    task_journal: Option<&TaskJournal>,
    req_id: &str,
    label: &str,
    turn: usize,
    total_tools: usize,
    failed_tools: usize,
    message: &str,
) {
    send_chunk(
        out_tx,
        task_journal,
        req_id,
        runtime_status_chunk(req_id, turn, label, "failed", message),
    );
    send_runtime_summary(
        out_tx,
        task_journal,
        req_id,
        label,
        turn,
        "error",
        total_tools,
        failed_tools,
        message,
    );
}

fn resolve_workspace(cwd: Option<&str>) -> Result<PathBuf> {
    let path = cwd
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?);
    let full = std::fs::canonicalize(&path)
        .with_context(|| format!("server-runtime 工作目录不存在: {}", path.display()))?;
    if !full.is_dir() {
        bail!("server-runtime 工作目录不是目录: {}", full.display());
    }
    Ok(full)
}

fn system_prompt(label: &str, read_only: bool, danger_full_access: bool) -> String {
    let runtime_identity = match label {
        "api-runtime" => "Route B local API runtime",
        "server-runtime" => "Route C server runtime",
        _ => "local agent runtime",
    };
    let mut prompt = r#"You are the Elon {{runtime_identity}} for a Windows PC project.
Return strict JSON only, without markdown fences.

Schema:
{
    "message": "short progress or final answer",
    "done": false,
    "actions": [
    {"tool": "list_dir", "path": "."},
    {"tool": "search_files", "query": "TODO", "path": "src", "max_results": 40},
    {"tool": "file_info", "path": "src/main.rs"},
    {"tool": "read_file", "path": "README.md"},
    {"tool": "read_file_range", "path": "src/main.rs", "start_line": 120, "line_count": 80},
    {"tool": "git_status"},
    {"tool": "git_diff", "path": "src/main.rs", "cached": false, "stat": false},
    {"tool": "git_log", "path": "src/main.rs", "limit": 20},
    {"tool": "git_show", "revision": "HEAD", "path": "src/main.rs", "stat": true},
    {"tool": "write_file", "path": "docs/note.md", "content": "full content"},
    {"tool": "apply_patch", "patch": "unified diff", "check_only": false},
    {"tool": "run_command", "program": "cargo", "args": ["test"], "reason": "verify project tests"},
    {"tool": "run_command", "command": "ipconfig /all", "shell": "cmd", "cwd": "C:/", "reason": "diagnose Windows network"}
  ]
}

Rules:
- In project_write/full_access mode, paths must be relative to the current project workspace.
- In danger_full_access mode, absolute paths and paths outside the project workspace are allowed.
- Prefer read-only actions first.
- Use search_files before broad file reads when you need to locate symbols, filenames, TODOs, errors, or related code.
- Use file_info before reading unknown files, binary-looking files, or directories.
- Use read_file_range instead of read_file for large files when you only need one section.
- Use git_status, git_diff, git_log, and git_show for read-only git inspection; do not spend run_command approvals on status/diff/log/show.
- In project_write/full_access mode, do not request destructive commands, privilege changes, downloads that execute code, persistence, credential access, or writes outside the project.
- Prefer apply_patch with unified diff for intentional edits to existing project files.
- Use write_file only when replacing a full file or creating a small new project file.
- In project_write/full_access mode, use run_command only for project build, format, lint, or test commands.
- Prefer structured run_command with program and args. The legacy command string field exists only for older clients.
- In danger_full_access mode, run_command may use either program/args or command/shell/cwd for arbitrary cmd, powershell, pwsh, bash, or sh commands on the user's PC.
- If your API supports native tool/function calls, use those tool calls for actions. Otherwise return the actions array in JSON.
- Set done=true when no further tool action is needed.
"#
    .replace("{{runtime_identity}}", runtime_identity);
    if read_only {
        prompt.push_str(
            "\nCurrent mode is read-only planning. Do not request write_file, apply_patch, or run_command. You may still use git_status, git_diff, git_log, and git_show.\n",
        );
    } else if danger_full_access {
        prompt.push_str(
            "\nCurrent mode is danger_full_access. The user has intentionally enabled full local command line and filesystem control for this project runtime. You may request arbitrary cmd/powershell/pwsh commands, set cwd, and read/write absolute local paths when needed to solve the user's task. Do not claim a command or file action has completed until its tool result is present.\n",
        );
    }
    prompt
}

async fn call_server_runtime(
    client: &reqwest::Client,
    server_url: &str,
    token: &str,
    messages: &[Value],
) -> Result<Value> {
    let url = format!(
        "{}/api/agent/runtime/chat",
        server_url.trim_end_matches('/')
    );
    let response = client
        .post(url)
        .bearer_auth(token)
        .json(&json!({ "messages": messages }))
        .send()
        .await?;
    let status = response.status();
    let body = limited_runtime_response_text(response, "服务器 AI runtime").await?;
    if !status.is_success() {
        bail!(
            "{}",
            runtime_http_error_message("服务器 AI runtime", status, &body)
        );
    }
    serde_json::from_str(&body).context("服务器 AI runtime 响应不是 JSON")
}

async fn call_api_runtime(
    client: &reqwest::Client,
    config: &ApiRuntimeConfig,
    messages: &[Value],
) -> Result<Value> {
    let url = format!("{}/chat/completions", config.api_base.trim_end_matches('/'));
    let mut attempts = VecDeque::from([(true, true)]);
    let mut attempted = Vec::<(bool, bool)>::new();
    let mut last_error = None;

    while let Some((json_mode, tools_mode)) = attempts.pop_front() {
        if attempted.contains(&(json_mode, tools_mode)) {
            continue;
        }
        attempted.push((json_mode, tools_mode));
        let response =
            send_api_runtime_request(client, &url, config, messages, json_mode, tools_mode).await?;
        let status = response.status();
        let body = limited_runtime_response_text(response, "本机 API runtime").await?;
        if status.is_success() {
            let parsed = serde_json::from_str::<Value>(&body)
                .context("本机 API runtime 响应不是 JSON")?;
            // 检查响应是否包含 tool_calls 或有效内容，避免"200 但内容无法处理"的情况
            let has_tool_calls = parsed
                .get("choices")
                .and_then(Value::as_array)
                .and_then(|c| c.first())
                .and_then(|c| c.get("message"))
                .and_then(|m| m.get("tool_calls"))
                .and_then(Value::as_array)
                .map(|tc| !tc.is_empty())
                .unwrap_or(false);
            let content_str = parsed
                .get("choices")
                .and_then(Value::as_array)
                .and_then(|c| c.first())
                .and_then(|c| c.get("message"))
                .and_then(|m| m.get("content"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim();
            let has_json_content = !content_str.is_empty()
                && (content_str.starts_with('{') || content_str.starts_with('['));
            if has_tool_calls || has_json_content {
                return Ok(parsed);
            }
            // 200 但内容不可处理（模型返回了思考文字而非 JSON/tool_calls）→ 降级重试
            // 优先保留 json_object 去掉 tools；再其次完全降级
            let degraded_mode = if json_mode && tools_mode && !attempted.contains(&(true, false)) {
                Some((true, false))
            } else if !attempted.contains(&(false, false)) {
                Some((false, false))
            } else {
                None
            };
            if let Some(next) = degraded_mode {
                tracing::warn!(
                    "api-runtime 200 但内容不可处理（模式 json={json_mode}/tools={tools_mode}），降级到 json={}/tools={} 重试",
                    next.0, next.1
                );
                attempts.push_front(next);
            }
            last_error = Some((status, body));
            continue;
        }
        tracing::warn!("api-runtime {status} body[{}c]: {body}", body.len());

        let retry_without_json =
            json_mode && api_runtime_should_retry_without_json_mode(status, &body);
        let retry_without_tools = tools_mode
            && crate::node_agent_api_runtime_tools::should_retry_without_tools(status, &body);
        if retry_without_json {
            attempts.push_back((false, tools_mode));
        }
        if retry_without_tools {
            attempts.push_back((json_mode, false));
        }
        if retry_without_json || retry_without_tools {
            attempts.push_back((false, false));
        }
        last_error = Some((status, body));
    }

    let (status, body) = last_error.ok_or_else(|| anyhow!("本机 API runtime 没有执行任何请求"))?;
    bail!(
        "{}",
        runtime_http_error_message("本机 API runtime", status, &body)
    );
}

async fn send_api_runtime_request(
    client: &reqwest::Client,
    url: &str,
    config: &ApiRuntimeConfig,
    messages: &[Value],
    json_mode: bool,
    tools_mode: bool,
) -> Result<reqwest::Response> {
    client
        .post(url)
        .bearer_auth(&config.api_key)
        .json(&api_runtime_chat_payload(
            config, messages, json_mode, tools_mode,
        ))
        .send()
        .await
        .context("本机 API runtime 请求失败")
}

fn api_runtime_chat_payload(
    config: &ApiRuntimeConfig,
    messages: &[Value],
    json_mode: bool,
    tools_mode: bool,
) -> Value {
    let mut payload = json!({
        "model": config.model,
        "messages": messages,
        "temperature": 0.2
    });
    // 当消息历史含 role:tool 时不发 json_object，避免部分模型（如混元）拒绝该组合
    let has_tool_results = messages
        .iter()
        .any(|m| m.get("role").and_then(Value::as_str) == Some("tool"));
    if json_mode && !has_tool_results {
        payload["response_format"] = json!({ "type": "json_object" });
    }
    if tools_mode {
        crate::node_agent_api_runtime_tools::add_tools_to_payload(&mut payload);
    }
    payload
}

fn api_runtime_should_retry_without_json_mode(status: reqwest::StatusCode, body: &str) -> bool {
    if !matches!(
        status,
        reqwest::StatusCode::BAD_REQUEST
            | reqwest::StatusCode::UNPROCESSABLE_ENTITY
            | reqwest::StatusCode::NOT_IMPLEMENTED
    ) {
        return false;
    }
    let lower = body.to_ascii_lowercase();
    lower.contains("response_format")
        || lower.contains("json_object")
        || lower.contains("json mode")
        || lower.contains("unsupported parameter")
        || lower.contains("unknown parameter")
        || lower.contains("unrecognized request argument")
}

fn runtime_http_error_message(label: &str, status: reqwest::StatusCode, body: &str) -> String {
    format!("{label} 返回 {status}: {}", operational_error_summary(body))
}

async fn limited_runtime_response_text(response: reqwest::Response, label: &str) -> Result<String> {
    if let Some(content_length) = response.content_length() {
        ensure_runtime_response_size(label, content_length as usize)?;
    }

    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.with_context(|| format!("读取 {label} 响应失败"))?;
        ensure_runtime_response_size(label, body.len().saturating_add(chunk.len()))?;
        body.extend_from_slice(&chunk);
    }

    String::from_utf8(body).with_context(|| format!("{label} 响应不是 UTF-8"))
}

fn ensure_runtime_response_size(label: &str, observed_bytes: usize) -> Result<()> {
    if observed_bytes > MAX_RUNTIME_HTTP_BODY_BYTES {
        bail!(
            "{}",
            runtime_response_too_large_message(label, observed_bytes)
        );
    }
    Ok(())
}

fn runtime_response_too_large_message(label: &str, observed_bytes: usize) -> String {
    format!(
        "{label} 响应过大：{} 字节，超过客户端安全上限 {} 字节，已中止读取",
        observed_bytes, MAX_RUNTIME_HTTP_BODY_BYTES
    )
}

fn first_value(lookup: &impl Fn(&str) -> Option<String>, names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        lookup(name)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn canceled_runtime_result(
    label: &str,
    model: Option<String>,
    usage: &RuntimeUsage,
) -> ServerRuntimeRunResult {
    ServerRuntimeRunResult {
        exit_ok: false,
        error: Some("用户已停止 PC AI runtime 任务".to_string()),
        model: model.or_else(|| Some(label.to_string())),
        prompt_tokens: usage.prompt_tokens,
        completion_tokens: usage.completion_tokens,
        total_tokens: usage.total_tokens,
    }
}

fn extract_assistant_content(response: &Value) -> Result<String> {
    if let Some(content) =
        crate::node_agent_api_runtime_tools::agent_response_from_tool_calls(response)?
    {
        return Ok(content);
    }
    if let Some(content) = response
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
    {
        return Ok(content.to_string());
    }
    bail!("服务器 AI runtime 响应缺少 choices[0].message.content")
}

fn parse_agent_response(content: &str) -> Result<Value> {
    let trimmed = content.trim();
    if let Ok(value) = serde_json::from_str(trimmed) {
        return Ok(value);
    }
    if let Some(value) = parse_first_json_object(trimmed) {
        return Ok(value);
    }
    bail!("server-runtime 返回内容不是 JSON")
}

fn parse_first_json_object(content: &str) -> Option<Value> {
    for (start, ch) in content.char_indices() {
        if ch != '{' {
            continue;
        }
        let Some(end) = matching_json_object_end(&content[start..]) else {
            continue;
        };
        let candidate = &content[start..start + end];
        if let Ok(value) = serde_json::from_str(candidate) {
            return Some(value);
        }
    }
    None
}

fn matching_json_object_end(content: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (offset, ch) in content.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => depth = depth.saturating_add(1),
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(offset + ch.len_utf8());
                }
            }
            _ => {}
        }
    }
    None
}

fn send_chunk(
    out_tx: &mpsc::UnboundedSender<Message>,
    task_journal: Option<&TaskJournal>,
    req_id: &str,
    text: String,
) {
    if let Some(journal) = task_journal {
        let _ = journal.record_cli_chunk(req_id, "runtime", &text);
    }
    let _ = out_tx.send(Message::Text(
        serde_json::to_string(&AgentToServer::CliChunk {
            req_id: req_id.to_string(),
            text,
        })
        .unwrap_or_default(),
    ));
}

#[derive(Default)]
struct RuntimeUsage {
    prompt_tokens: Option<u64>,
    completion_tokens: Option<u64>,
    total_tokens: Option<u64>,
}

impl RuntimeUsage {
    fn merge(&mut self, response: &Value) {
        let Some(usage) = response.get("usage") else {
            return;
        };
        self.prompt_tokens = usage
            .get("prompt_tokens")
            .and_then(Value::as_u64)
            .or(self.prompt_tokens);
        self.completion_tokens = usage
            .get("completion_tokens")
            .and_then(Value::as_u64)
            .or(self.completion_tokens);
        self.total_tokens = usage
            .get("total_tokens")
            .and_then(Value::as_u64)
            .or(self.total_tokens);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        api_runtime_chat_payload, api_runtime_config_from_lookup,
        api_runtime_should_retry_without_json_mode, ensure_runtime_response_size,
        parse_agent_response, run_runtime_loop, runtime_http_error_message,
        runtime_response_too_large_message, system_prompt, ApiRuntimeConfig, RuntimeLoopOptions,
        MAX_RUNTIME_HTTP_BODY_BYTES,
    };
    use crate::{
        node_agent_task_journal::TaskJournal, node_agent_tool_approval::ToolApprovalState,
        node_agent_tool_guard::ToolGuard,
    };
    use anyhow::anyhow;
    use homecli_proto::AgentToServer;
    use serde_json::{json, Value};
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::{SystemTime, UNIX_EPOCH},
    };
    use tokio::sync::{mpsc, watch};
    use tokio_tungstenite::tungstenite::Message;

    #[test]
    fn api_runtime_config_defaults_openai_base_but_requires_model() {
        assert!(api_runtime_config_from_lookup(|name| match name {
            "OPENAI_API_KEY" => Some("sk-test".to_string()),
            _ => None,
        })
        .is_none());

        let config = api_runtime_config_from_lookup(|name| match name {
            "OPENAI_API_KEY" => Some(" sk-test ".to_string()),
            "OPENAI_MODEL" => Some(" gpt-test ".to_string()),
            _ => None,
        })
        .expect("api key and model should create config");

        assert_eq!(config.api_base, "https://api.openai.com/v1");
        assert_eq!(config.api_key, "sk-test");
        assert_eq!(config.model, "gpt-test");
    }

    #[test]
    fn api_runtime_config_prefers_elon_specific_env() {
        let config = api_runtime_config_from_lookup(|name| match name {
            "ELON_AGENT_API_BASE" => Some("https://example.test/v1/".to_string()),
            "ELON_AGENT_API_KEY" => Some("elon-key".to_string()),
            "ELON_AGENT_MODEL" => Some("custom-model".to_string()),
            "OPENAI_API_KEY" => Some("openai-key".to_string()),
            _ => None,
        })
        .expect("elon env should create config");

        assert_eq!(config.api_base, "https://example.test/v1");
        assert_eq!(config.api_key, "elon-key");
        assert_eq!(config.model, "custom-model");
    }

    #[test]
    fn api_runtime_payload_uses_json_mode_and_tools_by_default() {
        let config = ApiRuntimeConfig {
            api_base: "https://example.test/v1".to_string(),
            api_key: "sk-test".to_string(),
            model: "gpt-test".to_string(),
        };
        let messages = vec![json!({"role": "user", "content": "Return JSON"})];

        let payload = api_runtime_chat_payload(&config, &messages, true, true);
        assert_eq!(payload["model"], "gpt-test");
        assert_eq!(payload["temperature"], 0.2);
        assert_eq!(payload["response_format"]["type"], "json_object");
        assert_eq!(payload["tool_choice"], "auto");
        assert!(payload["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["function"]["name"] == "file_info"));
        assert!(payload["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["function"]["name"] == "git_status"));
        assert!(payload["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["function"]["name"] == "git_diff"));
        assert!(payload["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["function"]["name"] == "git_log"));
        assert!(payload["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["function"]["name"] == "git_show"));
        assert!(payload["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["function"]["name"] == "run_command"));
        assert_eq!(payload["messages"][0]["content"], "Return JSON");

        let fallback = api_runtime_chat_payload(&config, &messages, false, false);
        assert!(fallback.get("response_format").is_none());
        assert!(fallback.get("tools").is_none());
    }

    #[test]
    fn api_runtime_json_mode_retry_is_limited_to_compatibility_errors() {
        assert!(api_runtime_should_retry_without_json_mode(
            reqwest::StatusCode::BAD_REQUEST,
            "Unrecognized request argument supplied: response_format"
        ));
        assert!(api_runtime_should_retry_without_json_mode(
            reqwest::StatusCode::UNPROCESSABLE_ENTITY,
            "json_object is not supported by this model"
        ));
        assert!(!api_runtime_should_retry_without_json_mode(
            reqwest::StatusCode::UNAUTHORIZED,
            "invalid api key"
        ));
        assert!(!api_runtime_should_retry_without_json_mode(
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            "rate limit"
        ));
    }

    #[test]
    fn system_prompt_matches_runtime_route_identity() {
        let route_b = system_prompt("api-runtime", false, false);
        let route_c = system_prompt("server-runtime", true, false);
        let danger = system_prompt("server-runtime", false, true);

        assert!(route_b.contains("Route B local API runtime"));
        assert!(route_b.contains("\"tool\": \"git_status\""));
        assert!(route_b.contains("\"tool\": \"git_diff\""));
        assert!(route_b.contains("\"tool\": \"git_log\""));
        assert!(route_b.contains("\"tool\": \"git_show\""));
        assert!(route_b.contains("Use git_status, git_diff, git_log, and git_show"));
        assert!(!route_b.contains("Route C server runtime for"));
        assert!(route_c.contains("Route C server runtime"));
        assert!(route_c.contains("read-only planning"));
        assert!(route_c.contains("Do not request write_file, apply_patch, or run_command"));
        assert!(route_c.contains("You may still use git_status, git_diff, git_log, and git_show"));
        assert!(danger.contains("danger_full_access"));
        assert!(danger.contains("arbitrary cmd/powershell/pwsh commands"));
        assert!(danger.contains("\"shell\": \"cmd\""));
    }

    #[test]
    fn runtime_http_error_message_redacts_provider_body() {
        let message = runtime_http_error_message(
            "本机 API runtime",
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            "429 rate limit: sk-secret and prompt text",
        );

        assert!(message.contains("429"));
        assert!(message.contains("rate_limit"));
        assert!(message.contains("fingerprint="));
        assert!(!message.contains("sk-secret"));
        assert!(!message.contains("prompt text"));
    }

    #[test]
    fn runtime_response_size_limit_allows_boundary() {
        ensure_runtime_response_size("服务器 AI runtime", MAX_RUNTIME_HTTP_BODY_BYTES)
            .expect("exact limit should be accepted");
    }

    #[test]
    fn runtime_response_size_limit_rejects_oversized_body() {
        let error =
            ensure_runtime_response_size("服务器 AI runtime", MAX_RUNTIME_HTTP_BODY_BYTES + 1)
                .unwrap_err();
        let message = error.to_string();

        assert!(message.contains("服务器 AI runtime 响应过大"));
        assert!(message.contains("已中止读取"));
        assert!(message.contains(&(MAX_RUNTIME_HTTP_BODY_BYTES + 1).to_string()));
    }

    #[test]
    fn runtime_response_too_large_message_does_not_include_body() {
        let message =
            runtime_response_too_large_message("本机 API runtime", MAX_RUNTIME_HTTP_BODY_BYTES + 9);

        assert!(message.contains("本机 API runtime 响应过大"));
        assert!(message.contains("1048585"));
        assert!(!message.contains("sk-secret"));
        assert!(!message.contains("prompt text"));
    }

    #[test]
    fn parse_agent_response_accepts_markdown_fenced_json() {
        let parsed = parse_agent_response(
            r#"```json
{"message":"ok","done":true,"actions":[]}
```"#,
        )
        .expect("fenced json should parse");

        assert_eq!(parsed["message"], "ok");
        assert_eq!(parsed["done"], true);
    }

    #[test]
    fn parse_agent_response_skips_non_json_braces_before_payload() {
        let parsed = parse_agent_response(
            r#"先说明：{这不是 JSON}
{"message":"继续","done":false,"actions":[{"tool":"list_dir","path":"."}]}
后续文字"#,
        )
        .expect("first valid json object should parse");

        assert_eq!(parsed["message"], "继续");
        assert_eq!(parsed["actions"][0]["tool"], "list_dir");
    }

    #[test]
    fn parse_agent_response_ignores_braces_inside_json_strings() {
        let parsed = parse_agent_response(
            r#"prefix {"message":"literal { brace } text","done":true,"actions":[]} suffix"#,
        )
        .expect("json string braces should not break scanning");

        assert_eq!(parsed["message"], "literal { brace } text");
        assert_eq!(parsed["done"], true);
    }

    #[tokio::test]
    async fn runtime_loop_emits_structured_status_events() {
        let workspace = temp_test_dir("runtime_loop_emits_structured_status_events");
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let (out_tx, mut out_rx) = mpsc::unbounded_channel();
        let runtime = tokio::spawn(async move {
            run_runtime_loop(
                RuntimeLoopOptions {
                    req_id: "req-status",
                    label: "test-runtime",
                    guard: ToolGuard::new(workspace, Some("project_write")),
                    prompt: "finish",
                    approval_state: Some(ToolApprovalState::default()),
                    cancel_rx,
                    out_tx,
                    task_journal: None,
                    initial_model: Some("test-model".to_string()),
                },
                move |_| async move {
                    Ok(chat_response(json!({
                        "message": "done",
                        "done": true,
                        "actions": []
                    })))
                },
            )
            .await
            .unwrap()
        });

        let thinking = next_tool_event(&mut out_rx, "runtime_status").await;
        assert_eq!(thinking["phase"], "thinking");
        assert_eq!(thinking["runtime"], "test-runtime");
        let completed = next_tool_event(&mut out_rx, "runtime_status").await;
        assert_eq!(completed["phase"], "completed");
        assert_eq!(completed["status"], "ok");

        let result = runtime.await.unwrap();
        assert!(result.exit_ok);
        let _ = cancel_tx.send(true);
    }

    #[tokio::test]
    async fn runtime_loop_executes_openai_tool_calls() {
        let workspace = temp_test_dir("runtime_loop_executes_openai_tool_calls");
        std::fs::write(workspace.join("README.md"), "hello\n").unwrap();
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let (out_tx, mut out_rx) = mpsc::unbounded_channel();
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_runtime = calls.clone();

        let runtime = tokio::spawn(async move {
            run_runtime_loop(
                RuntimeLoopOptions {
                    req_id: "req-tool-calls",
                    label: "api-runtime",
                    guard: ToolGuard::new(workspace, Some("project_write")),
                    prompt: "inspect files",
                    approval_state: Some(ToolApprovalState::default()),
                    cancel_rx,
                    out_tx,
                    task_journal: None,
                    initial_model: Some("gpt-test".to_string()),
                },
                move |_| {
                    let call_index = calls_for_runtime.fetch_add(1, Ordering::SeqCst);
                    async move {
                        if call_index == 0 {
                            Ok(chat_tool_call_response("list_dir", json!({ "path": "." })))
                        } else {
                            Ok(chat_response(json!({
                                "message": "done after list",
                                "done": true,
                                "actions": []
                            })))
                        }
                    }
                },
            )
            .await
            .unwrap()
        });

        let tool_call = next_tool_event(&mut out_rx, "tool_call").await;
        assert_eq!(tool_call["tool"], "list_dir");
        let tool_result = next_tool_event(&mut out_rx, "tool_result").await;
        assert_eq!(tool_result["status"], "ok");
        assert!(tool_result["result"]
            .as_str()
            .unwrap_or_default()
            .contains("README.md"));

        let result = runtime.await.unwrap();
        assert!(result.exit_ok);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        let _ = cancel_tx.send(true);
    }

    #[tokio::test]
    async fn runtime_loop_emits_canceled_summary_when_stopped_before_turn() {
        let workspace =
            temp_test_dir("runtime_loop_emits_canceled_summary_when_stopped_before_turn");
        let (_cancel_tx, cancel_rx) = watch::channel(true);
        let (out_tx, mut out_rx) = mpsc::unbounded_channel();
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_runtime = calls.clone();

        let result = run_runtime_loop(
            RuntimeLoopOptions {
                req_id: "req-canceled",
                label: "test-runtime",
                guard: ToolGuard::new(workspace, Some("project_write")),
                prompt: "finish",
                approval_state: Some(ToolApprovalState::default()),
                cancel_rx,
                out_tx,
                task_journal: None,
                initial_model: Some("test-model".to_string()),
            },
            move |_| {
                calls_for_runtime.fetch_add(1, Ordering::SeqCst);
                async move {
                    Ok(chat_response(json!({
                        "message": "should not run",
                        "done": true,
                        "actions": []
                    })))
                }
            },
        )
        .await
        .unwrap();

        let canceled = next_tool_event(&mut out_rx, "runtime_status").await;
        assert_eq!(canceled["phase"], "canceled");
        assert_eq!(canceled["status"], "canceled");
        let summary = next_tool_event(&mut out_rx, "runtime_summary").await;
        assert_eq!(summary["status"], "canceled");
        assert_eq!(summary["total_tools"], 0);
        assert!(!result.exit_ok);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn runtime_loop_emits_failure_summary_on_model_error() {
        let workspace = temp_test_dir("runtime_loop_emits_failure_summary_on_model_error");
        let (_cancel_tx, cancel_rx) = watch::channel(false);
        let (out_tx, mut out_rx) = mpsc::unbounded_channel();

        let error = match run_runtime_loop(
            RuntimeLoopOptions {
                req_id: "req-model-error",
                label: "test-runtime",
                guard: ToolGuard::new(workspace, Some("project_write")),
                prompt: "finish",
                approval_state: Some(ToolApprovalState::default()),
                cancel_rx,
                out_tx,
                task_journal: None,
                initial_model: Some("test-model".to_string()),
            },
            move |_| async move { Err(anyhow!("provider unavailable")) },
        )
        .await
        {
            Ok(_) => panic!("runtime should fail when model call fails"),
            Err(error) => error,
        };

        let thinking = next_tool_event(&mut out_rx, "runtime_status").await;
        assert_eq!(thinking["phase"], "thinking");
        let failed = next_tool_event(&mut out_rx, "runtime_status").await;
        assert_eq!(failed["phase"], "failed");
        assert_eq!(failed["status"], "error");
        assert!(failed["message"]
            .as_str()
            .unwrap_or_default()
            .contains("调用 test-runtime 失败"));
        assert!(failed["message"]
            .as_str()
            .unwrap_or_default()
            .contains("provider unavailable"));
        let summary = next_tool_event(&mut out_rx, "runtime_summary").await;
        assert_eq!(summary["status"], "error");
        assert_eq!(summary["failed_tools"], 0);
        assert!(format!("{error:#}").contains("provider unavailable"));
    }

    #[tokio::test]
    async fn runtime_denies_write_without_executing_tool() {
        let workspace = temp_test_dir("runtime_denies_write_without_executing_tool");
        let target = workspace.join("blocked.txt");
        let approval_state = ToolApprovalState::default();
        let approval_decider = approval_state.clone();
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let (out_tx, mut out_rx) = mpsc::unbounded_channel();
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_runtime = calls.clone();
        let req_id = "req-deny-write".to_string();
        let runtime_req_id = req_id.clone();
        let runtime = tokio::spawn(async move {
            run_runtime_loop(
                RuntimeLoopOptions {
                    req_id: &runtime_req_id,
                    label: "test-runtime",
                    guard: ToolGuard::new(workspace, Some("project_write")),
                    prompt: "write a file",
                    approval_state: Some(approval_state),
                    cancel_rx,
                    out_tx,
                    task_journal: None,
                    initial_model: Some("test-model".to_string()),
                },
                move |_| {
                    let call_index = calls_for_runtime.fetch_add(1, Ordering::SeqCst);
                    async move {
                        if call_index == 0 {
                            Ok(chat_response(json!({
                                "message": "need write",
                                "done": false,
                                "actions": [{
                                    "tool": "write_file",
                                    "path": "blocked.txt",
                                    "content": "should not be written"
                                }]
                            })))
                        } else {
                            Ok(chat_response(json!({
                                "message": "done after deny",
                                "done": true,
                                "actions": []
                            })))
                        }
                    }
                },
            )
            .await
            .unwrap()
        });

        let approval = next_tool_event(&mut out_rx, "tool_approval_required").await;
        assert_eq!(approval["approval_id"], "tap_1_1");
        assert_eq!(approval["tool"], "write_file");
        assert_eq!(approval["diff"]["source"], "write_file");
        assert_eq!(approval["diff"]["kind"], "create");
        assert_eq!(approval["diff"]["files"][0], "blocked.txt");
        assert!(approval["diff"]["preview"]
            .as_str()
            .unwrap_or_default()
            .contains("--- /dev/null"));
        assert!(
            approval["diff"]["new_sha256"]
                .as_str()
                .unwrap_or_default()
                .len()
                >= 64
        );
        assert!(approval["diff"]["old_sha256"].is_null());
        assert!(!target.exists(), "write_file must not run before approval");

        assert!(approval_decider.decide(&req_id, "tap_1_1", "deny").await);
        let denied = next_tool_event(&mut out_rx, "tool_result").await;
        assert_eq!(denied["status"], "error");
        assert!(denied["result"]
            .as_str()
            .unwrap_or_default()
            .contains("denied by user"));

        let result = runtime.await.unwrap();
        assert!(result.exit_ok);
        assert!(
            !target.exists(),
            "denied write_file must not create the file"
        );
        let _ = cancel_tx.send(true);
    }

    #[tokio::test]
    async fn runtime_records_canceled_approval_decision_in_journal() {
        let workspace = temp_test_dir("runtime_records_canceled_approval_decision_in_journal");
        let target = workspace.join("blocked.txt");
        let task_journal = TaskJournal::new(workspace.join(".journal"));
        let journal_for_runtime = task_journal.clone();
        let approval_state = ToolApprovalState::default();
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let (out_tx, mut out_rx) = mpsc::unbounded_channel();
        let req_id = "req-cancel-approval".to_string();
        let runtime_req_id = req_id.clone();
        let runtime = tokio::spawn(async move {
            run_runtime_loop(
                RuntimeLoopOptions {
                    req_id: &runtime_req_id,
                    label: "test-runtime",
                    guard: ToolGuard::new(workspace, Some("project_write")),
                    prompt: "write a file",
                    approval_state: Some(approval_state),
                    cancel_rx,
                    out_tx,
                    task_journal: Some(journal_for_runtime),
                    initial_model: Some("test-model".to_string()),
                },
                move |_| async move {
                    Ok(chat_response(json!({
                        "message": "need write",
                        "done": false,
                        "actions": [{
                            "tool": "write_file",
                            "path": "blocked.txt",
                            "content": "should not be written"
                        }]
                    })))
                },
            )
            .await
            .unwrap()
        });

        let approval = next_tool_event(&mut out_rx, "tool_approval_required").await;
        assert_eq!(approval["approval_id"], "tap_1_1");
        assert_eq!(approval["tool"], "write_file");
        assert_eq!(
            approval["approval_checkpoint"]["schema"],
            "elon.routebc.tool_approval_checkpoint.v1"
        );
        assert_eq!(
            approval["approval_checkpoint"]["restart_recovery"]["next_action"],
            "continue_from_snapshot"
        );
        assert_eq!(
            approval["approval_checkpoint"]["restart_recovery"]["supported"].as_bool(),
            Some(false)
        );
        assert!(
            approval["approval_checkpoint"]["action_sha256"]
                .as_str()
                .unwrap_or_default()
                .len()
                >= 64
        );
        assert!(
            !serde_json::to_string(&approval["approval_checkpoint"])
                .unwrap()
                .contains("should not be written"),
            "approval checkpoint must not store write_file content"
        );
        cancel_tx.send(true).expect("cancel should reach runtime");

        let decision = next_tool_event(&mut out_rx, "tool_approval_decision").await;
        assert_eq!(decision["approval_id"], "tap_1_1");
        assert_eq!(decision["decision"], "cancel");
        assert_eq!(decision["status"], "canceled");
        let canceled = next_tool_event(&mut out_rx, "runtime_status").await;
        assert_eq!(canceled["phase"], "canceled");

        let result = runtime.await.unwrap();
        assert!(!result.exit_ok);
        assert!(
            !target.exists(),
            "canceled approval must not execute write_file"
        );

        let snapshot = task_journal
            .snapshot(&req_id, 0, 20)
            .expect("approval decision should be replayable from local journal");
        assert_eq!(snapshot.approvals.approvals.len(), 1);
        let checkpoint = snapshot.approvals.approvals[0]
            .checkpoint
            .as_ref()
            .expect("approval checkpoint should persist through task journal");
        assert_eq!(
            checkpoint["schema"],
            "elon.routebc.tool_approval_checkpoint.v1"
        );
        assert_eq!(
            checkpoint["restart_recovery"]["next_action"],
            "continue_from_snapshot"
        );
        assert!(snapshot.events.iter().any(|entry| {
            entry.event.get("type").and_then(Value::as_str) == Some("tool_event")
                && entry
                    .event
                    .get("event")
                    .and_then(|event| event.get("type"))
                    .and_then(Value::as_str)
                    == Some("tool_approval_decision")
                && entry
                    .event
                    .get("event")
                    .and_then(|event| event.get("status"))
                    .and_then(Value::as_str)
                    == Some("canceled")
        }));
    }

    #[tokio::test]
    async fn runtime_rejects_stale_write_file_after_approval() {
        let workspace = temp_test_dir("runtime_rejects_stale_write_file_after_approval");
        let target = workspace.join("note.txt");
        tokio::fs::write(&target, "old\n").await.unwrap();
        let approval_state = ToolApprovalState::default();
        let approval_decider = approval_state.clone();
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let (out_tx, mut out_rx) = mpsc::unbounded_channel();
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_runtime = calls.clone();
        let req_id = "req-stale-write".to_string();
        let runtime_req_id = req_id.clone();
        let runtime = tokio::spawn(async move {
            run_runtime_loop(
                RuntimeLoopOptions {
                    req_id: &runtime_req_id,
                    label: "test-runtime",
                    guard: ToolGuard::new(workspace, Some("project_write")),
                    prompt: "write a file",
                    approval_state: Some(approval_state),
                    cancel_rx,
                    out_tx,
                    task_journal: None,
                    initial_model: Some("test-model".to_string()),
                },
                move |_| {
                    let call_index = calls_for_runtime.fetch_add(1, Ordering::SeqCst);
                    async move {
                        if call_index == 0 {
                            Ok(chat_response(json!({
                                "message": "need write",
                                "done": false,
                                "actions": [{
                                    "tool": "write_file",
                                    "path": "note.txt",
                                    "content": "new\n"
                                }]
                            })))
                        } else {
                            Ok(chat_response(json!({
                                "message": "done after stale",
                                "done": true,
                                "actions": []
                            })))
                        }
                    }
                },
            )
            .await
            .unwrap()
        });

        let approval = next_tool_event(&mut out_rx, "tool_approval_required").await;
        assert!(approval["diff"]["preview"]
            .as_str()
            .unwrap_or_default()
            .contains("-old"));
        tokio::fs::write(&target, "changed elsewhere\n")
            .await
            .unwrap();
        assert!(approval_decider.decide(&req_id, "tap_1_1", "approve").await);

        let stale = next_tool_event(&mut out_rx, "tool_result").await;
        assert_eq!(stale["status"], "error");
        assert!(stale["result"]
            .as_str()
            .unwrap_or_default()
            .contains("approval preview is stale"));
        assert_eq!(
            tokio::fs::read_to_string(&target).await.unwrap(),
            "changed elsewhere\n"
        );

        let result = runtime.await.unwrap();
        assert!(result.exit_ok);
        let _ = cancel_tx.send(true);
    }

    #[tokio::test]
    async fn runtime_writes_file_after_approval_when_preview_is_current() {
        let workspace = temp_test_dir("runtime_writes_file_after_approval_when_preview_is_current");
        let target = workspace.join("note.txt");
        let approval_state = ToolApprovalState::default();
        let approval_decider = approval_state.clone();
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let (out_tx, mut out_rx) = mpsc::unbounded_channel();
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_runtime = calls.clone();
        let req_id = "req-approve-write".to_string();
        let runtime_req_id = req_id.clone();
        let runtime = tokio::spawn(async move {
            run_runtime_loop(
                RuntimeLoopOptions {
                    req_id: &runtime_req_id,
                    label: "test-runtime",
                    guard: ToolGuard::new(workspace, Some("project_write")),
                    prompt: "write a file",
                    approval_state: Some(approval_state),
                    cancel_rx,
                    out_tx,
                    task_journal: None,
                    initial_model: Some("test-model".to_string()),
                },
                move |_| {
                    let call_index = calls_for_runtime.fetch_add(1, Ordering::SeqCst);
                    async move {
                        if call_index == 0 {
                            Ok(chat_response(json!({
                                "message": "need write",
                                "done": false,
                                "actions": [{
                                    "tool": "write_file",
                                    "path": "note.txt",
                                    "content": "approved\n"
                                }]
                            })))
                        } else {
                            Ok(chat_response(json!({
                                "message": "done after approve",
                                "done": true,
                                "actions": []
                            })))
                        }
                    }
                },
            )
            .await
            .unwrap()
        });

        let approval = next_tool_event(&mut out_rx, "tool_approval_required").await;
        assert_eq!(approval["diff"]["kind"], "create");
        assert!(approval_decider.decide(&req_id, "tap_1_1", "approve").await);

        let tool_call = next_tool_event(&mut out_rx, "tool_call").await;
        assert_eq!(tool_call["tool"], "write_file");
        let tool_result = next_tool_event(&mut out_rx, "tool_result").await;
        assert_eq!(tool_result["status"], "ok");
        assert_eq!(
            tokio::fs::read_to_string(&target).await.unwrap(),
            "approved\n"
        );

        let result = runtime.await.unwrap();
        assert!(result.exit_ok);
        let _ = cancel_tx.send(true);
    }

    #[tokio::test]
    async fn runtime_rejects_stale_apply_patch_after_approval() {
        let workspace = temp_test_dir("runtime_rejects_stale_apply_patch_after_approval");
        let target = workspace.join("note.txt");
        tokio::fs::write(&target, "old\n").await.unwrap();
        init_git_repo(&workspace);
        let approval_state = ToolApprovalState::default();
        let approval_decider = approval_state.clone();
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let (out_tx, mut out_rx) = mpsc::unbounded_channel();
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_runtime = calls.clone();
        let req_id = "req-stale-apply-patch".to_string();
        let runtime_req_id = req_id.clone();
        let patch = "diff --git a/note.txt b/note.txt\n--- a/note.txt\n+++ b/note.txt\n@@ -1 +1 @@\n-old\n+new\n".to_string();
        let patch_for_runtime = patch.clone();
        let runtime = tokio::spawn(async move {
            run_runtime_loop(
                RuntimeLoopOptions {
                    req_id: &runtime_req_id,
                    label: "test-runtime",
                    guard: ToolGuard::new(workspace, Some("project_write")),
                    prompt: "patch a file",
                    approval_state: Some(approval_state),
                    cancel_rx,
                    out_tx,
                    task_journal: None,
                    initial_model: Some("test-model".to_string()),
                },
                move |_| {
                    let call_index = calls_for_runtime.fetch_add(1, Ordering::SeqCst);
                    let patch = patch_for_runtime.clone();
                    async move {
                        if call_index == 0 {
                            Ok(chat_response(json!({
                                "message": "need patch",
                                "done": false,
                                "actions": [{
                                    "tool": "apply_patch",
                                    "patch": patch
                                }]
                            })))
                        } else {
                            Ok(chat_response(json!({
                                "message": "done after stale",
                                "done": true,
                                "actions": []
                            })))
                        }
                    }
                },
            )
            .await
            .unwrap()
        });

        let approval = next_tool_event(&mut out_rx, "tool_approval_required").await;
        assert_eq!(approval["tool"], "apply_patch");
        assert_eq!(approval["diff"]["source"], "apply_patch");
        assert_eq!(approval["diff"]["files"][0], "note.txt");
        assert!(approval["diff"]["preview"]
            .as_str()
            .unwrap_or_default()
            .contains("-old"));
        assert!(
            approval["diff"]["patch_sha256"]
                .as_str()
                .unwrap_or_default()
                .len()
                >= 64
        );
        tokio::fs::write(&target, "changed elsewhere\n")
            .await
            .unwrap();
        assert!(approval_decider.decide(&req_id, "tap_1_1", "approve").await);

        let stale = next_tool_event(&mut out_rx, "tool_result").await;
        assert_eq!(stale["status"], "error");
        assert!(stale["result"]
            .as_str()
            .unwrap_or_default()
            .contains("approval preview is stale"));
        assert_eq!(
            tokio::fs::read_to_string(&target).await.unwrap(),
            "changed elsewhere\n"
        );

        let result = runtime.await.unwrap();
        assert!(result.exit_ok);
        let _ = cancel_tx.send(true);
    }

    async fn next_tool_event(
        out_rx: &mut mpsc::UnboundedReceiver<Message>,
        event_type: &str,
    ) -> Value {
        let deadline = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while let Some(frame) = out_rx.recv().await {
                let Message::Text(text) = frame else {
                    continue;
                };
                let Ok(AgentToServer::CliChunk { text, .. }) =
                    serde_json::from_str::<AgentToServer>(&text)
                else {
                    continue;
                };
                let Ok(value) = serde_json::from_str::<Value>(text.trim()) else {
                    continue;
                };
                if value.get("type").and_then(Value::as_str) == Some(event_type) {
                    return value;
                }
            }
            panic!("event stream closed before {event_type}");
        })
        .await;
        deadline.unwrap_or_else(|_| panic!("timed out waiting for {event_type}"))
    }

    fn chat_response(agent: Value) -> Value {
        json!({
            "model": "test-model",
            "choices": [{
                "message": {
                    "content": serde_json::to_string(&agent).unwrap()
                }
            }]
        })
    }

    fn chat_tool_call_response(name: &str, arguments: Value) -> Value {
        json!({
            "model": "test-model",
            "choices": [{
                "message": {
                    "content": null,
                    "tool_calls": [{
                        "id": "call_test",
                        "type": "function",
                        "function": {
                            "name": name,
                            "arguments": serde_json::to_string(&arguments).unwrap()
                        }
                    }]
                }
            }]
        })
    }

    fn init_git_repo(path: &std::path::Path) {
        let status = std::process::Command::new("git")
            .args(["init"])
            .current_dir(path)
            .status()
            .unwrap();
        assert!(status.success());
    }

    fn temp_test_dir(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let path = std::env::temp_dir().join(format!("elon-{name}-{nanos}"));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }
}

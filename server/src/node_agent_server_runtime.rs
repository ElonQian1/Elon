// server/src/node_agent_server_runtime.rs

use crate::{
    node_agent_runtime_approval::{
        requires_tool_approval, wait_for_tool_approval, ApprovalOutcome,
    },
    node_agent_runtime_events::{
        runtime_status_chunk, tool_approval_checkpoint, tool_approval_decision_chunk,
        tool_approval_id, tool_approval_required_chunk_with_diff_and_checkpoint, tool_call_chunk,
        tool_name, tool_result_chunk,
    },
    node_agent_task_journal::TaskJournal,
    node_agent_tool_approval::ToolApprovalState,
    node_agent_tool_guard::ToolGuard,
};
use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use std::{future::Future, time::Duration};
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
        .no_proxy() // 绕过本机代理（代理可能停止或不稳定，混元 API 支持直连）
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
                        tool_call_id_results.push((
                            action_tool_call_id.clone(),
                            truncate_chars(&result, MAX_TOOL_RESULT_CHARS),
                        ));
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
                        tool_call_id_results.push((
                            action_tool_call_id.clone(),
                            truncate_chars(&result, MAX_TOOL_RESULT_CHARS),
                        ));
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
                        tool_call_id_results.push((
                            action_tool_call_id.clone(),
                            truncate_chars(&result, MAX_TOOL_RESULT_CHARS),
                        ));
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
                        tool_call_id_results.push((
                            action_tool_call_id.clone(),
                            truncate_chars(&result, MAX_TOOL_RESULT_CHARS),
                        ));
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
                    tool_call_id_results.push((
                        action_tool_call_id.clone(),
                        truncate_chars(&result, MAX_TOOL_RESULT_CHARS),
                    ));
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
            tool_call_id_results.push((
                action_tool_call_id.clone(),
                truncate_chars(&result, MAX_TOOL_RESULT_CHARS),
            ));
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

#[cfg(test)]
mod runtime_test;
pub(crate) mod utils;

use self::utils::*;

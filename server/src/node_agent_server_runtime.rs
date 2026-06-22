// server/src/node_agent_server_runtime.rs

use crate::{
    agent_runtime_error_summary::operational_error_summary,
    node_agent_runtime_approval::{
        requires_tool_approval, wait_for_tool_approval, ApprovalOutcome,
    },
    node_agent_runtime_events::{
        runtime_status_chunk, tool_approval_decision_chunk, tool_approval_id,
        tool_approval_required_chunk, tool_approval_required_chunk_with_diff, tool_call_chunk,
        tool_name, tool_result_chunk,
    },
    node_agent_task_journal::TaskJournal,
    node_agent_tool_approval::ToolApprovalState,
    node_agent_tool_guard::{truncate_chars, ToolGuard},
};
use anyhow::{anyhow, bail, Context, Result};
use homecli_proto::AgentToServer;
use serde_json::{json, Value};
use std::{future::Future, path::PathBuf, time::Duration};
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::tungstenite::Message;

const MAX_TURNS: usize = 8;
const MAX_TOOL_RESULT_CHARS: usize = 24_000;

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
        json!({"role": "system", "content": system_prompt(label, guard.read_only())}),
        json!({"role": "user", "content": prompt}),
    ];

    let mut usage = RuntimeUsage::default();
    let mut model = initial_model;
    for turn in 1..=MAX_TURNS {
        if *cancel_rx.borrow() {
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
        let response = tokio::select! {
            result = call_chat(call_messages.clone()) => {
                result.with_context(|| format!("调用 {label} 失败"))?
            }
            changed = cancel_rx.changed() => {
                if changed.is_ok() && *cancel_rx.borrow() {
                    return Ok(canceled_runtime_result(label, model, &usage));
                }
                call_chat(call_messages)
                    .await
                    .with_context(|| format!("调用 {label} 失败"))?
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
        let content = extract_assistant_content(&response)?;
        messages.push(json!({"role": "assistant", "content": content}));
        let agent = parse_agent_response(&content)?;
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
        for (index, action) in actions.into_iter().enumerate() {
            let tool_index = index + 1;
            let tool = tool_name(&action);
            let mut approved_write_file_diff = None;
            if requires_tool_approval(&guard, &action) {
                let approval_id = tool_approval_id(turn, tool_index);
                let write_file_approval_diff = match guard.write_file_diff_preview(&action).await {
                    Ok(diff) => diff,
                    Err(error) => {
                        let result = format!(
                            "error: {tool} approval preview unavailable: {error}; tool was not executed"
                        );
                        send_chunk(
                            &out_tx,
                            task_journal.as_ref(),
                            req_id,
                            tool_result_chunk(req_id, turn, tool_index, &tool, &result),
                        );
                        results.push(json!({
                            "tool": tool,
                            "result": truncate_chars(&result, MAX_TOOL_RESULT_CHARS),
                        }));
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
                            tool_result_chunk(req_id, turn, tool_index, &tool, &result),
                        );
                        results.push(json!({
                            "tool": tool,
                            "result": truncate_chars(&result, MAX_TOOL_RESULT_CHARS),
                        }));
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
                    match write_file_approval_diff.as_ref() {
                        Some(diff) => tool_approval_required_chunk_with_diff(
                            req_id,
                            turn,
                            tool_index,
                            &approval_id,
                            &action,
                            diff.clone(),
                        ),
                        None => tool_approval_required_chunk(
                            req_id,
                            turn,
                            tool_index,
                            &approval_id,
                            &action,
                        ),
                    },
                );
                match wait_for_tool_approval(&mut waiter, &mut cancel_rx).await {
                    ApprovalOutcome::Approved => {
                        approved_write_file_diff = write_file_approval_diff;
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
                            ),
                        );
                        let result = format!("error: {tool} denied by user: {reason}");
                        send_chunk(
                            &out_tx,
                            task_journal.as_ref(),
                            req_id,
                            tool_result_chunk(req_id, turn, tool_index, &tool, &result),
                        );
                        results.push(json!({
                            "tool": tool,
                            "result": truncate_chars(&result, MAX_TOOL_RESULT_CHARS),
                        }));
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
                            ),
                        );
                        let result =
                            format!("error: {tool} approval timed out; tool was not executed");
                        send_chunk(
                            &out_tx,
                            task_journal.as_ref(),
                            req_id,
                            tool_result_chunk(req_id, turn, tool_index, &tool, &result),
                        );
                        results.push(json!({
                            "tool": tool,
                            "result": truncate_chars(&result, MAX_TOOL_RESULT_CHARS),
                        }));
                        continue;
                    }
                    ApprovalOutcome::Canceled => {
                        return Ok(canceled_runtime_result(label, model, &usage));
                    }
                }
            }
            if let Some(diff) = approved_write_file_diff.as_ref() {
                if let Err(error) = guard
                    .verify_write_file_preview_unchanged(&action, diff)
                    .await
                {
                    let result = format!(
                        "error: {tool} approval preview is stale: {error}; tool was not executed"
                    );
                    send_chunk(
                        &out_tx,
                        task_journal.as_ref(),
                        req_id,
                        tool_result_chunk(req_id, turn, tool_index, &tool, &result),
                    );
                    results.push(json!({
                        "tool": tool,
                        "result": truncate_chars(&result, MAX_TOOL_RESULT_CHARS),
                    }));
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
                tool_result_chunk(req_id, turn, tool_index, &tool, &result),
            );
            results.push(json!({
                "tool": tool,
                "result": truncate_chars(&result, MAX_TOOL_RESULT_CHARS),
            }));
        }
        messages.push(json!({
            "role": "user",
            "content": format!("Tool results JSON:\n{}", serde_json::to_string(&results)?),
        }));

        if agent.get("done").and_then(Value::as_bool).unwrap_or(false) {
            send_chunk(
                &out_tx,
                task_journal.as_ref(),
                req_id,
                runtime_status_chunk(req_id, turn, label, "completed", "工具结果已处理，任务完成"),
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

    Ok(ServerRuntimeRunResult {
        exit_ok: false,
        error: Some(format!("{label} 超过 {MAX_TURNS} 轮仍未完成")),
        model: model.or_else(|| Some(label.to_string())),
        prompt_tokens: usage.prompt_tokens,
        completion_tokens: usage.completion_tokens,
        total_tokens: usage.total_tokens,
    })
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

fn system_prompt(label: &str, read_only: bool) -> String {
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
    {"tool": "read_file", "path": "README.md"},
    {"tool": "read_file_range", "path": "src/main.rs", "start_line": 120, "line_count": 80},
    {"tool": "write_file", "path": "docs/note.md", "content": "full content"},
    {"tool": "apply_patch", "patch": "unified diff", "check_only": false},
    {"tool": "run_command", "program": "git", "args": ["status", "--short"], "reason": "inspect git state"}
  ]
}

Rules:
- Paths must be relative to the current project workspace.
- Prefer read-only actions first.
- Use read_file_range instead of read_file for large files when you only need one section.
- Do not request destructive commands, privilege changes, downloads that execute code, persistence, credential access, or writes outside the project.
- Prefer apply_patch with unified diff for intentional edits to existing project files.
- Use write_file only when replacing a full file or creating a small new project file.
- Use run_command only for project Git, build, format, lint, or test commands.
- Prefer structured run_command with program and args. The legacy command string field exists only for older clients.
- Set done=true when no further tool action is needed.
"#
    .replace("{{runtime_identity}}", runtime_identity);
    if read_only {
        prompt.push_str(
            "\nCurrent mode is read-only planning. Do not request write_file, apply_patch, or run_command.\n",
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
    let body = response.text().await.unwrap_or_default();
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
    let response = client
        .post(url)
        .bearer_auth(&config.api_key)
        .json(&json!({
            "model": config.model,
            "messages": messages,
            "temperature": 0.2
        }))
        .send()
        .await?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        bail!(
            "{}",
            runtime_http_error_message("本机 API runtime", status, &body)
        );
    }
    serde_json::from_str(&body).context("本机 API runtime 响应不是 JSON")
}

fn runtime_http_error_message(label: &str, status: reqwest::StatusCode, body: &str) -> String {
    format!("{label} 返回 {status}: {}", operational_error_summary(body))
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
    let start = trimmed
        .find('{')
        .ok_or_else(|| anyhow!("server-runtime 返回内容不是 JSON"))?;
    let end = trimmed
        .rfind('}')
        .ok_or_else(|| anyhow!("server-runtime 返回内容不是 JSON"))?;
    serde_json::from_str(&trimmed[start..=end]).context("server-runtime JSON 解析失败")
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
        api_runtime_config_from_lookup, run_runtime_loop, runtime_http_error_message,
        system_prompt, RuntimeLoopOptions,
    };
    use crate::{node_agent_tool_approval::ToolApprovalState, node_agent_tool_guard::ToolGuard};
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
    fn system_prompt_matches_runtime_route_identity() {
        let route_b = system_prompt("api-runtime", false);
        let route_c = system_prompt("server-runtime", true);

        assert!(route_b.contains("Route B local API runtime"));
        assert!(!route_b.contains("Route C server runtime for"));
        assert!(route_c.contains("Route C server runtime"));
        assert!(route_c.contains("read-only planning"));
        assert!(route_c.contains("Do not request write_file, apply_patch, or run_command"));
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

// server/src/node_agent_server_runtime.rs

use crate::node_agent_tool_guard::{truncate_chars, ToolGuard};
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

pub(crate) async fn run_server_runtime_prompt(
    req_id: &str,
    config: ServerRuntimeConfig,
    cwd: Option<&str>,
    runtime_permission: Option<&str>,
    prompt: &str,
    cancel_rx: watch::Receiver<bool>,
    out_tx: mpsc::UnboundedSender<Message>,
) -> ServerRuntimeRunResult {
    match run_server_runtime_inner(
        req_id,
        config,
        cwd,
        runtime_permission,
        prompt,
        cancel_rx,
        out_tx,
    )
    .await
    {
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
    req_id: &str,
    cwd: Option<&str>,
    runtime_permission: Option<&str>,
    prompt: &str,
    cancel_rx: watch::Receiver<bool>,
    out_tx: mpsc::UnboundedSender<Message>,
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
    match run_api_runtime_inner(
        req_id,
        config,
        cwd,
        runtime_permission,
        prompt,
        cancel_rx,
        out_tx,
    )
    .await
    {
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
        &["ELON_AGENT_API_BASE", "OPENAI_API_BASE", "HUNYUAN_API_BASE"],
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
    req_id: &str,
    config: ServerRuntimeConfig,
    cwd: Option<&str>,
    runtime_permission: Option<&str>,
    prompt: &str,
    cancel_rx: watch::Receiver<bool>,
    out_tx: mpsc::UnboundedSender<Message>,
) -> Result<ServerRuntimeRunResult> {
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
        req_id,
        "server-runtime",
        guard,
        prompt,
        cancel_rx,
        out_tx,
        Some("server-runtime".to_string()),
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
    req_id: &str,
    config: ApiRuntimeConfig,
    cwd: Option<&str>,
    runtime_permission: Option<&str>,
    prompt: &str,
    cancel_rx: watch::Receiver<bool>,
    out_tx: mpsc::UnboundedSender<Message>,
) -> Result<ServerRuntimeRunResult> {
    let workspace = resolve_workspace(cwd)?;
    let guard = ToolGuard::new(workspace, runtime_permission);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(150))
        .build()
        .unwrap_or_default();
    let initial_model = Some(config.model.clone());
    run_runtime_loop(
        req_id,
        "api-runtime",
        guard,
        prompt,
        cancel_rx,
        out_tx,
        initial_model,
        move |messages| {
            let client = client.clone();
            let config = config.clone();
            async move { call_api_runtime(&client, &config, &messages).await }
        },
    )
    .await
}

async fn run_runtime_loop<F, Fut>(
    req_id: &str,
    label: &str,
    mut guard: ToolGuard,
    prompt: &str,
    mut cancel_rx: watch::Receiver<bool>,
    out_tx: mpsc::UnboundedSender<Message>,
    initial_model: Option<String>,
    mut call_chat: F,
) -> Result<ServerRuntimeRunResult>
where
    F: FnMut(Vec<Value>) -> Fut,
    Fut: Future<Output = Result<Value>>,
{
    let mut messages = vec![
        json!({"role": "system", "content": system_prompt(guard.read_only())}),
        json!({"role": "user", "content": prompt}),
    ];

    let mut usage = RuntimeUsage::default();
    let mut model = initial_model;
    for turn in 1..=MAX_TURNS {
        if *cancel_rx.borrow() {
            return Ok(canceled_runtime_result(label, model, &usage));
        }
        send_chunk(&out_tx, req_id, format!("[{label}] turn {turn}\n"));
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
            send_chunk(&out_tx, req_id, format!("{message}\n"));
        }

        let actions = agent
            .get("actions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if actions.is_empty() {
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
        for action in actions {
            let tool = action
                .get("tool")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            send_chunk(&out_tx, req_id, format!("[tool] {tool}\n"));
            let result = guard.invoke_action(&action).await;
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

    Ok(ServerRuntimeRunResult {
        exit_ok: false,
        error: Some(format!("server-runtime 超过 {MAX_TURNS} 轮仍未完成")),
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

fn system_prompt(read_only: bool) -> String {
    let mut prompt = r#"You are the Elon Route C server runtime for a Windows PC project.
Return strict JSON only, without markdown fences.

Schema:
{
  "message": "short progress or final answer",
  "done": false,
  "actions": [
    {"tool": "list_dir", "path": "."},
    {"tool": "read_file", "path": "README.md"},
    {"tool": "write_file", "path": "docs/note.md", "content": "full content"},
    {"tool": "run_command", "command": "git status --short", "reason": "inspect git state"}
  ]
}

Rules:
- Paths must be relative to the current project workspace.
- Prefer read-only actions first.
- Do not request destructive commands, privilege changes, downloads that execute code, persistence, credential access, or writes outside the project.
- Use write_file for intentional project files.
- Use run_command only for project Git, build, format, lint, or test commands.
- Set done=true when no further tool action is needed.
"#
    .to_string();
    if read_only {
        prompt.push_str(
            "\nCurrent mode is read-only planning. Do not request write_file or run_command.\n",
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
        bail!("服务器 AI runtime 返回 {status}: {body}");
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
        bail!("本机 API runtime 返回 {status}: {body}");
    }
    serde_json::from_str(&body).context("本机 API runtime 响应不是 JSON")
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
    match serde_json::from_str(trimmed) {
        Ok(value) => return Ok(value),
        Err(_) => {}
    }
    let start = trimmed
        .find('{')
        .ok_or_else(|| anyhow!("server-runtime 返回内容不是 JSON"))?;
    let end = trimmed
        .rfind('}')
        .ok_or_else(|| anyhow!("server-runtime 返回内容不是 JSON"))?;
    serde_json::from_str(&trimmed[start..=end]).context("server-runtime JSON 解析失败")
}

fn send_chunk(out_tx: &mpsc::UnboundedSender<Message>, req_id: &str, text: String) {
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
    use super::api_runtime_config_from_lookup;

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
}

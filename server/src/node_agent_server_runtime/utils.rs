use anyhow::{anyhow, bail, Context, Result};
use futures::StreamExt;
use serde_json::{json, Value};
use std::{collections::VecDeque, future::Future, path::PathBuf, time::Duration};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use homecli_proto::AgentToServer;
use crate::{
    agent_runtime_error_summary::operational_error_summary,
    node_agent_runtime_events::{
        runtime_status_chunk, runtime_summary_chunk, tool_approval_checkpoint,
        tool_approval_decision_chunk, tool_approval_id,
        tool_approval_required_chunk_with_diff_and_checkpoint, tool_call_chunk, tool_name,
        tool_result_chunk,
    },
    node_agent_task_journal::TaskJournal,
    node_agent_tool_approval::ToolApprovalState,
    node_agent_tool_guard::ToolGuard,
};
pub(crate) use crate::node_agent_tool_guard::truncate_chars;
use super::{MAX_TURNS, MAX_TOOL_RESULT_CHARS, MAX_RUNTIME_HTTP_BODY_BYTES, RuntimeLoopOptions, ServerRuntimeRunResult, ApiRuntimeConfig};

pub(crate) fn record_tool_result(
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

pub(crate) fn is_tool_error(result: &str) -> bool {
    result
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("error:")
}

pub(crate) fn send_runtime_summary(
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

pub(crate) fn send_runtime_canceled(
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

pub(crate) fn send_runtime_failure(
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

pub(crate) fn resolve_workspace(cwd: Option<&str>) -> Result<PathBuf> {
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

pub(crate) fn system_prompt(label: &str, read_only: bool, danger_full_access: bool) -> String {
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

pub(crate) async fn call_server_runtime(
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

pub(crate) async fn call_api_runtime(
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
            let parsed =
                serde_json::from_str::<Value>(&body).context("本机 API runtime 响应不是 JSON")?;
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

pub(crate) async fn send_api_runtime_request(
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

pub(crate) fn api_runtime_chat_payload(
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

pub(crate) fn api_runtime_should_retry_without_json_mode(status: reqwest::StatusCode, body: &str) -> bool {
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

pub(crate) fn runtime_http_error_message(label: &str, status: reqwest::StatusCode, body: &str) -> String {
    format!("{label} 返回 {status}: {}", operational_error_summary(body))
}

pub(crate) async fn limited_runtime_response_text(response: reqwest::Response, label: &str) -> Result<String> {
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

pub(crate) fn ensure_runtime_response_size(label: &str, observed_bytes: usize) -> Result<()> {
    if observed_bytes > MAX_RUNTIME_HTTP_BODY_BYTES {
        bail!(
            "{}",
            runtime_response_too_large_message(label, observed_bytes)
        );
    }
    Ok(())
}

pub(crate) fn runtime_response_too_large_message(label: &str, observed_bytes: usize) -> String {
    format!(
        "{label} 响应过大：{} 字节，超过客户端安全上限 {} 字节，已中止读取",
        observed_bytes, MAX_RUNTIME_HTTP_BODY_BYTES
    )
}

pub(crate) fn first_value(lookup: &impl Fn(&str) -> Option<String>, names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        lookup(name)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

pub(crate) fn canceled_runtime_result(
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

pub(crate) fn extract_assistant_content(response: &Value) -> Result<String> {
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

pub(crate) fn parse_agent_response(content: &str) -> Result<Value> {
    let trimmed = content.trim();
    if let Ok(value) = serde_json::from_str(trimmed) {
        return Ok(value);
    }
    if let Some(value) = parse_first_json_object(trimmed) {
        return Ok(value);
    }
    bail!("server-runtime 返回内容不是 JSON")
}

pub(crate) fn parse_first_json_object(content: &str) -> Option<Value> {
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

pub(crate) fn matching_json_object_end(content: &str) -> Option<usize> {
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

pub(crate) fn send_chunk(
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
pub(crate) struct RuntimeUsage {
    pub(crate) prompt_tokens: Option<u64>,
    pub(crate) completion_tokens: Option<u64>,
    pub(crate) total_tokens: Option<u64>,
}

impl RuntimeUsage {
    pub(crate) fn merge(&mut self, response: &Value) {
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

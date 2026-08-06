//! ACP v1 prompt runtime for official CLI agents such as Gemini CLI.

use anyhow::{anyhow, Context, Result};
use homecli_proto::AgentToServer;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Arc, OnceLock},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
    sync::{watch, Mutex},
    time::{Duration, Instant},
};
use tokio_tungstenite::tungstenite::Message;

use crate::{
    node_agent_acp_protocol as acp, node_agent_cli_prompt_runner::ws_text,
    node_agent_provider_auth_protocol::select_gemini_auth_method, node_agent_runtime::NodeRuntime,
};

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_RPC_LINE_BYTES: usize = 2 * 1024 * 1024;
const INITIALIZE_ID: i64 = 1;
const AUTHENTICATE_ID: i64 = 2;
const SESSION_ID: i64 = 3;
const PROMPT_ID: i64 = 4;

#[derive(Debug)]
pub(crate) struct AcpPromptResult {
    pub(crate) exit_ok: bool,
    pub(crate) error: Option<String>,
    pub(crate) session_id: Option<String>,
    pub(crate) model: Option<String>,
}

pub(crate) struct AcpPromptOptions<'a> {
    pub(crate) req_id: &'a str,
    pub(crate) program: &'a str,
    pub(crate) cwd: Option<&'a str>,
    pub(crate) prompt: &'a str,
    pub(crate) extra_args: &'a [String],
    pub(crate) read_only: bool,
    pub(crate) timeout_secs: u64,
    pub(crate) runtime: &'a Arc<NodeRuntime>,
    pub(crate) cancel_rx: watch::Receiver<bool>,
    pub(crate) out_tx: tokio::sync::mpsc::UnboundedSender<Message>,
}

pub(crate) async fn run_gemini_prompt(options: AcpPromptOptions<'_>) -> AcpPromptResult {
    match run_gemini_prompt_inner(options).await {
        Ok(result) => result,
        Err(error) => AcpPromptResult {
            exit_ok: false,
            error: Some(safe_error(&error.to_string())),
            session_id: None,
            model: Some("Gemini CLI · ACP v1".to_string()),
        },
    }
}

async fn run_gemini_prompt_inner(mut options: AcpPromptOptions<'_>) -> Result<AcpPromptResult> {
    let cwd = options
        .cwd
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .context("Gemini ACP 会话缺少工作目录")?;
    let (mut child, mut stdin, mut stdout) = spawn_agent(options.program, cwd).await?;

    write_message(&mut stdin, &acp::initialize_request(INITIALIZE_ID)).await?;
    let initialize = wait_for_response(&mut stdout, INITIALIZE_ID, HANDSHAKE_TIMEOUT).await?;
    let initialize = response_result(initialize, INITIALIZE_ID)?;
    let model = acp::agent_info_label(&initialize)
        .map(|label| format!("{label} · ACP v1"))
        .or_else(|| Some("Gemini CLI · ACP v1".to_string()));

    if let Some(method_id) = select_gemini_auth_method(&initialize) {
        write_message(
            &mut stdin,
            &acp::authenticate_request(AUTHENTICATE_ID, &method_id),
        )
        .await?;
        let authenticated =
            wait_for_response(&mut stdout, AUTHENTICATE_ID, HANDSHAKE_TIMEOUT).await?;
        response_result(authenticated, AUTHENTICATE_ID)
            .context("Gemini CLI 账号未完成认证，请先在账号中心登录")?;
    }

    let session_key = acp::session_scope_key(options.extra_args);
    let store_path = session_store_path(options.runtime).await;
    let session_id = open_session(
        &mut stdin,
        &mut stdout,
        &initialize,
        cwd,
        session_key.as_deref(),
        store_path.as_deref(),
    )
    .await?;

    let attachments = acp::attachment_paths(options.extra_args);
    write_message(
        &mut stdin,
        &acp::prompt_request(PROMPT_ID, &session_id, options.prompt, &attachments),
    )
    .await?;

    let deadline = Instant::now() + Duration::from_secs(options.timeout_secs.max(30));
    let mut tool_kinds = HashMap::<String, String>::new();
    let result = loop {
        let mut line = String::new();
        tokio::select! {
            changed = options.cancel_rx.changed() => {
                if changed.is_ok() && *options.cancel_rx.borrow() {
                    let _ = write_message(&mut stdin, &acp::cancel_notification(&session_id)).await;
                    break AcpPromptResult {
                        exit_ok: false,
                        error: Some("Gemini ACP 任务已取消".to_string()),
                        session_id: Some(session_id.clone()),
                        model: model.clone(),
                    };
                }
            }
            _ = tokio::time::sleep_until(deadline) => {
                let _ = write_message(&mut stdin, &acp::cancel_notification(&session_id)).await;
                break AcpPromptResult {
                    exit_ok: false,
                    error: Some(format!("Gemini ACP 任务超过 {} 秒", options.timeout_secs.max(30))),
                    session_id: Some(session_id.clone()),
                    model: model.clone(),
                };
            }
            read = stdout.read_line(&mut line) => {
                let bytes = read?;
                if bytes == 0 {
                    break AcpPromptResult {
                        exit_ok: false,
                        error: Some("Gemini ACP 进程在完成响应前退出".to_string()),
                        session_id: Some(session_id.clone()),
                        model: model.clone(),
                    };
                }
                if line.len() > MAX_RPC_LINE_BYTES {
                    return Err(anyhow!("Gemini ACP 消息超过大小限制"));
                }
                let Ok(message) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                if let Some(text) = acp::agent_message_text(&message) {
                    let _ = options.out_tx.send(ws_text(&AgentToServer::CliChunk {
                        req_id: options.req_id.to_string(),
                        text: text.to_string(),
                    }));
                    continue;
                }
                if let Some((tool_id, kind)) = acp::tool_call_descriptor(&message) {
                    tool_kinds.insert(tool_id, kind);
                    continue;
                }
                if message.get("method").and_then(Value::as_str) == Some("session/request_permission") {
                    let tool_id = message.pointer("/params/toolCall/toolCallId").and_then(Value::as_str);
                    let kind = tool_id.and_then(|id| tool_kinds.get(id)).map(String::as_str);
                    if let Some(response) = acp::permission_response(&message, kind, options.read_only) {
                        write_message(&mut stdin, &response).await?;
                    }
                    continue;
                }
                if let Some(response) = acp::response_result(&message, PROMPT_ID) {
                    match response {
                        Ok(value) => {
                            let stop_reason = value.get("stopReason").and_then(Value::as_str).unwrap_or("end_turn");
                            let exit_ok = !matches!(stop_reason, "cancelled");
                            break AcpPromptResult {
                                exit_ok,
                                error: (!exit_ok).then(|| format!("Gemini ACP 已停止：{stop_reason}")),
                                session_id: Some(session_id.clone()),
                                model: model.clone(),
                            };
                        }
                        Err(error) => {
                            break AcpPromptResult {
                                exit_ok: false,
                                error: Some(safe_error(&error)),
                                session_id: Some(session_id.clone()),
                                model: model.clone(),
                            };
                        }
                    }
                }
                if message.get("id").is_some() && message.get("method").is_some() {
                    if let Some(response) = acp::method_not_supported_response(&message) {
                        write_message(&mut stdin, &response).await?;
                    }
                }
            }
        }
    };
    stop_child(&mut child).await;
    Ok(result)
}

async fn open_session(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
    initialize: &Value,
    cwd: &str,
    session_key: Option<&str>,
    store_path: Option<&Path>,
) -> Result<String> {
    if acp::load_session_supported(initialize) {
        if let (Some(key), Some(path)) = (session_key, store_path) {
            if let Some(saved) = load_session_id(path, key).await {
                write_message(stdin, &acp::load_session_request(SESSION_ID, &saved, cwd)).await?;
                let response = wait_for_response(stdout, SESSION_ID, HANDSHAKE_TIMEOUT).await?;
                if response_result(response, SESSION_ID).is_ok() {
                    return Ok(saved);
                }
                remove_session_id(path, key).await;
            }
        }
    }

    write_message(stdin, &acp::new_session_request(SESSION_ID, cwd)).await?;
    let response = wait_for_response(stdout, SESSION_ID, HANDSHAKE_TIMEOUT).await?;
    let result = response_result(response, SESSION_ID)?;
    let session_id = acp::session_id(&result).context("Gemini ACP 未返回 sessionId")?;
    if let (Some(key), Some(path)) = (session_key, store_path) {
        save_session_id(path, key, &session_id).await?;
    }
    Ok(session_id)
}

async fn spawn_agent(
    program: &str,
    cwd: &str,
) -> Result<(Child, ChildStdin, BufReader<ChildStdout>)> {
    let mut std_command = elon_pc_dev_runtime::command_from_path(Path::new(program));
    std_command.arg("--acp").current_dir(cwd);
    let mut command = Command::from(std_command);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    crate::node_agent_exec::hide_tokio_command_window(&mut command);
    let mut child = command
        .spawn()
        .with_context(|| format!("无法启动 Gemini ACP：{program}"))?;
    let stdin = child.stdin.take().context("Gemini ACP stdin 不可用")?;
    let stdout = child.stdout.take().context("Gemini ACP stdout 不可用")?;
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
                line.clear();
            }
        });
    }
    Ok((child, stdin, BufReader::new(stdout)))
}

async fn write_message(stdin: &mut ChildStdin, message: &Value) -> Result<()> {
    let mut bytes = serde_json::to_vec(message)?;
    bytes.push(b'\n');
    stdin.write_all(&bytes).await?;
    stdin.flush().await?;
    Ok(())
}

async fn wait_for_response(
    stdout: &mut BufReader<ChildStdout>,
    expected_id: i64,
    wait: Duration,
) -> Result<Value> {
    tokio::time::timeout(wait, async {
        loop {
            let mut line = String::new();
            let bytes = stdout.read_line(&mut line).await?;
            if bytes == 0 {
                return Err(anyhow!("ACP agent 在握手期间退出"));
            }
            if line.len() > MAX_RPC_LINE_BYTES {
                return Err(anyhow!("ACP agent 握手消息超过大小限制"));
            }
            let Ok(message) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            if message.get("id").and_then(Value::as_i64) == Some(expected_id) {
                return Ok(message);
            }
        }
    })
    .await
    .map_err(|_| anyhow!("ACP agent 握手超时"))?
}

fn response_result(message: Value, expected_id: i64) -> Result<Value> {
    acp::response_result(&message, expected_id)
        .context("ACP agent 返回了不匹配的响应")?
        .map_err(|error| anyhow!(safe_error(&error)))
}

async fn stop_child(child: &mut Child) {
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill().await;
    }
    let _ = child.wait().await;
}

fn safe_error(message: &str) -> String {
    crate::node_agent_cli_redaction::redact_text(message)
        .chars()
        .take(1000)
        .collect()
}

#[derive(Default, Serialize, Deserialize)]
struct AcpSessionStore {
    #[serde(default)]
    sessions: HashMap<String, String>,
}

fn session_file_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

async fn session_store_path(runtime: &NodeRuntime) -> Option<PathBuf> {
    let root = runtime.node_data_root.read().await;
    root.paths
        .as_ref()
        .map(|paths| paths.cache().join("provider-sessions").join("acp-v1.json"))
}

async fn load_session_id(path: &Path, key: &str) -> Option<String> {
    let _guard = session_file_lock().lock().await;
    read_session_store(path).sessions.get(key).cloned()
}

async fn save_session_id(path: &Path, key: &str, session_id: &str) -> Result<()> {
    let _guard = session_file_lock().lock().await;
    let mut store = read_session_store(path);
    store
        .sessions
        .insert(key.to_string(), session_id.to_string());
    crate::node_agent_atomic_file::write(path, &serde_json::to_vec_pretty(&store)?)
}

async fn remove_session_id(path: &Path, key: &str) {
    let _guard = session_file_lock().lock().await;
    let mut store = read_session_store(path);
    if store.sessions.remove(key).is_some() {
        if let Ok(bytes) = serde_json::to_vec_pretty(&store) {
            let _ = crate::node_agent_atomic_file::write(path, &bytes);
        }
    }
}

fn read_session_store(path: &Path) -> AcpSessionStore {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

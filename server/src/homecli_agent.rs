//! Server side of the reverse-WSS tunnel that lets the cloud elon-cli relay
//! dispatch shell commands to a homecli agent running on a developer's PC.
//!
//! Endpoint: `GET /agent/ws`
//! Auth: `Authorization: Bearer <agent_secret>`; the `<agent_id>:<agent_secret>`
//! pair is configured via the `ELON_AGENT_SECRETS` env var
//! (format: `id1:secret1,id2:secret2`).
//!
//! Protocol: see [`homecli_proto`].

use anyhow::{anyhow, Result};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use futures::{SinkExt, StreamExt};
use homecli_proto::{AgentToServer, ServerToAgent};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::{mpsc, Mutex, RwLock};
use uuid::Uuid;

use crate::types::AppState;

// ── manager state ────────────────────────────────────────────────────────────

/// Snapshot of one connected PC agent.
#[derive(Clone)]
pub struct AgentEntry {
    pub agent_id: String,
    pub version: String,
    pub allowed_clis: Vec<String>,
    pub allowed_cwds: Vec<String>,
    pub connected_at: u64,
    /// Outbound queue: server pushes ServerToAgent here, WS writer drains.
    cmd_tx: mpsc::UnboundedSender<ServerToAgent>,
    /// For each in-flight task: where to forward AgentToServer events.
    pending: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<AgentToServer>>>>,
}

#[derive(Default)]
pub struct AgentManager {
    agents: RwLock<HashMap<String, AgentEntry>>,
}

impl AgentManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// 返回第一个当前在线的 PC agent ID（优先用于 CLI 委托）
    pub async fn any_connected_agent_id(&self) -> Option<String> {
        self.agents.read().await.keys().next().cloned()
    }

    /// 把 AI 提示发给 PC agent，让 PC 用指定 CLI（copilot/codex）执行，流式返回 CliChunk/CliDone。
    pub async fn dispatch_cli_prompt(
        &self,
        agent_id: &str,
        cli: String,
        extra_args: Vec<String>,
        prompt: String,
    ) -> Result<(String, mpsc::UnboundedReceiver<AgentToServer>)> {
        let req_id = Uuid::new_v4().to_string();
        let agents = self.agents.read().await;
        let agent = agents
            .get(agent_id)
            .ok_or_else(|| anyhow!("agent not connected: {agent_id}"))?;
        let (tx, rx) = mpsc::unbounded_channel();
        agent.pending.lock().await.insert(req_id.clone(), tx);
        agent
            .cmd_tx
            .send(ServerToAgent::CliPrompt {
                req_id: req_id.clone(),
                cli,
                extra_args,
                prompt,
            })
            .map_err(|_| anyhow!("agent writer closed"))?;
        Ok((req_id, rx))
    }

    pub async fn list(&self) -> Vec<AgentSummary> {
        self.agents
            .read()
            .await
            .values()
            .map(|a| AgentSummary {
                agent_id: a.agent_id.clone(),
                version: a.version.clone(),
                allowed_clis: a.allowed_clis.clone(),
                allowed_cwds: a.allowed_cwds.clone(),
                connected_at: a.connected_at,
            })
            .collect()
    }

    /// Send an HTTP request through the WS tunnel to the PC's local server.
    /// Returns a single AgentToServer::HttpResponse or HttpError.
    pub async fn dispatch_http(
        &self,
        agent_id: &str,
        method: String,
        path: String,
        headers: Vec<(String, String)>,
        body_b64: Option<String>,
    ) -> Result<AgentToServer> {
        let req_id = Uuid::new_v4().to_string();
        let agents = self.agents.read().await;
        let agent = agents
            .get(agent_id)
            .ok_or_else(|| anyhow!("agent not connected: {agent_id}"))?;
        let (tx, mut rx) = mpsc::unbounded_channel();
        agent.pending.lock().await.insert(req_id.clone(), tx);
        agent
            .cmd_tx
            .send(ServerToAgent::HttpRequest {
                req_id: req_id.clone(),
                method,
                path,
                headers,
                body_b64,
            })
            .map_err(|_| anyhow!("agent writer closed"))?;
        drop(agents);
        // Wait for the single response frame (no streaming for HTTP relay)
        match tokio::time::timeout(Duration::from_secs(60), rx.recv()).await {
            Ok(Some(msg)) => Ok(msg),
            Ok(None) => Err(anyhow!("agent disconnected before http response")),
            Err(_) => Err(anyhow!("http relay timeout (60s)")),
        }
    }

    /// Dispatch a new exec task to the given agent. Returns a receiver that
    /// streams the agent's events for this task until `TaskExit` or `TaskError`.
    pub async fn dispatch(
        &self,
        agent_id: &str,
        cli: String,
        args: Vec<String>,
        cwd: String,
        env: Vec<(String, String)>,
    ) -> Result<(String, mpsc::UnboundedReceiver<AgentToServer>)> {
        let agents = self.agents.read().await;
        let agent = agents
            .get(agent_id)
            .ok_or_else(|| anyhow!("agent not connected: {agent_id}"))?;
        let task_id = Uuid::new_v4().to_string();
        let (tx, rx) = mpsc::unbounded_channel();
        agent.pending.lock().await.insert(task_id.clone(), tx);
        agent
            .cmd_tx
            .send(ServerToAgent::Exec {
                task_id: task_id.clone(),
                cli,
                args,
                cwd,
                env,
            })
            .map_err(|_| anyhow!("agent writer closed"))?;
        Ok((task_id, rx))
    }
}

#[derive(Debug, Serialize)]
pub struct AgentSummary {
    pub agent_id: String,
    pub version: String,
    pub allowed_clis: Vec<String>,
    pub allowed_cwds: Vec<String>,
    pub connected_at: u64,
}

// ── secret config ────────────────────────────────────────────────────────────

/// Parse `ELON_AGENT_SECRETS=id1:secret1,id2:secret2` into a map.
pub fn load_secrets() -> HashMap<String, String> {
    let raw = std::env::var("ELON_AGENT_SECRETS").unwrap_or_default();
    raw.split(',')
        .filter_map(|pair| {
            let pair = pair.trim();
            if pair.is_empty() {
                return None;
            }
            let (id, secret) = pair.split_once(':')?;
            Some((id.trim().to_string(), secret.trim().to_string()))
        })
        .collect()
}

fn extract_bearer(headers: &HeaderMap) -> Option<&str> {
    let v = headers.get("authorization")?.to_str().ok()?;
    v.strip_prefix("Bearer ")
}

// ── /agent/ws handler ────────────────────────────────────────────────────────

pub async fn agent_ws_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let token = match extract_bearer(&headers) {
        Some(t) => t.to_string(),
        None => {
            return (StatusCode::UNAUTHORIZED, "missing bearer token").into_response();
        }
    };
    let secrets = load_secrets();
    if secrets.is_empty() {
        tracing::warn!("/agent/ws rejected: ELON_AGENT_SECRETS is empty");
        return (StatusCode::UNAUTHORIZED, "agent auth not configured").into_response();
    }
    ws.on_upgrade(move |socket| handle_agent_socket(socket, state, secrets, token))
}

async fn handle_agent_socket(
    socket: WebSocket,
    state: Arc<AppState>,
    secrets: HashMap<String, String>,
    presented_token: String,
) {
    if let Err(e) = run_agent_session(socket, state, secrets, presented_token).await {
        tracing::warn!("agent ws session ended: {e:#}");
    }
}

async fn run_agent_session(
    socket: WebSocket,
    state: Arc<AppState>,
    secrets: HashMap<String, String>,
    presented_token: String,
) -> Result<()> {
    let (mut ws_tx, mut ws_rx) = socket.split();

    // First frame must be Register.
    let first = tokio::time::timeout(Duration::from_secs(10), ws_rx.next())
        .await
        .map_err(|_| anyhow!("register timeout"))?
        .ok_or_else(|| anyhow!("ws closed before register"))?
        .map_err(|e| anyhow!("ws read: {e}"))?;
    let text = match first {
        Message::Text(t) => t,
        _ => return Err(anyhow!("expected text register frame")),
    };
    let register: AgentToServer = serde_json::from_str(&text)?;
    let (agent_id, version, allowed_clis, allowed_cwds, _proto_ver) = match register {
        AgentToServer::Register {
            agent_id,
            version,
            proto_version,
            allowed_clis,
            allowed_cwds,
        } => (agent_id, version, allowed_clis, allowed_cwds, proto_version),
        _ => return Err(anyhow!("first frame must be register")),
    };

    // Auth check: presented token must equal the secret bound to agent_id.
    let expected = match secrets.get(&agent_id) {
        Some(s) => s,
        None => return Err(anyhow!("unknown agent_id: {agent_id}")),
    };
    if !constant_time_eq(expected.as_bytes(), presented_token.as_bytes()) {
        return Err(anyhow!("bad secret for agent_id={agent_id}"));
    }

    tracing::info!(%agent_id, %version, "agent registered");

    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<ServerToAgent>();
    let pending: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<AgentToServer>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let entry = AgentEntry {
        agent_id: agent_id.clone(),
        version,
        allowed_clis,
        allowed_cwds,
        connected_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        cmd_tx: cmd_tx.clone(),
        pending: pending.clone(),
    };
    state
        .agent_manager
        .agents
        .write()
        .await
        .insert(agent_id.clone(), entry);

    // Writer: drain cmd_rx → ws_tx.
    let writer = tokio::spawn(async move {
        while let Some(msg) = cmd_rx.recv().await {
            let s = match serde_json::to_string(&msg) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("serialize ServerToAgent: {e}");
                    continue;
                }
            };
            if ws_tx.send(Message::Text(s)).await.is_err() {
                break;
            }
        }
        let _ = ws_tx.close().await;
    });

    // Reader: route AgentToServer events to the right pending task.
    let pending_r = pending.clone();
    let read_result: Result<()> = async {
        while let Some(frame) = ws_rx.next().await {
            let frame = frame.map_err(|e| anyhow!("ws read: {e}"))?;
            match frame {
                Message::Text(t) => match serde_json::from_str::<AgentToServer>(&t) {
                    Ok(msg) => {
                        if let Some(task_id) = msg.task_id() {
                            let task_id = task_id.to_string();
                            let mut p = pending_r.lock().await;
                            let drop_after = matches!(
                                &msg,
                                AgentToServer::TaskExit { .. } | AgentToServer::TaskError { .. }
                            );
                            if let Some(tx) = p.get(&task_id) {
                                let _ = tx.send(msg);
                            }
                            if drop_after {
                                p.remove(&task_id);
                            }
                        } else if let Some(req_id) = msg.req_id() {
                            // HTTP relay / CLI streaming — CliChunk 保留，其余删除
                            let req_id = req_id.to_string();
                            let is_final = msg.is_final_req_msg();
                            let mut p = pending_r.lock().await;
                            if is_final {
                                if let Some(tx) = p.remove(&req_id) {
                                    let _ = tx.send(msg);
                                }
                            } else if let Some(tx) = p.get(&req_id) {
                                let _ = tx.send(msg);
                            }
                        } else {
                            // Register/Pong without task_id — ignore (Register already consumed).
                        }
                    }
                    Err(e) => tracing::warn!("bad agent msg: {e}: {t}"),
                },
                Message::Close(_) => break,
                _ => {}
            }
        }
        Ok(())
    }
    .await;

    // Clean up.
    state.agent_manager.agents.write().await.remove(&agent_id);
    drop(cmd_tx);
    let _ = writer.await;
    tracing::info!(%agent_id, "agent disconnected");
    read_result
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

// ── /api/_test_dispatch handler (Phase 1 smoke test) ─────────────────────────

#[derive(Debug, Deserialize)]
pub struct TestDispatchReq {
    pub agent_id: String,
    pub cli: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: String,
}

#[derive(Debug, Serialize)]
pub struct TestDispatchResp {
    pub task_id: String,
    pub pid: Option<u32>,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TestCliPromptReq {
    pub agent_id: String,
    pub cli: String,
    #[serde(default)]
    pub extra_args: Vec<String>,
    pub prompt: String,
}

#[derive(Debug, Serialize)]
pub struct TestCliPromptResp {
    pub req_id: String,
    pub exit_ok: Option<bool>,
    pub text: String,
    pub error: Option<String>,
}

/// Synchronously dispatch a command to the named agent and collect everything
/// until exit/error. Decodes base64 stdout into a single concatenated string.
pub async fn test_dispatch(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<TestDispatchReq>,
) -> impl IntoResponse {
    // Require admin token (already used by /api/admin/* endpoints).
    let presented = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .unwrap_or("");
    if presented.is_empty() || presented != state.admin_token {
        return (StatusCode::UNAUTHORIZED, "admin token required").into_response();
    }

    let (task_id, mut rx) = match state
        .agent_manager
        .dispatch(&req.agent_id, req.cli, req.args, req.cwd, vec![])
        .await
    {
        Ok(v) => v,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, e.to_string()).into_response();
        }
    };

    let mut pid = None;
    let mut exit_code = None;
    let mut stdout = Vec::<u8>::new();
    let mut error = None;
    while let Some(msg) = rx.recv().await {
        match msg {
            AgentToServer::TaskStarted { pid: p, .. } => pid = Some(p),
            AgentToServer::TaskStdout { data, .. } => {
                if let Ok(bytes) = B64.decode(&data) {
                    stdout.extend_from_slice(&bytes);
                }
            }
            AgentToServer::TaskExit { code, .. } => {
                exit_code = code;
                break;
            }
            AgentToServer::TaskError { message, .. } => {
                error = Some(message);
                break;
            }
            _ => {}
        }
    }

    let resp = TestDispatchResp {
        task_id,
        pid,
        exit_code,
        stdout: String::from_utf8_lossy(&stdout).to_string(),
        error,
    };
    Json(resp).into_response()
}

/// Smoke test for CliPrompt flow (cloud -> PC relay -> local CLI -> stream back).
/// Requires ADMIN_TOKEN in Authorization header.
pub async fn test_cli_prompt(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<TestCliPromptReq>,
) -> impl IntoResponse {
    let presented = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .unwrap_or("");
    if presented.is_empty() || presented != state.admin_token {
        return (StatusCode::UNAUTHORIZED, "admin token required").into_response();
    }

    let (req_id, mut rx) = match state
        .agent_manager
        .dispatch_cli_prompt(&req.agent_id, req.cli, req.extra_args, req.prompt)
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };

    let mut text = String::new();
    let mut exit_ok = None;
    let mut error = None;

    loop {
        match tokio::time::timeout(Duration::from_secs(120), rx.recv()).await {
            Ok(Some(AgentToServer::CliChunk { text: chunk, .. })) => {
                text.push_str(&chunk);
            }
            Ok(Some(AgentToServer::CliDone {
                exit_ok: ok,
                error: e,
                ..
            })) => {
                exit_ok = Some(ok);
                error = e;
                break;
            }
            Ok(Some(_)) => {
                // Ignore unrelated message variants.
            }
            Ok(None) => {
                error = Some("cli prompt channel closed".to_string());
                break;
            }
            Err(_) => {
                error = Some("cli prompt timeout (120s)".to_string());
                break;
            }
        }
    }

    Json(TestCliPromptResp {
        req_id,
        exit_ok,
        text,
        error,
    })
    .into_response()
}

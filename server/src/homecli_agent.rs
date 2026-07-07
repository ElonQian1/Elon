//! Server side of the reverse-WSS tunnel that lets the cloud elon-cli relay
//! dispatch shell commands to a homecli agent running on a developer's PC.
//!
//! Endpoint: `GET /agent/ws`
//! Auth: `Authorization: Bearer <agent_secret>`; the `<agent_id>:<agent_secret>`
//! pair is configured via the `ELON_AGENT_SECRETS` env var
//! (format: `id1:secret1,id2:secret2`).
//!
//! Protocol: see [`homecli_proto`].

use crate::types::AppState;
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
use homecli_proto::{
    AgentToServer, NodeDevRuntimeProfile, NodeHardwareProfile, NodeStorageProfile, ServerToAgent,
};
use serde::{Deserialize, Serialize};
use sha2::Digest as _;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::{mpsc, oneshot, watch, Mutex, RwLock};
use uuid::Uuid;
// ── manager state ────────────────────────────────────────────────────────────
const TOOL_APPROVAL_ACK_TIMEOUT: Duration = Duration::from_secs(10);
const AGENT_WS_READ_TIMEOUT: Duration = Duration::from_secs(40);
const AGENT_DISPATCH_PROBE_TIMEOUT: Duration = Duration::from_secs(3);
const PROJECT_WORKSPACE_PROVISION_TIMEOUT_ENV: &str =
    "ELON_PROJECT_WORKSPACE_PROVISION_TIMEOUT_SECS";
const PROJECT_WORKSPACE_INSPECT_TIMEOUT_ENV: &str = "ELON_PROJECT_WORKSPACE_INSPECT_TIMEOUT_SECS";
const PROJECT_STORAGE_PREPARE_TIMEOUT_ENV: &str = "ELON_PROJECT_STORAGE_PREPARE_TIMEOUT_SECS";
mod heartbeat; mod journal;
mod public_dev_handshake;
mod summary;
use public_dev_handshake::record_node_public_dev_handshake;
pub use summary::AgentSummary;
#[cfg(test)]
#[path = "homecli_agent_tests.rs"]
mod homecli_agent_tests;
/// Snapshot of one connected PC agent.
#[derive(Clone)]
pub struct AgentEntry {
    session_id: String,
    pub agent_id: String,
    pub version: String,
    pub device_name: Option<String>,
    pub hardware: Option<NodeHardwareProfile>,
    pub storage: Option<NodeStorageProfile>,
    pub dev_runtime: Option<NodeDevRuntimeProfile>,
    pub lifecycle: Option<homecli_proto::NodeLifecycleReport>,
    pub allowed_clis: Vec<String>,
    pub allowed_cwds: Vec<String>,
    pub connected_at: u64,
    /// Outbound queue: server pushes ServerToAgent here, WS writer drains.
    pub(crate) cmd_tx: mpsc::UnboundedSender<ServerToAgent>,
    /// For each in-flight task: where to forward AgentToServer events.
    pub(crate) pending: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<AgentToServer>>>>,
    /// One-shot ACK waiters keyed by req_id + approval_id + dispatch_id.
    approval_acks: Arc<Mutex<HashMap<String, oneshot::Sender<bool>>>>,
    /// One-shot protocol ping waiters keyed by nonce.
    ping_acks: Arc<Mutex<HashMap<String, oneshot::Sender<()>>>>,
    /// Signals the session reader/writer to close when a newer connection replaces it.
    session_shutdown: watch::Sender<bool>,
}
pub struct CliPromptDispatch {
    pub req_id: String,
    pub rx: mpsc::UnboundedReceiver<AgentToServer>,
    cancel_handle: CliPromptCancelHandle,
}
impl CliPromptDispatch {
    pub fn into_parts(
        self,
    ) -> (
        String,
        mpsc::UnboundedReceiver<AgentToServer>,
        CliPromptCancelHandle,
    ) {
        (self.req_id, self.rx, self.cancel_handle)
    }
}
#[derive(Clone)]
pub struct CliPromptCancelHandle {
    req_id: String,
    cmd_tx: mpsc::UnboundedSender<ServerToAgent>,
}
impl CliPromptCancelHandle {
    pub fn req_id(&self) -> &str {
        &self.req_id
    }
    pub fn cancel(&self) -> bool {
        self.cmd_tx
            .send(ServerToAgent::Cancel {
                task_id: self.req_id.clone(),
            })
            .is_ok()
    }
}
#[derive(Default)]
pub struct AgentManager {
    pub(crate) agents: RwLock<HashMap<String, AgentEntry>>,
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
        self.dispatch_cli_prompt_in_cwd(agent_id, cli, extra_args, None, prompt)
            .await
    }
    /// 把 AI 提示发给 PC agent，并可指定 PC 侧工作目录。
    pub async fn dispatch_cli_prompt_in_cwd(
        &self,
        agent_id: &str,
        cli: String,
        extra_args: Vec<String>,
        cwd: Option<String>,
        prompt: String,
    ) -> Result<(String, mpsc::UnboundedReceiver<AgentToServer>)> {
        self.dispatch_cli_prompt_with_context(agent_id, cli, extra_args, cwd, None, prompt)
            .await
    }
    /// 把项目 AI 提示发给 PC agent，并带上会话上下文用于 PC 本地 worktree 隔离。
    pub async fn dispatch_cli_prompt_with_context(
        &self,
        agent_id: &str,
        cli: String,
        extra_args: Vec<String>,
        cwd: Option<String>,
        project_context: Option<homecli_proto::CliProjectContext>,
        prompt: String,
    ) -> Result<(String, mpsc::UnboundedReceiver<AgentToServer>)> {
        let dispatch = self
            .dispatch_cli_prompt_with_context_control(
                agent_id,
                cli,
                extra_args,
                cwd,
                project_context,
                prompt,
            )
            .await?;
        Ok((dispatch.req_id, dispatch.rx))
    }
    pub async fn dispatch_cli_prompt_with_context_control(
        &self,
        agent_id: &str,
        cli: String,
        extra_args: Vec<String>,
        cwd: Option<String>,
        project_context: Option<homecli_proto::CliProjectContext>,
        prompt: String,
    ) -> Result<CliPromptDispatch> {
        let req_id = Uuid::new_v4().to_string();
        let (cmd_tx, pending, ping_acks) = {
            let agents = self.agents.read().await;
            let agent = agents
                .get(agent_id)
                .ok_or_else(|| anyhow!("agent not connected: {agent_id}"))?;
            (
                agent.cmd_tx.clone(),
                agent.pending.clone(),
                agent.ping_acks.clone(),
            )
        };
        if let Err(error) =
            send_protocol_ping(agent_id, &cmd_tx, &ping_acks, AGENT_DISPATCH_PROBE_TIMEOUT).await
        {
            let _ = self
                .close_agent_session(agent_id, "dispatch probe failed")
                .await;
            return Err(anyhow!("agent not connected: {agent_id} ({error})"));
        }
        let (tx, rx) = mpsc::unbounded_channel();
        pending.lock().await.insert(req_id.clone(), tx);
        let cancel_handle = CliPromptCancelHandle {
            req_id: req_id.clone(),
            cmd_tx: cmd_tx.clone(),
        };
        if let Err(error) = cmd_tx
            .send(ServerToAgent::CliPrompt {
                req_id: req_id.clone(),
                cli,
                extra_args,
                cwd,
                project_context,
                prompt,
            })
            .map_err(|_| anyhow!("agent writer closed"))
        {
            pending.lock().await.remove(&req_id);
            return Err(error);
        }
        Ok(CliPromptDispatch {
            req_id,
            rx,
            cancel_handle,
        })
    }
    pub async fn send_tool_approval_decision(
        &self,
        req_id: &str,
        approval_id: &str,
        decision: &str,
    ) -> Result<bool> {
        let target = {
            let agents = self.agents.read().await;
            let mut target = None;
            for agent in agents.values() {
                let has_pending_req = agent.pending.lock().await.contains_key(req_id);
                if !has_pending_req {
                    continue;
                }
                target = Some((agent.cmd_tx.clone(), agent.approval_acks.clone()));
                break;
            }
            target
        };
        let Some((cmd_tx, approval_acks)) = target else {
            return Err(anyhow!(
                "pending CLI request not found for tool approval: {req_id}"
            ));
        };
        let dispatch_id = Uuid::new_v4().to_string();
        let ack_key = tool_approval_ack_key(req_id, approval_id, &dispatch_id);
        let (ack_tx, ack_rx) = oneshot::channel();
        approval_acks.lock().await.insert(ack_key.clone(), ack_tx);
        if cmd_tx
            .send(ServerToAgent::ToolApprovalDecision {
                req_id: req_id.to_string(),
                approval_id: approval_id.to_string(),
                dispatch_id,
                decision: decision.to_string(),
            })
            .is_err()
        {
            approval_acks.lock().await.remove(&ack_key);
            return Err(anyhow!("agent writer closed"));
        }
        match tokio::time::timeout(TOOL_APPROVAL_ACK_TIMEOUT, ack_rx).await {
            Ok(Ok(accepted)) => Ok(accepted),
            Ok(Err(_)) => Err(anyhow!(
                "tool approval ack channel closed: req_id={req_id}, approval_id={approval_id}"
            )),
            Err(_) => {
                approval_acks.lock().await.remove(&ack_key);
                Err(anyhow!(
                    "tool approval ack timeout: req_id={req_id}, approval_id={approval_id}"
                ))
            }
        }
    }
    pub async fn list(&self) -> Vec<AgentSummary> {
        self.agents
            .read()
            .await
            .values()
            .map(|a| AgentSummary {
                agent_id: a.agent_id.clone(),
                version: a.version.clone(),
                device_name: a.device_name.clone(),
                hardware: a.hardware.clone(),
                storage: a.storage.clone(),
                dev_runtime: a.dev_runtime.clone(),
                lifecycle: a.lifecycle.clone(),
                allowed_clis: a.allowed_clis.clone(),
                allowed_cwds: a.allowed_cwds.clone(),
                connected_at: a.connected_at,
            })
            .collect()
    }
    /// Close the currently registered session for an agent, forcing the PC client to reconnect.
    pub async fn close_agent_session(&self, agent_id: &str, reason: &str) -> bool {
        let shutdown = {
            let agents = self.agents.read().await;
            agents.get(agent_id).map(|entry| {
                (
                    entry.session_id.clone(),
                    entry.session_shutdown.clone(),
                    entry.pending.clone(),
                    entry.approval_acks.clone(),
                    entry.ping_acks.clone(),
                )
            })
        };
        let Some((session_id, shutdown, pending, approval_acks, ping_acks)) = shutdown else {
            return false;
        };
        tracing::warn!(%agent_id, %session_id, %reason, "closing PC agent session");
        fail_pending_requests(&pending, reason).await;
        fail_pending_approvals(&approval_acks).await;
        fail_pending_pings(&ping_acks).await;
        let _ = shutdown.send(true);
        true
    }
    /// 广播 UpdateClient 消息给所有在线节点，触发无感自动更新。
    /// 返回成功发送的节点数量。
    pub async fn broadcast_update_client(
        &self,
        version: Option<String>,
        download_url: Option<String>,
    ) -> usize {
        let agents = self.agents.read().await;
        let mut count = 0;
        for agent in agents.values() {
            if agent
                .cmd_tx
                .send(homecli_proto::ServerToAgent::UpdateClient {
                    version: version.clone(),
                    download_url: download_url.clone(),
                })
                .is_ok()
            {
                count += 1;
            }
        }
        count
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
    /// 向指定节点 agent 发起 LLM 流式推理请求。
    /// 返回 (req_id, receiver)，receiver 收到 LlmStreamChunk / LlmStreamEnd / LlmStreamError。
    pub async fn dispatch_llm_stream(
        &self,
        agent_id: &str,
        model: String,
        messages: Vec<serde_json::Value>,
        max_tokens: Option<u32>,
    ) -> Result<(String, mpsc::UnboundedReceiver<AgentToServer>)> {
        let req_id = Uuid::new_v4().to_string();
        self.dispatch_llm_stream_with_req_id(agent_id, req_id, model, messages, max_tokens)
            .await
    }
    pub async fn dispatch_llm_stream_with_req_id(
        &self,
        agent_id: &str,
        req_id: String,
        model: String,
        messages: Vec<serde_json::Value>,
        max_tokens: Option<u32>,
    ) -> Result<(String, mpsc::UnboundedReceiver<AgentToServer>)> {
        let agents = self.agents.read().await;
        let agent = agents
            .get(agent_id)
            .ok_or_else(|| anyhow!("agent not connected: {agent_id}"))?;
        let (tx, rx) = mpsc::unbounded_channel();
        agent.pending.lock().await.insert(req_id.clone(), tx);
        agent
            .cmd_tx
            .send(homecli_proto::ServerToAgent::LlmStreamRequest {
                req_id: req_id.clone(),
                model,
                messages,
                max_tokens,
            })
            .map_err(|_| anyhow!("agent writer closed"))?;
        Ok((req_id, rx))
    }
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
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

mod agent_session;
use agent_session::{
    fail_pending_approvals, fail_pending_pings, fail_pending_requests,
    project_storage_prepare_timeout, project_workspace_inspect_timeout,
    project_workspace_provision_timeout, run_agent_session, send_protocol_ping,
    tool_approval_ack_key,
};

mod workspace_dispatch;
mod test_dispatch;
pub use test_dispatch::{TestDispatchReq, TestDispatchResp, TestCliPromptReq, TestCliPromptResp, test_dispatch, test_cli_prompt};

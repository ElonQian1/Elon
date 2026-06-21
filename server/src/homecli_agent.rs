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
use tokio::sync::{mpsc, Mutex, RwLock};
use uuid::Uuid;

use crate::types::AppState;

// ── manager state ────────────────────────────────────────────────────────────

/// Snapshot of one connected PC agent.
#[derive(Clone)]
pub struct AgentEntry {
    pub agent_id: String,
    pub version: String,
    pub device_name: Option<String>,
    pub hardware: Option<NodeHardwareProfile>,
    pub storage: Option<NodeStorageProfile>,
    pub dev_runtime: Option<NodeDevRuntimeProfile>,
    pub allowed_clis: Vec<String>,
    pub allowed_cwds: Vec<String>,
    pub connected_at: u64,
    /// Outbound queue: server pushes ServerToAgent here, WS writer drains.
    cmd_tx: mpsc::UnboundedSender<ServerToAgent>,
    /// For each in-flight task: where to forward AgentToServer events.
    pending: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<AgentToServer>>>>,
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
        let agents = self.agents.read().await;
        let agent = agents
            .get(agent_id)
            .ok_or_else(|| anyhow!("agent not connected: {agent_id}"))?;
        let (tx, rx) = mpsc::unbounded_channel();
        agent.pending.lock().await.insert(req_id.clone(), tx);
        let cancel_handle = CliPromptCancelHandle {
            req_id: req_id.clone(),
            cmd_tx: agent.cmd_tx.clone(),
        };
        agent
            .cmd_tx
            .send(ServerToAgent::CliPrompt {
                req_id: req_id.clone(),
                cli,
                extra_args,
                cwd,
                project_context,
                prompt,
            })
            .map_err(|_| anyhow!("agent writer closed"))?;
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
    ) -> Result<()> {
        let agents = self.agents.read().await;
        for agent in agents.values() {
            let has_pending_req = agent.pending.lock().await.contains_key(req_id);
            if !has_pending_req {
                continue;
            }
            agent
                .cmd_tx
                .send(ServerToAgent::ToolApprovalDecision {
                    req_id: req_id.to_string(),
                    approval_id: approval_id.to_string(),
                    decision: decision.to_string(),
                })
                .map_err(|_| anyhow!("agent writer closed"))?;
            return Ok(());
        }
        Err(anyhow!(
            "pending CLI request not found for tool approval: {req_id}"
        ))
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

    /// Ask a PC node to create or reuse a managed project workspace.
    pub async fn dispatch_project_workspace_provision(
        &self,
        agent_id: &str,
        project_id: String,
        user_id: String,
        name: String,
        template: String,
        repo_url: Option<String>,
        branch: Option<String>,
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
            .send(homecli_proto::ServerToAgent::ProvisionProjectWorkspace {
                req_id: req_id.clone(),
                project_id,
                user_id,
                name,
                template,
                repo_url,
                branch,
            })
            .map_err(|_| anyhow!("agent writer closed"))?;
        drop(agents);

        match tokio::time::timeout(Duration::from_secs(120), rx.recv()).await {
            Ok(Some(msg)) => Ok(msg),
            Ok(None) => Err(anyhow!("agent disconnected before provisioning response")),
            Err(_) => Err(anyhow!("project workspace provisioning timeout (120s)")),
        }
    }

    /// Ask a storage-capable PC node to create or reuse a bare Git repo for a project.
    pub async fn dispatch_project_storage_repo_prepare(
        &self,
        agent_id: &str,
        project_id: String,
        user_id: String,
        name: String,
        branch: Option<String>,
        access_token: Option<String>,
        prepare_worktree: bool,
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
            .send(homecli_proto::ServerToAgent::PrepareProjectStorageRepo {
                req_id: req_id.clone(),
                project_id,
                user_id,
                name,
                branch,
                access_token,
                prepare_worktree,
            })
            .map_err(|_| anyhow!("agent writer closed"))?;
        drop(agents);

        match tokio::time::timeout(Duration::from_secs(60), rx.recv()).await {
            Ok(Some(msg)) => Ok(msg),
            Ok(None) => Err(anyhow!("agent disconnected before storage repo response")),
            Err(_) => Err(anyhow!("project storage repo prepare timeout (60s)")),
        }
    }

    /// Ask a PC node to inspect a project workspace and return a single status frame.
    pub async fn dispatch_project_workspace_inspect(
        &self,
        agent_id: &str,
        workspace_path: String,
    ) -> Result<AgentToServer> {
        let req_id = Uuid::new_v4().to_string();
        let agents = self.agents.read().await;
        let agent = agents
            .get(agent_id)
            .ok_or_else(|| anyhow!("agent not connected: {agent_id}"))?;
        let (tx, mut rx) = mpsc::unbounded_channel();
        let pending = agent.pending.clone();
        pending.lock().await.insert(req_id.clone(), tx);
        agent
            .cmd_tx
            .send(homecli_proto::ServerToAgent::InspectProjectWorkspace {
                req_id: req_id.clone(),
                workspace_path,
            })
            .map_err(|_| anyhow!("agent writer closed"))?;
        drop(agents);

        let outcome = match tokio::time::timeout(Duration::from_secs(6), rx.recv()).await {
            Ok(Some(msg)) => Ok(msg),
            Ok(None) => Err(anyhow!(
                "agent disconnected before workspace inspect response"
            )),
            Err(_) => Err(anyhow!("project workspace inspect timeout (6s)")),
        };
        if outcome.is_err() {
            pending.lock().await.remove(&req_id);
        }
        outcome
    }

    /// Ask a PC node to read fixed project documentation from a workspace.
    pub async fn dispatch_project_documents_read(
        &self,
        agent_id: &str,
        workspace_path: String,
        seed_defaults: bool,
    ) -> Result<AgentToServer> {
        let req_id = Uuid::new_v4().to_string();
        let agents = self.agents.read().await;
        let agent = agents
            .get(agent_id)
            .ok_or_else(|| anyhow!("agent not connected: {agent_id}"))?;
        let (tx, mut rx) = mpsc::unbounded_channel();
        let pending = agent.pending.clone();
        pending.lock().await.insert(req_id.clone(), tx);
        agent
            .cmd_tx
            .send(homecli_proto::ServerToAgent::ReadProjectDocuments {
                req_id: req_id.clone(),
                workspace_path,
                seed_defaults,
            })
            .map_err(|_| anyhow!("agent writer closed"))?;
        drop(agents);

        let outcome = match tokio::time::timeout(Duration::from_secs(8), rx.recv()).await {
            Ok(Some(msg)) => Ok(msg),
            Ok(None) => Err(anyhow!("agent disconnected before project docs response")),
            Err(_) => Err(anyhow!("project docs read timeout (8s)")),
        };
        if outcome.is_err() {
            pending.lock().await.remove(&req_id);
        }
        outcome
    }

    /// Ask a PC node to cleanup a managed project workspace and return a single status frame.
    pub async fn dispatch_project_workspace_cleanup(
        &self,
        agent_id: &str,
        project_id: String,
        workspace_path: String,
    ) -> Result<AgentToServer> {
        let req_id = Uuid::new_v4().to_string();
        let agents = self.agents.read().await;
        let agent = agents
            .get(agent_id)
            .ok_or_else(|| anyhow!("agent not connected: {agent_id}"))?;
        let (tx, mut rx) = mpsc::unbounded_channel();
        let pending = agent.pending.clone();
        pending.lock().await.insert(req_id.clone(), tx);
        agent
            .cmd_tx
            .send(homecli_proto::ServerToAgent::CleanupProjectWorkspace {
                req_id: req_id.clone(),
                project_id,
                workspace_path,
            })
            .map_err(|_| anyhow!("agent writer closed"))?;
        drop(agents);

        let outcome = match tokio::time::timeout(Duration::from_secs(45), rx.recv()).await {
            Ok(Some(msg)) => Ok(msg),
            Ok(None) => Err(anyhow!(
                "agent disconnected before workspace cleanup response"
            )),
            Err(_) => Err(anyhow!("project workspace cleanup timeout (45s)")),
        };
        if outcome.is_err() {
            pending.lock().await.remove(&req_id);
        }
        outcome
    }

    /// 向 PC 节点发起 TTS 合成请求，返回 TtsSynthesizeResponse 或 TtsSynthesizeError。
    /// timeout 设 180s（模型首次加载可能需要较长时间）。
    pub async fn dispatch_tts(
        &self,
        agent_id: &str,
        text: String,
        voice_id: Option<String>,
        emotion_id: Option<String>,
        intensity: Option<String>,
        provider: Option<String>,
    ) -> Result<AgentToServer> {
        let req_id = Uuid::new_v4().to_string();
        let agents = self.agents.read().await;
        let agent = agents
            .get(agent_id)
            .ok_or_else(|| anyhow!("TTS agent not connected: {agent_id}"))?;
        let (tx, mut rx) = mpsc::unbounded_channel();
        agent.pending.lock().await.insert(req_id.clone(), tx);
        agent
            .cmd_tx
            .send(homecli_proto::ServerToAgent::TtsSynthesizeRequest {
                req_id: req_id.clone(),
                text,
                voice_id,
                emotion_id,
                intensity,
                provider,
            })
            .map_err(|_| anyhow!("agent writer closed"))?;
        drop(agents);
        match tokio::time::timeout(Duration::from_secs(180), rx.recv()).await {
            Ok(Some(msg)) => Ok(msg),
            Ok(None) => Err(anyhow!("TTS agent disconnected before response")),
            Err(_) => Err(anyhow!("TTS synthesis timeout (180s)")),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AgentSummary {
    pub agent_id: String,
    pub version: String,
    pub device_name: Option<String>,
    pub hardware: Option<NodeHardwareProfile>,
    pub storage: Option<NodeStorageProfile>,
    pub dev_runtime: Option<NodeDevRuntimeProfile>,
    pub allowed_clis: Vec<String>,
    pub allowed_cwds: Vec<String>,
    pub connected_at: u64,
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
    let (
        agent_id,
        version,
        allowed_clis,
        allowed_cwds,
        _proto_ver,
        owner_user_id,
        device_name,
        hardware,
        storage,
        dev_runtime,
    ) = match register {
        AgentToServer::Register {
            agent_id,
            version,
            proto_version,
            allowed_clis,
            allowed_cwds,
            owner_user_id,
            device_name,
            hardware,
            storage,
            dev_runtime,
        } => (
            agent_id,
            version,
            allowed_clis,
            allowed_cwds,
            proto_version,
            owner_user_id,
            clean_optional(device_name),
            hardware,
            storage,
            dev_runtime,
        ),
        _ => return Err(anyhow!("first frame must be register")),
    };

    // Auth check: presented token must equal the secret bound to agent_id.
    // Priority: env var secrets (static, for legacy/admin agents) → DB credentials (dynamic, user-registered nodes).
    let auth_ok = if let Some(expected) = secrets.get(&agent_id) {
        constant_time_eq(expected.as_bytes(), presented_token.as_bytes())
    } else {
        // Check DB-stored node credentials (secret stored as SHA-256 hex)
        let presented_hash = hex::encode(sha2::Sha256::digest(presented_token.as_bytes()));
        matches!(
            state.store.get_node_credential_hash(&agent_id),
            Ok(Some(ref stored)) if stored == &presented_hash
        )
    };
    if !auth_ok {
        return Err(anyhow!("auth failed for agent_id={agent_id}"));
    }

    // If agent registered via DB credentials, resolve owner from DB
    let resolved_owner_user_id = if owner_user_id.is_some() {
        owner_user_id.clone()
    } else if !secrets.contains_key(&agent_id) {
        state
            .store
            .get_node_credential_owner(&agent_id)
            .ok()
            .flatten()
    } else {
        None
    };

    if let (Some(owner), Some(device)) = (&resolved_owner_user_id, &device_name) {
        if let Err(e) = state
            .store
            .update_node_credential_device_name(&agent_id, owner, device)
        {
            tracing::warn!(%agent_id, error = %e, "failed to update node device name");
        }
    }
    if let (Some(owner), Some(hardware)) = (&resolved_owner_user_id, hardware.as_ref()) {
        if let Err(e) = state.store.upsert_node_hardware_snapshot(
            &agent_id,
            owner,
            device_name.as_deref(),
            hardware,
        ) {
            tracing::warn!(%agent_id, error = %e, "failed to update node hardware snapshot");
        }
    }

    tracing::info!(%agent_id, %version, device_name = ?device_name, "agent registered");

    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<ServerToAgent>();
    let pending: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<AgentToServer>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let entry = AgentEntry {
        agent_id: agent_id.clone(),
        version,
        device_name: device_name.clone(),
        hardware: hardware.clone(),
        storage: storage.clone(),
        dev_runtime: dev_runtime.clone(),
        allowed_clis,
        allowed_cwds,
        connected_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        cmd_tx: cmd_tx.clone(),
        pending: pending.clone(),
    };

    // 若同一 agent_id 已有旧连接，通过旧 cmd_tx 发 Close 消息主动终止它，
    // 避免两个实例并存导致 WebSocket 资源竞争和 "Connection reset by peer"。
    {
        let mut agents = state.agent_manager.agents.write().await;
        if let Some(old_entry) = agents.get(&agent_id) {
            tracing::info!(%agent_id, "evicting previous agent session (same agent_id re-registered)");
            // 通知旧连接的所有挂起请求立即失败
            let mut old_pending = old_entry.pending.lock().await;
            let stale: Vec<_> = old_pending.drain().collect();
            drop(old_pending);
            for (req_id, sender) in stale {
                let _ = sender.send(AgentToServer::CliDone {
                    req_id,
                    exit_ok: false,
                    error: Some("节点重新注册，旧连接已关闭".to_string()),
                    prompt_tokens: None,
                    cached_input_tokens: None,
                    completion_tokens: None,
                    reasoning_tokens: None,
                    total_tokens: None,
                    model: None,
                    workspace_status: None,
                });
            }
            // 发 Close 让旧连接的 writer 关闭 WebSocket
            let _ = old_entry.cmd_tx.send(ServerToAgent::Ping { nonce: None }); // flush any pending
            drop(old_entry.cmd_tx.clone()); // 通过 drop sender 让 writer 退出
        }
        agents.insert(agent_id.clone(), entry);
    }

    // 注册到节点注册表（分布式节点功能）
    let connected_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let session_device_name = device_name.clone();
    let session_owner_user_id = resolved_owner_user_id.clone().unwrap_or_default();
    state
        .node_registry
        .register(
            agent_id.clone(),
            session_owner_user_id.clone(),
            device_name,
            hardware,
            storage,
            dev_runtime,
            vec![],
            connected_at,
        )
        .await;

    // 服务端 → 节点 ping 定时器：每 30s 发一次 ServerToAgent::Ping，
    // 节点收到后回 Pong，服务端 touch() 刷新 TTL（防止 90s 后被标为离线）。
    {
        let ping_tx = cmd_tx.clone();
        let registry = state.node_registry.clone();
        let aid = agent_id.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            let mut nonce: u64 = 0;
            loop {
                interval.tick().await;
                nonce += 1;
                if ping_tx
                    .send(ServerToAgent::Ping {
                        nonce: Some(nonce.to_string()),
                    })
                    .is_err()
                {
                    break; // 连接已断开
                }
                // 发出 ping 时也顺手 touch，防止节点 pong 漏到
                registry.touch(&aid).await;
            }
        });
    }

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
                            // Register/Pong without task_id — 处理节点专属消息
                            match &msg {
                                AgentToServer::RegisterCapabilities {
                                    models,
                                    allowed_clis,
                                    tts_worker_url,
                                    hardware,
                                    storage,
                                    dev_runtime,
                                } => {
                                    if let Some(hardware) = hardware.as_ref() {
                                        if !session_owner_user_id.is_empty() {
                                            if let Err(e) =
                                                state.store.upsert_node_hardware_snapshot(
                                                    &agent_id,
                                                    &session_owner_user_id,
                                                    session_device_name.as_deref(),
                                                    hardware,
                                                )
                                            {
                                                tracing::warn!(
                                                    %agent_id,
                                                    error = %e,
                                                    "failed to update node hardware snapshot"
                                                );
                                            }
                                        }
                                    }
                                    {
                                        let mut agents = state.agent_manager.agents.write().await;
                                        if let Some(entry) = agents.get_mut(&agent_id) {
                                            if !allowed_clis.is_empty() {
                                                entry.allowed_clis = allowed_clis.clone();
                                            }
                                            if hardware.is_some() {
                                                entry.hardware = hardware.clone();
                                            }
                                            if storage.is_some() {
                                                entry.storage = storage.clone();
                                            }
                                            if dev_runtime.is_some() {
                                                entry.dev_runtime = dev_runtime.clone();
                                            }
                                        }
                                    }
                                    state
                                        .node_registry
                                        .update_capabilities(
                                            &agent_id,
                                            models.clone(),
                                            tts_worker_url.clone(),
                                            hardware.clone(),
                                            storage.clone(),
                                            dev_runtime.clone(),
                                        )
                                        .await;
                                }
                                AgentToServer::Pong { .. } => {
                                    state.node_registry.touch(&agent_id).await;
                                }
                                _ => {}
                            }
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

    // Clean up: 先移除 agent，再通知挂起请求失败，避免断线后调用方永久阻塞
    state.agent_manager.agents.write().await.remove(&agent_id);
    state.node_registry.unregister(&agent_id).await;

    // 节点断线时，向所有还在等待响应的 CLI 请求发送 CliDone(exit_ok=false)，
    // 让 run_via_pc_agent 的 while rx.recv() 立即收到错误并返回，
    // 而不是永远阻塞到 HTTP 请求超时。
    {
        let mut p = pending.lock().await;
        let stale: Vec<(String, mpsc::UnboundedSender<AgentToServer>)> = p.drain().collect();
        drop(p);
        for (req_id, sender) in stale {
            let _ = sender.send(AgentToServer::CliDone {
                req_id,
                exit_ok: false,
                error: Some("PC节点已断线，请重试".to_string()),
                prompt_tokens: None,
                cached_input_tokens: None,
                completion_tokens: None,
                reasoning_tokens: None,
                total_tokens: None,
                model: None,
                workspace_status: None,
            });
        }
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_prompt_cancel_handle_sends_cancel_for_req_id() {
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
        let handle = CliPromptCancelHandle {
            req_id: "req-123".to_string(),
            cmd_tx,
        };

        assert!(handle.cancel());
        match cmd_rx.try_recv() {
            Ok(ServerToAgent::Cancel { task_id }) => assert_eq!(task_id, "req-123"),
            other => panic!("expected cancel message, got {other:?}"),
        }
    }
}

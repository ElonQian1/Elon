//! Server side of the reverse-WSS tunnel that lets the cloud elon-cli relay
//! dispatch shell commands to a homecli agent running on a developer's PC.
//!
//! Endpoint: `GET /agent/ws`
//! Auth: `Authorization: Bearer <agent_secret>`; the `<agent_id>:<agent_secret>`
//! pair is configured via the `ELON_AGENT_SECRETS` env var
//! (format: `id1:secret1,id2:secret2`).
//!
//! Protocol: see [`homecli_proto`].

use crate::{node_registry::AgentProcessSessionKey, types::AppState};
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
    collections::{HashMap, HashSet},
    sync::{atomic::AtomicBool, Arc},
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
const DURABLE_CLI_COMPLETION_PROTO_VERSION: u32 = 5;
const CLOUD_CONTROL_DEADLINE_PROTO_VERSION: u32 = 7;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CloudControlDispatchWindow {
    pub(crate) issued_at: String,
    pub(crate) ttl_ms: u64,
}

pub(crate) fn freeze_cloud_control_dispatch_window(
    deadline: &str,
) -> Result<CloudControlDispatchWindow> {
    freeze_cloud_control_dispatch_window_at(deadline, chrono::Utc::now())
}

fn freeze_cloud_control_dispatch_window_at(
    deadline: &str,
    issued_at: chrono::DateTime<chrono::Utc>,
) -> Result<CloudControlDispatchWindow> {
    let deadline = chrono::DateTime::parse_from_rfc3339(deadline)
        .map_err(|_| anyhow!("cloud authorization deadline is not valid RFC3339"))?
        .with_timezone(&chrono::Utc);
    let ttl_ms = deadline.signed_duration_since(issued_at).num_milliseconds();
    if ttl_ms <= 0 {
        return Err(anyhow!("cloud authorization deadline has expired"));
    }
    Ok(CloudControlDispatchWindow {
        issued_at: issued_at.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true),
        ttl_ms: ttl_ms as u64,
    })
}
mod android_device_host;
mod compute_plugin_sharing;
mod heartbeat;
mod journal;
mod pending_recovery;
mod public_dev_handshake;
mod session_fencing;
mod summary;
pub(crate) use compute_plugin_sharing::dispatch_durable_compute_plugin_sharing_intent;
use public_dev_handshake::record_node_public_dev_handshake;
pub use summary::AgentSummary;
#[cfg(test)]
#[path = "homecli_agent_build_cache_tests.rs"]
mod homecli_agent_build_cache_tests;
#[cfg(test)]
#[path = "homecli_agent_duplicate_dispatch_tests.rs"]
mod homecli_agent_duplicate_dispatch_tests;
#[cfg(test)]
#[path = "homecli_agent_tests.rs"]
mod homecli_agent_tests;
/// Snapshot of one connected PC agent.
#[derive(Clone)]
pub struct AgentEntry {
    /// Exact process-local replacement fence; never durable endpoint authority.
    process_session: AgentProcessSessionKey,
    pub agent_id: String,
    pub version: String,
    pub proto_version: u32,
    pub capabilities: Vec<String>,
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
    /// Request IDs that are durable CLI work and may survive a short WS disconnect.
    cli_pending_ids: Arc<Mutex<HashSet<String>>>,
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
    process_session: AgentProcessSessionKey,
}
impl CliPromptCancelHandle {
    pub fn req_id(&self) -> &str {
        &self.req_id
    }
    pub(crate) fn process_session(&self) -> &AgentProcessSessionKey {
        &self.process_session
    }
    pub fn cancel(&self) -> bool {
        self.cmd_tx
            .send(ServerToAgent::Cancel {
                task_id: self.req_id.clone(),
                audit: homecli_proto::CancelRequestAudit::now(
                    "server",
                    "cancel_handle",
                    "caller_requested",
                ),
            })
            .is_ok()
    }
}
#[derive(Default)]
pub struct AgentManager {
    pub(crate) agents: RwLock<HashMap<String, AgentEntry>>,
    recovering_cli: pending_recovery::RecoveringCliRequests,
    recovery_worker_started: AtomicBool,
}
impl AgentManager {
    pub fn new() -> Self {
        Self::default()
    }
    /// 返回第一个当前在线的 PC agent ID（优先用于 CLI 委托）
    pub async fn any_connected_agent_id(&self) -> Option<String> {
        self.agents.read().await.keys().next().cloned()
    }
    /// Best-effort cancel for a known CLI request on a specific online node.
    /// Authorization state is committed before callers invoke this method, so
    /// a disconnected writer must never roll back the server-side revocation.
    pub async fn cancel_cli_prompt_on_agent(&self, agent_id: &str, req_id: &str) -> bool {
        let cmd_tx = self
            .agents
            .read()
            .await
            .get(agent_id)
            .map(|agent| agent.cmd_tx.clone());
        cmd_tx.is_some_and(|cmd_tx| {
            cmd_tx
                .send(ServerToAgent::Cancel {
                    task_id: req_id.to_string(),
                    audit: homecli_proto::CancelRequestAudit::now(
                        "server",
                        "agent_manager",
                        "authorization_revoked",
                    ),
                })
                .is_ok()
        })
    }

    pub async fn agent_has_capability(&self, agent_id: &str, capability: &str) -> bool {
        self.agents.read().await.get(agent_id).is_some_and(|agent| {
            agent
                .capabilities
                .iter()
                .any(|candidate| candidate == capability)
        })
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
        self.dispatch_cli_prompt_with_context_control_id(
            req_id,
            agent_id,
            cli,
            extra_args,
            cwd,
            project_context,
            prompt,
        )
        .await
    }

    /// Dispatch a pre-authorized CLI request using the exact compute identity
    /// already persisted by the server. Retries reuse the same id so a node can
    /// re-attach to active/durable work without starting a second process.
    pub async fn dispatch_cli_prompt_with_context_control_id(
        &self,
        req_id: String,
        agent_id: &str,
        cli: String,
        extra_args: Vec<String>,
        cwd: Option<String>,
        project_context: Option<homecli_proto::CliProjectContext>,
        prompt: String,
    ) -> Result<CliPromptDispatch> {
        self.dispatch_cli_prompt_with_context_control_id_and_credential_binding(
            req_id,
            agent_id,
            cli,
            extra_args,
            cwd,
            project_context,
            None,
            true,
            None,
            prompt,
        )
        .await
    }

    pub async fn dispatch_cli_prompt_with_context_control_id_and_credential_binding(
        &self,
        req_id: String,
        agent_id: &str,
        cli: String,
        extra_args: Vec<String>,
        cwd: Option<String>,
        project_context: Option<homecli_proto::CliProjectContext>,
        codex_credential_binding: Option<homecli_proto::CliCodexCredentialBinding>,
        requires_cloud_control: bool,
        cloud_control_deadline: Option<String>,
        prompt: String,
    ) -> Result<CliPromptDispatch> {
        let req_id = req_id.trim().to_string();
        if req_id.is_empty() || req_id.len() > 200 || req_id.chars().any(char::is_control) {
            return Err(anyhow!("invalid pre-authorized CLI req_id"));
        }
        let (cmd_tx, pending, cli_pending_ids, ping_acks, process_session) = {
            let agents = self.agents.read().await;
            let agent = agents
                .get(agent_id)
                .ok_or_else(|| anyhow!("agent not connected: {agent_id}"))?;
            if agent.proto_version < DURABLE_CLI_COMPLETION_PROTO_VERSION {
                return Err(anyhow!(
                    "agent protocol v{} cannot safely run CLI work: durable completion protocol v{}+ is required",
                    agent.proto_version,
                    DURABLE_CLI_COMPLETION_PROTO_VERSION
                ));
            }
            if cloud_control_deadline.is_some()
                && agent.proto_version < CLOUD_CONTROL_DEADLINE_PROTO_VERSION
            {
                return Err(anyhow!(
                    "agent protocol v{} cannot enforce cloud authorization deadlines: protocol v{}+ is required",
                    agent.proto_version,
                    CLOUD_CONTROL_DEADLINE_PROTO_VERSION
                ));
            }
            require_project_build_cache(agent, project_context.as_ref())?;
            (
                agent.cmd_tx.clone(),
                agent.pending.clone(),
                agent.cli_pending_ids.clone(),
                agent.ping_acks.clone(),
                agent.process_session.clone(),
            )
        };
        if requires_cloud_control && cloud_control_deadline.is_none() {
            return Err(anyhow!(
                "cloud-controlled CLI dispatch requires an absolute authorization deadline"
            ));
        }
        if !requires_cloud_control && cloud_control_deadline.is_some() {
            return Err(anyhow!(
                "uncontrolled CLI dispatch cannot carry a cloud authorization deadline"
            ));
        }
        if let Err(error) =
            send_protocol_ping(agent_id, &cmd_tx, &ping_acks, AGENT_DISPATCH_PROBE_TIMEOUT).await
        {
            let _ = self
                .close_process_session(&process_session, "dispatch probe failed")
                .await;
            return Err(anyhow!("agent not connected: {agent_id} ({error})"));
        }
        // A reconnect retry reuses the pre-authorized req_id. Its previous
        // receiver was dropped by the retry loop, so it must not steal the
        // durable replay from the fresh active receiver installed below.
        let _ = self.take_recovering_cli(agent_id, &req_id).await;
        let cloud_control_window = if requires_cloud_control {
            Some(freeze_cloud_control_dispatch_window(
                cloud_control_deadline
                    .as_deref()
                    .expect("cloud-controlled dispatch validated a deadline"),
            )?)
        } else {
            None
        };
        let cloud_control_issued_at = cloud_control_window
            .as_ref()
            .map(|window| window.issued_at.clone());
        let cloud_control_ttl_ms = cloud_control_window.as_ref().map(|window| window.ttl_ms);
        let server_cancel_at = cloud_control_window
            .as_ref()
            .map(|window| {
                tokio::time::Instant::now()
                    .checked_add(Duration::from_millis(window.ttl_ms))
                    .ok_or_else(|| anyhow!("cloud authorization TTL exceeds server timer range"))
            })
            .transpose()?;
        let (tx, rx) = mpsc::unbounded_channel();
        {
            let mut pending = pending.lock().await;
            let mut cli_pending_ids = cli_pending_ids.lock().await;
            pending.insert(req_id.clone(), tx);
            cli_pending_ids.insert(req_id.clone());
        }
        let cancel_handle = CliPromptCancelHandle {
            req_id: req_id.clone(),
            cmd_tx: cmd_tx.clone(),
            process_session,
        };
        if let Err(error) = cmd_tx
            .send(ServerToAgent::CliPrompt {
                req_id: req_id.clone(),
                cli,
                extra_args,
                cwd,
                project_context,
                codex_credential_binding,
                requires_cloud_control,
                cloud_control_deadline,
                cloud_control_issued_at,
                cloud_control_ttl_ms,
                prompt,
            })
            .map_err(|_| anyhow!("agent writer closed"))
        {
            let mut pending = pending.lock().await;
            let mut cli_pending_ids = cli_pending_ids.lock().await;
            pending.remove(&req_id);
            cli_pending_ids.remove(&req_id);
            return Err(error);
        }
        if let Some(cancel_at) = server_cancel_at {
            let deadline_cmd_tx = cmd_tx.clone();
            let deadline_req_id = req_id.clone();
            let deadline_pending = pending.clone();
            tokio::spawn(async move {
                tokio::time::sleep_until(cancel_at).await;
                // A final frame removes the request from `pending` before it is
                // delivered to the caller. Do not turn every successfully
                // completed cloud request into a durable pre-start tombstone at
                // its former deadline; only an actually in-flight request still
                // needs the server-side deadline fence.
                if deadline_pending.lock().await.contains_key(&deadline_req_id) {
                    let _ = deadline_cmd_tx.send(ServerToAgent::Cancel {
                        task_id: deadline_req_id,
                        audit: homecli_proto::CancelRequestAudit::now(
                            "server",
                            "cloud_control_deadline",
                            "authorization_deadline_reached",
                        ),
                    });
                }
            });
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
                proto_version: a.proto_version,
                capabilities: a.capabilities.clone(),
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
        self.dispatch_with_project_context(agent_id, cli, args, cwd, env, None)
            .await
    }

    pub async fn dispatch_with_project_context(
        &self,
        agent_id: &str,
        cli: String,
        args: Vec<String>,
        cwd: String,
        env: Vec<(String, String)>,
        project_context: Option<homecli_proto::CliProjectContext>,
    ) -> Result<(String, mpsc::UnboundedReceiver<AgentToServer>)> {
        let agents = self.agents.read().await;
        let agent = agents
            .get(agent_id)
            .ok_or_else(|| anyhow!("agent not connected: {agent_id}"))?;
        require_project_build_cache(agent, project_context.as_ref())?;
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
                project_context,
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

fn require_project_build_cache(
    _agent: &AgentEntry,
    _project_context: Option<&homecli_proto::CliProjectContext>,
) -> Result<()> {
    // Existing/external projects must keep running on their proven node. The
    // capability remains required only by the explicit new-workspace protocol.
    Ok(())
}

fn require_project_build_cache_capability(agent: &AgentEntry) -> Result<()> {
    if !agent
        .capabilities
        .iter()
        .any(|capability| capability == homecli_proto::CAP_PROJECT_BUILD_CACHE_V1)
    {
        return Err(anyhow!(
            "PC 节点版本过旧，尚不支持新建托管项目工作区；既有外部项目仍可继续运行（节点版本 {}，协议 v{}，缺少能力 {}）",
            agent.version,
            agent.proto_version,
            homecli_proto::CAP_PROJECT_BUILD_CACHE_V1,
        ));
    }
    Ok(())
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
#[cfg(test)]
use agent_session::route_req_message_to_pending;
use agent_session::{
    project_storage_prepare_timeout, project_workspace_inspect_timeout,
    project_workspace_provision_timeout, run_agent_session, send_protocol_ping,
    tool_approval_ack_key,
};

mod test_dispatch;
mod workspace_dispatch;
pub use test_dispatch::{
    test_cli_prompt, test_dispatch, TestCliPromptReq, TestCliPromptResp, TestDispatchReq,
    TestDispatchResp,
};

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Result};
use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use homecli_proto::{
    AgentToServer, NodeDevRuntimeProfile, NodeHardwareProfile, NodeStorageProfile, ServerToAgent,
};
use tokio::sync::{mpsc, oneshot, watch, Mutex};
use uuid::Uuid;

use crate::{
    node_registry::AgentProcessSessionKey,
    realtime_metrics::{self, RealtimeChannel},
    types::AppState,
    ws_transport::try_json_text_message,
};

use super::session_fencing::{apply_current_session_capabilities, install_process_session};
use super::{
    clean_optional, heartbeat, journal, AgentEntry, AgentManager, AGENT_WS_READ_TIMEOUT,
    DURABLE_CLI_COMPLETION_PROTO_VERSION, PROJECT_STORAGE_PREPARE_TIMEOUT_ENV,
    PROJECT_WORKSPACE_INSPECT_TIMEOUT_ENV, PROJECT_WORKSPACE_PROVISION_TIMEOUT_ENV,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AgentSessionCloseReason {
    ReaderShutdown,
    ReaderClosed,
    ReaderTimeout,
    ReaderError,
    WriterClosed,
}

impl AgentSessionCloseReason {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::ReaderShutdown => "reader_shutdown",
            Self::ReaderClosed => "reader_closed",
            Self::ReaderTimeout => "reader_timeout",
            Self::ReaderError => "reader_error",
            Self::WriterClosed => "writer_closed",
        }
    }

    fn pending_failure_message(self) -> &'static str {
        match self {
            Self::ReaderTimeout => {
                "PC 节点通信临时中断：节点连接读超时，可能是网络中断或节点假在线；系统会等待节点重新连接并尝试恢复。"
            }
            Self::WriterClosed => {
                "PC 节点通信临时中断：服务器向节点发送消息失败，可能是节点已断线或正在重启；系统会等待节点重新连接并尝试恢复。"
            }
            Self::ReaderError => {
                "PC 节点通信临时中断：服务器读取节点连接失败，可能是网络中断或节点正在重启；系统会等待节点重新连接并尝试恢复。"
            }
            Self::ReaderShutdown | Self::ReaderClosed => {
                "PC 节点通信临时中断：服务器正在更新升级或 Win 端正在更新升级/重启时会临时断开；系统会等待节点重新连接并尝试恢复。"
            }
        }
    }
}

pub(super) async fn run_agent_session(
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
        proto_version,
        capabilities,
        owner_user_id,
        device_name,
        install_id,
        hardware,
        storage,
        dev_runtime,
        lifecycle,
    ) = match register {
        AgentToServer::Register {
            agent_id,
            version,
            proto_version,
            capabilities,
            allowed_clis,
            allowed_cwds,
            owner_user_id,
            device_name,
            install_id,
            hardware,
            storage,
            dev_runtime,
            lifecycle,
        } => (
            agent_id,
            version,
            allowed_clis,
            allowed_cwds,
            proto_version,
            capabilities,
            owner_user_id,
            clean_optional(device_name),
            clean_optional(install_id),
            hardware,
            storage,
            dev_runtime,
            lifecycle,
        ),
        _ => return Err(anyhow!("first frame must be register")),
    };
    let authorized = super::legacy_session_authority::authenticate_and_prepare(
        &state,
        &secrets,
        &presented_token,
        &agent_id,
        &version,
        proto_version,
        &allowed_clis,
        owner_user_id.as_deref(),
        device_name.as_deref(),
        install_id.as_deref(),
        hardware.as_ref(),
        dev_runtime.as_ref(),
    )
    .await?;
    let resolved_owner_user_id = authorized.owner_user_id;
    let resolved_install_id = authorized.install_id;
    let credential_proof = authorized.credential_proof;
    tracing::info!(%agent_id, %version, proto_version, device_name = ?device_name, "agent registered");
    state.agent_manager.ensure_cli_recovery_worker();
    let session_id = Uuid::new_v4().to_string();
    let process_session = AgentProcessSessionKey::new(agent_id.clone(), session_id.clone());
    let session_version = version.clone();
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<ServerToAgent>();
    let (control_tx, mut control_rx) = mpsc::unbounded_channel::<Message>();
    let pending: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<AgentToServer>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let cli_pending_ids = Arc::new(Mutex::new(HashSet::new()));
    let approval_acks: Arc<Mutex<HashMap<String, oneshot::Sender<bool>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let ping_acks: Arc<Mutex<HashMap<String, oneshot::Sender<()>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let (session_shutdown, session_shutdown_rx) = watch::channel(false);
    let entry = AgentEntry {
        process_session: process_session.clone(),
        agent_id: agent_id.clone(),
        version,
        proto_version,
        capabilities: capabilities.clone(),
        device_name: device_name.clone(),
        hardware: hardware.clone(),
        storage: storage.clone(),
        dev_runtime: dev_runtime.clone(),
        lifecycle: lifecycle.clone(),
        allowed_clis,
        allowed_cwds,
        connected_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        cmd_tx: cmd_tx.clone(),
        pending: pending.clone(),
        cli_pending_ids: cli_pending_ids.clone(),
        approval_acks: approval_acks.clone(),
        ping_acks: ping_acks.clone(),
        session_shutdown: session_shutdown.clone(),
    };

    let session_device_name = device_name.clone();
    let session_owner_user_id = resolved_owner_user_id.clone().unwrap_or_default();
    install_process_session(
        &state,
        &process_session,
        entry,
        session_owner_user_id.clone(),
        resolved_install_id,
        credential_proof,
    )
    .await?;

    super::compute_plugin_sharing::spawn_current_compute_plugin_sharing_session_replay(
        Arc::clone(&state),
        process_session.clone(),
        proto_version,
        &capabilities,
    );

    // 服务端 → 节点 ping 定时器：短间隔 Ping，并要求 Pong ACK。
    // 普通家庭/办公网络会静默丢弃空闲 TCP；没有 ACK 就主动摘掉会话，
    // 避免下一次用户请求先撞上“假在线”旧连接。
    {
        heartbeat::spawn_agent_heartbeat(
            agent_id.clone(),
            control_tx.clone(),
            ping_acks.clone(),
            session_shutdown.clone(),
            session_shutdown_rx.clone(),
        );
    }

    // Writer: drain cmd_rx → ws_tx.
    let mut writer_shutdown_rx = session_shutdown_rx.clone();
    let writer_session_shutdown = session_shutdown.clone();
    let writer = tokio::spawn(async move {
        let mut close_reason = None;
        loop {
            let outbound = tokio::select! {
                biased;
                _ = writer_shutdown_rx.changed() => break,
                control = control_rx.recv() => match control {
                    Some(msg) => msg,
                    None => {
                        close_reason = Some(AgentSessionCloseReason::WriterClosed);
                        let _ = writer_session_shutdown.send(true);
                        break;
                    }
                },
                msg = cmd_rx.recv() => match msg {
                    Some(msg) => match try_json_text_message(&msg) {
                        Ok(frame) => frame,
                        Err(e) => {
                            tracing::error!("serialize ServerToAgent: {e}");
                            continue;
                        }
                    },
                    None => {
                        close_reason = Some(AgentSessionCloseReason::WriterClosed);
                        let _ = writer_session_shutdown.send(true);
                        break;
                    }
                },
            };
            if ws_tx.send(outbound).await.is_err() {
                close_reason = Some(AgentSessionCloseReason::WriterClosed);
                let _ = writer_session_shutdown.send(true);
                break;
            }
        }
        let _ = ws_tx.close().await;
        close_reason
    });

    // Reader: route AgentToServer events to the right pending task.
    let pending_r = pending.clone();
    let cli_pending_ids_r = cli_pending_ids.clone();
    let approval_acks_r = approval_acks.clone();
    let ping_acks_r = ping_acks.clone();
    let mut reader_shutdown_rx = session_shutdown_rx.clone();
    let (read_result, mut close_reason): (Result<()>, AgentSessionCloseReason) = async {
        let close_reason = loop {
            let frame = tokio::select! {
                _ = reader_shutdown_rx.changed() => {
                    break AgentSessionCloseReason::ReaderShutdown;
                }
                maybe_frame = tokio::time::timeout(AGENT_WS_READ_TIMEOUT, ws_rx.next()) => {
                    match maybe_frame {
                        Ok(Some(frame)) => match frame {
                            Ok(frame) => frame,
                            Err(error) => return (
                                Err(anyhow!("ws read: {error}")),
                                AgentSessionCloseReason::ReaderError,
                            ),
                        },
                        Ok(None) => break AgentSessionCloseReason::ReaderClosed,
                        Err(_) => return (
                            Err(anyhow!(
                                "agent ws read timeout ({}s)",
                                AGENT_WS_READ_TIMEOUT.as_secs()
                            )),
                            AgentSessionCloseReason::ReaderTimeout,
                        ),
                    }
                }
            };
            match frame {
                Message::Text(t) => match serde_json::from_str::<AgentToServer>(&t) {
                    Ok(msg) => {
                        if !state.node_registry.touch_exact(&process_session).await {
                            break AgentSessionCloseReason::ReaderShutdown;
                        }
                        if let AgentToServer::CliCompletionReplay { completion } = &msg {
                            let ack = state
                                .agent_manager
                                .with_current_process_session(&process_session, |_| {
                                    crate::ai_cli::pc_completion_replay::handle_pc_cli_completion_replay(
                                        state.as_ref(),
                                        &agent_id,
                                        resolved_owner_user_id.as_deref(),
                                        install_id.as_deref(),
                                        completion.clone(),
                                    )
                                })
                                .await;
                            let Some(ack) = ack else {
                                break AgentSessionCloseReason::ReaderShutdown;
                            };
                            if matches!(
                                &ack,
                                ServerToAgent::CliCompletionAck { accepted: true, .. }
                            ) {
                                let delivered = state
                                    .agent_manager
                                    .deliver_accepted_cli_replay(&agent_id, completion)
                                    .await;
                                tracing::debug!(
                                    %agent_id,
                                    req_id = %completion.req_id,
                                    delivered,
                                    "accepted PC CLI replay routed to in-memory receiver"
                                );
                            }
                            if cmd_tx.send(ack).is_err() {
                                return (
                                    Err(anyhow!("agent writer closed before completion ACK")),
                                    AgentSessionCloseReason::WriterClosed,
                                );
                            }
                            continue;
                        }
                        if let AgentToServer::CliLocalTaskSync { snapshot } = &msg {
                            let ack = state
                                .agent_manager
                                .with_current_process_session(&process_session, |_| {
                                    crate::ai_cli::pc_completion_replay::handle_pc_local_task_sync(
                                        state.as_ref(),
                                        &agent_id,
                                        resolved_owner_user_id.as_deref(),
                                        install_id.as_deref(),
                                        snapshot.clone(),
                                    )
                                })
                                .await;
                            let Some(ack) = ack else {
                                break AgentSessionCloseReason::ReaderShutdown;
                            };
                            if cmd_tx.send(ack).is_err() {
                                return (
                                    Err(anyhow!("agent writer closed before local task sync ACK")),
                                    AgentSessionCloseReason::WriterClosed,
                                );
                            }
                            continue;
                        }
                        if let AgentToServer::ToolApprovalDecisionAck {
                            req_id,
                            approval_id,
                            dispatch_id,
                            accepted,
                        } = &msg
                        {
                            let ack_key = tool_approval_ack_key(req_id, approval_id, dispatch_id);
                            match state
                                .agent_manager
                                .deliver_current_tool_approval_ack(
                                    &process_session,
                                    &approval_acks_r,
                                    &ack_key,
                                    *accepted,
                                )
                                .await
                            {
                                Some(true) => {}
                                Some(false) => {
                                    tracing::warn!(
                                        %req_id,
                                        %approval_id,
                                        %dispatch_id,
                                        "unexpected tool approval ACK"
                                    );
                                }
                                None => break AgentSessionCloseReason::ReaderShutdown,
                            }
                            continue;
                        }
                        if msg.task_id().is_some() {
                            if state
                                .agent_manager
                                .deliver_current_task_message(&process_session, &pending_r, msg)
                                .await
                                .is_none()
                            {
                                break AgentSessionCloseReason::ReaderShutdown;
                            }
                        } else if msg.req_id().is_some() {
                            if state
                                .agent_manager
                                .deliver_current_req_message(
                                    &process_session,
                                    &pending_r,
                                    &cli_pending_ids_r,
                                    msg,
                                )
                                .await
                                .is_none()
                            {
                                break AgentSessionCloseReason::ReaderShutdown;
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
                                    lifecycle,
                                } => {
                                    if !apply_current_session_capabilities(
                                        &state,
                                        &process_session,
                                        &session_owner_user_id,
                                        session_device_name.as_deref(),
                                        &session_version,
                                        models,
                                        allowed_clis,
                                        tts_worker_url.as_deref(),
                                        hardware.as_ref(),
                                        storage.as_ref(),
                                        dev_runtime.as_ref(),
                                        lifecycle.as_ref(),
                                    )
                                    .await
                                    {
                                        break AgentSessionCloseReason::ReaderShutdown;
                                    }
                                }
                                AgentToServer::Pong { nonce } => {
                                    if let Some(nonce) = nonce.as_deref() {
                                        if let Some(tx) = ping_acks_r.lock().await.remove(nonce) {
                                            let _ = tx.send(());
                                        }
                                    }
                                    state.node_registry.touch_exact(&process_session).await;
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(e) => tracing::warn!("bad agent msg: {e}: {t}"),
                },
                Message::Ping(payload) => {
                    state.node_registry.touch_exact(&process_session).await;
                    if control_tx.send(Message::Pong(payload)).is_err() {
                        break AgentSessionCloseReason::WriterClosed;
                    }
                }
                Message::Pong(_) => {
                    state.node_registry.touch_exact(&process_session).await;
                }
                Message::Close(_) => break AgentSessionCloseReason::ReaderClosed,
                Message::Binary(_) => {}
            }
        };
        (Ok(()), close_reason)
    }
    .await;

    // Clean up: 先移除 agent，再通知挂起请求失败，避免断线后调用方永久阻塞
    let removed_current_session = {
        let mut agents = state.agent_manager.agents.write().await;
        let is_current = agents
            .get(&agent_id)
            .is_some_and(|entry| entry.process_session == process_session);
        if is_current {
            agents.remove(&agent_id);
            state.node_registry.unregister_exact(&process_session).await;
            true
        } else {
            false
        }
    };
    if !removed_current_session {
        tracing::info!(
            %agent_id,
            %session_id,
            "stale PC agent session ended after a newer session was registered"
        );
    }

    // CLI receiver stays alive through a bounded reconnect window; all other
    // request types retain the historical fail-fast behavior.
    let _ = session_shutdown.send(true);
    drop(cmd_tx);
    let writer_close_reason = writer.await.ok().flatten();
    if matches!(close_reason, AgentSessionCloseReason::ReaderShutdown) {
        if let Some(reason) = writer_close_reason {
            close_reason = reason;
        }
    }

    state
        .agent_manager
        .recover_session_pending(
            &agent_id,
            &pending,
            &cli_pending_ids,
            close_reason.pending_failure_message(),
        )
        .await;
    {
        fail_pending_approvals(&approval_acks).await;
    }
    {
        fail_pending_pings(&ping_acks).await;
    }

    let close_reason_name = close_reason.as_str();
    realtime_metrics::record_close_with_store(
        &state.store,
        RealtimeChannel::HomecliAgent,
        close_reason_name,
    );
    tracing::info!(%agent_id, close_reason = close_reason_name, "agent disconnected");
    read_result
}

/// Route one req_id-scoped message through the session's transient waiter map.
/// Final messages remove the waiter before delivery so deadline tasks cannot
/// emit a stale Cancel after the CLI process has already completed.
#[cfg(test)]
pub(super) async fn route_req_message_to_pending(
    pending: &Arc<Mutex<HashMap<String, mpsc::UnboundedSender<AgentToServer>>>>,
    cli_pending_ids: &Arc<Mutex<HashSet<String>>>,
    msg: AgentToServer,
) -> bool {
    let Some(req_id) = msg.req_id().map(str::to_owned) else {
        return false;
    };
    let is_final = msg.is_final_req_msg();
    let mut pending = pending.lock().await;
    if is_final {
        cli_pending_ids.lock().await.remove(&req_id);
        if let Some(tx) = pending.remove(&req_id) {
            let _ = tx.send(msg);
        }
    } else if let Some(tx) = pending.get(&req_id) {
        let _ = tx.send(msg);
    }
    true
}

pub(super) fn tool_approval_ack_key(req_id: &str, approval_id: &str, dispatch_id: &str) -> String {
    format!("{req_id}:{approval_id}:{dispatch_id}")
}

pub(super) async fn send_protocol_ping(
    agent_id: &str,
    cmd_tx: &mpsc::UnboundedSender<ServerToAgent>,
    ping_acks: &Arc<Mutex<HashMap<String, oneshot::Sender<()>>>>,
    timeout: Duration,
) -> Result<()> {
    let nonce = Uuid::new_v4().to_string();
    let (ack_tx, ack_rx) = oneshot::channel();
    ping_acks.lock().await.insert(nonce.clone(), ack_tx);
    if cmd_tx
        .send(ServerToAgent::Ping {
            nonce: Some(nonce.clone()),
        })
        .is_err()
    {
        ping_acks.lock().await.remove(&nonce);
        return Err(anyhow!("agent writer closed before ping: {agent_id}"));
    }
    match tokio::time::timeout(timeout, ack_rx).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(_)) => Err(anyhow!("agent ping waiter closed: {agent_id}")),
        Err(_) => {
            ping_acks.lock().await.remove(&nonce);
            Err(anyhow!(
                "agent ping ack timeout after {}s: {agent_id}",
                timeout.as_secs()
            ))
        }
    }
}

pub(super) async fn fail_pending_approvals(
    approval_acks: &Arc<Mutex<HashMap<String, oneshot::Sender<bool>>>>,
) {
    let mut approval_acks = approval_acks.lock().await;
    let stale: Vec<_> = approval_acks.drain().collect();
    drop(approval_acks);
    for (_, sender) in stale {
        let _ = sender.send(false);
    }
}

pub(super) async fn fail_pending_pings(
    ping_acks: &Arc<Mutex<HashMap<String, oneshot::Sender<()>>>>,
) {
    let mut ping_acks = ping_acks.lock().await;
    ping_acks.clear();
}

pub(super) fn project_workspace_provision_timeout() -> Duration {
    env_timeout(PROJECT_WORKSPACE_PROVISION_TIMEOUT_ENV, 30, 5, 180)
}

pub(super) fn project_workspace_inspect_timeout() -> Duration {
    env_timeout(PROJECT_WORKSPACE_INSPECT_TIMEOUT_ENV, 3, 1, 30)
}

pub(super) fn project_storage_prepare_timeout() -> Duration {
    env_timeout(PROJECT_STORAGE_PREPARE_TIMEOUT_ENV, 15, 5, 120)
}

pub(super) fn env_timeout(name: &str, default_secs: u64, min_secs: u64, max_secs: u64) -> Duration {
    let seconds = std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(default_secs)
        .clamp(min_secs, max_secs);
    Duration::from_secs(seconds)
}

// ── /api/_test_dispatch handler (Phase 1 smoke test) ─────────────────────────

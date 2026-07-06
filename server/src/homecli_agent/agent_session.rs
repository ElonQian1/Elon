use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Result};
use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use homecli_proto::{AgentToServer, NodeDevRuntimeProfile, NodeHardwareProfile, NodeStorageProfile, ServerToAgent};
use tokio::sync::{mpsc, oneshot, watch, Mutex};
use uuid::Uuid;

use sha2::Digest as _;

use crate::types::AppState;

use super::{
    heartbeat, journal,
    AgentEntry, AgentManager, AGENT_WS_READ_TIMEOUT, TOOL_APPROVAL_ACK_TIMEOUT,
    PROJECT_WORKSPACE_PROVISION_TIMEOUT_ENV, PROJECT_WORKSPACE_INSPECT_TIMEOUT_ENV,
    PROJECT_STORAGE_PREPARE_TIMEOUT_ENV,
    clean_optional,
};
use super::public_dev_handshake::record_node_public_dev_handshake;

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
        _proto_ver,
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
    if let Some(owner) = &resolved_owner_user_id {
        if let Err(e) = state.store.update_node_credential_registration_info(
            &agent_id,
            owner,
            install_id.as_deref(),
            device_name.as_deref(),
        ) {
            tracing::warn!(%agent_id, error = %e, "failed to update node registration info");
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
    if let Some(owner) = &resolved_owner_user_id {
        record_node_public_dev_handshake(
            &state,
            &agent_id,
            owner,
            &version,
            &allowed_clis,
            dev_runtime.as_ref(),
            "failed to record node handshake",
        )
        .await;
    }
    tracing::info!(%agent_id, %version, device_name = ?device_name, "agent registered");
    let session_id = Uuid::new_v4().to_string();
    let session_version = version.clone();
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<ServerToAgent>();
    let (control_tx, mut control_rx) = mpsc::unbounded_channel::<Message>();
    let pending: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<AgentToServer>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let approval_acks: Arc<Mutex<HashMap<String, oneshot::Sender<bool>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let ping_acks: Arc<Mutex<HashMap<String, oneshot::Sender<()>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let (session_shutdown, session_shutdown_rx) = watch::channel(false);
    let entry = AgentEntry {
        session_id: session_id.clone(),
        agent_id: agent_id.clone(),
        version,
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
        approval_acks: approval_acks.clone(),
        ping_acks: ping_acks.clone(),
        session_shutdown: session_shutdown.clone(),
    };

    // 若同一 agent_id 已有旧连接，通过旧 cmd_tx 发 Close 消息主动终止它，
    // 避免两个实例并存导致 WebSocket 资源竞争和 "Connection reset by peer"。
    {
        let mut agents = state.agent_manager.agents.write().await;
        if let Some(old_entry) = agents.get(&agent_id) {
            tracing::info!(
                %agent_id,
                old_session_id = %old_entry.session_id,
                new_session_id = %session_id,
                "evicting previous agent session (same agent_id re-registered)"
            );
            // 通知旧连接的所有挂起请求立即失败
            fail_pending_requests(&old_entry.pending, "PC 节点通信临时中断：Win 端正在更新升级/重启或节点重新注册，旧连接已关闭。").await;
            fail_pending_approvals(&old_entry.approval_acks).await;
            fail_pending_pings(&old_entry.ping_acks).await;
            let _ = old_entry.session_shutdown.send(true);
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
            lifecycle,
            vec![],
            connected_at,
        )
        .await;

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
    let writer = tokio::spawn(async move {
        loop {
            let outbound = tokio::select! {
                biased;
                _ = writer_shutdown_rx.changed() => break,
                control = control_rx.recv() => match control {
                    Some(msg) => msg,
                    None => break,
                },
                msg = cmd_rx.recv() => match msg {
                    Some(msg) => match serde_json::to_string(&msg) {
                        Ok(text) => Message::Text(text),
                        Err(e) => {
                            tracing::error!("serialize ServerToAgent: {e}");
                            continue;
                        }
                    },
                    None => break,
                },
            };
            if ws_tx.send(outbound).await.is_err() {
                break;
            }
        }
        let _ = ws_tx.close().await;
    });

    // Reader: route AgentToServer events to the right pending task.
    let pending_r = pending.clone();
    let approval_acks_r = approval_acks.clone();
    let ping_acks_r = ping_acks.clone();
    let mut reader_shutdown_rx = session_shutdown_rx.clone();
    let read_result: Result<()> = async {
        loop {
            let frame = tokio::select! {
                _ = reader_shutdown_rx.changed() => break,
                maybe_frame = tokio::time::timeout(AGENT_WS_READ_TIMEOUT, ws_rx.next()) => {
                    match maybe_frame {
                        Ok(Some(frame)) => frame.map_err(|e| anyhow!("ws read: {e}"))?,
                        Ok(None) => break,
                        Err(_) => return Err(anyhow!(
                            "agent ws read timeout ({}s)",
                            AGENT_WS_READ_TIMEOUT.as_secs()
                        )),
                    }
                }
            };
            match frame {
                Message::Text(t) => match serde_json::from_str::<AgentToServer>(&t) {
                    Ok(msg) => {
                        state.node_registry.touch(&agent_id).await;
                        if let AgentToServer::ToolApprovalDecisionAck {
                            req_id,
                            approval_id,
                            dispatch_id,
                            accepted,
                        } = &msg
                        {
                            let ack_key = tool_approval_ack_key(req_id, approval_id, dispatch_id);
                            if let Some(tx) = approval_acks_r.lock().await.remove(&ack_key) {
                                let _ = tx.send(*accepted);
                            } else {
                                tracing::warn!(
                                    %req_id,
                                    %approval_id,
                                    %dispatch_id,
                                    "unexpected tool approval ACK"
                                );
                            }
                            continue;
                        }
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
                                    lifecycle,
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
                                    if !session_owner_user_id.is_empty() {
                                        record_node_public_dev_handshake(
                                            &state,
                                            &agent_id,
                                            &session_owner_user_id,
                                            &session_version,
                                            &allowed_clis,
                                            dev_runtime.as_ref(),
                                            "failed to record node capability handshake",
                                        )
                                        .await;
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
                                            if lifecycle.is_some() {
                                                entry.lifecycle = lifecycle.clone();
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
                                            lifecycle.clone(),
                                        )
                                        .await;
                                }
                                AgentToServer::Pong { nonce } => {
                                    if let Some(nonce) = nonce.as_deref() {
                                        if let Some(tx) = ping_acks_r.lock().await.remove(nonce) {
                                            let _ = tx.send(());
                                        }
                                    }
                                    state.node_registry.touch(&agent_id).await;
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(e) => tracing::warn!("bad agent msg: {e}: {t}"),
                },
                Message::Ping(payload) => {
                    state.node_registry.touch(&agent_id).await;
                    if control_tx.send(Message::Pong(payload)).is_err() {
                        break;
                    }
                }
                Message::Pong(_) => {
                    state.node_registry.touch(&agent_id).await;
                }
                Message::Close(_) => break,
                Message::Binary(_) => {}
            }
        }
        Ok(())
    }
    .await;

    // Clean up: 先移除 agent，再通知挂起请求失败，避免断线后调用方永久阻塞
    let removed_current_session = {
        let mut agents = state.agent_manager.agents.write().await;
        let is_current = agents
            .get(&agent_id)
            .map(|entry| entry.session_id == session_id)
            .unwrap_or(false);
        if is_current {
            agents.remove(&agent_id);
            true
        } else {
            false
        }
    };
    if removed_current_session {
        state.node_registry.unregister(&agent_id).await;
    } else {
        tracing::info!(
            %agent_id,
            %session_id,
            "stale PC agent session ended after a newer session was registered"
        );
    }

    // 节点断线时，向所有还在等待响应的 CLI 请求发送 CliDone(exit_ok=false)，
    // 让 run_via_pc_agent 的 while rx.recv() 立即收到错误并返回，
    // 而不是永远阻塞到 HTTP 请求超时。
    {
        fail_pending_requests(&pending, "PC 节点通信临时中断：服务器正在更新升级或 Win 端正在更新升级/重启时会临时断开；系统会等待节点重新连接并尝试恢复。").await;
    }
    {
        fail_pending_approvals(&approval_acks).await;
    }
    {
        fail_pending_pings(&ping_acks).await;
    }

    drop(cmd_tx);
    let _ = writer.await;
    tracing::info!(%agent_id, "agent disconnected");
    read_result
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

pub(super) async fn fail_pending_requests(
    pending: &Arc<Mutex<HashMap<String, mpsc::UnboundedSender<AgentToServer>>>>,
    message: &str,
) {
    let mut pending = pending.lock().await;
    let stale: Vec<(String, mpsc::UnboundedSender<AgentToServer>)> = pending.drain().collect();
    drop(pending);
    for (req_id, sender) in stale {
        let _ = sender.send(AgentToServer::CliDone {
            req_id,
            exit_ok: false,
            error: Some(message.to_string()),
            session_id: None,
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

pub(super) async fn fail_pending_pings(ping_acks: &Arc<Mutex<HashMap<String, oneshot::Sender<()>>>>) {
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

pub(super) fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

// ── /api/_test_dispatch handler (Phase 1 smoke test) ─────────────────────────


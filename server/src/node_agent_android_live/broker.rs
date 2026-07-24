use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use axum::extract::ws::{Message, WebSocket};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::sync::{mpsc, oneshot, Mutex, RwLock};

use super::protocol::{
    LivePatchOperation, LivePropertyValue, LiveSessionView, LiveSourceProofView, LiveStylePatch,
    LiveUiNode, RuntimeWelcome, PROTOCOL_VERSION,
};

const PATCH_ACK_TIMEOUT: Duration = Duration::from_secs(5);
const FRAME_ACK_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_HISTORY: usize = 100;

#[derive(Default)]
pub(crate) struct LiveUiBroker {
    sessions: RwLock<HashMap<String, Arc<LiveUiSession>>>,
    pub(crate) debug_deployments: super::deployment_serialization::DebugDeploymentRegistry,
    pub(crate) debug_runtime_preparations: super::build_verify::PreparationRegistry,
    pub(crate) build_verifications: super::build_verify::BuildVerifyOperationRegistry,
    pub(crate) debug_integration: super::debug_integration::DebugIntegrationCoordinator,
}

pub(crate) struct LiveUiSession {
    pub(crate) id: String,
    pub(crate) token: String,
    pub(crate) device_id: String,
    pub(crate) device_identity: String,
    pub(crate) debug_project_id: String,
    pub(crate) package_name: String,
    pub(crate) project_root: Option<String>,
    pub(crate) device_port: u16,
    pub(crate) created_at: String,
    state: RwLock<LiveSessionState>,
    runtime_tx: RwLock<Option<mpsc::UnboundedSender<Message>>>,
    pending: Mutex<HashMap<String, oneshot::Sender<Value>>>,
}

#[derive(Clone)]
pub(crate) struct LiveCommitSnapshot {
    pub(crate) project_root: Option<String>,
    pub(crate) nodes: Vec<LiveUiNode>,
    pub(crate) patches: Vec<LiveStylePatch>,
}

#[derive(Default)]
struct LiveSessionState {
    connected: bool,
    runtime_stage: Option<String>,
    runtime_build_id: Option<String>,
    runtime_version: Option<String>,
    tree_revision: u64,
    nodes: Vec<LiveUiNode>,
    history: Vec<PatchJournalEntry>,
    future: Vec<PatchJournalEntry>,
    source_proof: Option<LiveSourceProofView>,
    last_seen_at: Option<String>,
    last_error: Option<String>,
}

#[derive(Clone)]
struct PatchJournalEntry {
    forward: LiveStylePatch,
    inverse: LiveStylePatch,
}

impl LiveUiBroker {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn for_node(install_id: &str, integration_root: std::path::PathBuf) -> Self {
        let fingerprint = super::node_debug_fingerprint(install_id).unwrap_or_default();
        Self {
            debug_deployments: super::deployment_serialization::DebugDeploymentRegistry::for_node(
                install_id,
            ),
            debug_integration: super::debug_integration::DebugIntegrationCoordinator::new(
                integration_root,
                fingerprint,
            ),
            ..Self::default()
        }
    }

    pub(crate) async fn remove_session(&self, session_id: &str) -> Option<Arc<LiveUiSession>> {
        self.sessions.write().await.remove(session_id)
    }

    pub(crate) async fn session(&self, session_id: &str) -> Result<Arc<LiveUiSession>> {
        self.sessions
            .read()
            .await
            .get(session_id)
            .cloned()
            .ok_or_else(|| anyhow!("Live UI 会话不存在或已结束"))
    }

    pub(crate) async fn connected_session_for_project(
        &self,
        project_root: &str,
    ) -> Option<Arc<LiveUiSession>> {
        let expected = canonical_or_raw(project_root);
        let sessions = self
            .sessions
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let mut matched = Vec::new();
        for session in sessions {
            let Some(root) = session.project_root.as_deref() else {
                continue;
            };
            if canonical_or_raw(root) != expected || !session.view().await.connected {
                continue;
            }
            matched.push(session);
        }
        matched.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        matched.into_iter().next()
    }

    /// Selects a real Runtime session for project-scoped verification.  Unlike the
    /// convenience lookup used by the UI, this deliberately refuses to guess when
    /// two devices/runtimes claim the same checkout.
    pub(crate) async fn unique_connected_runtime_for_project(
        &self,
        project_root: &str,
    ) -> Result<Arc<LiveUiSession>> {
        let expected = canonical_or_raw(project_root);
        let sessions = self
            .sessions
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let mut matched = Vec::new();
        for session in sessions {
            let Some(root) = session.project_root.as_deref() else {
                continue;
            };
            let view = session.view().await;
            if canonical_or_raw(root) == expected
                && view.connected
                && view.node_count > 0
                && session.device_id != "ui-design-bootstrap"
                && session.package_name != "ui.design.bootstrap"
            {
                matched.push(session);
            }
        }
        match matched.len() {
            1 => Ok(matched.remove(0)),
            0 => bail!(
                "项目没有已连接且已上报节点的真实 Android Runtime；请先连接真机/模拟器会话"
            ),
            count => bail!(
                "项目同时存在 {count} 个真实 Android Runtime，会话身份不唯一；请显式结束旧会话后重试"
            ),
        }
    }

    pub(crate) async fn session_for_project(
        &self,
        project_root: &str,
    ) -> Option<Arc<LiveUiSession>> {
        if let Some(session) = self.connected_session_for_project(project_root).await {
            return Some(session);
        }
        let expected = canonical_or_raw(project_root);
        let mut matched = self
            .sessions
            .read()
            .await
            .values()
            .filter(|session| {
                session
                    .project_root
                    .as_deref()
                    .is_some_and(|root| canonical_or_raw(root) == expected)
            })
            .cloned()
            .collect::<Vec<_>>();
        matched.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        matched.into_iter().next()
    }

    pub(crate) async fn runtime_session_for(
        &self,
        project_root: &str,
        device_id: &str,
        package_name: &str,
        device_identity: &str,
        debug_project_id: &str,
    ) -> Option<Arc<LiveUiSession>> {
        let expected_root = canonical_or_raw(project_root);
        let mut matched = self
            .sessions
            .read()
            .await
            .values()
            .filter(|session| {
                session.device_id == device_id
                    && session.device_identity == device_identity
                    && session.debug_project_id == debug_project_id
                    && session.package_name == package_name
                    && session
                        .project_root
                        .as_deref()
                        .is_some_and(|root| canonical_or_raw(root) == expected_root)
            })
            .cloned()
            .collect::<Vec<_>>();
        matched.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        matched.into_iter().next()
    }

    pub(crate) async fn effective_session_id(&self, session_id: &str) -> Result<String> {
        let requested = self.session(session_id).await?;
        if requested.view().await.connected {
            return Ok(requested.id.clone());
        }
        let Some(root) = requested.project_root.as_deref() else {
            return Ok(requested.id.clone());
        };
        Ok(self
            .connected_session_for_project(root)
            .await
            .map(|session| session.id.clone())
            .unwrap_or_else(|| requested.id.clone()))
    }

    pub(crate) async fn session_view(&self, session_id: &str) -> Result<LiveSessionView> {
        Ok(self.session(session_id).await?.view().await)
    }

    pub(crate) async fn authorize_session(&self, session_id: &str, token: &str) -> Result<()> {
        let session = self.session(session_id).await?;
        if !constant_time_eq(session.token.as_bytes(), token.as_bytes()) {
            bail!("Live UI 会话令牌无效");
        }
        Ok(())
    }

    pub(crate) async fn tree(&self, session_id: &str) -> Result<(u64, Vec<LiveUiNode>)> {
        let session = self.session(session_id).await?;
        let state = session.state.read().await;
        Ok((state.tree_revision, state.nodes.clone()))
    }

    pub(crate) async fn attach_runtime(
        &self,
        session_id: &str,
        token: &str,
        socket: WebSocket,
    ) -> Result<()> {
        let session = self.session(session_id).await?;
        if !constant_time_eq(session.token.as_bytes(), token.as_bytes()) {
            let mut state = session.state.write().await;
            state.runtime_stage = Some("WEBSOCKET_AUTH_REJECTED".to_string());
            state.last_error = Some("Runtime WebSocket 会话认证失败".to_string());
            bail!("Live UI 会话令牌无效");
        }
        let (mut sink, mut stream) = socket.split();
        let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
        *session.runtime_tx.write().await = Some(tx.clone());
        {
            let mut state = session.state.write().await;
            state.connected = true;
            state.runtime_stage = Some("WEBSOCKET_AUTHORIZED".to_string());
            state.last_seen_at = Some(Utc::now().to_rfc3339());
            state.last_error = None;
        }
        tx.send(Message::Text(serde_json::to_string(&RuntimeWelcome {
            protocol_version: PROTOCOL_VERSION,
            message_type: "broker.welcome",
            session_id: session.id.clone(),
            accepted: true,
        })?))
        .map_err(|_| anyhow!("无法发送 Live UI welcome"))?;
        session.record_runtime_stage("BROKER_WELCOME_SENT").await;

        let writer = tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if sink.send(message).await.is_err() {
                    break;
                }
            }
        });

        while let Some(incoming) = stream.next().await {
            match incoming {
                Ok(Message::Text(text)) => {
                    if let Err(error) = session.handle_runtime_text(&text).await {
                        session.set_error(format!("{error:#}")).await;
                    }
                }
                Ok(Message::Ping(payload)) => {
                    let _ = tx.send(Message::Pong(payload));
                }
                Ok(Message::Close(_)) => break,
                Ok(_) => {}
                Err(error) => {
                    session
                        .set_error(format!("Runtime WebSocket: {error}"))
                        .await;
                    break;
                }
            }
        }
        writer.abort();
        // A replacement Runtime connection may have been installed while this older
        // socket was winding down. Only clear the sender/state when this connection
        // still owns the session; otherwise the stale task would disconnect the new one.
        let cleared_current_connection = {
            let mut runtime_tx = session.runtime_tx.write().await;
            if runtime_tx
                .as_ref()
                .is_some_and(|current| current.same_channel(&tx))
            {
                *runtime_tx = None;
                true
            } else {
                false
            }
        };
        if cleared_current_connection {
            let mut state = session.state.write().await;
            state.connected = false;
            state.runtime_stage = Some("WEBSOCKET_DISCONNECTED".to_string());
        }
        Ok(())
    }

    pub(crate) async fn apply_patch(
        &self,
        session_id: &str,
        mut patch: LiveStylePatch,
    ) -> Result<Value> {
        patch.prepare(session_id);
        patch.validate().map_err(anyhow::Error::msg)?;
        let session = self.session(session_id).await?;
        session.send_patch(patch, JournalMode::Record).await
    }

    pub(crate) async fn apply_probe_patch(
        &self,
        session_id: &str,
        mut patch: LiveStylePatch,
    ) -> Result<(Value, LiveStylePatch)> {
        patch.prepare(session_id);
        patch.validate().map_err(anyhow::Error::msg)?;
        let session = self.session(session_id).await?;
        let ack = session.send_patch(patch.clone(), JournalMode::Skip).await?;
        let inverse = inverse_patch(&patch, &ack)
            .ok_or_else(|| anyhow!("Android Probe ACK 缺少 beforeValues，无法安全恢复"))?;
        Ok((ack, inverse))
    }

    pub(crate) async fn restore_probe_patch(
        &self,
        session_id: &str,
        mut inverse: LiveStylePatch,
    ) -> Result<Value> {
        inverse.prepare(session_id);
        inverse.validate().map_err(anyhow::Error::msg)?;
        self.session(session_id)
            .await?
            .send_patch(inverse, JournalMode::Skip)
            .await
    }

    pub(crate) async fn undo(&self, session_id: &str) -> Result<Value> {
        let session = self.session(session_id).await?;
        let entry = {
            let mut state = session.state.write().await;
            state
                .history
                .pop()
                .ok_or_else(|| anyhow!("没有可撤销的 Live UI 修改"))?
        };
        match session
            .send_patch(entry.inverse.clone(), JournalMode::Skip)
            .await
        {
            Ok(ack) => {
                session.state.write().await.future.push(entry);
                Ok(ack)
            }
            Err(error) => {
                session.state.write().await.history.push(entry);
                Err(error)
            }
        }
    }

    pub(crate) async fn redo(&self, session_id: &str) -> Result<Value> {
        let session = self.session(session_id).await?;
        let entry = {
            let mut state = session.state.write().await;
            state
                .future
                .pop()
                .ok_or_else(|| anyhow!("没有可重做的 Live UI 修改"))?
        };
        match session
            .send_patch(entry.forward.clone(), JournalMode::Skip)
            .await
        {
            Ok(ack) => {
                session.state.write().await.history.push(entry);
                Ok(ack)
            }
            Err(error) => {
                session.state.write().await.future.push(entry);
                Err(error)
            }
        }
    }
}

fn canonical_or_raw(value: &str) -> String {
    std::path::PathBuf::from(value)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(value))
        .to_string_lossy()
        .trim_end_matches(['/', '\\'])
        .to_ascii_lowercase()
}

#[derive(Clone, Copy)]
enum JournalMode {
    Record,
    Skip,
}

impl LiveUiSession {
    #[cfg(test)]
    pub(crate) async fn set_runtime_state_for_test(
        &self,
        nodes: Vec<LiveUiNode>,
        runtime_build_id: Option<String>,
    ) {
        let mut state = self.state.write().await;
        state.connected = true;
        state.runtime_build_id = runtime_build_id;
        state.tree_revision = state.tree_revision.saturating_add(1);
        state.nodes = nodes;
    }

    pub(crate) async fn reset_for_redeploy(&self) {
        *self.runtime_tx.write().await = None;
        self.pending.lock().await.clear();
        let mut state = self.state.write().await;
        state.connected = false;
        state.runtime_stage = Some("WAITING_FOR_RUNTIME_START".to_string());
        state.runtime_build_id = None;
        state.runtime_version = None;
        state.tree_revision = state.tree_revision.saturating_add(1);
        state.nodes.clear();
        state.history.clear();
        state.future.clear();
        state.source_proof = None;
        state.last_error = None;
    }

    pub(crate) async fn record_runtime_stage(&self, stage: &'static str) {
        self.state.write().await.runtime_stage = Some(stage.to_string());
    }

    pub(crate) async fn runtime_stage(&self) -> Option<String> {
        self.state.read().await.runtime_stage.clone()
    }

    pub(crate) async fn record_source_proof(&self, proof: LiveSourceProofView) {
        self.state.write().await.source_proof = Some(proof);
    }

    pub(crate) async fn commit_snapshot(&self) -> LiveCommitSnapshot {
        let state = self.state.read().await;
        LiveCommitSnapshot {
            project_root: self.project_root.clone(),
            nodes: state.nodes.clone(),
            patches: state
                .history
                .iter()
                .map(|entry| entry.forward.clone())
                .collect(),
        }
    }

    pub(crate) async fn view(&self) -> LiveSessionView {
        let state = self.state.read().await;
        LiveSessionView {
            id: self.id.clone(),
            device_id: self.device_id.clone(),
            package_name: self.package_name.clone(),
            project_root: self.project_root.clone(),
            device_port: self.device_port,
            created_at: self.created_at.clone(),
            connected: state.connected,
            runtime_build_id: state.runtime_build_id.clone(),
            runtime_version: state.runtime_version.clone(),
            tree_revision: state.tree_revision,
            node_count: state.nodes.len(),
            history_count: state.history.len(),
            redo_count: state.future.len(),
            source_proof: state.source_proof.clone(),
            last_seen_at: state.last_seen_at.clone(),
            last_error: state.last_error.clone(),
        }
    }

    async fn handle_runtime_text(&self, text: &str) -> Result<()> {
        let value: Value = serde_json::from_str(text)?;
        let message_type = value
            .get("messageType")
            .and_then(Value::as_str)
            .unwrap_or_default();
        match message_type {
            "runtime.hello" => {
                let protocol = value
                    .get("protocolVersion")
                    .and_then(Value::as_u64)
                    .unwrap_or_default() as u32;
                if protocol != PROTOCOL_VERSION {
                    bail!("Runtime 协议版本不兼容: {protocol}");
                }
                let mut state = self.state.write().await;
                state.runtime_build_id = value
                    .get("appBuildId")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                state.runtime_version = value
                    .get("runtimeVersion")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                state.runtime_stage = Some("RUNTIME_HELLO_RECEIVED".to_string());
                state.last_seen_at = Some(Utc::now().to_rfc3339());
            }
            "tree.snapshot" => {
                let nodes: Vec<LiveUiNode> = serde_json::from_value(
                    value.get("nodes").cloned().unwrap_or_else(|| json!([])),
                )?;
                let mut state = self.state.write().await;
                state.tree_revision = value
                    .get("treeRevision")
                    .and_then(Value::as_u64)
                    .unwrap_or(state.tree_revision + 1);
                state.nodes = nodes;
                state.runtime_stage = Some("TREE_SNAPSHOT_RECEIVED".to_string());
                state.last_seen_at = Some(Utc::now().to_rfc3339());
            }
            "patch.ack" | "patch.reject" | "frame.snapshot" | "frame.reject" | "icon.snapshot"
            | "icon.reject" => {
                let request_id = value
                    .get("requestId")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if let Some(waiter) = self.pending.lock().await.remove(request_id) {
                    let _ = waiter.send(value);
                }
            }
            "runtime.heartbeat" => {
                self.state.write().await.last_seen_at = Some(Utc::now().to_rfc3339());
            }
            "runtime.error" => {
                self.set_error(
                    value
                        .get("error")
                        .and_then(Value::as_str)
                        .unwrap_or("Android Runtime 未知错误")
                        .to_string(),
                )
                .await;
            }
            _ => bail!("未知 Runtime 消息: {message_type}"),
        }
        Ok(())
    }

    async fn send_patch(&self, patch: LiveStylePatch, journal: JournalMode) -> Result<Value> {
        let tx =
            self.runtime_tx.read().await.clone().ok_or_else(|| {
                anyhow!("Android Live Runtime 尚未连接，请确认安装并打开 Debug APK")
            })?;
        let (waiter_tx, waiter_rx) = oneshot::channel();
        self.pending
            .lock()
            .await
            .insert(patch.request_id.clone(), waiter_tx);
        tx.send(Message::Text(serde_json::to_string(&patch)?))
            .map_err(|_| anyhow!("Android Live Runtime 连接已断开"))?;
        let ack = match tokio::time::timeout(PATCH_ACK_TIMEOUT, waiter_rx).await {
            Ok(Ok(ack)) => ack,
            Ok(Err(_)) => {
                self.pending.lock().await.remove(&patch.request_id);
                bail!("Android Patch ACK 通道已关闭");
            }
            Err(_) => {
                self.pending.lock().await.remove(&patch.request_id);
                bail!("等待 Android Patch ACK 超时");
            }
        };
        if ack.get("messageType").and_then(Value::as_str) == Some("patch.reject") {
            bail!(
                "Android 拒绝 Patch: {}",
                ack.get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("未知错误")
            );
        }
        if matches!(journal, JournalMode::Record) {
            if let Some(inverse) = inverse_patch(&patch, &ack) {
                let mut state = self.state.write().await;
                if let Some(last) = state
                    .history
                    .last_mut()
                    .filter(|last| patches_share_gesture(&last.forward, &patch))
                {
                    last.forward = patch;
                } else {
                    state.history.push(PatchJournalEntry {
                        forward: patch,
                        inverse,
                    });
                }
                if state.history.len() > MAX_HISTORY {
                    state.history.remove(0);
                }
                state.future.clear();
                state.source_proof = None;
            }
        }
        Ok(ack)
    }

    pub(crate) async fn request_frame(&self) -> Result<Value> {
        let first_tx = self
            .runtime_tx
            .read()
            .await
            .clone()
            .ok_or_else(|| anyhow!("Android Live Runtime 尚未连接，无法获取进程内真实帧"))?;
        match self.request_frame_on(first_tx.clone()).await {
            Ok(frame) => Ok(frame),
            Err(first_error) => {
                // A production tree can be large enough that the Runtime socket is
                // replaced while encoding a frame.  Keep the immutable session
                // identity and retry once, but only on a genuinely new connection.
                let replacement = tokio::time::timeout(Duration::from_secs(4), async {
                    loop {
                        if let Some(tx) = self.runtime_tx.read().await.clone() {
                            if !tx.same_channel(&first_tx) {
                                break tx;
                            }
                        }
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                })
                .await
                .map_err(|_| first_error)?;
                self.request_frame_on(replacement).await
            }
        }
    }

    async fn request_frame_on(&self, tx: mpsc::UnboundedSender<Message>) -> Result<Value> {
        let request_id = format!("frame_{}", uuid::Uuid::new_v4().simple());
        let (waiter_tx, waiter_rx) = oneshot::channel();
        self.pending
            .lock()
            .await
            .insert(request_id.clone(), waiter_tx);
        tx.send(Message::Text(serde_json::to_string(&json!({
            "protocolVersion": PROTOCOL_VERSION,
            "messageType": "frame.request",
            "requestId": request_id,
            "quality": 72,
        }))?))
        .map_err(|_| anyhow!("Android Live Runtime 连接已断开"))?;
        let frame = match tokio::time::timeout(FRAME_ACK_TIMEOUT, waiter_rx).await {
            Ok(Ok(frame)) => frame,
            Ok(Err(_)) => {
                self.pending.lock().await.remove(&request_id);
                bail!("Android 真实帧通道已关闭");
            }
            Err(_) => {
                self.pending.lock().await.remove(&request_id);
                bail!("等待 Android 真实帧超时");
            }
        };
        if frame.get("messageType").and_then(Value::as_str) == Some("frame.reject") {
            bail!(
                "Android 拒绝真实帧请求: {}",
                frame
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("未知错误")
            );
        }
        Ok(frame)
    }

    async fn set_error(&self, error: String) {
        let mut state = self.state.write().await;
        state.last_error = Some(error);
        state.last_seen_at = Some(Utc::now().to_rfc3339());
    }
}

pub(super) fn patches_share_gesture(left: &LiveStylePatch, right: &LiveStylePatch) -> bool {
    let Some(left_gesture) = left.gesture_id.as_deref() else {
        return false;
    };
    let Some(right_gesture) = right.gesture_id.as_deref() else {
        return false;
    };
    left_gesture == right_gesture
        && left.target.scope == right.target.scope
        && left.target.runtime_node_id == right.target.runtime_node_id
        && left.target.definition_id == right.target.definition_id
        && left.target.instance_key == right.target.instance_key
        && left
            .operations
            .iter()
            .map(|operation| operation.property.as_str())
            .eq(right
                .operations
                .iter()
                .map(|operation| operation.property.as_str()))
}

fn inverse_patch(patch: &LiveStylePatch, ack: &Value) -> Option<LiveStylePatch> {
    let before = ack.get("beforeValues")?.as_object()?;
    let operations = patch
        .operations
        .iter()
        .filter_map(|operation| {
            before
                .get(&operation.property)
                .cloned()
                .and_then(|value| serde_json::from_value::<LivePropertyValue>(value).ok())
                .map(|value| LivePatchOperation {
                    property: operation.property.clone(),
                    value,
                })
        })
        .collect::<Vec<_>>();
    if operations.is_empty() {
        return None;
    }
    let mut inverse = patch.clone();
    inverse.request_id = String::new();
    inverse.gesture_id = patch.gesture_id.as_ref().map(|id| format!("undo:{id}"));
    inverse.operations = operations;
    inverse.prepare(&patch.session_id);
    Some(inverse)
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |diff, (a, b)| diff | (a ^ b))
        == 0
}

mod session_factory;
#[cfg(test)]
mod tests;

#[path = "broker/runtime_icon.rs"]
mod runtime_icon;

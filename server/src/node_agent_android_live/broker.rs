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
    LivePatchOperation, LivePropertyValue, LiveSessionView, LiveStylePatch, LiveUiNode,
    RuntimeWelcome, PROTOCOL_VERSION,
};

const PATCH_ACK_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_HISTORY: usize = 100;

#[derive(Default)]
pub(crate) struct LiveUiBroker {
    sessions: RwLock<HashMap<String, Arc<LiveUiSession>>>,
}

pub(crate) struct LiveUiSession {
    pub(crate) id: String,
    pub(crate) token: String,
    pub(crate) device_id: String,
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
    runtime_build_id: Option<String>,
    runtime_version: Option<String>,
    tree_revision: u64,
    nodes: Vec<LiveUiNode>,
    history: Vec<PatchJournalEntry>,
    future: Vec<PatchJournalEntry>,
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

    pub(crate) async fn create_session(
        &self,
        device_id: String,
        package_name: String,
        project_root: Option<String>,
        device_port: u16,
    ) -> Arc<LiveUiSession> {
        let session = Arc::new(LiveUiSession {
            id: format!("live_{}", uuid::Uuid::new_v4().simple()),
            token: uuid::Uuid::new_v4().simple().to_string(),
            device_id,
            package_name,
            project_root,
            device_port,
            created_at: Utc::now().to_rfc3339(),
            state: RwLock::new(LiveSessionState::default()),
            runtime_tx: RwLock::new(None),
            pending: Mutex::new(HashMap::new()),
        });
        self.sessions
            .write()
            .await
            .insert(session.id.clone(), session.clone());
        session
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

    pub(crate) async fn session_view(&self, session_id: &str) -> Result<LiveSessionView> {
        Ok(self.session(session_id).await?.view().await)
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
            bail!("Live UI 会话令牌无效");
        }
        let (mut sink, mut stream) = socket.split();
        let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
        *session.runtime_tx.write().await = Some(tx.clone());
        {
            let mut state = session.state.write().await;
            state.connected = true;
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
        *session.runtime_tx.write().await = None;
        session.state.write().await.connected = false;
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

#[derive(Clone, Copy)]
enum JournalMode {
    Record,
    Skip,
}

impl LiveUiSession {
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
                state.last_seen_at = Some(Utc::now().to_rfc3339());
            }
            "patch.ack" | "patch.reject" => {
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
                state.history.push(PatchJournalEntry {
                    forward: patch,
                    inverse,
                });
                if state.history.len() > MAX_HISTORY {
                    state.history.remove(0);
                }
                state.future.clear();
            }
        }
        Ok(ack)
    }

    async fn set_error(&self, error: String) {
        let mut state = self.state.write().await;
        state.last_error = Some(error);
        state.last_seen_at = Some(Utc::now().to_rfc3339());
    }
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

use super::{security, CaptureDiagnostic};
use futures::{SinkExt, StreamExt};
use reqwest::Url;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    time::{Duration, Instant},
};
use tokio::{net::TcpStream, time::sleep};
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

pub(super) type CdpSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub(super) struct CdpClient {
    pub(super) socket: CdpSocket,
    next_id: u64,
    ignored_response_ids: HashSet<u64>,
    pub(super) events: Vec<Value>,
    allowed_origins: BTreeSet<String>,
    target_origin: String,
    auth_headers: BTreeMap<String, String>,
    pub(super) blocked_request_count: u32,
}

impl CdpClient {
    pub(super) fn new(
        socket: CdpSocket,
        allowed_origins: BTreeSet<String>,
        target_origin: String,
        auth_headers: BTreeMap<String, String>,
    ) -> Self {
        Self {
            socket,
            next_id: 1,
            ignored_response_ids: HashSet::new(),
            events: Vec::new(),
            allowed_origins,
            target_origin,
            auth_headers,
            blocked_request_count: 0,
        }
    }

    pub(super) async fn command(
        &mut self,
        method: &str,
        params: Value,
        session: Option<&str>,
        deadline: Instant,
    ) -> Result<Value, CaptureDiagnostic> {
        let id = self.next_id;
        self.next_id += 1;
        let mut request = json!({"id":id,"method":method,"params":params});
        if let Some(session) = session {
            request["sessionId"] = json!(session);
        }
        self.send(request, deadline).await?;
        loop {
            let message = tokio::time::timeout(remaining(deadline), self.socket.next())
                .await
                .map_err(|_| protocol_timeout())?
                .ok_or_else(|| protocol_error("浏览器 CDP 连接提前关闭"))?
                .map_err(|_| protocol_error("浏览器 CDP 消息读取失败"))?;
            let Message::Text(text) = message else {
                if let Message::Ping(payload) = message {
                    let _ = self.socket.send(Message::Pong(payload)).await;
                }
                continue;
            };
            let value: Value = serde_json::from_str(&text)
                .map_err(|_| protocol_error("浏览器 CDP 返回无效 JSON"))?;
            if value.get("method").and_then(Value::as_str) == Some("Fetch.requestPaused") {
                self.handle_paused_request(&value, deadline).await?;
                continue;
            }
            if let Some(response_id) = value.get("id").and_then(Value::as_u64) {
                if self.ignored_response_ids.remove(&response_id) {
                    continue;
                }
                if response_id == id {
                    if let Some(error) = value.get("error") {
                        let code = error.get("code").and_then(Value::as_i64).unwrap_or(0);
                        return Err(protocol_error(&format!(
                            "浏览器拒绝 CDP 命令 {method}（code={code}）"
                        )));
                    }
                    return Ok(value.get("result").cloned().unwrap_or_else(|| json!({})));
                }
            } else {
                self.events.push(value);
            }
        }
    }

    async fn handle_paused_request(
        &mut self,
        event: &Value,
        deadline: Instant,
    ) -> Result<(), CaptureDiagnostic> {
        let request_id = event
            .pointer("/params/requestId")
            .and_then(Value::as_str)
            .ok_or_else(|| protocol_error("Fetch.requestPaused 缺少 requestId"))?;
        let session = event.get("sessionId").and_then(Value::as_str);
        let raw_url = event
            .pointer("/params/request/url")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let request_origin = Url::parse(raw_url)
            .ok()
            .and_then(|url| security::origin(&url).ok());
        let allowed = request_origin
            .as_ref()
            .is_some_and(|origin| self.allowed_origins.contains(origin));
        let (method, params) = if allowed {
            let mut params = json!({"requestId":request_id});
            if request_origin.as_deref() == Some(self.target_origin.as_str())
                && !self.auth_headers.is_empty()
            {
                let mut headers = event
                    .pointer("/params/request/headers")
                    .and_then(Value::as_object)
                    .map(|headers| {
                        headers
                            .iter()
                            .map(|(name, value)| {
                                json!({"name":name,"value":value.as_str().unwrap_or_default()})
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                headers.extend(
                    self.auth_headers
                        .iter()
                        .map(|(name, value)| json!({"name":name,"value":value})),
                );
                params["headers"] = Value::Array(headers);
            }
            ("Fetch.continueRequest", params)
        } else {
            self.blocked_request_count = self.blocked_request_count.saturating_add(1);
            (
                "Fetch.failRequest",
                json!({"requestId":request_id,"errorReason":"BlockedByClient"}),
            )
        };
        let id = self.next_id;
        self.next_id += 1;
        self.ignored_response_ids.insert(id);
        let mut request = json!({"id":id,"method":method,"params":params});
        if let Some(session) = session {
            request["sessionId"] = json!(session);
        }
        self.send(request, deadline).await
    }

    async fn send(&mut self, value: Value, deadline: Instant) -> Result<(), CaptureDiagnostic> {
        tokio::time::timeout(
            remaining(deadline),
            self.socket.send(Message::Text(value.to_string())),
        )
        .await
        .map_err(|_| protocol_timeout())?
        .map_err(|_| protocol_error("浏览器 CDP 消息发送失败"))
    }
}

pub(super) async fn short_pause() {
    sleep(Duration::from_millis(100)).await;
}

pub(super) fn safe_browser_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .chars()
        .take(500)
        .collect()
}

pub(super) fn number(value: &Value, key: &str) -> Result<f64, CaptureDiagnostic> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| protocol_error("浏览器几何响应缺少数值"))
}

pub(super) struct NetworkState {
    pub(super) inflight: HashSet<String>,
    pub(super) document_status: Option<u16>,
    pub(super) last_activity: Instant,
}

impl NetworkState {
    pub(super) fn new() -> Self {
        Self {
            inflight: HashSet::new(),
            document_status: None,
            last_activity: Instant::now(),
        }
    }

    pub(super) fn consume(&mut self, events: &mut Vec<Value>) {
        for event in events.drain(..) {
            match event.get("method").and_then(Value::as_str) {
                Some("Network.requestWillBeSent") => {
                    if let Some(id) = event.pointer("/params/requestId").and_then(Value::as_str) {
                        self.inflight.insert(id.to_string());
                    }
                    self.last_activity = Instant::now();
                }
                Some("Network.loadingFinished" | "Network.loadingFailed") => {
                    if let Some(id) = event.pointer("/params/requestId").and_then(Value::as_str) {
                        self.inflight.remove(id);
                    }
                    self.last_activity = Instant::now();
                }
                Some("Network.responseReceived") => {
                    if event.pointer("/params/type").and_then(Value::as_str) == Some("Document") {
                        self.document_status = event
                            .pointer("/params/response/status")
                            .and_then(Value::as_f64)
                            .map(|value| value as u16);
                    }
                    self.last_activity = Instant::now();
                }
                _ => {}
            }
        }
    }
}

fn protocol_error(message: &str) -> CaptureDiagnostic {
    CaptureDiagnostic::new(
        "BROWSER_PROTOCOL_ERROR",
        message,
        true,
        "重启 Windows 节点或升级本机 Edge/Chrome 后重试",
    )
}

fn protocol_timeout() -> CaptureDiagnostic {
    CaptureDiagnostic::new(
        "BROWSER_PROTOCOL_TIMEOUT",
        "等待浏览器 CDP 响应超时",
        true,
        "确认浏览器未被安全软件阻塞后重试",
    )
}

fn remaining(deadline: Instant) -> Duration {
    deadline
        .saturating_duration_since(Instant::now())
        .max(Duration::from_millis(1))
}

//! P2P 同WiFi APK 中继模块
//!
//! 工作流：
//!   1. Seeder（已安装 APP 的手机）连接 WS /app/peer/ws?version_code=N
//!   2. 服务器分配 peer_id，注册到 peer_registry
//!   3. 下载方 GET /app/relay/peer/:peer_id/apk
//!   4. 中继 handler 通过 mpsc channel 向 WS handler 发送 PeerRequest
//!   5. WS handler 向 seeder 发送 "SEND_APK" 指令
//!   6. Seeder 以 Binary WS 消息流式传输 APK，最后发一条 Text "DONE"
//!   7. WS handler 收集所有 chunk，通过 oneshot 把完整 Vec<u8> 交给中继 handler
//!   8. 中继 handler 以 HTTP Response 返回给下载方
//!
//! /app/version.json 也在此模块动态生成，注入在线 seeder 的 mirrors 字段。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, Query, State, WebSocketUpgrade};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use futures::{Sink, SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::{mpsc, oneshot};

use crate::{
    realtime_metrics::{self, RealtimeChannel},
    types::{AppState, PeerEntry, PeerRequest},
    ws_transport::text_message,
};

/// 全局单调递增 ID（多 seeder 并发注册时无需锁）
static PEER_COUNTER: AtomicU64 = AtomicU64::new(1);

// ─── WS 注册入口 ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct PeerWsQuery {
    pub version_code: Option<i64>,
}

#[derive(Debug)]
enum PeerWsEvent {
    Text(String),
    Binary(Vec<u8>),
    Ping(Vec<u8>),
    Pong,
    Close,
    ReadError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PeerWsCloseReason {
    RequestChannelClosed,
    PeerClosed,
    PeerReadError,
    PeerReaderEnded,
    PeerWriteError,
}

impl PeerWsCloseReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::RequestChannelClosed => "request_channel_closed",
            Self::PeerClosed => "peer_closed",
            Self::PeerReadError => "peer_read_error",
            Self::PeerReaderEnded => "peer_reader_ended",
            Self::PeerWriteError => "peer_write_error",
        }
    }

    fn transfer_failure_message(self) -> &'static str {
        match self {
            Self::PeerWriteError => "种子节点发送指令失败",
            Self::PeerReadError => "种子节点连接读取失败",
            Self::PeerClosed => "种子节点已断开",
            Self::PeerReaderEnded | Self::RequestChannelClosed => "传输中断",
        }
    }
}

async fn collect_peer_transfer<S>(
    msg_rx: &mut mpsc::Receiver<PeerWsEvent>,
    ws_tx: &mut S,
) -> Result<Vec<u8>, PeerWsCloseReason>
where
    S: Sink<Message> + Unpin,
{
    let mut chunks: Vec<Vec<u8>> = Vec::new();
    loop {
        match msg_rx.recv().await {
            Some(PeerWsEvent::Binary(data)) => chunks.push(data),
            Some(PeerWsEvent::Text(text)) if text.trim() == "DONE" => {
                return Ok(chunks.into_iter().flatten().collect());
            }
            Some(PeerWsEvent::Ping(payload)) => {
                if ws_tx.send(Message::Pong(payload)).await.is_err() {
                    return Err(PeerWsCloseReason::PeerWriteError);
                }
            }
            Some(PeerWsEvent::Pong) => {}
            Some(PeerWsEvent::Text(_)) => {}
            Some(PeerWsEvent::Close) => return Err(PeerWsCloseReason::PeerClosed),
            Some(PeerWsEvent::ReadError) => return Err(PeerWsCloseReason::PeerReadError),
            None => return Err(PeerWsCloseReason::PeerReaderEnded),
        }
    }
}

async fn handle_idle_peer_event<S>(
    event: Option<PeerWsEvent>,
    ws_tx: &mut S,
) -> Result<(), PeerWsCloseReason>
where
    S: Sink<Message> + Unpin,
{
    match event {
        Some(PeerWsEvent::Ping(payload)) => {
            if ws_tx.send(Message::Pong(payload)).await.is_err() {
                Err(PeerWsCloseReason::PeerWriteError)
            } else {
                Ok(())
            }
        }
        Some(PeerWsEvent::Close) => Err(PeerWsCloseReason::PeerClosed),
        Some(PeerWsEvent::ReadError) => Err(PeerWsCloseReason::PeerReadError),
        None => Err(PeerWsCloseReason::PeerReaderEnded),
        Some(PeerWsEvent::Text(_)) | Some(PeerWsEvent::Binary(_)) | Some(PeerWsEvent::Pong) => {
            Ok(())
        }
    }
}

/// GET /app/peer/ws?version_code=N
/// Seeder 连接后注册为可用种子；服务器持续保持连接以便中继请求。
pub async fn peer_ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<PeerWsQuery>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_peer_ws(socket, state, params.version_code.unwrap_or(0)))
}

async fn handle_peer_ws(socket: WebSocket, state: Arc<AppState>, version_code: i64) {
    let peer_id = format!("p{:016x}", PEER_COUNTER.fetch_add(1, Ordering::Relaxed));
    let short = peer_id[..6].to_string(); // 日志前缀

    // 创建请求通道：中继 handler → 本 WS handler
    let (req_tx, mut req_rx) = mpsc::channel::<PeerRequest>(2);

    state.peer_registry.write().await.insert(
        peer_id.clone(),
        PeerEntry {
            version_code,
            tx: req_tx,
        },
    );
    tracing::info!("🌱 Seeder 注册: {} (versionCode={})", short, version_code);

    let (mut ws_tx, ws_rx) = socket.split();

    // 告知 seeder 注册成功及其 peer_id（可选，方便调试）
    if ws_tx
        .send(text_message(format!(
            r#"{{"type":"registered","peer_id":"{}"}}"#,
            peer_id
        )))
        .await
        .is_err()
    {
        realtime_metrics::record_close_with_store(
            &state.store,
            RealtimeChannel::PeerRelay,
            PeerWsCloseReason::PeerWriteError.as_str(),
        );
        state.peer_registry.write().await.remove(&peer_id);
        tracing::info!(
            peer_id = %short,
            close_reason = PeerWsCloseReason::PeerWriteError.as_str(),
            "Seeder 断开"
        );
        return;
    }

    // 把 WS 收到的消息转发到独立 channel，避免 select! 和内层循环同时持有 ws_rx
    let (msg_tx, mut msg_rx) = mpsc::channel::<PeerWsEvent>(64);
    let reader_short = short.clone();
    tokio::spawn(async move {
        let mut ws_rx = ws_rx;
        while let Some(frame) = ws_rx.next().await {
            let event = match frame {
                Ok(Message::Text(text)) => PeerWsEvent::Text(text.to_string()),
                Ok(Message::Binary(bytes)) => PeerWsEvent::Binary(bytes.to_vec()),
                Ok(Message::Ping(payload)) => PeerWsEvent::Ping(payload),
                Ok(Message::Pong(_)) => PeerWsEvent::Pong,
                Ok(Message::Close(_)) => {
                    let _ = msg_tx.send(PeerWsEvent::Close).await;
                    break;
                }
                Err(e) => {
                    tracing::warn!(peer_id = %reader_short, error = %e, "Seeder WS read error");
                    let _ = msg_tx.send(PeerWsEvent::ReadError).await;
                    break;
                }
            };
            if msg_tx.send(event).await.is_err() {
                break;
            }
        }
    });

    let close_reason = loop {
        tokio::select! {
            // ── 收到来自中继 handler 的传输请求 ──────────────────────────────
            maybe_req = req_rx.recv() => {
                let Some(req) = maybe_req else {
                    break PeerWsCloseReason::RequestChannelClosed;
                };

                // 指令 seeder 开始发送 APK
                if ws_tx.send(text_message("SEND_APK")).await.is_err() {
                    let reason = PeerWsCloseReason::PeerWriteError;
                    let _ = req.response_tx.send(Err(reason.transfer_failure_message().into()));
                    break reason;
                }

                // 收集 seeder 发来的二进制 chunk，直到 "DONE" 或连接断开
                match collect_peer_transfer(&mut msg_rx, &mut ws_tx).await {
                    Ok(data) => {
                    tracing::info!(
                        "✅ Seeder {} 传输完成 ({} KB)",
                        short,
                        data.len() / 1024
                    );
                    let _ = req.response_tx.send(Ok(data));
                    }
                    Err(reason) => {
                        let _ = req.response_tx.send(Err(reason.transfer_failure_message().into()));
                        break reason;
                    }
                }
            }

            // ── WS 主动断开或心跳 ────────────────────────────────────────────
            msg = msg_rx.recv() => {
                if let Err(reason) = handle_idle_peer_event(msg, &mut ws_tx).await {
                    break reason;
                }
            }
        }
    };

    let _ = ws_tx.close().await;
    state.peer_registry.write().await.remove(&peer_id);
    let close_reason_name = close_reason.as_str();
    realtime_metrics::record_close_with_store(
        &state.store,
        RealtimeChannel::PeerRelay,
        close_reason_name,
    );
    tracing::info!(
        peer_id = %short,
        close_reason = close_reason_name,
        "Seeder 断开"
    );
}

// ─── APK 中继下载 ─────────────────────────────────────────────────────────────

/// GET /app/relay/peer/:peer_id/apk
/// 下载方调用此接口，服务器向对应 seeder 请求 APK 并中继给下载方。
pub async fn relay_apk(
    Path(peer_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // 查找 seeder 发送通道
    let tx = {
        state
            .peer_registry
            .read()
            .await
            .get(&peer_id)
            .map(|e| e.tx.clone())
    };

    let Some(tx) = tx else {
        return (StatusCode::NOT_FOUND, "种子节点不在线").into_response();
    };

    let (resp_tx, resp_rx) = oneshot::channel();
    if tx
        .send(PeerRequest {
            response_tx: resp_tx,
        })
        .await
        .is_err()
    {
        return (StatusCode::SERVICE_UNAVAILABLE, "种子节点忙").into_response();
    }

    // 等待 seeder 传输（最多 15 秒，避免半死 peer 长时间挂住下载方）
    match tokio::time::timeout(std::time::Duration::from_secs(15), resp_rx).await {
        Ok(Ok(Ok(data))) => {
            let mut headers = axum::http::HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                "application/vnd.android.package-archive".parse().unwrap(),
            );
            headers.insert(
                header::CONTENT_DISPOSITION,
                r#"attachment; filename="ElonSpeed-latest.apk""#.parse().unwrap(),
            );
            headers.insert(
                header::CONTENT_LENGTH,
                data.len().to_string().parse().unwrap(),
            );
            (StatusCode::OK, headers, data).into_response()
        }
        Ok(Ok(Err(e))) => (StatusCode::BAD_GATEWAY, e).into_response(),
        Ok(Err(_)) => (StatusCode::BAD_GATEWAY, "种子节点已断开").into_response(),
        Err(_) => (StatusCode::GATEWAY_TIMEOUT, "等待种子节点超时").into_response(),
    }
}

// ─── 动态 version.json ────────────────────────────────────────────────────────

/// GET /app/version.json
/// 读取磁盘上的 version.json，动态注入在线 seeder 的 mirrors 字段。
pub async fn version_json(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let path = state.data_dir.join("app").join("version.json");

    let content = match tokio::fs::read_to_string(&path).await {
        Ok(s) => s,
        Err(_) => {
            return (StatusCode::NOT_FOUND, "version.json 不存在").into_response();
        }
    };

    // 去除 UTF-8 BOM（PowerShell Set-Content 可能写入 EF BB BF 前缀）
    let content = content.trim_start_matches('\u{FEFF}').to_owned();

    let mut json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "version.json 解析失败").into_response();
        }
    };

    let public_url = state.public_url.trim_end_matches('/');
    json["downloadUrl"] =
        serde_json::Value::String(format!("{public_url}/app/ElonSpeed-latest.apk"));
    json["downloadPageUrl"] = serde_json::Value::String(format!("{public_url}/app/download"));

    // 注入 mirrors：优先注入 LAN PC 种子（priority=10），再注入手机P2P中继（priority=5）
    let current_vc = json
        .get("versionCode")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    // LAN PC 直连 mirrors（开发电脑，局域网下载最快）
    let mut all_mirrors: Vec<serde_json::Value> =
        crate::lan_peer::get_active_lan_mirrors(&state, current_vc).await;

    // 手机P2P 中继 mirrors
    let reg = state.peer_registry.read().await;
    if !reg.is_empty() {
        let phone_mirrors: Vec<serde_json::Value> = reg
            .iter()
            .filter(|(_, e)| e.version_code >= current_vc)
            .map(|(id, _)| {
                serde_json::json!({
                    "url": format!("{}/app/relay/peer/{}/apk", state.public_url, id),
                    "type": "wifi",
                    "priority": 5
                })
            })
            .collect();
        all_mirrors.extend(phone_mirrors);
    }
    drop(reg);

    if !all_mirrors.is_empty() {
        json["mirrors"] = serde_json::Value::Array(all_mirrors);
    }

    let body = serde_json::to_string(&json).unwrap_or(content);
    (
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        body,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{channel::mpsc as futures_mpsc, StreamExt};

    #[test]
    fn peer_ws_close_reason_has_stable_diagnostics() {
        let cases = [
            (
                PeerWsCloseReason::RequestChannelClosed,
                "request_channel_closed",
                "传输中断",
            ),
            (
                PeerWsCloseReason::PeerClosed,
                "peer_closed",
                "种子节点已断开",
            ),
            (
                PeerWsCloseReason::PeerReadError,
                "peer_read_error",
                "种子节点连接读取失败",
            ),
            (
                PeerWsCloseReason::PeerReaderEnded,
                "peer_reader_ended",
                "传输中断",
            ),
            (
                PeerWsCloseReason::PeerWriteError,
                "peer_write_error",
                "种子节点发送指令失败",
            ),
        ];

        for (reason, expected_name, expected_message) in cases {
            assert_eq!(reason.as_str(), expected_name);
            assert_eq!(reason.transfer_failure_message(), expected_message);
        }
    }

    #[tokio::test]
    async fn collect_peer_transfer_concatenates_binary_until_done() {
        let (event_tx, mut event_rx) = mpsc::channel(4);
        event_tx
            .send(PeerWsEvent::Binary(vec![1, 2, 3]))
            .await
            .unwrap();
        event_tx
            .send(PeerWsEvent::Binary(vec![4, 5]))
            .await
            .unwrap();
        event_tx
            .send(PeerWsEvent::Text("DONE".to_string()))
            .await
            .unwrap();
        drop(event_tx);
        let (mut sink_tx, _sink_rx) = futures_mpsc::channel::<Message>(4);

        let data = collect_peer_transfer(&mut event_rx, &mut sink_tx)
            .await
            .unwrap();

        assert_eq!(data, vec![1, 2, 3, 4, 5]);
    }

    #[tokio::test]
    async fn idle_peer_event_replies_to_ping() {
        let (mut sink_tx, mut sink_rx) = futures_mpsc::channel::<Message>(4);

        handle_idle_peer_event(Some(PeerWsEvent::Ping(b"nonce".to_vec())), &mut sink_tx)
            .await
            .unwrap();

        match sink_rx.next().await {
            Some(Message::Pong(payload)) => assert_eq!(payload, b"nonce".to_vec()),
            other => panic!("expected pong, got {other:?}"),
        }
    }
}

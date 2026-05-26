//! 全局通知 WS 通道（/ws/app）
//!
//! 统一实时推送接口，取代分散的 /ws/notify。
//! 所有消息通过 `type` 字段区分，Android 端单条连接即可接收全部类型事件。
//!
//! 当前支持：
//!   - `app_update_available`：有新版本 APK 可安装
//!
//! 未来可扩展：
//!   - `friend_message`：好友消息（接入好友 TX 后自动生效）
//!   - `notification`：系统通知
//!   - `presence`：在线状态变更
//!
//! 连接参数（均可选）：
//!   - `token`：JWT 认证令牌，已登录用户传入以便接收个人事件
//!   - `version_code`：客户端当前版本号，连接后如有更新会立即推送

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::Response,
};
use futures::{SinkExt, StreamExt};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::broadcast::error::RecvError;

use crate::types::AppState;

pub async fn global_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let client_version_code = query
        .get("version_code")
        .and_then(|v| v.parse::<i64>().ok());
    // `token` 预留给未来好友消息等需要身份的事件
    // let _token = query.get("token").cloned();
    ws.on_upgrade(move |socket| handle(socket, state, client_version_code))
}

async fn handle(socket: WebSocket, state: Arc<AppState>, client_version_code: Option<i64>) {
    let (mut tx, mut rx) = socket.split();
    let mut update_rx = crate::app_update::subscribe();

    // 连接时若服务器已有更新版本，立即推送一次，无需等待下次广播
    if let Some(event) =
        crate::app_update::latest_update_event_for_client(&state, client_version_code).await
    {
        if tx.send(Message::Text(event)).await.is_err() {
            return;
        }
    }

    loop {
        tokio::select! {
            msg = update_rx.recv() => {
                match msg {
                    Ok(event) if crate::app_update::is_newer_for_client(&event, client_version_code) => {
                        if tx.send(Message::Text(event)).await.is_err() { break; }
                    }
                    Err(RecvError::Lagged(_)) => { /* 跳过积压消息，等下一条 */ }
                    _ => {}
                }
            }
            incoming = rx.next() => {
                match incoming {
                    Some(Ok(Message::Ping(p))) => {
                        if tx.send(Message::Pong(p)).await.is_err() { break; }
                    }
                    Some(Ok(_)) => {} // 目前忽略客户端发来的文本/二进制
                    _ => break,      // 连接关闭或错误
                }
            }
        }
    }
}

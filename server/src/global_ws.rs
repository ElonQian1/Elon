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
    let authenticated_user_id = query
        .get("token")
        .and_then(|token| state.store.authenticate_token(token).ok())
        .map(|user| user.id);
    ws.on_upgrade(move |socket| handle(socket, state, client_version_code, authenticated_user_id))
}

async fn handle(
    socket: WebSocket,
    state: Arc<AppState>,
    client_version_code: Option<i64>,
    authenticated_user_id: Option<String>,
) {
    let (mut tx, mut rx) = socket.split();
    let mut update_rx = crate::app_update::subscribe();
    let mut friend_rx = crate::friend_events::subscribe();
    let mut group_rx = crate::friend_events::subscribe_groups();
    let mut project_task_rx = crate::project_events::subscribe();
    let mut project_ai_rx = crate::project_events::subscribe_group_ai();
    let mut project_message_rx = crate::project_events::subscribe_message_updated();
    let mut presence_rx = crate::presence_events::subscribe();
    let mut typing_rx = crate::typing_events::subscribe();
    let mut billing_rx = crate::billing_events::subscribe();
    let mut read_receipt_rx = crate::read_receipt_events::subscribe();
    let mut join_req_rx = crate::join_request_events::subscribe();

    // 认证用户上线：注册在线状态并广播给所有已连接用户
    if let Some(ref uid) = authenticated_user_id {
        mark_online(&state, uid).await;
    }

    // 连接时若服务器已有更新版本，立即推送一次，无需等待下次广播
    if let Some(event) =
        crate::app_update::latest_update_event_for_client(&state, client_version_code).await
    {
        if tx.send(Message::Text(event)).await.is_err() {
            // 连接前就断了，直接下线注销
            if let Some(ref uid) = authenticated_user_id {
                mark_offline(&state, uid).await;
            }
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
            msg = friend_rx.recv(), if authenticated_user_id.is_some() => {
                match msg {
                    Ok(event) if authenticated_user_id.as_deref() == Some(event.to_user_id.as_str()) => {
                        let Some(payload) = event.to_json() else { continue; };
                        if tx.send(Message::Text(payload)).await.is_err() { break; }
                    }
                    Err(RecvError::Lagged(_)) => { /* 下次列表刷新会补齐未读状态 */ }
                    _ => {}
                }
            }
            msg = group_rx.recv(), if authenticated_user_id.is_some() => {
                match msg {
                    Ok(event) if authenticated_user_id
                        .as_ref()
                        .is_some_and(|user_id| event.recipient_user_ids.iter().any(|id| id == user_id)) => {
                        let Some(payload) = event.to_json() else { continue; };
                        if tx.send(Message::Text(payload)).await.is_err() { break; }
                    }
                    Err(RecvError::Lagged(_)) => { /* 下次群列表刷新会补齐未读状态 */ }
                    _ => {}
                }
            }
            // 方案5: 项目任务完成——推给其他项目成员
            msg = project_task_rx.recv(), if authenticated_user_id.is_some() => {
                match msg {
                    Ok(event) if authenticated_user_id
                        .as_ref()
                        .is_some_and(|uid| event.member_user_ids.iter().any(|id| id == uid)) => {
                        let Some(payload) = event.to_json() else { continue; };
                        if tx.send(Message::Text(payload)).await.is_err() { break; }
                    }
                    Err(RecvError::Lagged(_)) => { /* 项目列表刷新时可服主务器查最新状态 */ }
                    _ => {}
                }
            }
            msg = project_ai_rx.recv(), if authenticated_user_id.is_some() => {
                match msg {
                    Ok(event) if authenticated_user_id
                        .as_ref()
                        .is_some_and(|uid| event.member_user_ids.iter().any(|id| id == uid)) => {
                        let Some(payload) = event.to_json() else { continue; };
                        if tx.send(Message::Text(payload)).await.is_err() { break; }
                    }
                    Err(RecvError::Lagged(_)) => {}
                    _ => {}
                }
            }
            msg = project_message_rx.recv(), if authenticated_user_id.is_some() => {
                match msg {
                    Ok(event) if authenticated_user_id
                        .as_ref()
                        .is_some_and(|uid| event.member_user_ids.iter().any(|id| id == uid)) => {
                        let Some(payload) = event.to_json() else { continue; };
                        if tx.send(Message::Text(payload)).await.is_err() { break; }
                    }
                    Err(RecvError::Lagged(_)) => {}
                    _ => {}
                }
            }
            // 在线状态变更——推给所有已认证连接（客户端按好友关系过滤）
            msg = presence_rx.recv(), if authenticated_user_id.is_some() => {
                match msg {
                    Ok(event) => {
                        // 不把自己的上线事件推回给自己
                        if authenticated_user_id.as_deref() != Some(event.user_id.as_str()) {
                            let Some(payload) = event.to_json() else { continue; };
                            if tx.send(Message::Text(payload)).await.is_err() { break; }
                        }
                    }
                    Err(RecvError::Lagged(_)) => { /* 下次列表刷新可以重新获取在线状态 */ }
                    _ => {}
                }
            }
            // typing 事件 —— 只推给目标用户的连接
            msg = typing_rx.recv(), if authenticated_user_id.is_some() => {
                match msg {
                    Ok(event) if authenticated_user_id.as_deref() == Some(event.to_user_id.as_str()) => {
                        let Some(payload) = event.to_json() else { continue; };
                        if tx.send(Message::Text(payload)).await.is_err() { break; }
                    }
                    Err(RecvError::Lagged(_)) => { /* 跳过积压，不影响体验 */ }
                    _ => {}
                }
            }
            // 余额低于阈值——只推给对应用户
            msg = billing_rx.recv(), if authenticated_user_id.is_some() => {
                match msg {
                    Ok(event) if authenticated_user_id.as_deref() == Some(event.user_id.as_str()) => {
                        let Some(payload) = event.to_json() else { continue; };
                        if tx.send(Message::Text(payload)).await.is_err() { break; }
                    }
                    Err(RecvError::Lagged(_)) => {}
                    _ => {}
                }
            }
            // 已读回执——只推给消息原发送方
            msg = read_receipt_rx.recv(), if authenticated_user_id.is_some() => {
                match msg {
                    Ok(event) if authenticated_user_id.as_deref() == Some(event.to_user_id.as_str()) => {
                        let Some(payload) = event.to_json() else { continue; };
                        if tx.send(Message::Text(payload)).await.is_err() { break; }
                    }
                    Err(RecvError::Lagged(_)) => { /* 跳过积压，客户端轮询时可重新获取 */ }
                    _ => {}
                }
            }
            // 加入申请事件——精确推给 target_user_id（owner 或申请人）
            msg = join_req_rx.recv(), if authenticated_user_id.is_some() => {
                match msg {
                    Ok(event) if authenticated_user_id.as_deref() == Some(event.target_user_id.as_str()) => {
                        let Some(payload) = event.to_json() else { continue; };
                        if tx.send(Message::Text(payload)).await.is_err() { break; }
                    }
                    Err(RecvError::Lagged(_)) => {}
                    _ => {}
                }
            }
            incoming = rx.next() => {
                match incoming {
                    Some(Ok(Message::Ping(p))) => {
                        if tx.send(Message::Pong(p)).await.is_err() { break; }
                    }
                    // 解析客户端发来的文本消息
                    Some(Ok(Message::Text(text))) => {
                        if let (Some(uid), Ok(json)) = (
                            authenticated_user_id.as_deref(),
                            serde_json::from_str::<serde_json::Value>(&text),
                        ) {
                            let msg_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");
                            if msg_type == "typing" {
                                if let Some(to_uid) = json.get("toUserId").and_then(|v| v.as_str()) {
                                    crate::typing_events::publish(uid.to_string(), to_uid.to_string());
                                }
                            }
                        }
                    }
                    Some(Ok(_)) => {} // 其余类型忽略
                    _ => break,      // 连接关闭或错误
                }
            }
        }
    }

    // 用户断线：注销在线状态
    if let Some(ref uid) = authenticated_user_id {
        mark_offline(&state, uid).await;
    }
}

async fn mark_online(state: &AppState, user_id: &str) {
    let mut online = state.online_users.write().await;
    let count = online.entry(user_id.to_string()).or_insert(0);
    *count += 1;
    if *count == 1 {
        crate::presence_events::publish_online(user_id.to_string());
    }
}

async fn mark_offline(state: &AppState, user_id: &str) {
    let mut online = state.online_users.write().await;
    let Some(count) = online.get_mut(user_id) else {
        return;
    };
    if *count > 1 {
        *count -= 1;
        return;
    }
    online.remove(user_id);
    crate::presence_events::publish_offline(user_id.to_string());
}

//! 方案 A：`/ws/voice/virtual-mic` —— 把 Android PCM 写入服务器的 PipeWire 虚拟麦克风。

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use futures::{SinkExt, StreamExt};
use std::{collections::HashMap, sync::Arc};
use tracing::{info, warn};

use crate::{
    billing,
    project_auth::{auth_from_headers_or_query, json_error},
    types::AppState,
    voice_audio_format::{check_format_declaration, check_pcm16_frame, PcmCheck},
    voice_config::{VirtualMicConfig, MAX_BUFFERED_BYTES},
    voice_protocol::{resolve_authenticated_voice_user, ClientControl, ServerEvent},
    voice_pwcat::PwcatHandle,
};

pub async fn ws_virtual_mic_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
    ws: WebSocketUpgrade,
) -> Response {
    let caller = match auth_from_headers_or_query(&state, &headers, &query) {
        Ok(user) => user,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "未登录"),
    };
    if let Err(msg) = billing::check_can_call(&state.store, &caller.id) {
        return json_error(StatusCode::PAYMENT_REQUIRED, msg);
    }
    let authenticated_user_id = caller.id;
    ws.on_upgrade(move |socket| async move {
        if let Err(err) = handle(socket, authenticated_user_id).await {
            warn!(target: "voice", "virtual-mic 连接异常退出: {err:#}");
        }
    })
    .into_response()
}

async fn handle(socket: WebSocket, authenticated_user_id: String) -> anyhow::Result<()> {
    let cfg = VirtualMicConfig::from_env();
    let (mut sender, mut receiver) = socket.split();

    // 1. 等待 Hello
    let hello = match receiver.next().await {
        Some(Ok(Message::Text(t))) => serde_json::from_str::<ClientControl>(&t).ok(),
        _ => None,
    };
    let Some(ClientControl::Hello {
        user_id,
        sample_rate,
        channels,
        ..
    }) = hello
    else {
        let _ = sender
            .send(Message::Text(
                ServerEvent::Error {
                    code: "bad_hello",
                    message: "首帧必须是 hello 文本消息".into(),
                }
                .to_json(),
            ))
            .await;
        return Ok(());
    };
    let user_id = match resolve_authenticated_voice_user(&authenticated_user_id, user_id) {
        Ok(user_id) => user_id,
        Err(message) => {
            let _ = sender
                .send(Message::Text(
                    ServerEvent::Error {
                        code: "user_mismatch",
                        message,
                    }
                    .to_json(),
                ))
                .await;
            return Ok(());
        }
    };
    if let Err(msg) = check_format_declaration(sample_rate, channels) {
        let _ = sender
            .send(Message::Text(
                ServerEvent::Error {
                    code: "bad_format",
                    message: msg,
                }
                .to_json(),
            ))
            .await;
        return Ok(());
    }

    // 2. 启动 pw-cat
    let mut pwcat = match PwcatHandle::spawn(&cfg) {
        Ok(handle) => handle,
        Err(err) => {
            warn!(target: "voice", user_id, "启动 pw-cat 失败: {err:#}");
            let _ = sender
                .send(Message::Text(
                    ServerEvent::Error {
                        code: "pwcat_spawn",
                        message: format!("启动虚拟麦失败：{err}"),
                    }
                    .to_json(),
                ))
                .await;
            return Ok(());
        }
    };

    let _ = sender
        .send(Message::Text(
            ServerEvent::Ready {
                mode: "virtual_mic",
            }
            .to_json(),
        ))
        .await;

    info!(target: "voice", user_id, sink = %cfg.target_sink, "virtual-mic 会话已建立");

    // 3. 主循环：二进制帧 → pw-cat；commit/close → 结束句子
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Binary(bytes)) => {
                match check_pcm16_frame(&bytes, MAX_BUFFERED_BYTES) {
                    PcmCheck::Ok => {}
                    PcmCheck::OddBytes => {
                        let _ = sender
                            .send(Message::Text(
                                ServerEvent::Error {
                                    code: "odd_bytes",
                                    message: "PCM16 帧字节数必须是偶数".into(),
                                }
                                .to_json(),
                            ))
                            .await;
                        continue;
                    }
                    PcmCheck::TooLarge => {
                        let _ = sender
                            .send(Message::Text(
                                ServerEvent::Error {
                                    code: "too_large",
                                    message: "单帧过大".into(),
                                }
                                .to_json(),
                            ))
                            .await;
                        continue;
                    }
                }
                if let Err(err) = pwcat.write_pcm(&bytes).await {
                    warn!(target: "voice", "写 pw-cat 失败: {err:#}");
                    break;
                }
                let _ = sender
                    .send(Message::Text(
                        ServerEvent::VirtualMicFed {
                            bytes: pwcat.written_bytes(),
                        }
                        .to_json(),
                    ))
                    .await;
            }
            Ok(Message::Text(text)) => {
                let ctrl: Option<ClientControl> = serde_json::from_str(&text).ok();
                match ctrl {
                    Some(ClientControl::Commit) => {
                        let _ = pwcat.write_silence_ms(cfg.end_silence_ms).await;
                    }
                    Some(ClientControl::Close) | None => break,
                    Some(ClientControl::Hello { .. }) => {} // 忽略重复 hello
                }
            }
            Ok(Message::Close(_)) | Err(_) => break,
            _ => {}
        }
    }

    pwcat.shutdown().await;
    Ok(())
}

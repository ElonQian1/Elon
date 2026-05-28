//! 方案 B：`/ws/voice/transcribe` —— PCM 流 → OpenAI Realtime → 转写文本 → 派发给 CLI。

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tracing::{info, warn};

use crate::{
    types::AppState,
    voice_audio_format::{check_format_declaration, check_pcm16_frame, PcmCheck},
    voice_config::{RealtimeTranscribeConfig, MAX_BUFFERED_BYTES},
    voice_openai_realtime::{RealtimeTranscriber, TranscriptEvent},
    voice_protocol::{ClientControl, ServerEvent},
    voice_to_cli::{dispatch_transcript, DispatchTarget},
};

pub async fn ws_transcribe_handler(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| async move {
        if let Err(err) = handle(state, socket).await {
            warn!(target: "voice", "transcribe 连接异常退出: {err:#}");
        }
    })
}

async fn handle(state: Arc<AppState>, socket: WebSocket) -> anyhow::Result<()> {
    let cfg = RealtimeTranscribeConfig::from_env();
    let (mut sender, mut receiver) = socket.split();

    // 1. 等待 Hello
    let hello = match receiver.next().await {
        Some(Ok(Message::Text(t))) => serde_json::from_str::<ClientControl>(&t).ok(),
        _ => None,
    };
    let Some(ClientControl::Hello {
        user_id,
        project_id,
        conversation_id,
        sample_rate,
        channels,
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

    // 2. 连接 Realtime Transcription
    let mut transcriber = match RealtimeTranscriber::connect(&cfg).await {
        Ok(t) => t,
        Err(err) => {
            warn!(target: "voice", user_id, "连接 Realtime 转写失败: {err:#}");
            let _ = sender
                .send(Message::Text(
                    ServerEvent::Error {
                        code: "realtime_connect",
                        message: format!("连接转写服务失败：{err}"),
                    }
                    .to_json(),
                ))
                .await;
            return Ok(());
        }
    };

    let _ = sender
        .send(Message::Text(
            ServerEvent::Ready { mode: "transcribe" }.to_json(),
        ))
        .await;
    info!(target: "voice", user_id, "transcribe 会话已建立");

    let target = DispatchTarget {
        user_id: user_id.clone(),
        project_id,
        conversation_id,
    };

    // 3. AI 回复通道：AI 任务产生的 WsMessage JSON 通过这个 channel 流回
    let (ai_reply_tx, mut ai_reply_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    // 4. 并发：客户端循环 + 转写事件循环 + AI 回复流
    loop {
        tokio::select! {
            biased;
            client_msg = receiver.next() => {
                let Some(msg) = client_msg else { break; };
                match msg {
                    Ok(Message::Binary(bytes)) => {
                        match check_pcm16_frame(&bytes, MAX_BUFFERED_BYTES) {
                            PcmCheck::Ok => {}
                            PcmCheck::OddBytes => {
                                let _ = sender.send(Message::Text(ServerEvent::Error {
                                    code: "odd_bytes",
                                    message: "PCM16 帧字节数必须是偶数".into(),
                                }.to_json())).await;
                                continue;
                            }
                            PcmCheck::TooLarge => {
                                let _ = sender.send(Message::Text(ServerEvent::Error {
                                    code: "too_large",
                                    message: "单帧过大".into(),
                                }.to_json())).await;
                                continue;
                            }
                        }
                        if let Err(err) = transcriber.append_pcm(bytes.to_vec()) {
                            warn!(target: "voice", "上行音频失败: {err:#}");
                            break;
                        }
                    }
                    Ok(Message::Text(text)) => {
                        match serde_json::from_str::<ClientControl>(&text).ok() {
                            Some(ClientControl::Commit) => {
                                let _ = transcriber.commit();
                            }
                            Some(ClientControl::Close) | None => break,
                            Some(ClientControl::Hello { .. }) => {}
                        }
                    }
                    Ok(Message::Close(_)) | Err(_) => break,
                    _ => {}
                }
            }
            Some(event) = transcriber.event_rx.recv() => {
                match event {
                    TranscriptEvent::Delta(text) => {
                        let _ = sender.send(Message::Text(
                            ServerEvent::TranscriptDelta { text }.to_json()
                        )).await;
                    }
                    TranscriptEvent::Final(text) => {
                        let _ = sender.send(Message::Text(
                            ServerEvent::TranscriptFinal { text: text.clone() }.to_json()
                        )).await;
                        // 每次 Final 都用同一个 ai_reply_tx（可 clone，多轮共用一个 rx）
                        match dispatch_transcript(&state, &target, &text, ai_reply_tx.clone()).await {
                            Ok(outcome) => {
                                let _ = sender.send(Message::Text(ServerEvent::CliDispatched {
                                    ok: outcome.ok,
                                    message: outcome.message,
                                }.to_json())).await;
                            }
                            Err(err) => {
                                let _ = sender.send(Message::Text(ServerEvent::Error {
                                    code: "dispatch",
                                    message: err.to_string(),
                                }.to_json())).await;
                            }
                        }
                    }
                    TranscriptEvent::Error(msg) => {
                        let _ = sender.send(Message::Text(ServerEvent::Error {
                            code: "realtime",
                            message: msg,
                        }.to_json())).await;
                    }
                    TranscriptEvent::Closed => break,
                }
            }
            // AI 任务产生的进度/完成/错误消息，透传给手机
            Some(raw_json) = ai_reply_rx.recv() => {
                let _ = sender.send(Message::Text(raw_json)).await;
            }
        }
    }

    transcriber.close();
    Ok(())
}

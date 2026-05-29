//! 方案 B：`/ws/voice/transcribe` —— PCM 流 → ASR → 转写文本 → 派发给 CLI。
//!
//! **三级 ASR 降级链**（依序尝试，客户端感知不到切换）：
//!
//!   Tier 1: `WHISPER_LOCAL_URL` 已设置
//!           → `voice_whisper_local` HTTP（本地 Whisper，免费，批量，需 commit 后返回）
//!
//!   Tier 2: `OPENAI_API_KEY` 已设置
//!           → `voice_openai_realtime` WebSocket（流式 delta，实时，按量付费）
//!
//!   Tier 3: 任意 OPENAI_API_KEY / WHISPER_REST_KEY / AGENT_*_KEY
//!           → `voice_whisper_rest` REST POST `/v1/audio/transcriptions`
//!             （批量，Tier 2 连接失败时自动降级，复用已有 API key）

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
    voice_whisper_local,
    voice_whisper_rest,
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

    // 2. 路由：本地 Whisper 优先，若未配置则回退 OpenAI Realtime
    if let Some(whisper_cfg) = voice_whisper_local::WhisperLocalConfig::from_env() {
        let target = DispatchTarget {
            user_id: user_id.clone(),
            project_id,
            conversation_id,
        };
        let (ai_reply_tx, mut ai_reply_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let _ = sender
            .send(Message::Text(
                ServerEvent::Ready { mode: "transcribe_local" }.to_json(),
            ))
            .await;
        info!(target: "voice", user_id, "whisper-local 会话已建立");

        let mut pcm_buf: Vec<u8> = Vec::new();
        loop {
            tokio::select! {
                biased;
                client_msg = receiver.next() => {
                    let Some(msg) = client_msg else { break; };
                    match msg {
                        Ok(Message::Binary(bytes)) => {
                            match check_pcm16_frame(&bytes, MAX_BUFFERED_BYTES) {
                                PcmCheck::Ok => pcm_buf.extend_from_slice(&bytes),
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
                        }
                        Ok(Message::Text(text)) => {
                            match serde_json::from_str::<ClientControl>(&text).ok() {
                                Some(ClientControl::Commit) => {
                                    if pcm_buf.is_empty() { continue; }
                                    let pcm = std::mem::take(&mut pcm_buf);
                                    let transcript = match voice_whisper_local::transcribe_pcm(
                                        &whisper_cfg, &pcm, sample_rate, channels,
                                    ).await {
                                        Ok(t) if t.is_empty() => continue,
                                        Ok(t) => t,
                                        Err(err) => {
                                            warn!(target: "voice", user_id, "whisper 转写失败: {err:#}");
                                            let _ = sender.send(Message::Text(ServerEvent::Error {
                                                code: "whisper",
                                                message: format!("本地转写失败：{err}"),
                                            }.to_json())).await;
                                            continue;
                                        }
                                    };
                                    let _ = sender.send(Message::Text(
                                        ServerEvent::TranscriptFinal { text: transcript.clone() }.to_json()
                                    )).await;
                                    match dispatch_transcript(&state, &target, &transcript, ai_reply_tx.clone()).await {
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
                                Some(ClientControl::Close) | None => break,
                                Some(ClientControl::Hello { .. }) => {}
                            }
                        }
                        Ok(Message::Close(_)) | Err(_) => break,
                        _ => {}
                    }
                }
                Some(raw_json) = ai_reply_rx.recv() => {
                    let _ = sender.send(Message::Text(raw_json)).await;
                }
            }
        }
        return Ok(());
    }

    // 2-fallback. 连接 OpenAI Realtime Transcription
    let mut transcriber = match RealtimeTranscriber::connect(&cfg).await {
        Ok(t) => t,
        Err(err) => {
            warn!(target: "voice", user_id, "Realtime 连接失败，尝试 Tier 3 REST 降级: {err:#}");

            // ── Tier 3: Whisper REST（buffer 模式，commit 后批量转写）──
            let agents_cfg = state.agents_config.read().await;
            let candidates = voice_whisper_rest::WhisperRestCandidate::collect(&agents_cfg);
            drop(agents_cfg);

            if candidates.is_empty() {
                warn!(target: "voice", user_id, "无可用 ASR 服务（无 key 配置）");
                let _ = sender
                    .send(Message::Text(
                        ServerEvent::Error {
                            code: "no_asr",
                            message: "语音识别服务暂不可用（服务器未配置 OPENAI_API_KEY / WHISPER_REST_KEY）".into(),
                        }
                        .to_json(),
                    ))
                    .await;
                return Ok(());
            }

            let target = DispatchTarget { user_id: user_id.clone(), project_id, conversation_id };
            let (ai_reply_tx, mut ai_reply_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
            let _ = sender
                .send(Message::Text(ServerEvent::Ready { mode: "transcribe_rest" }.to_json()))
                .await;
            info!(target: "voice", user_id, "Tier 3 REST 转写会话已建立（{} 个候选）", candidates.len());

            let mut pcm_buf: Vec<u8> = Vec::new();
            loop {
                tokio::select! {
                    biased;
                    client_msg = receiver.next() => {
                        let Some(msg) = client_msg else { break; };
                        match msg {
                            Ok(Message::Binary(bytes)) => {
                                match check_pcm16_frame(&bytes, MAX_BUFFERED_BYTES) {
                                    PcmCheck::Ok => pcm_buf.extend_from_slice(&bytes),
                                    PcmCheck::OddBytes => {
                                        let _ = sender.send(Message::Text(ServerEvent::Error {
                                            code: "odd_bytes",
                                            message: "PCM16 帧字节数必须是偶数".into(),
                                        }.to_json())).await;
                                    }
                                    PcmCheck::TooLarge => {
                                        let _ = sender.send(Message::Text(ServerEvent::Error {
                                            code: "too_large",
                                            message: "单帧过大".into(),
                                        }.to_json())).await;
                                    }
                                }
                            }
                            Ok(Message::Text(text)) => {
                                match serde_json::from_str::<ClientControl>(&text).ok() {
                                    Some(ClientControl::Commit) => {
                                        if pcm_buf.is_empty() { continue; }
                                        let pcm = std::mem::take(&mut pcm_buf);
                                        let transcript = match voice_whisper_rest::transcribe_with_fallback(
                                            &candidates, &pcm, sample_rate, channels,
                                        ).await {
                                            Ok(t) if t.is_empty() => continue,
                                            Ok(t) => t,
                                            Err(e) => {
                                                warn!(target: "voice", user_id, "Tier 3 全部候选失败: {e:#}");
                                                let _ = sender.send(Message::Text(ServerEvent::Error {
                                                    code: "asr_all_failed",
                                                    message: format!("所有转写服务均失败：{e}"),
                                                }.to_json())).await;
                                                continue;
                                            }
                                        };
                                        let _ = sender.send(Message::Text(
                                            ServerEvent::TranscriptFinal { text: transcript.clone() }.to_json()
                                        )).await;
                                        match dispatch_transcript(&state, &target, &transcript, ai_reply_tx.clone()).await {
                                            Ok(outcome) => {
                                                let _ = sender.send(Message::Text(ServerEvent::CliDispatched {
                                                    ok: outcome.ok, message: outcome.message,
                                                }.to_json())).await;
                                            }
                                            Err(e) => {
                                                let _ = sender.send(Message::Text(ServerEvent::Error {
                                                    code: "dispatch", message: e.to_string(),
                                                }.to_json())).await;
                                            }
                                        }
                                    }
                                    Some(ClientControl::Close) | None => break,
                                    Some(ClientControl::Hello { .. }) => {}
                                }
                            }
                            Ok(Message::Close(_)) | Err(_) => break,
                            _ => {}
                        }
                    }
                    Some(raw_json) = ai_reply_rx.recv() => {
                        let _ = sender.send(Message::Text(raw_json)).await;
                    }
                }
            }
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

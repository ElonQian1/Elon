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
        Query, State,
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use futures::{
    future::BoxFuture,
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use std::{collections::HashMap, sync::Arc};
use tracing::{info, warn};

use crate::{
    billing,
    project_auth::{auth_from_headers_or_query, json_error},
    types::AppState,
    voice_audio_format::{check_format_declaration, check_pcm16_frame, PcmCheck},
    voice_config::{RealtimeTranscribeConfig, MAX_BUFFERED_BYTES},
    voice_openai_realtime::{RealtimeTranscriber, TranscriptEvent},
    voice_protocol::{resolve_authenticated_voice_user, ClientControl, ServerEvent},
    voice_to_cli::{dispatch_transcript, DispatchTarget},
    voice_whisper_local, voice_whisper_rest,
};

pub async fn ws_transcribe_handler(
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
        if let Err(err) = handle(state, socket, authenticated_user_id).await {
            warn!(target: "voice", "transcribe 连接异常退出: {err:#}");
        }
    })
    .into_response()
}

async fn handle(
    state: Arc<AppState>,
    socket: WebSocket,
    authenticated_user_id: String,
) -> anyhow::Result<()> {
    let cfg = RealtimeTranscribeConfig::from_env();
    let (mut sender, mut receiver) = socket.split();

    // 1. 等待 Hello
    let hello = match receiver.next().await {
        Some(Ok(Message::Text(t))) => serde_json::from_str::<ClientControl>(&t).ok(),
        _ => None,
    };
    let Some(ClientControl::Hello {
        user_id,
        target: voice_target,
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

    // 2. 路由：本地 Whisper 优先，若未配置则回退 OpenAI Realtime
    if let Some(whisper_cfg) = voice_whisper_local::WhisperLocalConfig::from_env() {
        let target = DispatchTarget {
            user_id: user_id.clone(),
            voice_target: voice_target.clone(),
            project_id: project_id.clone(),
            conversation_id: conversation_id.clone(),
        };
        let _ = sender
            .send(Message::Text(
                ServerEvent::Ready {
                    mode: "transcribe_local",
                }
                .to_json(),
            ))
            .await;
        info!(target: "voice", user_id, "whisper-local 会话已建立");
        run_buffered_loop(
            &state,
            &mut sender,
            &mut receiver,
            target,
            "whisper-local".to_string(),
            sample_rate,
            channels,
            move |pcm| {
                let cfg = whisper_cfg.clone();
                Box::pin(async move {
                    voice_whisper_local::transcribe_pcm(&cfg, &pcm, sample_rate, channels).await
                })
            },
        )
        .await;
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

            let target = DispatchTarget {
                user_id: user_id.clone(),
                voice_target: voice_target.clone(),
                project_id: project_id.clone(),
                conversation_id: conversation_id.clone(),
            };
            let _ = sender
                .send(Message::Text(
                    ServerEvent::Ready {
                        mode: "transcribe_rest",
                    }
                    .to_json(),
                ))
                .await;
            info!(target: "voice", user_id, "Tier 3 REST 转写会话已建立（{} 个候选）", candidates.len());
            run_buffered_loop(
                &state,
                &mut sender,
                &mut receiver,
                target,
                "whisper-rest".to_string(),
                sample_rate,
                channels,
                move |pcm| {
                    let cands = candidates.clone();
                    Box::pin(async move {
                        voice_whisper_rest::transcribe_with_fallback(
                            &cands,
                            &pcm,
                            sample_rate,
                            channels,
                        )
                        .await
                    })
                },
            )
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
        voice_target,
        project_id,
        conversation_id,
    };

    // 3. AI 回复通道：AI 任务产生的 WsMessage JSON 通过这个 channel 流回
    let (ai_reply_tx, mut ai_reply_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    // 4. 并发：客户端循环 + 转写事件循环 + AI 回复流
    let mut turn_pcm_bytes: usize = 0;
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
                        turn_pcm_bytes += bytes.len();
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
                        if turn_pcm_bytes > 0 {
                            crate::compute_usage::record_pcm_asr(
                                &state.store,
                                &target.user_id,
                                "voice_transcribe_realtime",
                                &cfg.model,
                                turn_pcm_bytes,
                                sample_rate,
                                channels,
                            );
                            turn_pcm_bytes = 0;
                        }
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

/// Tier 1（本地 Whisper）和 Tier 3（REST 降级）共用的缓冲式 PCM → ASR → 派发循环。
///
/// `transcribe` 接收一次 Commit 积累的 PCM16 字节，返回转写文本（空串表示静音跳过）。
/// 该函数统一处理：PCM 帧缓冲、Commit 触发转写、TranscriptFinal 推送、CLI 派发
/// 以及 AI 回复消息回流——两个 Tier 的差异只在传入的 `transcribe` 闭包里。
async fn run_buffered_loop(
    state: &Arc<AppState>,
    sender: &mut SplitSink<WebSocket, Message>,
    receiver: &mut SplitStream<WebSocket>,
    target: DispatchTarget,
    usage_model: String,
    sample_rate: u32,
    channels: u16,
    transcribe: impl Fn(Vec<u8>) -> BoxFuture<'static, anyhow::Result<String>>,
) {
    let (ai_reply_tx, mut ai_reply_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
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
                                let pcm_len = pcm.len();
                                let transcript = match transcribe(pcm).await {
                                    Ok(t) if t.trim().is_empty() => {
                                        crate::compute_usage::record_pcm_asr(
                                            &state.store,
                                            &target.user_id,
                                            "voice_transcribe_buffered",
                                            &usage_model,
                                            pcm_len,
                                            sample_rate,
                                            channels,
                                        );
                                        continue;
                                    }
                                    Ok(t) => t,
                                    Err(err) => {
                                        warn!(target: "voice", "ASR 转写失败: {err:#}");
                                        let _ = sender.send(Message::Text(ServerEvent::Error {
                                            code: "asr_failed",
                                            message: format!("转写失败：{err}"),
                                        }.to_json())).await;
                                        continue;
                                    }
                                };
                                crate::compute_usage::record_pcm_asr(
                                    &state.store,
                                    &target.user_id,
                                    "voice_transcribe_buffered",
                                    &usage_model,
                                    pcm_len,
                                    sample_rate,
                                    channels,
                                );
                                let _ = sender.send(Message::Text(
                                    ServerEvent::TranscriptFinal { text: transcript.clone() }.to_json()
                                )).await;
                                match dispatch_transcript(state, &target, &transcript, ai_reply_tx.clone()).await {
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
}

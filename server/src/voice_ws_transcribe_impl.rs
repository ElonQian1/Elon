use super::*;

pub(super) async fn handle(
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
        group_id,
        sample_rate,
        channels,
    }) = hello
    else {
        let _ = send_json(
            &mut sender,
            &ServerEvent::Error {
                code: "bad_hello",
                message: "首帧必须是 hello 文本消息".into(),
            },
        )
        .await;
        return Ok(());
    };
    let user_id = match resolve_authenticated_voice_user(&authenticated_user_id, user_id) {
        Ok(user_id) => user_id,
        Err(message) => {
            let _ = send_json(
                &mut sender,
                &ServerEvent::Error {
                    code: "user_mismatch",
                    message,
                },
            )
            .await;
            return Ok(());
        }
    };
    if let Err(msg) = check_format_declaration(sample_rate, channels) {
        let _ = send_json(
            &mut sender,
            &ServerEvent::Error {
                code: "bad_format",
                message: msg,
            },
        )
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
            group_id: group_id.clone(),
        };
        let _ = send_json(
            &mut sender,
            &ServerEvent::Ready {
                mode: "transcribe_local",
            },
        )
        .await;
        info!(target: "voice", user_id, "whisper-local 会话已建立");
        run_buffered_loop(&state, &mut sender, &mut receiver, target, move |pcm| {
            let cfg = whisper_cfg.clone();
            Box::pin(async move {
                voice_whisper_local::transcribe_pcm(&cfg, &pcm, sample_rate, channels).await
            })
        })
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
                let _ = send_json(
                    &mut sender,
                    &ServerEvent::Error {
                        code: "no_asr",
                        message:
                            "语音识别服务暂不可用（服务器未配置 OPENAI_API_KEY / WHISPER_REST_KEY）"
                                .into(),
                    },
                )
                .await;
                return Ok(());
            }

            let target = DispatchTarget {
                user_id: user_id.clone(),
                voice_target: voice_target.clone(),
                project_id: project_id.clone(),
                conversation_id: conversation_id.clone(),
                group_id: group_id.clone(),
            };
            let _ = send_json(
                &mut sender,
                &ServerEvent::Ready {
                    mode: "transcribe_rest",
                },
            )
            .await;
            info!(target: "voice", user_id, "Tier 3 REST 转写会话已建立（{} 个候选）", candidates.len());
            run_buffered_loop(&state, &mut sender, &mut receiver, target, move |pcm| {
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
            })
            .await;
            return Ok(());
        }
    };

    let _ = send_json(&mut sender, &ServerEvent::Ready { mode: "transcribe" }).await;
    info!(target: "voice", user_id, "transcribe 会话已建立");

    let target = DispatchTarget {
        user_id: user_id.clone(),
        voice_target,
        project_id,
        conversation_id,
        group_id,
    };

    // 3. AI 回复通道：AI 任务产生的 WsMessage JSON 通过这个 channel 流回
    let (ai_reply_tx, mut ai_reply_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    // 4. 并发：客户端循环 + 转写事件循环 + AI 回复流
    let mut turn_pcm_bytes: usize = 0;
    let mut close_reason = WsCloseReason::WriteFailed;
    loop {
        tokio::select! {
            biased;
            client_msg = receiver.next() => {
                match receive_data_or_control(client_msg, &mut sender).await {
                    WsIncoming::Binary(bytes) => {
                        match check_pcm16_frame(&bytes, MAX_BUFFERED_BYTES) {
                            PcmCheck::Ok => {}
                            PcmCheck::OddBytes => {
                                let _ = send_json(&mut sender, &ServerEvent::Error {
                                    code: "odd_bytes",
                                    message: "PCM16 帧字节数必须是偶数".into(),
                                }).await;
                                continue;
                            }
                            PcmCheck::TooLarge => {
                                let _ = send_json(&mut sender, &ServerEvent::Error {
                                    code: "too_large",
                                    message: "单帧过大".into(),
                                }).await;
                                continue;
                            }
                        }
                        turn_pcm_bytes += bytes.len();
                        if let Err(err) = transcriber.append_pcm(bytes.to_vec()) {
                            warn!(target: "voice", "上行音频失败: {err:#}");
                            break;
                        }
                    }
                    WsIncoming::Text(text) => {
                        match serde_json::from_str::<ClientControl>(&text).ok() {
                            Some(ClientControl::Commit) => {
                                let _ = transcriber.commit();
                            }
                            Some(ClientControl::Close) | None => {
                                close_reason = WsCloseReason::ClientControlClose;
                                break;
                            }
                            Some(ClientControl::Hello { .. }) => {}
                        }
                    }
                    WsIncoming::Closed(reason) => {
                        close_reason = reason;
                        break;
                    }
                    WsIncoming::Continue => {}
                }
            }
            Some(event) = transcriber.event_rx.recv() => {
                match event {
                    TranscriptEvent::Delta(text) => {
                        let _ = send_json(&mut sender, &ServerEvent::TranscriptDelta { text }).await;
                    }
                    TranscriptEvent::Final(text) => {
                        if turn_pcm_bytes > 0 {
                            turn_pcm_bytes = 0;
                        }
                        let _ = send_json(
                            &mut sender,
                            &ServerEvent::TranscriptFinal { text: text.clone() },
                        )
                        .await;
                        // 每次 Final 都用同一个 ai_reply_tx（可 clone，多轮共用一个 rx）
                        match dispatch_transcript(&state, &target, &text, ai_reply_tx.clone()).await {
                            Ok(outcome) => {
                                let _ = send_json(&mut sender, &ServerEvent::CliDispatched {
                                    ok: outcome.ok,
                                    message: outcome.message,
                                }).await;
                            }
                            Err(err) => {
                                let _ = send_json(&mut sender, &ServerEvent::Error {
                                    code: "dispatch",
                                    message: err.to_string(),
                                }).await;
                            }
                        }
                    }
                    TranscriptEvent::Error(msg) => {
                        let _ = send_json(&mut sender, &ServerEvent::Error {
                            code: "realtime",
                            message: msg,
                        }).await;
                    }
                    TranscriptEvent::Closed => break,
                }
            }
            // AI 任务产生的进度/完成/错误消息，透传给手机
            Some(raw_json) = ai_reply_rx.recv() => {
                if !send_text(&mut sender, raw_json).await {
                    close_reason = WsCloseReason::WriteFailed;
                    break;
                }
            }
        }
    }

    transcriber.close();
    realtime_metrics::record_close_with_store(
        &state.store,
        RealtimeChannel::VoiceTranscribe,
        close_reason.as_str(),
    );
    Ok(())
}

/// Tier 1（本地 Whisper）和 Tier 3（REST 降级）共用的缓冲式 PCM → ASR → 派发循环。
///
/// `transcribe` 接收一次 Commit 积累的 PCM16 字节，返回转写文本（空串表示静音跳过）。
/// 该函数统一处理：PCM 帧缓冲、Commit 触发转写、TranscriptFinal 推送、CLI 派发
/// 以及 AI 回复消息回流——两个 Tier 的差异只在传入的 `transcribe` 闭包里。
pub(super) async fn run_buffered_loop(
    state: &Arc<AppState>,
    sender: &mut SplitSink<WebSocket, Message>,
    receiver: &mut SplitStream<WebSocket>,
    target: DispatchTarget,
    transcribe: impl Fn(Vec<u8>) -> BoxFuture<'static, anyhow::Result<String>>,
) {
    let (ai_reply_tx, mut ai_reply_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let mut pcm_buf: Vec<u8> = Vec::new();
    let close_reason = loop {
        tokio::select! {
            biased;
            client_msg = receiver.next() => {
                match receive_data_or_control(client_msg, sender).await {
                    WsIncoming::Binary(bytes) => {
                        match check_pcm16_frame(&bytes, MAX_BUFFERED_BYTES) {
                            PcmCheck::Ok => pcm_buf.extend_from_slice(&bytes),
                            PcmCheck::OddBytes => {
                                let _ = send_json(sender, &ServerEvent::Error {
                                    code: "odd_bytes",
                                    message: "PCM16 帧字节数必须是偶数".into(),
                                }).await;
                                continue;
                            }
                            PcmCheck::TooLarge => {
                                let _ = send_json(sender, &ServerEvent::Error {
                                    code: "too_large",
                                    message: "单帧过大".into(),
                                }).await;
                                continue;
                            }
                        }
                    }
                    WsIncoming::Text(text) => {
                        match serde_json::from_str::<ClientControl>(&text).ok() {
                            Some(ClientControl::Commit) => {
                                if pcm_buf.is_empty() { continue; }
                                let pcm = std::mem::take(&mut pcm_buf);
                                let transcript = match transcribe(pcm).await {
                                    Ok(t) if t.trim().is_empty() => {
                                        continue;
                                    }
                                    Ok(t) => t,
                                    Err(err) => {
                                        warn!(target: "voice", "ASR 转写失败: {err:#}");
                                        let _ = send_json(sender, &ServerEvent::Error {
                                            code: "asr_failed",
                                            message: format!("转写失败：{err}"),
                                        }).await;
                                        continue;
                                    }
                                };
                                let _ = send_json(
                                    sender,
                                    &ServerEvent::TranscriptFinal { text: transcript.clone() },
                                ).await;
                                match dispatch_transcript(state, &target, &transcript, ai_reply_tx.clone()).await {
                                    Ok(outcome) => {
                                        let _ = send_json(sender, &ServerEvent::CliDispatched {
                                            ok: outcome.ok,
                                            message: outcome.message,
                                        }).await;
                                    }
                                    Err(err) => {
                                        let _ = send_json(sender, &ServerEvent::Error {
                                            code: "dispatch",
                                            message: err.to_string(),
                                        }).await;
                                    }
                                }
                            }
                            Some(ClientControl::Close) | None => {
                                break WsCloseReason::ClientControlClose;
                            }
                            Some(ClientControl::Hello { .. }) => {}
                        }
                    }
                    WsIncoming::Closed(reason) => {
                        break reason;
                    }
                    WsIncoming::Continue => {}
                }
            }
            Some(raw_json) = ai_reply_rx.recv() => {
                if !send_text(sender, raw_json).await {
                    break WsCloseReason::WriteFailed;
                }
            }
        }
    };
    realtime_metrics::record_close_with_store(
        &state.store,
        RealtimeChannel::VoiceTranscribe,
        close_reason.as_str(),
    );
}

//! 方案 C：`/ws/voice/realtime-chat` —— Android PCM ↔ OpenAI Realtime 语音对话。

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
    billing, friend_events,
    project_auth::{auth_from_headers_or_query, json_error},
    social_ai::realtime_social_ai_prompt,
    store::{SOCIAL_AI_DISPLAY_NAME, SOCIAL_AI_USER_ID},
    types::AppState,
    voice_audio_format::{check_format_declaration, check_pcm16_frame, PcmCheck},
    voice_config::{RealtimeChatConfig, MAX_BUFFERED_BYTES, REALTIME_SAMPLE_RATE_HZ},
    voice_openai_realtime_chat::{RealtimeChatEvent, RealtimeChatSession},
    voice_protocol::{
        resolve_authenticated_voice_user, ClientControl, ServerEvent, VOICE_TARGET_PHONE_CONTROL,
        VOICE_TARGET_SOCIAL_AI_DIRECT,
    },
};

pub async fn ws_realtime_chat_handler(
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
            warn!(target: "voice", "realtime chat 连接异常退出: {err:#}");
        }
    })
    .into_response()
}

async fn handle(
    state: Arc<AppState>,
    socket: WebSocket,
    authenticated_user_id: String,
) -> anyhow::Result<()> {
    let (mut sender, mut receiver) = socket.split();
    let hello = match receiver.next().await {
        Some(Ok(Message::Text(t))) => serde_json::from_str::<ClientControl>(&t).ok(),
        _ => None,
    };
    let Some(ClientControl::Hello {
        user_id,
        target,
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
    if target.as_deref() != Some(VOICE_TARGET_SOCIAL_AI_DIRECT)
        && target.as_deref() != Some(VOICE_TARGET_PHONE_CONTROL)
    {
        let _ = sender
            .send(Message::Text(
                ServerEvent::Error {
                    code: "unsupported_target",
                    message: "全双工实时通话支持 social_ai_direct 或 phone_control".into(),
                }
                .to_json(),
            ))
            .await;
        return Ok(());
    }
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

    let history = state
        .store
        .list_recent_friend_messages_for_social_ai(&user_id, SOCIAL_AI_USER_ID, 12)
        .unwrap_or_default();
    let instructions = if target.as_deref() == Some(VOICE_TARGET_PHONE_CONTROL) {
        phone_control_realtime_prompt()
    } else {
        realtime_social_ai_prompt(&history)
    };
    let cfg = RealtimeChatConfig::from_env();
    let api_key = {
        let agents_cfg = state.agents_config.read().await;
        cfg.read_api_key_from_agents(&agents_cfg)
    };
    let Some(api_key) = api_key else {
        let _ = sender
            .send(Message::Text(
                ServerEvent::Error {
                    code: "realtime_chat_connect",
                    message: cfg.missing_key_message(),
                }
                .to_json(),
            ))
            .await;
        return Ok(());
    };
    let mut session = match RealtimeChatSession::connect(&cfg, instructions, api_key).await {
        Ok(session) => session,
        Err(err) => {
            let _ = sender
                .send(Message::Text(
                    ServerEvent::Error {
                        code: "realtime_chat_connect",
                        message: err.to_string(),
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
                mode: "realtime_chat",
            }
            .to_json(),
        ))
        .await;
    info!(target: "voice", user_id, "realtime chat 会话已建立");

    let mut turn_input_pcm_bytes: usize = 0;
    let mut turn_output_pcm_bytes: usize = 0;
    let accounting_session_id = uuid::Uuid::new_v4().simple().to_string();
    let mut accounting_turn_index: u64 = 0;
    loop {
        tokio::select! {
            biased;
            client_msg = receiver.next() => {
                let Some(msg) = client_msg else { break; };
                match msg {
                    Ok(Message::Binary(bytes)) => {
                        match check_pcm16_frame(&bytes, MAX_BUFFERED_BYTES) {
                            PcmCheck::Ok => {
                                turn_input_pcm_bytes += bytes.len();
                                if let Err(err) = session.append_pcm(bytes.to_vec()) {
                                    warn!(target: "voice", "Realtime Chat 上行音频失败: {err:#}");
                                    break;
                                }
                            }
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
                                let _ = session.commit();
                            }
                            Some(ClientControl::Close) | None => break,
                            Some(ClientControl::Hello { .. }) => {}
                        }
                    }
                    Ok(Message::Close(_)) | Err(_) => break,
                    _ => {}
                }
            }
            Some(event) = session.event_rx.recv() => {
                match event {
                    RealtimeChatEvent::SessionUpdated => {}
                    RealtimeChatEvent::UserSpeechStarted => {
                        let _ = sender.send(Message::Text(ServerEvent::RealtimeSpeechStarted.to_json())).await;
                    }
                    RealtimeChatEvent::UserSpeechStopped => {
                        let _ = sender.send(Message::Text(ServerEvent::RealtimeSpeechStopped.to_json())).await;
                    }
                    RealtimeChatEvent::UserTranscriptDelta(text) => {
                        let _ = sender.send(Message::Text(ServerEvent::TranscriptDelta { text }.to_json())).await;
                    }
                    RealtimeChatEvent::UserTranscriptFinal(text) => {
                        let text = text.trim().to_string();
                        if !text.is_empty() {
                            store_user_voice_message(&state, &user_id, &text);
                            let _ = sender.send(Message::Text(ServerEvent::TranscriptFinal { text }.to_json())).await;
                        }
                    }
                    RealtimeChatEvent::AiTranscriptDelta(text) => {
                        let _ = sender.send(Message::Text(ServerEvent::RealtimeAiTranscriptDelta { text }.to_json())).await;
                    }
                    RealtimeChatEvent::AiTranscriptDone(text) => {
                        let text = text.trim().to_string();
                        if !text.is_empty() {
                            store_ai_voice_message(&state, &user_id, &text);
                            let _ = sender.send(Message::Text(ServerEvent::RealtimeAiTranscriptDone { text }.to_json())).await;
                        }
                    }
                    RealtimeChatEvent::AudioDelta(bytes) => {
                        turn_output_pcm_bytes += bytes.len();
                        let _ = sender.send(Message::Binary(bytes)).await;
                    }
                    RealtimeChatEvent::AudioDone => {
                        let _ = sender.send(Message::Text(ServerEvent::RealtimeResponseDone.to_json())).await;
                    }
                    RealtimeChatEvent::ResponseDone { response_id, usage } => {
                        accounting_turn_index += 1;
                        let accounting_key = response_id
                            .as_deref()
                            .map(|id| format!("voice_realtime_chat:{user_id}:{id}"))
                            .unwrap_or_else(|| {
                                format!(
                                    "voice_realtime_chat:{user_id}:{accounting_session_id}:{accounting_turn_index}"
                                )
                            });
                        match usage {
                            Some(usage) => {
                                let _ = crate::token_usage_api::record_trusted_usage_with_key(
                                    &state.store,
                                    &user_id,
                                    "voice_realtime_chat",
                                    crate::compute_usage::USAGE_MODE_VOICE_REALTIME,
                                    Some(&cfg.model),
                                    &usage,
                                    Some(&accounting_key),
                                );
                            }
                            None if turn_input_pcm_bytes > 0 || turn_output_pcm_bytes > 0 => {
                                crate::compute_usage::record_realtime_voice_estimate_with_key(
                                    &state.store,
                                    &user_id,
                                    "voice_realtime_chat",
                                    &cfg.model,
                                    turn_input_pcm_bytes,
                                    turn_output_pcm_bytes,
                                    REALTIME_SAMPLE_RATE_HZ,
                                    1,
                                    Some(&accounting_key),
                                );
                            }
                            None => {}
                        }
                        turn_input_pcm_bytes = 0;
                        turn_output_pcm_bytes = 0;
                        let _ = sender.send(Message::Text(ServerEvent::RealtimeResponseDone.to_json())).await;
                    }
                    RealtimeChatEvent::Error(message) => {
                        let _ = sender.send(Message::Text(ServerEvent::Error {
                            code: "realtime_chat",
                            message,
                        }.to_json())).await;
                    }
                    RealtimeChatEvent::Closed => break,
                }
            }
        }
    }

    session.close();
    Ok(())
}

fn store_user_voice_message(state: &Arc<AppState>, user_id: &str, text: &str) {
    match state
        .store
        .send_friend_message(user_id, SOCIAL_AI_USER_ID, text, None)
    {
        Ok(message) => friend_events::publish_friend_message(&message),
        Err(err) => warn!(target: "voice", "保存用户实时语音消息失败: {err:#}"),
    }
}

fn store_ai_voice_message(state: &Arc<AppState>, user_id: &str, text: &str) {
    match state.store.insert_direct_social_ai_reply(user_id, text) {
        Ok(message) => friend_events::publish_friend_message(&message),
        Err(err) => warn!(
            target: "voice",
            ai = SOCIAL_AI_DISPLAY_NAME,
            "保存一龙AI实时语音回复失败: {err:#}"
        ),
    }
}

/// 悬浮球手机控制专属 system prompt。
///
/// AI 的职责：
///  - 聊天类请求 → 简短口语回答（≤30字，适合 TTS 朗读）
///  - 手机控制类请求 → 返回纯 JSON 自动化脚本（无任何多余文字）
///
/// 脚本格式必须严格 JSON，客户端 ScriptEngine 会解析并执行。
fn phone_control_realtime_prompt() -> String {
    r#"你是一个手机语音助手，名字叫小龙。用户通过语音和你交流。

## 核心职责
1. 闲聊和问答 → 简短口语回答（30字以内，语气自然，像朋友聊天）
2. 手机控制指令（打开应用、搜索、点击、发消息等）→ 立即回复纯 JSON 脚本

## 手机控制脚本格式（必须严格遵守）
遇到操控手机的指令，不要说任何多余的话，只输出如下 JSON：
{"steps":[{"type":"LAUNCH_APP","params":{"package":"com.tencent.mm"}},{"type":"FIND_AND_TAP","params":{"text":"搜索"}},{"type":"INPUT_TEXT","params":{"text":"奶茶店"}}]}

## 常用步骤类型
- LAUNCH_APP: {"package":"包名"}
- FIND_AND_TAP: {"text":"界面文字"}
- INPUT_TEXT: {"text":"要输入的内容"}
- GLOBAL_ACTION: {"action":"BACK|HOME|RECENTS"}
- WAIT: {"ms":1000}

## 常用包名
- 微信: com.tencent.mm
- QQ: com.tencent.mobileqq
- 小红书: com.xingin.xhs
- 抖音: com.ss.android.ugc.aweme
- 淘宝: com.taobao.taobao
- 京东: com.jingdong.app.mall
- 支付宝: com.eg.android.AlipayGphone
- 设置: com.android.settings

## 重要规则
- 操控指令：只输出 JSON，一个字都不要多
- 闲聊：30字内，口语化，不要列条目
- 分不清是聊天还是控制时，优先当闲聊处理，等用户说更具体
"#.to_string()
}

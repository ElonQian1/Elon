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
    realtime_metrics::{self, RealtimeChannel},
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
    ws_transport::{receive_data_or_control, send_json, WsCloseReason, WsIncoming},
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

#[path = "voice_ws_realtime_chat_impl.rs"]
mod impl_funcs;
use self::impl_funcs::*;

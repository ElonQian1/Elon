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
    StreamExt,
};
use std::{collections::HashMap, sync::Arc};
use tracing::{info, warn};

use crate::{
    project_auth::{auth_from_headers_or_query, json_error},
    realtime_metrics::{self, RealtimeChannel},
    types::AppState,
    voice_audio_format::{check_format_declaration, check_pcm16_frame, PcmCheck},
    voice_config::{RealtimeTranscribeConfig, MAX_BUFFERED_BYTES},
    voice_openai_realtime::{RealtimeTranscriber, TranscriptEvent},
    voice_protocol::{resolve_authenticated_voice_user, ClientControl, ServerEvent},
    voice_to_cli::{dispatch_transcript, DispatchTarget},
    voice_whisper_local, voice_whisper_rest,
    ws_transport::{receive_data_or_control, send_json, send_text, WsCloseReason, WsIncoming},
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
    let authenticated_user_id = caller.id;
    ws.on_upgrade(move |socket| async move {
        if let Err(err) = handle(state, socket, authenticated_user_id).await {
            warn!(target: "voice", "transcribe 连接异常退出: {err:#}");
        }
    })
    .into_response()
}

#[path = "voice_ws_transcribe_impl.rs"]
mod impl_funcs;
use self::impl_funcs::*;

//! User-facing chat and voice contract for external app clients.
//!
//! fb2 uses this endpoint after exchanging its own login for a main-project
//! session. The response is intentionally protocol-shaped so fb2 can implement
//! the same chat controls without depending on main-project Android code.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;

use crate::{
    external_app_registry::{external_app_by_id, public_external_app_config},
    project_auth::{auth_from_headers, json_error},
    types::AppState,
    voice_protocol::{
        VOICE_TARGET_EXTERNAL_GROUP, VOICE_TARGET_SOCIAL_AI_DIRECT, VOICE_TARGET_TRANSCRIBE_ONLY,
    },
    voice_tts_rewrite::MAX_TTS_TEXT_CHARS,
};

const MAX_ASR_UPLOAD_BYTES: usize = 10 * 1024 * 1024;

pub async fn get_chat_bootstrap(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(app_id): Path<String>,
) -> Response {
    let app = match external_app_by_id(&app_id) {
        Some(app) => app,
        None => {
            return json_error(
                StatusCode::NOT_FOUND,
                format!("未知外部应用：{}", app_id.trim()),
            )
        }
    };
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };

    let default_group_id = app
        .default_groups
        .iter()
        .find(|group| group.auto_join)
        .or_else(|| app.default_groups.first())
        .map(|group| group.group_id);

    Json(json!({
        "app": public_external_app_config(app),
        "user": user,
        "auth": {
            "type": "bearer",
            "source": "POST /api/external/apps/{app_id}/accounts/session",
            "websocket": {
                "authorizationHeader": true,
                "queryToken": "token"
            }
        },
        "chat": {
            "groupsEndpoint": "/api/me/groups",
            "messagesEndpointTemplate": "/api/me/groups/{groupId}/messages",
            "aiReplyEndpointTemplate": "/api/me/groups/{groupId}/messages/{messageId}/ai-reply",
            "defaultGroupId": default_group_id,
            "mentionToTriggerAi": "@EL",
            "externalGroups": app.default_groups,
        },
        "voice": {
            "asr": {
                "uploadEndpoint": "/api/voice/asr",
                "method": "POST",
                "contentType": "multipart/form-data",
                "audioField": "audio",
                "resultField": "text",
                "maxAudioBytes": MAX_ASR_UPLOAD_BYTES,
                "optionalFields": ["format", "language", "beam_size", "vad_filter", "condition_on_previous_text"]
            },
            "tts": {
                "catalogEndpoint": "/api/voice/tts/catalog",
                "synthesizeEndpoint": "/api/voice/tts",
                "method": "POST",
                "contentType": "application/json",
                "maxTextChars": MAX_TTS_TEXT_CHARS,
                "requestFields": ["text", "voiceId", "emotionId", "intensity", "provider", "rewrite", "agentName"],
                "responseKind": "audio",
                "diagnosticHeaders": [
                    "x-elon-tts-provider",
                    "x-elon-tts-voice",
                    "x-elon-tts-emotion",
                    "x-elon-tts-worker",
                    "x-elon-tts-worker-voice",
                    "x-elon-tts-worker-fallback"
                ],
                "clientFallback": "android_system_or_browser_speech_synthesis"
            },
            "realtimeTranscribe": {
                "websocketEndpoint": "/ws/voice/transcribe",
                "sampleRate": 24000,
                "channels": 1,
                "pcmFormat": "pcm16le",
                "helloTargets": [
                    {
                        "target": VOICE_TARGET_TRANSCRIBE_ONLY,
                        "description": "only return transcript_final; fb2 decides where to send it"
                    },
                    {
                        "target": VOICE_TARGET_EXTERNAL_GROUP,
                        "groupIdField": "group_id",
                        "description": "send transcript as a group message; @EL triggers main-project group AI"
                    },
                    {
                        "target": VOICE_TARGET_SOCIAL_AI_DIRECT,
                        "description": "direct one-on-one AI voice input"
                    }
                ]
            }
        },
        "experience": {
            "inputModes": ["text", "hold_to_talk", "voice_message", "realtime_transcribe", "auto_tts"],
            "controls": {
                "holdToTalk": true,
                "slideToCancel": true,
                "tapAiMessageToReplayTts": true,
                "stopTtsWhenRecordingStarts": true,
                "editableTranscriptBeforeSend": true
            },
            "recommendedFlow": [
                "create main-project session from fb2 backend",
                "load this bootstrap contract",
                "render defaultGroupId as the first chat room",
                "use /api/voice/asr for stable first-version voice input",
                "use /api/voice/tts after AI replies and fall back to device TTS"
            ]
        }
    }))
    .into_response()
}

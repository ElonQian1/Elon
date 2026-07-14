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
use serde_json::{json, Value};
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
        "voice": voice_contract(),
        "aiReply": ai_reply_contract(app.id),
        "billing": billing_contract(app.id),
        "experience": experience_contract()
    }))
    .into_response()
}

fn voice_contract() -> Value {
    json!({
        "androidSdk": {
            "module": "android/chat-voice-kit",
            "package": "com.elon.chatvoice",
            "publicComponents": [
                "VoiceComposerView",
                "VoiceComposerBootstrap",
                "VoiceComposerConfig",
                "VoiceComposerAsrConfig",
                "VoiceComposerCallbacks",
                "ChatVoiceRecordingOverlay",
                "ChatVoiceInteractionContract",
                "ChatVoiceEventSink",
                "ChatVoiceSpeaker"
            ],
            "doNotCopyMainAppInternals": [
                "com.elon.app.MainSpeechInputActions",
                "com.elon.app.VoiceRecordingOverlay",
                "主项目聊天页面内部类"
            ]
        },
        "composer": {
            "component": "com.elon.chatvoice.VoiceComposerView",
            "requiredForMainProjectLikeExperience": true,
            "recommendedConfigApi": "VoiceComposerBootstrap.applyFb2GroupChatConfig(...)",
            "inputModes": ["TEXT", "VOICE"],
            "voiceModeCenterControl": "整条按住 说话按钮",
            "overlayComponent": "com.elon.chatvoice.ChatVoiceRecordingOverlay",
            "defaultConfig": {
                "chatMode": "FRIEND_CHAT",
                "releaseZone": "SEND",
                "recordingOverlayEnabled": true,
                "languageTag": "zh-CN",
                "preferOfflineAsr": false,
                "asr": {
                    "serverFallbackEnabled": true,
                    "serverConfigRequired": true,
                    "localResultTimeoutMs": 4500,
                    "localEngineFallbackEnabled": true,
                    "prewarmLocalEngine": true
                }
            },
            "states": [
                "IDLE",
                "PREPARING",
                "RECORDING",
                "CANCELING",
                "PROCESSING",
                "SERVER_PROCESSING",
                "TOO_SHORT",
                "PERMISSION_DENIED",
                "ERROR",
                "TTS_PLAYING"
            ],
            "zones": ["SEND", "AI_REPLY", "TRANSCRIBE", "CANCEL"],
            "callbacks": [
                "onTextSubmit",
                "onVoiceRecognized",
                "onVoiceRecorded",
                "onVoiceServerFallbackStarted",
                "onVoiceCanceled",
                "onPermissionRequired",
                "onVoiceError",
                "onStateChanged",
                "onPlusClick"
            ],
            "acceptanceRules": [
                "fb2 常规聊天页必须接 VoiceComposerView，而不是只接 ASR/TTS 接口。",
                "按住说话时必须显示 SDK 内置录音浮层，除非页面按 ChatVoiceInteractionContract 完整还原。",
                "系统 ASR 无 final/error 或超时时，必须进入 SERVER_PROCESSING 并上传录音到 /api/voice/asr。",
                "ASR/TTS 不因 AI 余额为 0 被阻断。"
            ]
        },
        "asr": {
            "localFirst": true,
            "serverFallback": true,
            "uploadEndpoint": "/api/voice/asr",
            "method": "POST",
            "contentType": "multipart/form-data",
            "audioField": "audio",
            "resultField": "text",
            "maxAudioBytes": MAX_ASR_UPLOAD_BYTES,
            "optionalFields": ["format", "language", "beam_size", "vad_filter", "condition_on_previous_text"],
            "billing": "free_auth_and_limits_only"
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
            "clientFallback": "android_system_or_browser_speech_synthesis",
            "billing": "free_auth_and_limits_only"
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
    })
}

fn ai_reply_contract(app_id: &str) -> Value {
    if app_id == "bb64a" {
        return json!({
            "schema": "external_app.ai_reply.v1",
            "app_id": "bb64a",
            "owner": "main_project",
            "billableUnit": "ai_reply_generation",
            "quotaGate": "before_model_call",
            "freePreparationSteps": ["local_mcp_discovery", "bb64a_doctor", "context_budgeting"],
            "triggers": [
                {
                    "name": "windows_client_help_panel",
                    "entry": "User asks a question inside ElonSpeed Windows",
                    "endpoint": "/api/me/groups/{groupId}/messages",
                    "topicHintSource": "latest user support question",
                    "contextSource": "BB64A local MCP bb64a_doctor snapshot"
                },
                {
                    "name": "source_node_bugfix",
                    "entry": "Sanitized repeated diagnostics indicate product bug",
                    "endpoint": "main-project source-node task pipeline",
                    "topicHintSource": "support bundle plus source repository context",
                    "contextSource": "BB64A diagnostic context pack and source-code node"
                }
            ],
            "externalContext": {
                "contractEndpoint": "/api/external/apps/bb64a/context-contract",
                "primarySource": "bb64a:local-mcp/bb64a_doctor",
                "fallbackSource": "bb64a:/debug/status",
                "queryFields": [
                    "external_user_id",
                    "device_id",
                    "topic_hint",
                    "local_mcp_endpoint",
                    "include_os_snapshot",
                    "include_sensitive_subscriptions"
                ],
                "promptMetadata": [
                    "usage_policy",
                    "context_quality",
                    "external_metrics",
                    "context_audit_id",
                    "tool_contract",
                    "executed_external_app_tools"
                ]
            },
            "answerRules": [
                "Distinguish local user configuration problems from likely BB64A product bugs.",
                "Prefer bb64a_doctor before suggesting repair actions.",
                "Dangerous runtime tools are available for fast repair workflows; describe the intended local effect before using them.",
                "Do not expose raw subscription URLs, tokens or unrelated local file contents in source-node bug reports."
            ],
            "failureBehavior": {
                "contextUnavailable": "Ask the user to keep ElonSpeed Windows open and enable the local AI troubleshooting panel.",
                "localMcpUnavailable": "Fall back to ordinary support guidance and explain that local diagnostics are unavailable.",
                "sourceNodeUnavailable": "Diagnose locally and defer product-code repair until source-node capacity is available."
            }
        });
    }

    json!({
        "schema": "external_app.ai_reply.v1",
        "owner": "main_project",
        "billableUnit": "ai_reply_generation",
        "quotaGate": "before_model_call",
        "freePreparationSteps": ["asr", "tts", "external_context_fetch", "context_budgeting"],
        "triggers": [
            {
                "name": "group_mention",
                "entry": "group message containing @EL",
                "endpoint": "/api/me/groups/{groupId}/messages",
                "topicHintSource": "latest meaningful user text after removing @EL",
                "contextSource": "fb2 Context Pack when group is mapped to fb2"
            },
            {
                "name": "selected_message_ai_reply",
                "entry": "long press group message and choose AI回复",
                "endpoint": "/api/me/groups/{groupId}/messages/{messageId}/ai-reply",
                "topicHintSource": "selected message body after removing @EL",
                "contextSource": "fb2 Context Pack when group is mapped to fb2"
            },
            {
                "name": "group_summary",
                "entry": "create group summary or auto-split summary topic",
                "endpoint": "main-project group summary APIs",
                "topicHintSource": "topic, title, instructions, or split topic",
                "contextSource": "fb2 Context Pack when group is mapped to fb2"
            }
        ],
        "externalContext": {
            "contractEndpoint": "/api/external/apps/fb2/context-contract",
            "primarySource": "fb2:/api/main-project/context/pack",
            "fallbackSource": "fb2:/api/main-project/context/today-matches",
            "queryFields": [
                "group_id",
                "external_user_id",
                "topic_hint",
                "limit",
                "discussion_limit",
                "order_limit",
                "lottery_type",
                "include_platform_orders"
            ],
            "promptMetadata": [
                "usage_policy",
                "answer_policy",
                "answer_rules",
                "context_quality",
                "context_budget",
                "external_metrics",
                "context_audit_id",
                "tool_contract",
                "executed_external_app_tools"
            ]
        },
        "answerRules": [
            "必须区分 fb2 数据事实、群友观点和 AI 推断。",
            "涉及比赛预测时必须说明不确定性，不承诺命中。",
            "订单剖析只能使用当前用户可见订单；平台订单默认只能使用匿名聚合。",
            "没有 fb2 上下文时不能编造比赛、赔率、订单或群友观点。"
        ],
        "failureBehavior": {
            "contextUnavailable": "继续基于群聊历史保守回答，并说明 fb2 业务数据暂不可用。",
            "insufficientBalance": "只阻断 AI 生成回复；ASR/TTS 和上下文拉取仍允许。",
            "missingCurrentUserOrders": "可以分析公开比赛和群观点，但必须说明当前用户订单不可见。"
        }
    })
}

fn experience_contract() -> Value {
    json!({
        "inputModes": ["text", "hold_to_talk", "voice_message", "realtime_transcribe", "auto_tts"],
        "controls": {
            "holdToTalk": true,
            "slideToCancel": true,
            "voiceTextModeToggle": true,
            "fullWidthHoldToTalkButton": true,
            "recordingOverlay": true,
            "aiReplyZone": true,
            "transcribeZone": true,
            "tapAiMessageToReplayTts": true,
            "stopTtsWhenRecordingStarts": true,
            "editableTranscriptBeforeSend": true
        },
        "usagePolicy": {
            "asr": "free",
            "tts": "free",
            "contextFetch": "free",
            "aiReplyGeneration": "billable"
        },
        "recommendedFlow": [
            "create main-project session from fb2 backend",
            "load this bootstrap contract",
            "render defaultGroupId as the first chat room",
            "use android/chat-voice-kit VoiceComposerView for the input bar",
            "prefer VoiceComposerBootstrap.applyFb2GroupChatConfig(...) to map chat-bootstrap into SDK config",
            "configure VoiceComposerAsrConfig with serverFallbackEnabled=true and serverConfig",
            "use /api/voice/tts after AI replies and fall back to device TTS",
            "only check AI quota immediately before model reply generation"
        ]
    })
}

fn billing_contract(app_id: &str) -> Value {
    json!({
        "balanceEndpoint": "/api/me/balance",
        "billingEventsEndpoint": "/api/me/billing",
        "usageStatsEndpointTemplate": "/api/user/{userId}/usage/stats",
        "trialCredit": {
            "grantedBy": "POST /api/external/apps/{app_id}/accounts/session",
            "configKey": format!("external_app_{app_id}_trial_credit_fen"),
            "appliesTo": ["ai_reply_generation", "social_ai", "match_analysis_text", "order_analysis_text"],
            "doesNotApplyTo": ["android_system_asr", "cloud_asr_fallback", "tts", "context_fetch"]
        },
        "gates": {
            "beforeAsr": "never_check_ai_balance",
            "beforeTts": "never_check_ai_balance",
            "beforeContextFetch": "auth_and_limits_only",
            "beforeAiReplyGeneration": "check_balance_or_trial_credit"
        },
        "insufficientBalanceBehavior": {
            "block": ["ai_reply_generation"],
            "allow": ["asr", "tts", "context_fetch", "text_chat_without_model_generation"],
            "message": "AI 回复额度不足，请领取试用额度、充值或联系管理员；语音转文字和语音播放仍可继续使用。"
        }
    })
}

#[cfg(test)]
#[path = "external_app_chat_bootstrap_tests.rs"]
mod tests;

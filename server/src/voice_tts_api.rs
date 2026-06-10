//! 服务器 TTS API。
//!
//! - `GET /api/voice/tts/catalog`：返回女声、情绪、强度和 Worker 状态。
//! - `POST /api/voice/tts`：鉴权后合成音频；Worker 未配置时明确返回 503。

use axum::{
    body::Body,
    extract::State,
    http::{
        header::{CACHE_CONTROL, CONTENT_TYPE},
        HeaderMap, HeaderValue, StatusCode,
    },
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tracing::warn;

use crate::{
    agent_api_loop::resolve_agent,
    agent_llm_call::call_chat_llm,
    project_auth::{auth_from_headers, json_error},
    types::AppState,
    voice_tts_catalog::{self, ResolvedTtsStyle, TtsProvider},
    voice_tts_rewrite::{
        clean_llm_rewrite, prepare_text_for_speech, take_chars, MAX_TTS_TEXT_CHARS,
    },
    voice_tts_worker::{self, TtsWorkerConfig},
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsRequest {
    pub text: String,
    pub voice_id: Option<String>,
    pub emotion_id: Option<String>,
    pub intensity: Option<String>,
    pub provider: Option<TtsProvider>,
    pub rewrite: Option<bool>,
    pub agent_name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsCatalogResponse {
    pub worker_configured: bool,
    pub worker_url: Option<String>,
    pub default_provider: String,
    pub llm_rewrite_enabled: bool,
    /// 用户有在线 PC 节点且配置了模型 TTS Worker（CosyVoice3 / IndexTTS2 等高质量 AI 合成）
    pub pc_model_tts_available: bool,
    /// PC 节点实际使用的模型引擎（如 cosyvoice3），无 PC 节点时为 None
    pub pc_model_provider: Option<String>,
    pub voices: Vec<voice_tts_catalog::TtsVoicePreset>,
    pub emotions: Vec<voice_tts_catalog::TtsEmotionPreset>,
    pub intensities: Vec<voice_tts_catalog::TtsIntensityPreset>,
}

pub async fn catalog_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Json<TtsCatalogResponse> {
    let worker = TtsWorkerConfig::from_env();
    // 尝试鉴权，成功则查询该用户是否有在线 PC 节点提供模型 TTS
    let (pc_model_tts_available, pc_model_provider) = if let Ok(user) = auth_from_headers(&state, &headers) {
        match state.node_registry.find_tts_node_for_user(&user.id).await {
            Some((_node_id, _url)) => {
                // 查询该节点的具体提供方
                let provider = state.node_registry
                    .list_online().await
                    .into_iter()
                    .find(|n| n.owner_user_id == user.id && n.tts_worker_url.is_some())
                    .and_then(|n| {
                        // 根据服务器 env ELON_TTS_MODEL_PROVIDER 或默认 cosyvoice3
                        n.tts_worker_url.map(|_| 
                            std::env::var("ELON_TTS_MODEL_PROVIDER")
                                .unwrap_or_else(|_| "cosyvoice3".to_string())
                        )
                    });
                (true, provider)
            }
            None => (false, None),
        }
    } else {
        (false, None)
    };

    // 云端 worker 的 provider 标签：优先读实际 ELON_TTS_EDGE_DEFAULT_VOICE（如果配置了 edge-tts）
    // 否则记为实际的 worker type
    let effective_provider = if pc_model_tts_available {
        pc_model_provider.clone().unwrap_or_else(|| "cosyvoice3".to_string())
    } else {
        worker.as_ref()
            .map(|cfg| {
                // 如果 worker URL 是 edge-tts，返回 edge_tts，不再用 env ELON_TTS_PROVIDER
                let url = &cfg.base_url;
                if url.contains("5010") || url.contains("edge") {
                    "edge_tts".to_string()
                } else {
                    cfg.default_provider.as_worker_id().to_string()
                }
            })
            .unwrap_or_else(|| "auto".to_string())
    };

    Json(TtsCatalogResponse {
        worker_configured: worker.is_some() || pc_model_tts_available,
        worker_url: worker.as_ref().map(|cfg| cfg.base_url.clone()),
        default_provider: effective_provider,
        llm_rewrite_enabled: llm_rewrite_enabled(),
        pc_model_tts_available,
        pc_model_provider,
        voices: voice_tts_catalog::voices(),
        emotions: voice_tts_catalog::emotions(),
        intensities: voice_tts_catalog::intensities(),
    })
}

pub async fn synthesize_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<TtsRequest>,
) -> Response {
    let caller = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "未登录"),
    };

    let original_text = req.text.trim();
    if original_text.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "TTS 文本不能为空");
    }
    if original_text.chars().count() > MAX_TTS_TEXT_CHARS {
        return json_error(
            StatusCode::BAD_REQUEST,
            format!("TTS 文本过长，请控制在 {MAX_TTS_TEXT_CHARS} 字以内"),
        );
    }

    let style = voice_tts_catalog::resolve_style(
        req.voice_id.as_deref(),
        req.emotion_id.as_deref(),
        req.intensity.as_deref(),
        req.provider,
        original_text,
    );
    let mut spoken_text = prepare_text_for_speech(original_text, &style);
    if req.rewrite.unwrap_or(true) && llm_rewrite_enabled() {
        spoken_text = rewrite_with_llm(&state, &caller.id, &req, &style, &spoken_text).await;
    }
    let tts_key = crate::billing_lifecycle::new_compute_call_id("voice_tts_synthesis");
    let mut tts_billing_call = match crate::compute_usage::reserve_tts_synthesis(
        &state.store,
        &caller.id,
        &tts_key,
        "voice_tts_synthesis",
        style.provider.as_worker_id(),
        &spoken_text,
    ) {
        Ok(call) => call,
        Err(msg) => return json_error(StatusCode::PAYMENT_REQUIRED, msg),
    };

    // ── 优先级：PC 节点 GPU 模型 TTS > 云端 Worker TTS ──────────────────────
    // 若用户有在线 PC 节点且配置了 TTS Worker URL，优先走 PC 节点（高质量模型 TTS）
    if let Some((node_id, _tts_url)) = state.node_registry.find_tts_node_for_user(&caller.id).await
    {
        match state
            .agent_manager
            .dispatch_tts(
                &node_id,
                spoken_text.clone(),
                req.voice_id.clone(),
                req.emotion_id.clone(),
                req.intensity.clone(),
                req.provider.map(|p| p.as_worker_id().to_string()),
            )
            .await
        {
            Ok(homecli_proto::AgentToServer::TtsSynthesizeResponse {
                audio_b64,
                mime,
                worker_voice,
                ..
            }) => {
                use base64::engine::general_purpose::STANDARD as B64;
                use base64::Engine as _;
                match B64.decode(audio_b64.as_bytes()) {
                    Ok(bytes) => {
                        crate::compute_usage::record_tts_synthesis_with_key(
                            &state.store,
                            &caller.id,
                            "voice_tts_synthesis",
                            "pc_node",
                            &spoken_text,
                            bytes.len(),
                            Some(tts_billing_call.key()),
                        );
                        tts_billing_call.mark_settled();
                        let audio = voice_tts_worker::TtsAudio {
                            bytes,
                            content_type: mime,
                            cache_status: "miss",
                            worker: Some(node_id),
                            worker_voice,
                            worker_requested_voice: req.voice_id.clone(),
                            worker_fallback: None,
                        };
                        return audio_response(audio, &style);
                    }
                    Err(e) => warn!(target: "voice_tts", "PC 节点音频 base64 解码失败: {e}"),
                }
            }
            Ok(homecli_proto::AgentToServer::TtsSynthesizeError { message, .. }) => {
                warn!(target: "voice_tts", "PC 节点 TTS 失败，回退到云端 Worker: {message}");
            }
            Ok(_) => {}
            Err(e) => warn!(target: "voice_tts", "PC 节点 TTS 请求失败，回退到云端 Worker: {e:#}"),
        }
    }

    // 回退：云端本地 TTS Worker（edge-tts 等）
    if let Some(worker) = TtsWorkerConfig::from_env() {
        match voice_tts_worker::synthesize(&state, &worker, &style, original_text, &spoken_text)
            .await
        {
            Ok(audio) => {
                if audio.cache_status != "hit" {
                    crate::compute_usage::record_tts_synthesis_with_key(
                        &state.store,
                        &caller.id,
                        "voice_tts_synthesis",
                        style.provider.as_worker_id(),
                        &spoken_text,
                        audio.bytes.len(),
                        Some(tts_billing_call.key()),
                    );
                    tts_billing_call.mark_settled();
                } else {
                    tts_billing_call.release_no_usage();
                }
                return audio_response(audio, &style);
            }
            Err(err) => {
                warn!(target: "voice_tts", "云端 TTS Worker 失败，尝试 PC 节点: {err:#}");
            }
        }
    }

    json_error(
        StatusCode::SERVICE_UNAVAILABLE,
        "TTS 服务暂不可用（云端 Worker 未配置，且无在线 PC 节点提供 TTS）",
    )
}

async fn rewrite_with_llm(
    state: &Arc<AppState>,
    user_id: &str,
    req: &TtsRequest,
    style: &ResolvedTtsStyle,
    fallback_text: &str,
) -> String {
    let agent_name = req
        .agent_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let workspace = state.get_user_workspace(user_id);
    let agent = match resolve_agent(state, &workspace, agent_name).await {
        Ok(agent) => agent,
        Err(err) => {
            warn!(target: "voice_tts", "TTS LLM 改写没有可用代理: {err}");
            return fallback_text.to_string();
        }
    };

    let messages = vec![
        json!({
            "role": "system",
            "content": format!(
                "你是中文 TTS 台词改写器。把输入改成适合朗读的短台词，只输出改写文本。保持事实和含义，不新增承诺，不解释，不加引号。声音角色：{}。目标情绪：{}。文本风格：{}。最长 220 个汉字。",
                style.voice.label, style.emotion.label, style.emotion.text_style
            )
        }),
        json!({
            "role": "user",
            "content": fallback_text
        }),
    ];
    let response = match call_chat_llm(state, &agent, &messages, user_id, "voice_tts_rewrite").await
    {
        Ok(response) => response,
        Err(err) => {
            warn!(target: "voice_tts", "TTS LLM 改写失败: {err}");
            return fallback_text.to_string();
        }
    };
    let rewritten = response["choices"][0]["message"]["content"]
        .as_str()
        .map(|value| clean_llm_rewrite(value, fallback_text))
        .unwrap_or_else(|| fallback_text.to_string());
    take_chars(&rewritten, 220)
}

fn audio_response(audio: voice_tts_worker::TtsAudio, style: &ResolvedTtsStyle) -> Response {
    let mut response = Body::from(audio.bytes).into_response();
    *response.status_mut() = StatusCode::OK;
    let headers = response.headers_mut();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_str(&audio.content_type)
            .unwrap_or_else(|_| HeaderValue::from_static("audio/wav")),
    );
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=3600"),
    );
    insert_header(
        headers,
        "x-elon-tts-provider",
        style.provider.as_worker_id(),
    );
    insert_header(headers, "x-elon-tts-voice", style.voice.id);
    insert_header(headers, "x-elon-tts-emotion", style.emotion.id);
    insert_header(headers, "x-elon-tts-intensity", style.intensity.id);
    insert_header(headers, "x-elon-tts-cache", audio.cache_status);
    if let Some(worker) = audio.worker.as_deref() {
        insert_header(headers, "x-elon-tts-worker", worker);
    }
    if let Some(worker_voice) = audio.worker_voice.as_deref() {
        insert_header(headers, "x-elon-tts-worker-voice", worker_voice);
    }
    if let Some(requested_voice) = audio.worker_requested_voice.as_deref() {
        insert_header(
            headers,
            "x-elon-tts-worker-requested-voice",
            requested_voice,
        );
    }
    if let Some(fallback) = audio.worker_fallback {
        insert_header(
            headers,
            "x-elon-tts-worker-fallback",
            if fallback { "true" } else { "false" },
        );
    }
    response
}

fn insert_header(headers: &mut HeaderMap, name: &'static str, value: &str) {
    if let Ok(value) = HeaderValue::from_str(value) {
        headers.insert(name, value);
    }
}

fn llm_rewrite_enabled() -> bool {
    std::env::var("ELON_TTS_LLM_REWRITE_ENABLED")
        .map(|value| matches!(value.trim().to_lowercase().as_str(), "1" | "true" | "on"))
        .unwrap_or(false)
}

//! `POST /api/voice/asr` —— 接收完整音频文件，调本地 Whisper 或 OpenAI Whisper REST，返回转写文本。
//!
//! 客户端场景：手机本地 SpeechRecognizer 全部引擎失败后，把 MediaRecorder 录好的 AAC/M4A 文件
//! 直接上传到这里，服务器用 Whisper 转写后返回文字，供语音气泡携带原文使用。
//!
//! 鉴权：Bearer token（与 `/api/user/:id/speech/translate` 相同策略）
//!
//! 请求：`multipart/form-data`
//!   - `audio`  : 音频文件字节（任意格式，Whisper/OpenAI 均支持 m4a/aac/mp4/webm/ogg/wav/mp3）
//!   - `format` : 可选，MIME 或后缀提示（如 "audio/m4a"），默认 "audio/m4a"
//!
//! 响应：`{"text": "..."}`  ← 成功
//!         `{"error": "..."}` ← 失败（4xx/5xx）

use axum::{
    extract::{Multipart, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::sync::Arc;
use tracing::{info, warn};

use crate::{
    billing,
    project_auth::{auth_from_headers, json_error},
    types::AppState,
    voice_whisper_local, voice_whisper_rest,
};

#[derive(Serialize)]
struct AsrResponse {
    text: String,
}

/// 最大上传 10 MB（约 3 分钟 AAC 128kbps，语音消息通常 < 60 秒）
const MAX_AUDIO_BYTES: usize = 10 * 1024 * 1024;

pub async fn asr_upload_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Response {
    // 鉴权
    let caller = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "未登录"),
    };
    if let Err(msg) = billing::check_can_call(&state.store, &caller.id) {
        return json_error(StatusCode::PAYMENT_REQUIRED, msg);
    }

    // 解析 multipart
    let mut audio_bytes: Option<Vec<u8>> = None;
    let mut mime_hint = "audio/m4a".to_string();
    let mut language_override: Option<String> = None;
    let mut beam_size_override: Option<u8> = None;
    let mut vad_filter_override: Option<bool> = None;
    let mut condition_on_previous_override: Option<bool> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        let file_name = field.file_name().unwrap_or("").to_string();
        match name.as_str() {
            "audio" => {
                // 读取文件名中的扩展名作为格式提示
                if file_name.ends_with(".wav") {
                    mime_hint = "audio/wav".to_string();
                } else if file_name.ends_with(".mp4") || file_name.ends_with(".m4a") {
                    mime_hint = "audio/m4a".to_string();
                } else if file_name.ends_with(".ogg") || file_name.ends_with(".oga") {
                    mime_hint = "audio/ogg".to_string();
                } else if file_name.ends_with(".webm") {
                    mime_hint = "audio/webm".to_string();
                } else if file_name.ends_with(".mp3") {
                    mime_hint = "audio/mp3".to_string();
                }
                match field.bytes().await {
                    Ok(b) => {
                        if b.len() > MAX_AUDIO_BYTES {
                            return json_error(
                                StatusCode::PAYLOAD_TOO_LARGE,
                                "音频文件过大（最大 10 MB）",
                            );
                        }
                        audio_bytes = Some(b.to_vec());
                    }
                    Err(e) => {
                        warn!(target: "voice_asr", "读取 audio 字段失败: {e}");
                        return json_error(StatusCode::BAD_REQUEST, "读取音频数据失败");
                    }
                }
            }
            "format" => {
                if let Ok(s) = field.text().await {
                    let s = s.trim().to_string();
                    if !s.is_empty() {
                        mime_hint = s;
                    }
                }
            }
            "language" => {
                if let Ok(s) = field.text().await {
                    let s = s.trim().to_string();
                    if !s.is_empty() {
                        language_override = Some(s);
                    }
                }
            }
            "beam_size" => {
                if let Ok(s) = field.text().await {
                    if let Ok(v) = s.trim().parse::<u8>() {
                        beam_size_override = Some(v.min(10).max(1));
                    }
                }
            }
            "vad_filter" => {
                if let Ok(s) = field.text().await {
                    let s = s.trim().to_lowercase();
                    vad_filter_override = Some(s == "true" || s == "1");
                }
            }
            "condition_on_previous_text" => {
                if let Ok(s) = field.text().await {
                    let s = s.trim().to_lowercase();
                    condition_on_previous_override = Some(s == "true" || s == "1");
                }
            }
            _ => {}
        }
    }

    let audio = match audio_bytes {
        Some(b) if !b.is_empty() => b,
        _ => return json_error(StatusCode::BAD_REQUEST, "请求中缺少 audio 字段"),
    };

    info!(target: "voice_asr", "收到 ASR 上传请求，音频 {} 字节，格式 {}，语言 {:?}，beam_size {:?}，vad_filter {:?}，condition_prev {:?}",
          audio.len(), mime_hint, language_override, beam_size_override, vad_filter_override, condition_on_previous_override);

    // 优先级：本地 Whisper → OpenAI Whisper REST
    let text = transcribe_audio(
        &state,
        &audio,
        &mime_hint,
        language_override.as_deref(),
        beam_size_override,
        vad_filter_override,
        condition_on_previous_override,
    )
    .await;

    match text {
        Ok(t) if t.trim().is_empty() => {
            crate::compute_usage::record_encoded_asr(
                &state.store,
                &caller.id,
                "voice_asr_upload",
                "upload",
                audio.len(),
            );
            // Whisper 返回空串：可能是纯噪声/静音，不算错误
            Json(AsrResponse {
                text: String::new(),
            })
            .into_response()
        }
        Ok(t) => {
            crate::compute_usage::record_encoded_asr(
                &state.store,
                &caller.id,
                "voice_asr_upload",
                "upload",
                audio.len(),
            );
            info!(target: "voice_asr", "ASR 转写成功：{}", &t[..t.len().min(80)]);
            Json(AsrResponse { text: t }).into_response()
        }
        Err(e) => {
            warn!(target: "voice_asr", "ASR 转写失败: {e:#}");
            json_error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("语音识别服务暂不可用：{e}"),
            )
        }
    }
}

/// 统一转写入口：本地 Whisper > OpenAI Whisper REST。
///
/// 注意：本地 Whisper 的 `transcribe_pcm` 期望 WAV/PCM，而这里收到的是任意编码音频。
/// 本地 Whisper 服务（python-whisper/faster-whisper）通常支持多格式，
/// 但接口期望 WAV 头，因此我们对本地 Whisper 直接发原始字节（filename=audio.m4a）；
/// 如果本地 Whisper 服务不支持 M4A，会返回错误，此时自动降级到 Whisper REST。
async fn transcribe_audio(
    state: &Arc<AppState>,
    audio: &[u8],
    mime: &str,
    language: Option<&str>,
    beam_size: Option<u8>,
    vad_filter: Option<bool>,
    condition_on_previous_text: Option<bool>,
) -> anyhow::Result<String> {
    // --- Tier 1: 本地 Whisper（直接发文件字节） ---
    if let Some(mut cfg) = voice_whisper_local::WhisperLocalConfig::from_env() {
        // 用户指定的语言覆盖服务器环境变量默认值
        if let Some(lang) = language {
            cfg.language = lang.to_string();
        }
        if let Some(v) = beam_size {
            cfg.beam_size = v;
        }
        if let Some(v) = vad_filter {
            cfg.vad_filter = v;
        }
        if let Some(v) = condition_on_previous_text {
            cfg.condition_on_previous_text = v;
        }
        match transcribe_raw_via_local_whisper(&cfg, audio, mime).await {
            Ok(t) => return Ok(t),
            Err(e) => {
                warn!(target: "voice_asr", "本地 Whisper 失败，降级到 REST: {e:#}");
            }
        }
    }

    // --- Tier 2: OpenAI Whisper REST（各 AGENT_*_KEY / OPENAI_API_KEY / WHISPER_REST_KEY） ---
    let agents_cfg = state.agents_config.read().await;
    let candidates = voice_whisper_rest::WhisperRestCandidate::collect(&agents_cfg);
    drop(agents_cfg);
    if !candidates.is_empty() {
        return transcribe_raw_via_rest(&candidates, audio, mime).await;
    }

    anyhow::bail!("未配置任何 ASR 后端（WHISPER_LOCAL_URL / OPENAI_API_KEY 均未设置）")
}

/// 把原始音频字节直接 POST 到本地 Whisper `/transcribe`（不做 PCM 转换）。
async fn transcribe_raw_via_local_whisper(
    cfg: &voice_whisper_local::WhisperLocalConfig,
    audio: &[u8],
    mime: &str,
) -> anyhow::Result<String> {
    use anyhow::Context;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Resp {
        transcript: String,
    }

    // 推断文件扩展名，本地 Whisper 服务可能用它判断解码器
    let ext = mime_to_ext(mime);
    let filename = format!("audio.{ext}");

    let audio_part = reqwest::multipart::Part::bytes(audio.to_vec())
        .file_name(filename)
        .mime_str(mime)
        .context("设置 MIME 失败")?;
    let lang_part = reqwest::multipart::Part::text(cfg.language.clone());
    let beam_part = reqwest::multipart::Part::text(cfg.beam_size.to_string());
    let vad_part = reqwest::multipart::Part::text(cfg.vad_filter.to_string());
    let cond_part = reqwest::multipart::Part::text(cfg.condition_on_previous_text.to_string());
    let form = reqwest::multipart::Form::new()
        .part("audio", audio_part)
        .part("language", lang_part)
        .part("beam_size", beam_part)
        .part("vad_filter", vad_part)
        .part("condition_on_previous_text", cond_part);

    let url = format!("{}/transcribe", cfg.base_url);
    let resp = reqwest::Client::new()
        .post(&url)
        .multipart(form)
        .timeout(std::time::Duration::from_secs(90))
        .send()
        .await
        .context("本地 Whisper HTTP 请求失败")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("本地 Whisper 错误 {status}: {body}");
    }

    let parsed: Resp = resp.json().await.context("解析本地 Whisper 响应失败")?;
    Ok(parsed.transcript.trim().to_string())
}

/// 把原始音频字节直接 POST 到 OpenAI `/v1/audio/transcriptions`。
async fn transcribe_raw_via_rest(
    candidates: &[voice_whisper_rest::WhisperRestCandidate],
    audio: &[u8],
    mime: &str,
) -> anyhow::Result<String> {
    use anyhow::Context;

    // 从 WhisperRestCandidate 拿到配置（base_url + api_key + language）
    let mut errors: Vec<String> = Vec::new();
    for c in candidates {
        match c.transcribe_raw(audio, mime).await {
            Ok(t) => return Ok(t),
            Err(e) => errors.push(format!("[{}] {}", c.label, e)),
        }
    }
    anyhow::bail!("Whisper REST 所有候选均失败：{}", errors.join("; "))
}

fn mime_to_ext(mime: &str) -> &'static str {
    match mime {
        "audio/wav" | "audio/x-wav" => "wav",
        "audio/mp4" | "audio/m4a" | "audio/x-m4a" => "m4a",
        "audio/ogg" | "audio/oga" => "ogg",
        "audio/webm" => "webm",
        "audio/mp3" | "audio/mpeg" => "mp3",
        "audio/aac" => "aac",
        _ => "m4a",
    }
}

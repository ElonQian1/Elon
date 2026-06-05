//! 独立 TTS Worker 适配层。
//!
//! Rust 主服务只负责协议、鉴权、缓存和调用。IndexTTS2/CosyVoice/GPT-SoVITS
//! 由 Python/ONNX Worker 暴露 HTTP 接口，避免模型依赖污染主进程发布链路。

use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::fs;

use crate::{
    types::AppState,
    voice_tts_catalog::{ResolvedTtsStyle, TtsProvider},
};

#[derive(Debug, Clone)]
pub struct TtsWorkerConfig {
    pub base_url: String,
    pub default_provider: TtsProvider,
    pub bearer_token: Option<String>,
    pub timeout: Duration,
    pub cache_enabled: bool,
}

impl TtsWorkerConfig {
    pub fn from_env() -> Option<Self> {
        let base_url = std::env::var("ELON_TTS_WORKER_URL")
            .ok()
            .map(|value| value.trim().trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty())?;
        let default_provider = std::env::var("ELON_TTS_PROVIDER")
            .ok()
            .map(|value| TtsProvider::from_env_value(&value))
            .unwrap_or(TtsProvider::Auto);
        let timeout_secs = std::env::var("ELON_TTS_TIMEOUT_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(120);
        let cache_enabled = std::env::var("ELON_TTS_CACHE_ENABLED")
            .map(|value| !matches!(value.trim().to_lowercase().as_str(), "0" | "false" | "off"))
            .unwrap_or(true);

        Some(Self {
            base_url,
            default_provider,
            bearer_token: std::env::var("ELON_TTS_WORKER_TOKEN")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            timeout: Duration::from_secs(timeout_secs),
            cache_enabled,
        })
    }
}

#[derive(Debug, Clone)]
pub struct TtsAudio {
    pub bytes: Vec<u8>,
    pub content_type: String,
    pub cache_status: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkerRequest<'a> {
    provider: &'a str,
    text: &'a str,
    original_text: &'a str,
    voice_id: &'a str,
    voice_label: &'a str,
    voice_prompt: &'a str,
    voice_audio: &'a str,
    emotion_id: &'a str,
    emotion_label: &'a str,
    emotion_audio: &'a str,
    text_style: &'a str,
    pause_style: &'a str,
    intensity: &'a str,
    emo_alpha: f32,
    speed: f32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkerJsonResponse {
    audio_base64: String,
    mime: Option<String>,
}

pub async fn synthesize(
    state: &Arc<AppState>,
    cfg: &TtsWorkerConfig,
    style: &ResolvedTtsStyle,
    original_text: &str,
    spoken_text: &str,
) -> Result<TtsAudio> {
    let provider = effective_provider(cfg, style).as_worker_id();
    let worker_req = WorkerRequest {
        provider,
        text: spoken_text,
        original_text,
        voice_id: style.voice.id,
        voice_label: style.voice.label,
        voice_prompt: style.voice.role_prompt,
        voice_audio: style.voice.prompt_audio,
        emotion_id: style.emotion.id,
        emotion_label: style.emotion.label,
        emotion_audio: style.emotion.emotion_audio,
        text_style: style.emotion.text_style,
        pause_style: style.emotion.pause_style,
        intensity: style.intensity.id,
        emo_alpha: style.emo_alpha,
        speed: style.emotion.speed,
    };

    let cache_key = cache_key(&worker_req)?;
    if cfg.cache_enabled {
        if let Some(hit) = read_cache(state, &cache_key).await? {
            return Ok(hit);
        }
    }

    let audio = call_worker(state, cfg, &worker_req).await?;
    if cfg.cache_enabled {
        write_cache(state, &cache_key, &audio).await?;
    }
    Ok(audio)
}

fn effective_provider(cfg: &TtsWorkerConfig, style: &ResolvedTtsStyle) -> TtsProvider {
    match cfg.default_provider {
        TtsProvider::Auto => style.provider,
        provider => provider,
    }
}

async fn call_worker(
    state: &Arc<AppState>,
    cfg: &TtsWorkerConfig,
    worker_req: &WorkerRequest<'_>,
) -> Result<TtsAudio> {
    let url = format!("{}/synthesize", cfg.base_url);
    let mut request = state
        .http_client
        .post(&url)
        .timeout(cfg.timeout)
        .json(worker_req);
    if let Some(token) = &cfg.bearer_token {
        request = request.bearer_auth(token);
    }

    let response = request.send().await.context("TTS Worker 请求失败")?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("TTS Worker 返回错误 {status}: {body}");
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("audio/wav")
        .to_string();

    if content_type.starts_with("application/json") {
        let parsed: WorkerJsonResponse = response
            .json()
            .await
            .context("解析 TTS Worker JSON 响应失败")?;
        let bytes = general_purpose::STANDARD
            .decode(parsed.audio_base64.as_bytes())
            .context("TTS Worker audio_base64 解码失败")?;
        return Ok(TtsAudio {
            bytes,
            content_type: parsed.mime.unwrap_or_else(|| "audio/wav".to_string()),
            cache_status: "miss",
        });
    }

    let bytes = response
        .bytes()
        .await
        .context("读取 TTS Worker 音频响应失败")?
        .to_vec();
    Ok(TtsAudio {
        bytes,
        content_type,
        cache_status: "miss",
    })
}

fn cache_key(worker_req: &WorkerRequest<'_>) -> Result<String> {
    let payload = serde_json::to_vec(worker_req).context("序列化 TTS 缓存键失败")?;
    let mut hasher = Sha256::new();
    hasher.update(payload);
    Ok(hex::encode(hasher.finalize()))
}

async fn read_cache(state: &Arc<AppState>, key: &str) -> Result<Option<TtsAudio>> {
    let dir = cache_dir(state, key);
    for (name, content_type) in [
        ("audio.wav", "audio/wav"),
        ("audio.mp3", "audio/mpeg"),
        ("audio.ogg", "audio/ogg"),
        ("audio.m4a", "audio/mp4"),
    ] {
        let path = dir.join(name);
        if fs::metadata(&path).await.is_ok() {
            let bytes = fs::read(path).await.context("读取 TTS 缓存失败")?;
            return Ok(Some(TtsAudio {
                bytes,
                content_type: content_type.to_string(),
                cache_status: "hit",
            }));
        }
    }
    Ok(None)
}

async fn write_cache(state: &Arc<AppState>, key: &str, audio: &TtsAudio) -> Result<()> {
    let dir = cache_dir(state, key);
    fs::create_dir_all(&dir)
        .await
        .context("创建 TTS 缓存目录失败")?;
    let path = dir.join(cache_file_name(&audio.content_type));
    fs::write(path, &audio.bytes)
        .await
        .context("写入 TTS 缓存失败")?;
    Ok(())
}

fn cache_dir(state: &Arc<AppState>, key: &str) -> PathBuf {
    let prefix = key.get(0..2).unwrap_or("xx");
    state.data_dir.join("tts_cache").join(prefix).join(key)
}

fn cache_file_name(content_type: &str) -> &'static str {
    if content_type.contains("mpeg") || content_type.contains("mp3") {
        "audio.mp3"
    } else if content_type.contains("ogg") {
        "audio.ogg"
    } else if content_type.contains("mp4") || content_type.contains("m4a") {
        "audio.m4a"
    } else {
        "audio.wav"
    }
}

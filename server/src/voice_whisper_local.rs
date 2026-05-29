//! 本地 Whisper HTTP 转写客户端。
//!
//! 当环境变量 `WHISPER_LOCAL_URL` 存在时，`voice_ws_transcribe.rs` 使用本模块
//! 代替 OpenAI Realtime API，实现免费的离线 ASR。
//!
//! 流程：
//!   1. 客户端持续发来 PCM16 二进制帧 → 服务端缓冲
//!   2. 客户端发 `commit` → 本模块把缓冲区打包成 WAV → POST /transcribe
//!   3. 返回完整文本（非流式，延迟约 1-3s，对讲模式下可接受）

use anyhow::{Context, Result};
use serde::Deserialize;

/// 本地 Whisper 服务配置，从环境变量读取。
#[derive(Debug, Clone)]
pub struct WhisperLocalConfig {
    /// Whisper 服务 base URL，如 `http://127.0.0.1:5001`。
    pub base_url: String,
    /// 转写目标语言，默认 `zh`。
    pub language: String,
    /// beam_size：解码宽度，1=最快(贪心) 5=平衡 10=最准，默认 5。
    pub beam_size: u8,
    /// vad_filter：是否启用静音过滤，默认 true。
    pub vad_filter: bool,
    /// condition_on_previous_text：是否参考上一句识别结果，默认 false。
    pub condition_on_previous_text: bool,
}

impl WhisperLocalConfig {
    /// 如果 `WHISPER_LOCAL_URL` 环境变量存在则返回 Some，否则 None（回退到 OpenAI Realtime）。
    pub fn from_env() -> Option<Self> {
        let url = std::env::var("WHISPER_LOCAL_URL").ok()?;
        Some(Self {
            base_url: url.trim_end_matches('/').to_string(),
            language: std::env::var("ELON_VOICE_TRANSCRIBE_LANGUAGE")
                .unwrap_or_else(|_| "zh".to_string()),
            beam_size: 5,
            vad_filter: true,
            condition_on_previous_text: false,
        })
    }
}

#[derive(Deserialize)]
struct WhisperResponse {
    transcript: String,
}

/// 把 PCM16 LE 字节打包成 WAV，POST 到本地 Whisper `/transcribe`，返回转写文本。
///
/// - `pcm`         : 原始 PCM16 LE 字节（无 WAV 头）
/// - `sample_rate` : 采样率（Android 端发 24000）
/// - `channels`    : 声道数（通常为 1）
pub async fn transcribe_pcm(
    cfg: &WhisperLocalConfig,
    pcm: &[u8],
    sample_rate: u32,
    channels: u16,
) -> Result<String> {
    if pcm.is_empty() {
        return Ok(String::new());
    }

    let wav_bytes = pcm_to_wav(pcm, sample_rate, channels);

    let audio_part = reqwest::multipart::Part::bytes(wav_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .context("设置 MIME 类型失败")?;
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
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await
        .context("Whisper HTTP 请求失败")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Whisper 服务错误 {status}: {body}");
    }

    let parsed: WhisperResponse = resp.json().await.context("解析 Whisper 响应失败")?;
    Ok(parsed.transcript.trim().to_string())
}

/// 将裸 PCM16 LE 字节封装成合法 WAV 文件字节（内存，无 IO）。
fn pcm_to_wav(pcm: &[u8], sample_rate: u32, channels: u16) -> Vec<u8> {
    let data_len = pcm.len() as u32;
    let byte_rate = sample_rate * channels as u32 * 2; // PCM16 = 2 bytes/sample
    let block_align = channels * 2;

    let mut wav = Vec::with_capacity(44 + pcm.len());
    // RIFF 头
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    // fmt chunk
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // chunk size
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM format
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    // data chunk
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend_from_slice(pcm);
    wav
}

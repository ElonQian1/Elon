//! Tier 3 降级 ASR：OpenAI 兼容的 Whisper REST API。
//!
//! 当 Tier 1（本地 Whisper）和 Tier 2（OpenAI Realtime WS）都不可用时启用。
//!
//! 协议：`POST /v1/audio/transcriptions` （multipart/form-data），
//! 返回 `{"text": "..."}` 或 OpenAI 标准错误格式。
//!
//! **Key 解析顺序**（`WhisperRestClient::resolve` 依次尝试）：
//!   1. `OPENAI_API_KEY`  + `https://api.openai.com/v1`
//!   2. `WHISPER_REST_KEY` + `WHISPER_REST_URL`（自定义端点，可指向代理/兼容服务）
//!   3. `AGENT_OPENAI_KEY` 等已注册的 OpenAI-compatible agent（api_base 含 openai.com）
//!   4. 所有 agent key，依序尝试并以第一个成功者为准（运行时 round-robin fallback）
//!
//! 这样，只要服务器配置了任意一个有 Whisper 能力的 key，转写就能工作。

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::agent_config::AgentsConfig;

/// REST 转写配置（已解析好 base_url + api_key + model）。
#[derive(Debug, Clone)]
pub struct WhisperRestCandidate {
    pub label: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub language: String,
}

impl WhisperRestCandidate {
    /// 从环境变量 + agents 配置中收集所有候选，调用方按序尝试。
    pub fn collect(agents_cfg: &AgentsConfig) -> Vec<Self> {
        let lang = std::env::var("ELON_VOICE_TRANSCRIBE_LANGUAGE")
            .unwrap_or_else(|_| "zh".to_string());
        let mut candidates: Vec<Self> = Vec::new();

        // ── 1. OPENAI_API_KEY（官方端点，优先） ──
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            if !key.trim().is_empty() {
                candidates.push(Self {
                    label: "openai-env".into(),
                    base_url: "https://api.openai.com/v1".into(),
                    api_key: key,
                    model: std::env::var("WHISPER_REST_MODEL")
                        .unwrap_or_else(|_| "whisper-1".into()),
                    language: lang.clone(),
                });
            }
        }

        // ── 2. WHISPER_REST_KEY + WHISPER_REST_URL（自定义代理/兼容服务） ──
        if let (Ok(key), Ok(url)) = (
            std::env::var("WHISPER_REST_KEY"),
            std::env::var("WHISPER_REST_URL"),
        ) {
            if !key.trim().is_empty() && !url.trim().is_empty() {
                candidates.push(Self {
                    label: "whisper-rest-custom".into(),
                    base_url: url.trim_end_matches('/').to_string(),
                    api_key: key,
                    model: std::env::var("WHISPER_REST_MODEL")
                        .unwrap_or_else(|_| "whisper-1".into()),
                    language: lang.clone(),
                });
            }
        }

        // ── 3. 已注册的 agents，按 OpenAI 兼容性排序 ──
        // openai.com 优先（若 OPENAI_API_KEY 没有但 AGENT_OPENAI_KEY 有）；
        // 其次其他 OpenAI 兼容端点（如 Together、Groq、DeepSeek 等）；
        // 最后所有其他（如 Hunyuan/TokenHub —— 通常不支持 /v1/audio/transcriptions，
        // 但如果它们接入了代理服务就能用，尝试无副作用）。
        let mut openai_agents: Vec<&crate::agent_config::AgentConfig> = Vec::new();
        let mut other_agents: Vec<&crate::agent_config::AgentConfig> = Vec::new();

        for cfg in agents_cfg.agents.values() {
            if cfg.api_base.contains("openai.com") {
                openai_agents.push(cfg);
            } else {
                other_agents.push(cfg);
            }
        }
        openai_agents.sort_by_key(|a| a.name.clone());
        other_agents.sort_by_key(|a| a.name.clone());

        for cfg in openai_agents.into_iter().chain(other_agents.into_iter()) {
            // 跳过已经添加过的（OPENAI_API_KEY 可能与 AGENT_OPENAI_KEY 相同）
            if candidates.iter().any(|c| c.api_key == cfg.api_key) {
                continue;
            }
            candidates.push(Self {
                label: format!("agent-{}", cfg.name),
                base_url: cfg.api_base.trim_end_matches('/').to_string(),
                api_key: cfg.api_key.clone(),
                model: "whisper-1".into(), // 通用；若 agent 用其他模型可通过 WHISPER_REST_MODEL 覆盖
                language: lang.clone(),
            });
        }

        candidates
    }

    /// 把 PCM16 LE 字节转写为文字。
    ///
    /// 返回 `Ok(String)` 表示成功（可能为空串）；`Err` 表示本 candidate 不可用，调用方应尝试下一个。
    pub async fn transcribe(&self, pcm: &[u8], sample_rate: u32, channels: u16) -> Result<String> {
        if pcm.is_empty() {
            return Ok(String::new());
        }

        let wav = pcm_to_wav(pcm, sample_rate, channels);
        let url = format!("{}/audio/transcriptions", self.base_url);

        let audio_part = reqwest::multipart::Part::bytes(wav)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .context("设置 MIME 失败")?;
        let form = reqwest::multipart::Form::new()
            .part("file", audio_part)
            .text("model", self.model.clone())
            .text("language", self.language.clone())
            .text("response_format", "json");

        let resp = reqwest::Client::new()
            .post(&url)
            .bearer_auth(&self.api_key)
            .multipart(form)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .with_context(|| format!("[{}] HTTP 请求失败 {url}", self.label))?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            anyhow::bail!("[{}] 转写失败 {status}: {body}", self.label);
        }

        // 兼容 {"text":"..."} 和 {"transcript":"..."} 两种格式
        #[derive(Deserialize)]
        struct Resp {
            text: Option<String>,
            transcript: Option<String>,
        }
        let parsed: Resp = serde_json::from_str(&body)
            .with_context(|| format!("[{}] 解析响应失败: {body}", self.label))?;
        let text = parsed
            .text
            .or(parsed.transcript)
            .unwrap_or_default()
            .trim()
            .to_string();
        Ok(text)
    }
}

/// 依次尝试所有候选，返回第一个成功的转写结果。
///
/// 所有候选失败时返回包含所有错误原因的 `Err`。
pub async fn transcribe_with_fallback(
    candidates: &[WhisperRestCandidate],
    pcm: &[u8],
    sample_rate: u32,
    channels: u16,
) -> Result<String> {
    if candidates.is_empty() {
        anyhow::bail!("无可用的 Whisper REST 服务（未配置 OPENAI_API_KEY / WHISPER_REST_KEY）");
    }

    let mut errors: Vec<String> = Vec::new();
    for c in candidates {
        match c.transcribe(pcm, sample_rate, channels).await {
            Ok(text) => {
                if !errors.is_empty() {
                    tracing::warn!(
                        target: "voice",
                        label = %c.label,
                        "Whisper REST 前序候选失败，当前候选成功：{}",
                        errors.join("; ")
                    );
                }
                tracing::info!(target: "voice", label = %c.label, "Whisper REST 转写成功");
                return Ok(text);
            }
            Err(e) => {
                tracing::warn!(target: "voice", label = %c.label, "候选失败: {e:#}");
                errors.push(format!("{}: {}", c.label, e));
            }
        }
    }
    anyhow::bail!("所有 Whisper REST 候选均失败：{}", errors.join(" | "))
}

// ── WAV 封装（复用自 voice_whisper_local） ────────────────────────────────────

fn pcm_to_wav(pcm: &[u8], sample_rate: u32, channels: u16) -> Vec<u8> {
    let data_len = pcm.len() as u32;
    let byte_rate = sample_rate * channels as u32 * 2;
    let block_align = channels * 2;

    let mut wav = Vec::with_capacity(44 + pcm.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend_from_slice(pcm);
    wav
}

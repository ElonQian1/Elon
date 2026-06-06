//! 实时语音管线的统一配置。
//!
//! 设计原则（长期主义）：
//! - 所有可调参数走环境变量，便于不同部署环境（PC/服务器/CI）使用同一份代码
//! - 默认值与 OpenAI Realtime Transcription 文档对齐：24kHz / mono / PCM16
//! - 配置体积保持小巧，避免演化成"超大配置中心"

use std::env;

use crate::agent_config::AgentsConfig;

/// 实时音频采样率（Hz）。
/// OpenAI Realtime Transcription `audio/pcm` 要求 24kHz。
pub const REALTIME_SAMPLE_RATE_HZ: u32 = 24_000;

/// 实时音频声道数。
pub const REALTIME_CHANNELS: u16 = 1;

/// 单个 PCM16 样本的字节数。
pub const PCM16_BYTES_PER_SAMPLE: usize = 2;

/// 每帧推荐时长（毫秒）。Android 端按这个值切帧。
pub const DEFAULT_FRAME_MS: u32 = 40;

/// 单连接最大队列字节数，防止恶意客户端撑爆内存。
pub const MAX_BUFFERED_BYTES: usize = 4 * 1024 * 1024;

/// 方案 A（虚拟麦克风）配置。
#[derive(Debug, Clone)]
pub struct VirtualMicConfig {
    /// `pw-cat` 可执行文件路径。
    pub pwcat_path: String,
    /// 目标 sink 名称，对应 `pactl load-module module-null-sink sink_name=<>`。
    pub target_sink: String,
    /// 一句话结束后补的静音时长（毫秒），帮助 Codex 判断句尾。
    pub end_silence_ms: u64,
}

impl VirtualMicConfig {
    pub fn from_env() -> Self {
        Self {
            pwcat_path: env::var("ELON_PWCAT_PATH").unwrap_or_else(|_| "pw-cat".to_string()),
            target_sink: env::var("ELON_VOICE_SINK").unwrap_or_else(|_| "codex_sink".to_string()),
            end_silence_ms: env::var("ELON_VOICE_END_SILENCE_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(700),
        }
    }
}

/// 方案 B（Realtime 转写）配置。
#[derive(Debug, Clone)]
pub struct RealtimeTranscribeConfig {
    /// OpenAI Realtime WebSocket URL。
    pub ws_url: String,
    /// 转写模型，例如 `gpt-4o-mini-transcribe` / `whisper-1`。
    pub model: String,
    /// 转写语言，默认中文。
    pub language: String,
    /// API Key 环境变量名（避免在内存里到处复制密钥）。
    pub api_key_env: String,
}

impl RealtimeTranscribeConfig {
    pub fn from_env() -> Self {
        Self {
            ws_url: env::var("ELON_VOICE_REALTIME_URL").unwrap_or_else(|_| {
                "wss://api.openai.com/v1/realtime?intent=transcription".to_string()
            }),
            model: env::var("ELON_VOICE_TRANSCRIBE_MODEL")
                .unwrap_or_else(|_| "gpt-4o-mini-transcribe".to_string()),
            language: env::var("ELON_VOICE_TRANSCRIBE_LANGUAGE")
                .unwrap_or_else(|_| "zh".to_string()),
            api_key_env: env::var("ELON_VOICE_API_KEY_ENV")
                .unwrap_or_else(|_| "OPENAI_API_KEY".to_string()),
        }
    }

    /// 从环境变量读取 API Key。
    pub fn read_api_key(&self) -> Option<String> {
        env::var(&self.api_key_env)
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    }
}

/// 方案 C（Realtime 全双工语音对话）配置。
#[derive(Debug, Clone)]
pub struct RealtimeChatConfig {
    /// OpenAI Realtime WebSocket URL，可只填基础 URL，代码会自动追加 model 参数。
    pub ws_url: String,
    /// 语音到语音模型，例如 `gpt-realtime-2`。
    pub model: String,
    /// 输出音色。
    pub voice: String,
    /// API Key 环境变量名。
    pub api_key_env: String,
}

impl RealtimeChatConfig {
    pub fn from_env() -> Self {
        let model = env::var("ELON_VOICE_REALTIME_CHAT_MODEL")
            .unwrap_or_else(|_| "gpt-realtime".to_string());
        Self {
            ws_url: env::var("ELON_VOICE_REALTIME_CHAT_URL")
                .unwrap_or_else(|_| "wss://api.openai.com/v1/realtime".to_string()),
            model: normalize_realtime_chat_model(&model).to_string(),
            voice: env::var("ELON_VOICE_REALTIME_CHAT_VOICE")
                .unwrap_or_else(|_| "marin".to_string()),
            api_key_env: env::var("ELON_VOICE_API_KEY_ENV")
                .unwrap_or_else(|_| "OPENAI_API_KEY".to_string()),
        }
    }

    pub fn websocket_url(&self) -> String {
        if self.ws_url.contains("model=") {
            self.ws_url.clone()
        } else {
            let separator = if self.ws_url.contains('?') { '&' } else { '?' };
            format!("{}{}model={}", self.ws_url, separator, self.model)
        }
    }

    pub fn read_api_key_from_agents(&self, agents_cfg: &AgentsConfig) -> Option<String> {
        read_trimmed_env(&self.api_key_env)
            .or_else(|| read_trimmed_env("OPENAI_API_KEY"))
            .or_else(|| read_trimmed_env("AGENT_OPENAI_KEY"))
            .or_else(|| {
                agents_cfg
                    .agents
                    .values()
                    .filter(|agent| agent.api_base.contains("openai.com"))
                    .map(|agent| agent.api_key.trim().to_string())
                    .find(|key| !key.is_empty())
            })
    }

    pub fn missing_key_message(&self) -> String {
        let mut envs = vec![
            self.api_key_env.as_str(),
            "OPENAI_API_KEY",
            "AGENT_OPENAI_KEY",
        ];
        envs.dedup();
        format!(
            "服务器未配置 OpenAI Realtime API Key，请设置 {}，或在 agents.json 中配置 api_base 为 openai.com 的 OpenAI 代理",
            envs.join(" / ")
        )
    }
}

fn read_trimmed_env(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn normalize_realtime_chat_model(model: &str) -> &str {
    match model.trim() {
        // 早期内部配置误写为 gpt-realtime-2；官方 GA Realtime WebSocket
        // 模型名是 gpt-realtime，继续传旧值会在建连阶段失败。
        "gpt-realtime-2" => "gpt-realtime",
        trimmed => trimmed,
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_realtime_chat_model;

    #[test]
    fn realtime_chat_model_uses_ga_default_name() {
        assert_eq!(
            normalize_realtime_chat_model("gpt-realtime"),
            "gpt-realtime"
        );
    }

    #[test]
    fn realtime_chat_model_maps_legacy_invalid_alias() {
        assert_eq!(
            normalize_realtime_chat_model("gpt-realtime-2"),
            "gpt-realtime"
        );
    }
}

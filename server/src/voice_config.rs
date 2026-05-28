//! 实时语音管线的统一配置。
//!
//! 设计原则（长期主义）：
//! - 所有可调参数走环境变量，便于不同部署环境（PC/服务器/CI）使用同一份代码
//! - 默认值与 OpenAI Realtime Transcription 文档对齐：24kHz / mono / PCM16
//! - 配置体积保持小巧，避免演化成"超大配置中心"

use std::env;

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
            pwcat_path: env::var("ELON_PWCAT_PATH")
                .unwrap_or_else(|_| "pw-cat".to_string()),
            target_sink: env::var("ELON_VOICE_SINK")
                .unwrap_or_else(|_| "codex_sink".to_string()),
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

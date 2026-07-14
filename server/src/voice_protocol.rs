//! 实时语音 WebSocket 的统一应用层协议。
//!
//! 设计原则：
//! - 二进制帧 = 裸 PCM16 LE 音频数据，不做任何包装（最少开销）
//! - 文本帧 = JSON 控制消息，`type` 字段必填
//! - 客户端→服务器、服务器→客户端的消息**类型完全分离**，互不混淆
//! - 协议字段保持 snake_case，与现有 router 风格一致

use serde::{Deserialize, Serialize};

pub const VOICE_TARGET_SOCIAL_AI_DIRECT: &str = "social_ai_direct";
/// 只做实时转写，不自动投递给项目、好友或群聊。
pub const VOICE_TARGET_TRANSCRIBE_ONLY: &str = "transcribe_only";
/// 把转写文本作为群消息发送，适合 fb2 等外部应用复用主项目群聊体验。
pub const VOICE_TARGET_EXTERNAL_GROUP: &str = "external_group";
/// 悬浮球手机控制 target：语音识别后由 OpenAI Realtime 处理，
/// AI 回复手机自动化 JSON 脚本或闲聊文本。
pub const VOICE_TARGET_PHONE_CONTROL: &str = "phone_control";

/// 客户端（Android）发给服务器的控制消息。
///
/// 二进制帧承载音频；这里只定义文本帧。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientControl {
    /// 客户端的会话握手；服务器据此选择投递目标。
    Hello {
        user_id: String,
        /// 可选：语音转写后的投递目标。
        /// 缺省时沿用项目聊天；`social_ai_direct` 表示一龙AI 私聊。
        target: Option<String>,
        /// 可选：把转写文本投递到指定项目对话（方案 B 使用）。
        project_id: Option<String>,
        conversation_id: Option<String>,
        /// 可选：把转写文本投递到指定群聊；`external_group` target 必填。
        group_id: Option<String>,
        /// 客户端声明的采样率，必须等于 24000；不等于则服务器拒绝。
        sample_rate: u32,
        channels: u16,
    },
    /// 提交当前音频缓冲：方案 B → 触发转写；方案 A → 写入静音并尝试结束句子。
    Commit,
    /// 显式关闭：客户端主动收尾。
    Close,
}

/// 服务器发回客户端的事件。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerEvent {
    /// 握手成功。
    Ready { mode: &'static str },
    /// 转写中间结果（方案 B）。
    TranscriptDelta { text: String },
    /// 转写最终结果（方案 B）。
    TranscriptFinal { text: String },
    /// 已把音频写入虚拟麦（方案 A）。
    VirtualMicFed { bytes: u64 },
    /// CLI 投递结果（方案 B 转写完成后）。
    CliDispatched { ok: bool, message: String },
    /// Realtime Chat：检测到用户开始说话，客户端应立即清空未播放的 AI 音频。
    RealtimeSpeechStarted,
    /// Realtime Chat：检测到用户停止说话。
    RealtimeSpeechStopped,
    /// Realtime Chat：AI 回复字幕增量。
    RealtimeAiTranscriptDelta { text: String },
    /// Realtime Chat：AI 回复字幕完成。
    RealtimeAiTranscriptDone { text: String },
    /// Realtime Chat：一次模型回复完成。
    RealtimeResponseDone,
    /// 通用错误。
    Error { code: &'static str, message: String },
}

impl ServerEvent {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            r#"{"type":"error","code":"serialize","message":"serialize failed"}"#.to_string()
        })
    }
}

pub fn resolve_authenticated_voice_user(
    authenticated_user_id: &str,
    claimed_user_id: String,
) -> Result<String, String> {
    if authenticated_user_id == "local-owner" {
        return Ok(claimed_user_id);
    }
    if authenticated_user_id == claimed_user_id {
        Ok(authenticated_user_id.to_string())
    } else {
        Err("登录用户与语音会话 user_id 不一致".to_string())
    }
}

#[cfg(test)]
#[path = "voice_protocol_tests.rs"]
mod tests;

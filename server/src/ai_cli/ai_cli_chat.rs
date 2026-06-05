use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

use super::{
    ai_cli_process::{configured_timeout_cap, is_cli_timeout_error},
    ai_cli_trace::record_lightweight_chat_fallback,
    IntentGateResult,
};
use crate::{
    intent_router,
    types::{AppState, WsMessage},
};

const DEFAULT_CHAT_TIMEOUT_CAP_SECS: u64 = 30;
pub(crate) const DEFAULT_TINY_CHAT_TIMEOUT_CAP_SECS: u64 = 8;

pub(crate) fn chat_timeout_cap_secs(tiny_chat_task: bool) -> u64 {
    if tiny_chat_task {
        configured_timeout_cap(
            "AI_CLI_TINY_CHAT_TIMEOUT_SECS",
            DEFAULT_TINY_CHAT_TIMEOUT_CAP_SECS,
        )
    } else {
        configured_timeout_cap("AI_CLI_CHAT_TIMEOUT_SECS", DEFAULT_CHAT_TIMEOUT_CAP_SECS)
    }
}

pub(crate) fn is_tiny_chat_message(user_message: &str) -> bool {
    let compact: String = user_message
        .trim()
        .to_lowercase()
        .chars()
        .filter(|ch| {
            !ch.is_whitespace()
                && !matches!(
                    ch,
                    '!' | '！'
                        | '?'
                        | '？'
                        | '.'
                        | '。'
                        | ','
                        | '，'
                        | ';'
                        | '；'
                        | ':'
                        | '：'
                        | '~'
                        | '～'
                )
        })
        .take(32)
        .collect();
    if compact.is_empty() {
        return false;
    }
    let compact_chars = compact.chars().count();
    if compact_chars <= 2 {
        return true;
    }
    matches!(
        compact.as_str(),
        "你好"
            | "您好"
            | "嗨"
            | "哈喽"
            | "哈啰"
            | "在吗"
            | "在嘛"
            | "早"
            | "早上好"
            | "晚上好"
            | "hi"
            | "hello"
            | "hey"
            | "yo"
    ) || (compact_chars <= 4
        && (compact.contains("你好")
            || compact.contains("您好")
            || compact.contains("哈喽")
            || compact.contains("在吗")))
}

pub(crate) fn codex_network_or_timeout_error(error: &anyhow::Error) -> bool {
    is_cli_timeout_error(error)
        || crate::codex_health::is_codex_network_error_text(&error.to_string())
}

#[cfg(test)]
pub(crate) fn intent_gate_timeout_chat_result(user_message: &str) -> IntentGateResult {
    intent_gate_fallback_chat_result(user_message, "timeout")
}

/// 意图门控降级到普通聊天路由。
pub(crate) fn intent_gate_fallback_chat_result(
    user_message: &str,
    cause: &str,
) -> IntentGateResult {
    let chat_reply = if is_tiny_chat_message(user_message) {
        "你好，我在。刚才服务端意图确认环节没完成，我先按普通聊天处理，避免误进入慢速开发流程。"
    } else {
        "我先按普通聊天处理，避免误进入慢速开发流程。你可以继续说；如果要我修改代码、编译或发布，请直接告诉我具体任务。"
    };
    IntentGateResult {
        route: intent_router::CapabilityRoute::ChatAgent,
        confidence: 0.5,
        reason: format!("intent_gate_fallback:{}", cause),
        chat_reply: Some(chat_reply.into()),
    }
}

pub(crate) fn finish_lightweight_chat_fallback(
    state: &Arc<AppState>,
    trace_id: Option<&str>,
    tx: &UnboundedSender<String>,
    user_message: &str,
    reason: &'static str,
    error: &str,
) -> Result<()> {
    record_lightweight_chat_fallback(state, trace_id, reason, error);
    let message = if is_tiny_chat_message(user_message) {
        "你好，我在。刚才服务端 Codex CLI 会话响应超过轻量聊天限时，我先结束本轮，避免手机一直卡住；你继续发消息就可以。"
    } else {
        "这次服务端 Codex CLI 没有在轻量聊天限时内返回结果。我已经结束本轮，避免手机一直等待；你可以继续发消息，或直接说要进入开发流程检查原因。"
    };
    let _ = tx.send(
        WsMessage::Done {
            message: message.into(),
            apk_url: None,
            image_url: None,
            model_used: None,
            node_id: None,
        }
        .to_json(),
    );
    Ok(())
}

//! Reserved official-web-chat adapter boundary.
//!
//! A CLI login or first-party OIDC identity never satisfies this interface.

use serde::Serialize;
use thiserror::Error;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct WebChatAdapterDescriptor {
    pub(crate) id: &'static str,
    pub(crate) vendor: &'static str,
    pub(crate) label: &'static str,
    pub(crate) protocol: &'static str,
    pub(crate) implementation_state: &'static str,
    pub(crate) enabled: bool,
    pub(crate) blocked_reason_code: &'static str,
    pub(crate) reason: &'static str,
}

pub(crate) fn reserved_web_chat_adapters() -> [WebChatAdapterDescriptor; 2] {
    [
        WebChatAdapterDescriptor {
            id: "chatgpt_web",
            vendor: "openai",
            label: "ChatGPT 网页聊天",
            protocol: "official_web_chat_adapter_reserved_v1",
            implementation_state: "reserved",
            enabled: false,
            blocked_reason_code: "official_web_chat_adapter_unavailable",
            reason: "等待官方网页聊天嵌入或获批接口；Codex 登录不能冒充 ChatGPT 网页会话。",
        },
        WebChatAdapterDescriptor {
            id: "gemini_web",
            vendor: "google",
            label: "Gemini 网页聊天",
            protocol: "official_web_chat_adapter_reserved_v1",
            implementation_state: "reserved",
            enabled: false,
            blocked_reason_code: "official_web_chat_adapter_unavailable",
            reason: "等待官方网页聊天嵌入或获批接口；Gemini CLI 登录不能冒充 Gemini 网页会话。",
        },
    ]
}

#[allow(dead_code)]
pub(crate) fn start_web_chat_session(adapter_id: &str) -> Result<(), WebChatAdapterUnavailable> {
    let descriptor = reserved_web_chat_adapters()
        .into_iter()
        .find(|descriptor| descriptor.id == adapter_id)
        .ok_or(WebChatAdapterUnavailable::UnknownAdapter)?;
    Err(WebChatAdapterUnavailable::Disabled(
        descriptor.blocked_reason_code,
    ))
}

#[derive(Debug, Error)]
pub(crate) enum WebChatAdapterUnavailable {
    #[error("未知网页聊天适配器")]
    UnknownAdapter,
    #[error("网页聊天适配器不可用: {0}")]
    Disabled(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_adapters_cannot_be_enabled_by_cli_or_environment_state() {
        for adapter in reserved_web_chat_adapters() {
            assert!(!adapter.enabled);
            assert!(matches!(
                start_web_chat_session(adapter.id),
                Err(WebChatAdapterUnavailable::Disabled(_))
            ));
        }
    }
}

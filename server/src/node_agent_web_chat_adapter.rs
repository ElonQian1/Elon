//! Reserved official-web-chat adapter boundary.
//!
//! A CLI login or first-party OIDC identity never satisfies this interface.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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
    pub(crate) actual_state: &'static str,
    pub(crate) accepted_auth_source: &'static str,
    pub(crate) cli_login_reusable: bool,
    pub(crate) browser_cookie_reusable: bool,
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
            actual_state: "unavailable",
            accepted_auth_source: "future_vendor_approved_web_authorization_only",
            cli_login_reusable: false,
            browser_cookie_reusable: false,
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
            actual_state: "unavailable",
            accepted_auth_source: "future_vendor_approved_web_authorization_only",
            cli_login_reusable: false,
            browser_cookie_reusable: false,
        },
    ]
}

#[derive(Clone, Debug, Deserialize)]
#[allow(dead_code)]
pub(crate) struct WebChatAuthorizationRequest {
    pub(crate) adapter_id: String,
    pub(crate) request_id: String,
    #[serde(default)]
    pub(crate) explicit_consent: bool,
    #[serde(default)]
    pub(crate) requested_scopes: Vec<String>,
}

pub(crate) fn lifecycle_contract() -> Value {
    json!({
        "schema": "elon.official_web_chat_adapter_contract.v1",
        "current_state": "unavailable",
        "future_lifecycle": ["authorization_pending", "active", "expired", "revoked"],
        "required_controls": [
            "vendor_approved_authorization", "explicit_user_consent", "least_privilege_scopes",
            "server_side_session_binding", "refresh_rotation", "revocation", "metadata_only_audit"
        ],
        "forbidden_inputs": ["cli_auth_cache", "browser_cookie_import", "webview_cookie_export"],
        "audit_fields": ["request_id_hash", "adapter_id", "action", "result", "occurred_at"],
        "audit_secrets": "never_store_tokens_cookies_or_chat_content"
    })
}

#[allow(dead_code)]
pub(crate) fn start_web_chat_session(
    request: &WebChatAuthorizationRequest,
) -> Result<(), WebChatAdapterUnavailable> {
    let descriptor = reserved_web_chat_adapters()
        .into_iter()
        .find(|descriptor| descriptor.id == request.adapter_id)
        .ok_or(WebChatAdapterUnavailable::UnknownAdapter)?;
    Err(WebChatAdapterUnavailable::Disabled(
        descriptor.blocked_reason_code,
    ))
}

#[allow(dead_code)]
pub(crate) fn refresh_web_chat_session(adapter_id: &str) -> Result<(), WebChatAdapterUnavailable> {
    disabled_operation(adapter_id)
}

#[allow(dead_code)]
pub(crate) fn revoke_web_chat_session(adapter_id: &str) -> Result<(), WebChatAdapterUnavailable> {
    disabled_operation(adapter_id)
}

fn disabled_operation(adapter_id: &str) -> Result<(), WebChatAdapterUnavailable> {
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
    fn phase2_contract_reserved_web_chat_adapters_remain_disabled() {
        for adapter in reserved_web_chat_adapters() {
            assert!(!adapter.enabled);
            assert!(!adapter.cli_login_reusable);
            assert!(!adapter.browser_cookie_reusable);
            let request = WebChatAuthorizationRequest {
                adapter_id: adapter.id.to_string(),
                request_id: "web-chat-test".to_string(),
                explicit_consent: true,
                requested_scopes: vec!["chat".to_string()],
            };
            assert!(matches!(
                start_web_chat_session(&request),
                Err(WebChatAdapterUnavailable::Disabled(_))
            ));
            assert!(matches!(
                refresh_web_chat_session(adapter.id),
                Err(WebChatAdapterUnavailable::Disabled(_))
            ));
            assert!(matches!(
                revoke_web_chat_session(adapter.id),
                Err(WebChatAdapterUnavailable::Disabled(_))
            ));
        }
        assert_eq!(lifecycle_contract()["current_state"], "unavailable");
    }
}

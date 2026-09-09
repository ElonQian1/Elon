use super::{ProviderAdapter, LOCAL_AI_WINDOW_PREFIX};

#[derive(Clone, Copy)]
pub(super) struct ProviderDefinition {
    pub(super) kind: ProviderKind,
    pub(super) id: &'static str,
    pub(super) display_name: &'static str,
    pub(super) start_url: &'static str,
    pub(super) start_host: &'static str,
    pub(super) login_mode: &'static str,
    pub(super) renderer_status: &'static str,
    pub(super) adapter: Option<ProviderAdapter>,
    pub(super) allowed_hosts: &'static [&'static str],
    pub(super) allowed_domain_suffixes: &'static [&'static str],
    pub(super) allowed_identity_hosts: &'static [&'static str],
    pub(super) blocked_identity_hosts: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ProviderKind {
    AiAssistant,
    Exchange,
}

pub(super) const CHATGPT: ProviderDefinition = ProviderDefinition {
    kind: ProviderKind::AiAssistant,
    id: "chatgpt",
    display_name: "ChatGPT",
    start_url: "https://chatgpt.com/",
    start_host: "chatgpt.com",
    login_mode: "manual_web",
    renderer_status: "active",
    adapter: Some(ProviderAdapter::ChatGpt),
    allowed_hosts: &[],
    allowed_domain_suffixes: &["chatgpt.com", "openai.com"],
    allowed_identity_hosts: &[
        "accounts.google.com",
        "appleid.apple.com",
        "login.live.com",
        "account.live.com",
        "login.microsoft.com",
        "login.microsoftonline.com",
        "login.windows.net",
    ],
    blocked_identity_hosts: &[],
};

pub(super) const GOOGLE_AI_MODE: ProviderDefinition = ProviderDefinition {
    kind: ProviderKind::AiAssistant,
    id: "google-ai-mode",
    display_name: "Google AI 模式",
    start_url: "https://www.google.com/aimode",
    start_host: "google.com/aimode",
    login_mode: "guest_web_system_login",
    renderer_status: "active",
    adapter: Some(ProviderAdapter::GoogleWeb),
    allowed_hosts: &["google.com", "www.google.com"],
    allowed_domain_suffixes: &[],
    allowed_identity_hosts: &[],
    blocked_identity_hosts: &["accounts.google.com"],
};

pub(super) const BINANCE: ProviderDefinition = ProviderDefinition {
    kind: ProviderKind::Exchange,
    id: "binance",
    display_name: "Binance 合约网格",
    start_url: "https://www.binance.com/zh-CN/trading-bots/futures/grid/NEARUSDT",
    start_host: "www.binance.com/zh-CN/trading-bots/futures/grid",
    login_mode: "manual_web",
    renderer_status: "reserved",
    adapter: None,
    allowed_hosts: &["www.binance.com", "accounts.binance.com"],
    allowed_domain_suffixes: &[],
    allowed_identity_hosts: &[],
    blocked_identity_hosts: &[],
};

const PROVIDERS: &[ProviderDefinition] = &[GOOGLE_AI_MODE, CHATGPT, BINANCE];

pub(super) fn provider(provider_id: &str) -> Result<&'static ProviderDefinition, String> {
    PROVIDERS
        .iter()
        .find(|provider| provider.id == provider_id.trim())
        .ok_or_else(|| format!("不支持的本地网页服务：{provider_id}"))
}

pub(super) fn provider_for_kind(
    provider_id: &str,
    kind: ProviderKind,
) -> Result<&'static ProviderDefinition, String> {
    provider(provider_id).and_then(|provider| {
        (provider.kind == kind)
            .then_some(provider)
            .ok_or_else(|| format!("网页服务类型不匹配：{provider_id}"))
    })
}

pub(super) fn providers_for_kind(
    kind: ProviderKind,
) -> impl Iterator<Item = &'static ProviderDefinition> {
    PROVIDERS
        .iter()
        .filter(move |provider| provider.kind == kind)
}

pub(super) fn provider_for_window_label(label: &str) -> Option<&'static ProviderDefinition> {
    PROVIDERS
        .iter()
        .find(|provider| label.starts_with(&format!("{LOCAL_AI_WINDOW_PREFIX}{}-", provider.id)))
}

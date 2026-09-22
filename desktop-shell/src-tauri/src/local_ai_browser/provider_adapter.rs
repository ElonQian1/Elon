use super::{
    adapter::{self, SanitizedAdapterEvent},
    adapter_command::{self, PageCommandBinding},
    binance_exchange_adapter, chatgpt_adapter_bootstrap, google_ai_mode,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ProviderAdapter {
    ChatGpt,
    GoogleWeb,
    /// Read-only observer sharing the Android Binance grid adapters; no trading surface.
    Binance,
}

impl ProviderAdapter {
    pub(super) const fn version(self) -> u32 {
        match self {
            Self::ChatGpt => chatgpt_adapter_bootstrap::ADAPTER_VERSION,
            Self::GoogleWeb => google_ai_mode::ADAPTER_VERSION,
            Self::Binance => binance_exchange_adapter::ADAPTER_VERSION,
        }
    }

    pub(super) fn initialization_script(self) -> String {
        match self {
            Self::ChatGpt => chatgpt_adapter_bootstrap::initialization_script(),
            Self::GoogleWeb => google_ai_mode::initialization_script(),
            Self::Binance => binance_exchange_adapter::initialization_script(),
        }
    }

    pub(super) fn supported_actions(self) -> &'static [&'static str] {
        match self {
            Self::ChatGpt => adapter_command::CHATGPT_ACTIONS,
            Self::GoogleWeb => adapter_command::GOOGLE_AI_MODE_ACTIONS,
            Self::Binance => adapter_command::BINANCE_ACTIONS,
        }
    }

    pub(super) fn sanitize_event(self, payload: &str) -> Result<SanitizedAdapterEvent, String> {
        match self {
            Self::ChatGpt => adapter::sanitize_event(payload),
            Self::GoogleWeb => google_ai_mode::sanitize_event(payload),
            Self::Binance => binance_exchange_adapter::sanitize_event(payload),
        }
    }

    pub(super) fn page_invocation_script(self, raw_command: &str) -> Result<String, String> {
        match self {
            Self::ChatGpt => adapter_command::page_invocation_script(
                "__elonChatGptBridge",
                PageCommandBinding::ChatGptDocument,
                raw_command,
            ),
            Self::GoogleWeb => adapter_command::page_invocation_script(
                "__elonGoogleWebBridge",
                PageCommandBinding::None,
                raw_command,
            ),
            Self::Binance => adapter_command::page_invocation_script(
                "__elonBinanceWinBridge",
                PageCommandBinding::None,
                raw_command,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_adapter_owns_vendor_specific_bridges_and_actions() {
        let chatgpt = ProviderAdapter::ChatGpt;
        let google = ProviderAdapter::GoogleWeb;
        assert!(chatgpt.supported_actions().contains(&"list_conversations"));
        assert!(google.supported_actions().contains(&"list_conversations"));
        assert!(google.supported_actions().contains(&"open_conversation"));
        assert_eq!(chatgpt.version(), chatgpt_adapter_bootstrap::ADAPTER_VERSION);
        assert_eq!(google.version(), google_ai_mode::ADAPTER_VERSION);
        assert!(chatgpt
            .page_invocation_script(r#"{"action":"snapshot"}"#)
            .unwrap()
            .contains("__elonChatGptDocumentToken"));
        assert!(google
            .page_invocation_script(r#"{"action":"snapshot"}"#)
            .unwrap()
            .contains("__elonGoogleWebBridge"));
        let binance = ProviderAdapter::Binance;
        assert_eq!(binance.supported_actions(), adapter_command::BINANCE_ACTIONS);
        assert!(!binance.supported_actions().contains(&"send_prompt"));
        assert!(binance
            .page_invocation_script(r#"{"action":"refresh"}"#)
            .unwrap()
            .contains("__elonBinanceWinBridge"));
    }
}

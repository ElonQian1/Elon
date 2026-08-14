use super::{
    adapter::{self, SanitizedAdapterEvent},
    adapter_command::{self, PageCommandBinding},
    chatgpt_adapter_bootstrap, google_ai_mode,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ProviderAdapter {
    ChatGpt,
    GoogleWeb,
}

impl ProviderAdapter {
    pub(super) fn initialization_script(self) -> String {
        match self {
            Self::ChatGpt => chatgpt_adapter_bootstrap::initialization_script(),
            Self::GoogleWeb => google_ai_mode::initialization_script(),
        }
    }

    pub(super) fn supported_actions(self) -> &'static [&'static str] {
        match self {
            Self::ChatGpt => adapter_command::CHATGPT_ACTIONS,
            Self::GoogleWeb => adapter_command::GOOGLE_AI_MODE_ACTIONS,
        }
    }

    pub(super) fn sanitize_event(self, payload: &str) -> Result<SanitizedAdapterEvent, String> {
        match self {
            Self::ChatGpt => adapter::sanitize_event(payload),
            Self::GoogleWeb => google_ai_mode::sanitize_event(payload),
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
        assert!(!google.supported_actions().contains(&"list_conversations"));
        assert!(chatgpt
            .page_invocation_script(r#"{"action":"snapshot"}"#)
            .unwrap()
            .contains("__elonChatGptDocumentToken"));
        assert!(google
            .page_invocation_script(r#"{"action":"snapshot"}"#)
            .unwrap()
            .contains("__elonGoogleWebBridge"));
    }
}

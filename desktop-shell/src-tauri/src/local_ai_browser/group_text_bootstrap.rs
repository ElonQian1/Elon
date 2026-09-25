// Group-only policy; the actual text sender is shared with personal sessions.
pub(super) fn initialization_script() -> String {
    let marker = "window.__elonChatGptBootstrapStage = 'chatgpt_text_bootstrap';";
    super::chatgpt_adapter_bootstrap::initialization_script().replace(marker, &format!(
        "window.__elonChatGptFreshTextTemporaryEnabled = true;\nwindow.__elonChatGptFreshTextJournalEnabled = false;\n{marker}"
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn group_bootstrap_loads_reviewed_sender_before_adapter_and_disables_journal() {
        let source = initialization_script();
        assert!(source.contains("window.__elonChatGptFreshTextTemporaryEnabled = true"));
        assert!(source.contains("window.__elonChatGptFreshTextJournalEnabled = false"));
        assert!(
            source
                .find("chatgpt_web_private_runtime_bindings.js")
                .unwrap()
                < source.find("chatgpt_web_fresh_text_context.js").unwrap()
        );
        assert!(
            source
                .find("chatgpt_web_fresh_text_transaction.js")
                .unwrap()
                < source
                    .find("chatgpt_web_text_transaction_orchestrator.js")
                    .unwrap()
        );
        assert!(
            source.find("chatgpt_web_private_owned_stream.js").unwrap()
                < source
                    .find("chatgpt_web_private_stream_transport.js")
                    .unwrap()
        );
        assert!(
            !super::super::chatgpt_adapter_bootstrap::initialization_script()
                .contains("window.__elonChatGptFreshTextTemporaryEnabled = true")
        );
    }
}

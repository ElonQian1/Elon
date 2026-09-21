// Group tasks reuse the reviewed Android transport, not a second HTTP sender.
const ASSETS: &[(&str, &str)] = &[
    (
        "chatgpt_web_private_runtime_bindings.js",
        include_str!(
            "../../../../android/app/src/main/assets/chatgpt_web_private_runtime_bindings.js"
        ),
    ),
    (
        "chatgpt_web_committed_owner_path.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_committed_owner_path.js"),
    ),
    (
        "chatgpt_web_committed_composer_owner.js",
        include_str!(
            "../../../../android/app/src/main/assets/chatgpt_web_committed_composer_owner.js"
        ),
    ),
    (
        "chatgpt_web_private_auth_context.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_private_auth_context.js"),
    ),
    (
        "chatgpt_web_private_model_contract.js",
        include_str!(
            "../../../../android/app/src/main/assets/chatgpt_web_private_model_contract.js"
        ),
    ),
    (
        "chatgpt_web_private_model_catalog.js",
        include_str!(
            "../../../../android/app/src/main/assets/chatgpt_web_private_model_catalog.js"
        ),
    ),
    (
        "chatgpt_web_private_model_state.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_private_model_state.js"),
    ),
    (
        "chatgpt_web_private_text_runtime_submit.js",
        include_str!(
            "../../../../android/app/src/main/assets/chatgpt_web_private_text_runtime_submit.js"
        ),
    ),
    (
        "chatgpt_web_runtime_generation_state.js",
        include_str!(
            "../../../../android/app/src/main/assets/chatgpt_web_runtime_generation_state.js"
        ),
    ),
    (
        "chatgpt_web_private_regenerate_contract.js",
        include_str!(
            "../../../../android/app/src/main/assets/chatgpt_web_private_regenerate_contract.js"
        ),
    ),
    (
        "chatgpt_web_private_regenerate_runtime.js",
        include_str!(
            "../../../../android/app/src/main/assets/chatgpt_web_private_regenerate_runtime.js"
        ),
    ),
    (
        "chatgpt_web_private_stop_runtime.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_private_stop_runtime.js"),
    ),
    (
        "chatgpt_web_fresh_text_attachments.js",
        include_str!(
            "../../../../android/app/src/main/assets/chatgpt_web_fresh_text_attachments.js"
        ),
    ),
    (
        "chatgpt_web_fresh_text_attachment_identity.js",
        include_str!(
            "../../../../android/app/src/main/assets/chatgpt_web_fresh_text_attachment_identity.js"
        ),
    ),
    (
        "chatgpt_web_fresh_text_request.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_fresh_text_request.js"),
    ),
    (
        "chatgpt_web_fresh_text_context.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_fresh_text_context.js"),
    ),
    (
        "chatgpt_web_private_text_input.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_private_text_input.js"),
    ),
    (
        "chatgpt_web_fresh_text_user_identity.js",
        include_str!(
            "../../../../android/app/src/main/assets/chatgpt_web_fresh_text_user_identity.js"
        ),
    ),
    (
        "chatgpt_web_fresh_regenerate_context.js",
        include_str!(
            "../../../../android/app/src/main/assets/chatgpt_web_fresh_regenerate_context.js"
        ),
    ),
    (
        "chatgpt_web_fresh_text_reconcile.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_fresh_text_reconcile.js"),
    ),
    (
        "chatgpt_web_fresh_text_stop.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_fresh_text_stop.js"),
    ),
    (
        "chatgpt_web_fresh_text_recovery.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_fresh_text_recovery.js"),
    ),
    (
        "chatgpt_web_fresh_text_stream.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_fresh_text_stream.js"),
    ),
    (
        "chatgpt_web_fresh_text_journal_store.js",
        include_str!(
            "../../../../android/app/src/main/assets/chatgpt_web_fresh_text_journal_store.js"
        ),
    ),
    (
        "chatgpt_web_fresh_text_journal.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_fresh_text_journal.js"),
    ),
    (
        "chatgpt_web_fresh_text_recovery_context.js",
        include_str!(
            "../../../../android/app/src/main/assets/chatgpt_web_fresh_text_recovery_context.js"
        ),
    ),
    (
        "chatgpt_web_fresh_text_recovery_session.js",
        include_str!(
            "../../../../android/app/src/main/assets/chatgpt_web_fresh_text_recovery_session.js"
        ),
    ),
    (
        "chatgpt_web_fresh_text_receipts.js",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_fresh_text_receipts.js"),
    ),
    (
        "chatgpt_web_fresh_text_transaction.js",
        include_str!(
            "../../../../android/app/src/main/assets/chatgpt_web_fresh_text_transaction.js"
        ),
    ),
];

pub(super) fn initialization_script() -> String {
    let extra = ASSETS
        .iter()
        .map(|(name, source)| format!("window.__elonChatGptBootstrapStage = '{name}';\n{source}"))
        .collect::<Vec<_>>()
        .join("\n");
    let marker =
        "window.__elonChatGptBootstrapStage = 'chatgpt_web_text_transaction_orchestrator.js';";
    let stream_marker =
        "window.__elonChatGptBootstrapStage = 'chatgpt_web_private_stream_transport.js';";
    super::chatgpt_adapter_bootstrap::initialization_script().replace(stream_marker, &format!(
        "window.__elonChatGptBootstrapStage = 'chatgpt_web_private_owned_stream.js';\n{}\n{stream_marker}",
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_private_owned_stream.js")
    )).replace(marker, &format!(
        "window.__elonChatGptFreshTextTemporaryEnabled = true;\nwindow.__elonChatGptFreshTextJournalEnabled = false;\n{extra}\n{marker}"
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

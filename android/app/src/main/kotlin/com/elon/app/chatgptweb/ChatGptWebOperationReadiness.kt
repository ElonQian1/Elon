package com.elon.app.chatgptweb

/** Admission only: individual commands still validate context, handles and confirmations. */
internal object ChatGptWebOperationReadiness {
    enum class Requirement { LOCAL, CACHED_DIRECTORY, DOCUMENT, DIRECTORY_READ, ACCOUNT_READ, ACCOUNT_MUTATION, COMPOSER }

    private val groups = mapOf(
        Requirement.LOCAL to setOf(
            "state", "open_chatgpt_web", "chatgpt_refresh", "chatgpt_select_view",
            "chatgpt_cancel_file_download", "chatgpt_get_capability_matrix",
        ),
        Requirement.CACHED_DIRECTORY to setOf("chatgpt_get_conversations", "chatgpt_get_navigation"),
        Requirement.DOCUMENT to setOf(
            "chatgpt_get_context", "chatgpt_find_controls", "chatgpt_copy_last_response",
            "chatgpt_reveal_message", "chatgpt_refresh_controls", "chatgpt_open_conversation",
            "chatgpt_list_composer_options", "chatgpt_dismiss_composer_options",
            "chatgpt_list_features", "chatgpt_dismiss_features", "chatgpt_cancel_library_files",
            "chatgpt_private_protocol_probe", "chatgpt_stop_generation",
            "chatgpt_cancel_directory_page",
        ),
        Requirement.DIRECTORY_READ to setOf("chatgpt_list_conversations"),
        Requirement.ACCOUNT_READ to setOf(
            "chatgpt_list_conversation_files", "chatgpt_list_library_files",
            "chatgpt_browse_directory_page",
            "chatgpt_download_conversation_file", "chatgpt_download_library_file",
        ),
        Requirement.ACCOUNT_MUTATION to setOf(
            "chatgpt_set_conversation_pinned", "chatgpt_set_conversation_archived",
            "chatgpt_rename_conversation", "chatgpt_move_conversation_to_project",
            // Their owners still protect current-chat drafts and share bindings before writes.
            "chatgpt_delete_conversation", "chatgpt_share_conversation", "chatgpt_mutate_library_file",
        ),
        // These still use the official composer/runtime transaction. Do not relax them implicitly.
        Requirement.COMPOSER to setOf(
            "set_input_text", "chatgpt_set_page_input_text", "send_input", "chatgpt_send_page_input",
            "chatgpt_invoke_control", "chatgpt_set_control_text", "chatgpt_set_control_selected",
            "chatgpt_select_control_choice", "chatgpt_set_control_slider", "chatgpt_set_control_expanded",
            "chatgpt_new_conversation", "chatgpt_verify_private_stream_watchdog", "chatgpt_regenerate_response",
            "chatgpt_toggle_private_read_aloud",
            "chatgpt_start_dictation",
            "chatgpt_prepare_realtime_voice", "chatgpt_start_realtime_voice", "chatgpt_cancel_dictation",
            "chatgpt_submit_dictation", "chatgpt_remove_attachment", "chatgpt_reveal_project_choice",
            "chatgpt_select_composer_option", "chatgpt_select_feature", "chatgpt_record_verification_cases",
            "chatgpt_attach_library_file",
        ),
    )

    fun requirement(action: String): Requirement? = groups.entries.singleOrNull { action in it.value }?.key

    fun rejection(
        action: String,
        snapshot: ChatGptWebSnapshot?,
        adapterCurrent: Boolean,
        bridgeReady: Boolean,
    ): String? {
        val required = requirement(action) ?: return "unsupported_action"
        if (required == Requirement.LOCAL || required == Requirement.CACHED_DIRECTORY) return null
        // Preserve the existing admission for composer transactions, including its error contract.
        if (required == Requirement.COMPOSER && !bridgeReady) return "bridge_not_ready"
        if (!adapterCurrent) return "adapter_generation_not_ready"
        if (required == Requirement.COMPOSER) return null
        // A current manifest/command channel can arrive before the independent chat snapshot.
        // The page adapter still enforces the live WebView origin on every command.
        if (snapshot == null) return null
        if (ChatGptWebAccessPolicy.requiresLogin(snapshot) ||
            ChatGptWebNavigationPolicy.isAuthenticationPage(snapshot.url)) return "login_required"
        if (!ChatGptWebNavigationPolicy.supportsEnhancedMode(snapshot.url)) return "unsupported_page"
        // Private requests validate their own account-bound context; mutations also retain confirmation checks.
        // A DOM-derived authenticated flag is not that context and must not block acquisition.
        return null
    }
}

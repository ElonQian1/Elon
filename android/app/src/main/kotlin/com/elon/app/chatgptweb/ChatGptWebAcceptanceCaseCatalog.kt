package com.elon.app.chatgptweb

internal object ChatGptWebAcceptanceCaseCatalog {
    private val verificationCases = mapOf(
        "official_authentication" to "safe/authenticated_session",
        "official_fullscreen_fallback" to "safe/read_only_surface",
        "native_chat_composer" to "reversible/send_probe",
        "streaming_and_stop" to "reversible/send_probe_with_stop",
        "conversation_context_paging" to "safe/read_only_surface",
        "conversation_history" to "safe/read_only_surface",
        "conversation_create_and_switch" to "reversible/send_probe",
        "model_selection" to "reversible/reversible_controls",
        "attachment_lifecycle" to "supervised/attachment_lifecycle",
        "composer_tools" to "reversible/tool_execution_with_citations",
        "web_search" to "reversible/composer_controls",
        "deep_research" to "reversible/composer_tool_execution/deep_research",
        "image_generation" to "reversible/composer_tool_execution/image_generation",
        "canvas" to "reversible/composer_tool_execution/canvas",
        "study_mode" to "reversible/composer_tool_execution/study_mode",
        "agent_mode" to "supervised/composer_tool_execution/agent_mode",
        "dictation" to "supervised/dictation_transcription",
        "realtime_voice" to "supervised/realtime_voice_round_trip",
        "rich_message_rendering" to "reversible/message_structure",
        "complex_output_rendering" to "reversible/message_structure",
        "message_copy" to "reversible/copy_receipt_without_content_readback",
        "message_regenerate" to "reversible/regenerate_response",
        "message_action_context" to "safe/message_actions",
        "message_actions" to "supervised/message_actions",
        "feature_navigation" to "safe/feature_pages",
        "projects" to "safe/feature_page/projects",
        "tasks" to "safe/feature_page/tasks",
        "library" to "safe/feature_page/library",
        "gpts" to "safe/feature_page/gpts",
        "apps" to "safe/feature_page/apps",
        "work" to "safe/feature_page/work",
        "health" to "supervised/feature_page/health",
        "finances" to "supervised/feature_page/finances",
        "settings" to "safe/settings_overlay_form_controls",
        "account_menu" to "safe/account_menu_structure",
        "account_mutations" to "supervised/account_mutations",
        "conversation_management" to "safe/conversation_management_structure",
        "conversation_mutations" to "supervised/conversation_mutations",
        "adaptive_form_controls" to "safe/settings_overlay_idempotent_form_controls",
        "disclosure_controls" to "reversible/reversible_controls",
        "official_change_detection" to "safe/read_only_surface",
        "stable_mcp_and_adb_controls" to "safe/read_only_surface",
        "session_continuity_and_recovery" to "safe/session_recovery",
        "session_long_running_stability" to "safe/session_long_running_stability",
    )

    private val discoveryCases = mapOf(
        "deep_research" to "reversible/composer_tool_discovery/deep_research",
        "image_generation" to "reversible/composer_tool_discovery/image_generation",
        "canvas" to "reversible/composer_tool_discovery/canvas",
        "study_mode" to "reversible/composer_tool_discovery/study_mode",
        "agent_mode" to "reversible/composer_tool_discovery/agent_mode",
    )

    fun verificationCase(featureId: String): String? = verificationCases[featureId]

    fun discoveryCase(featureId: String): String? = discoveryCases[featureId]

    fun verificationCaseIds(): Set<String> = verificationCases.values.toSortedSet()

    fun evidenceCaseIds(): Set<String> =
        (verificationCases.values + discoveryCases.values).toSortedSet()
}

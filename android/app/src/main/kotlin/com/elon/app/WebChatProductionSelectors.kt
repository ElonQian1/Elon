package com.elon.app

/** Stable semantic selectors for the real friend-chat surface, not the diagnostic Activity. */
internal object WebChatProductionSelectors {
    const val SEND = "web-chat-send"
    const val STOP_GENERATION = "web-chat-stop-generation"
    const val SUGGESTIONS = "web-chat-suggestions"
    const val WORK_ATTACHMENT = "展开更多输入功能"
    const val REALTIME_VOICE_SURFACE = "web-chat-realtime-voice:surface"
    const val REALTIME_VOICE_STATUS = "web-chat-realtime-voice:status"
    const val REALTIME_VOICE_CLOSE = "web-chat-realtime-voice:close"
    const val REALTIME_VOICE_RETRY = "web-chat-realtime-voice:retry"
    const val REALTIME_VOICE_OFFICIAL_FALLBACK = "web-chat-realtime-voice:official-fallback"
    const val REALTIME_VOICE_LOGIN_SURFACE = "web-chat-realtime-voice-login:surface"
    const val REALTIME_VOICE_LOGIN_METHOD = "web-chat-realtime-voice-login:method"
    const val REALTIME_VOICE_LOGIN_OFFICIAL = "web-chat-realtime-voice-login:official"
    const val REALTIME_VOICE_LOGIN_CANCEL = "web-chat-realtime-voice-login:cancel"

    fun composerInput(provider: WebChatProviderId): String =
        "web-chat-composer-input:${provider.wireValue}"

    fun attachment(provider: WebChatProviderId): String =
        "web-chat-attachment:${provider.wireValue}"

    fun composerTools(provider: WebChatProviderId): String =
        "web-chat-composer-tools:${provider.wireValue}"

    fun pageActions(provider: WebChatProviderId): String =
        "web-chat-page-actions:${provider.wireValue}"

    fun suggestion(provider: WebChatProviderId, controlId: String): String =
        "web-chat-suggestion:${provider.wireValue}:${stable(controlId)}"

    fun composerAction(streaming: Boolean): String =
        if (streaming) STOP_GENERATION else SEND

    private fun stable(value: String): String = value.trim()
        .map { character ->
            if (character.isLetterOrDigit() || character == '-' || character == '_') character else '_'
        }
        .joinToString("")
        .take(80)
        .ifBlank { "unknown" }
}

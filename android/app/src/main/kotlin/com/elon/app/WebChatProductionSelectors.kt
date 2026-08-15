package com.elon.app

/** Stable semantic selectors for the real friend-chat surface, not the diagnostic Activity. */
internal object WebChatProductionSelectors {
    const val SEND = "web-chat-send"
    const val STOP_GENERATION = "web-chat-stop-generation"
    const val WORK_ATTACHMENT = "展开更多输入功能"

    fun composerInput(provider: WebChatProviderId): String =
        "web-chat-composer-input:${provider.wireValue}"

    fun attachment(provider: WebChatProviderId): String =
        "web-chat-attachment:${provider.wireValue}"

    fun composerTools(provider: WebChatProviderId): String =
        "web-chat-composer-tools:${provider.wireValue}"

    fun pageActions(provider: WebChatProviderId): String =
        "web-chat-page-actions:${provider.wireValue}"

    fun composerAction(streaming: Boolean): String =
        if (streaming) STOP_GENERATION else SEND
}

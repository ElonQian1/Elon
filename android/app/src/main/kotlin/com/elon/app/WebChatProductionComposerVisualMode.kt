package com.elon.app

internal enum class WebChatProductionComposerVisualMode {
    INPUT_MODE,
    SEND,
    STOP,
}

internal object WebChatProductionComposerVisualModeResolver {
    fun resolve(
        streaming: Boolean,
        hasText: Boolean,
        hasAttachments: Boolean,
        voiceMode: Boolean,
        composerExpanded: Boolean,
    ): WebChatProductionComposerVisualMode {
        if (streaming) return WebChatProductionComposerVisualMode.STOP
        val canSend = (hasText || hasAttachments) &&
            !voiceMode &&
            (composerExpanded || hasAttachments)
        return if (canSend) {
            WebChatProductionComposerVisualMode.SEND
        } else {
            WebChatProductionComposerVisualMode.INPUT_MODE
        }
    }
}

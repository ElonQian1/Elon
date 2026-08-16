package com.elon.app

internal enum class WebChatProductionVoiceInputRoute {
    WEB_TEXT_TRANSCRIPTION,
    VOICE_MESSAGE,
    CONFIGURED_WORK_INPUT,
}

internal object WebChatProductionVoiceInputPolicy {
    fun resolve(
        webChatModeActive: Boolean,
        friendChatActive: Boolean,
    ): WebChatProductionVoiceInputRoute = when {
        webChatModeActive -> WebChatProductionVoiceInputRoute.WEB_TEXT_TRANSCRIPTION
        friendChatActive -> WebChatProductionVoiceInputRoute.VOICE_MESSAGE
        else -> WebChatProductionVoiceInputRoute.CONFIGURED_WORK_INPUT
    }

    fun allowsDirectCloudAiFallback(webChatModeActive: Boolean): Boolean = !webChatModeActive
}

package com.elon.app

internal enum class WebChatRealtimeVoiceBackgroundStatus(val wireValue: String) {
    CONNECTING("connecting"),
    LISTENING("listening"),
    THINKING("thinking"),
    SPEAKING("speaking"),
    PAUSED("paused"),
    ERROR("error"),
}

internal enum class WebChatRealtimeVoiceBackgroundControl(val wireValue: String) {
    PAUSE("pause"),
    RESUME("resume"),
    HANG_UP("hang_up"),
}

internal enum class WebChatRealtimeVoiceBackgroundControlSource(val wireValue: String) {
    USER("user"),
    MEDIA("media"),
}

internal object WebChatRealtimeVoiceBackgroundProtocol {
    const val ACTION_START = "com.elon.app.webchat.voice.background.START"
    const val ACTION_UPDATE = "com.elon.app.webchat.voice.background.UPDATE"
    const val ACTION_HOST_VISIBILITY = "com.elon.app.webchat.voice.background.HOST_VISIBILITY"
    const val ACTION_STOP = "com.elon.app.webchat.voice.background.STOP"
    const val ACTION_CONTROL = "com.elon.app.webchat.voice.background.CONTROL"
    const val ACTION_PAUSE = "com.elon.app.webchat.voice.background.PAUSE"
    const val ACTION_RESUME = "com.elon.app.webchat.voice.background.RESUME"
    const val ACTION_HANG_UP = "com.elon.app.webchat.voice.background.HANG_UP"
    const val EXTRA_STATUS = "status"
    const val EXTRA_DETAIL = "detail"
    const val EXTRA_HOST_VISIBLE = "host_visible"
    const val EXTRA_CONTROL = "control"
    const val EXTRA_SOURCE = "source"

    fun status(value: String?): WebChatRealtimeVoiceBackgroundStatus =
        WebChatRealtimeVoiceBackgroundStatus.entries.firstOrNull { it.wireValue == value }
            ?: WebChatRealtimeVoiceBackgroundStatus.CONNECTING

    fun control(value: String?): WebChatRealtimeVoiceBackgroundControl? =
        WebChatRealtimeVoiceBackgroundControl.entries.firstOrNull { it.wireValue == value }

    fun source(value: String?): WebChatRealtimeVoiceBackgroundControlSource =
        WebChatRealtimeVoiceBackgroundControlSource.entries.firstOrNull { it.wireValue == value }
            ?: WebChatRealtimeVoiceBackgroundControlSource.USER
}

internal object WebChatRealtimeVoiceBackgroundStatusPolicy {
    fun from(state: WebChatRealtimeVoiceState): WebChatRealtimeVoiceBackgroundStatus =
        when (state.lifecycle) {
            WebChatRealtimeVoiceLifecycle.CONNECTING,
            WebChatRealtimeVoiceLifecycle.ENDING -> WebChatRealtimeVoiceBackgroundStatus.CONNECTING
            WebChatRealtimeVoiceLifecycle.FAILED -> WebChatRealtimeVoiceBackgroundStatus.ERROR
            WebChatRealtimeVoiceLifecycle.ACTIVE,
            WebChatRealtimeVoiceLifecycle.HANGUP_UNCONFIRMED -> if (state.paused) {
                WebChatRealtimeVoiceBackgroundStatus.PAUSED
            } else when (state.turn) {
                WebChatRealtimeVoiceTurn.LISTENING -> WebChatRealtimeVoiceBackgroundStatus.LISTENING
                WebChatRealtimeVoiceTurn.THINKING -> WebChatRealtimeVoiceBackgroundStatus.THINKING
                WebChatRealtimeVoiceTurn.SPEAKING -> WebChatRealtimeVoiceBackgroundStatus.SPEAKING
                WebChatRealtimeVoiceTurn.UNKNOWN,
                WebChatRealtimeVoiceTurn.IDLE -> WebChatRealtimeVoiceBackgroundStatus.LISTENING
            }
        }
}

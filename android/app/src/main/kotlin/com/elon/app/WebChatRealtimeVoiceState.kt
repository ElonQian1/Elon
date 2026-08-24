package com.elon.app

internal enum class WebChatRealtimeVoiceLifecycle {
    CONNECTING,
    ACTIVE,
    ENDING,
    FAILED,
}

internal enum class WebChatRealtimeVoiceTurn {
    UNKNOWN,
    IDLE,
    LISTENING,
    THINKING,
    SPEAKING,
}

internal data class WebChatRealtimeVoiceState(
    val lifecycle: WebChatRealtimeVoiceLifecycle,
    val detail: String,
    val turn: WebChatRealtimeVoiceTurn = WebChatRealtimeVoiceTurn.UNKNOWN,
    val context: WebChatRealtimeVoiceContext? = null,
)

internal enum class WebChatRealtimeVoiceVisibleState(val label: String) {
    CONNECTING("连接中"),
    IDLE("待机中"),
    LISTENING("正在聆听"),
    THINKING("思考中"),
    SPEAKING("回答中"),
    ENDING("结束中"),
    FAILED("连接异常"),
}

internal object WebChatRealtimeVoiceStatePolicy {
    fun visibleState(state: WebChatRealtimeVoiceState): WebChatRealtimeVoiceVisibleState =
        when (state.lifecycle) {
            WebChatRealtimeVoiceLifecycle.CONNECTING -> WebChatRealtimeVoiceVisibleState.CONNECTING
            WebChatRealtimeVoiceLifecycle.ENDING -> WebChatRealtimeVoiceVisibleState.ENDING
            WebChatRealtimeVoiceLifecycle.FAILED -> WebChatRealtimeVoiceVisibleState.FAILED
            WebChatRealtimeVoiceLifecycle.ACTIVE -> when (state.turn) {
                WebChatRealtimeVoiceTurn.LISTENING -> WebChatRealtimeVoiceVisibleState.LISTENING
                WebChatRealtimeVoiceTurn.THINKING -> WebChatRealtimeVoiceVisibleState.THINKING
                WebChatRealtimeVoiceTurn.SPEAKING -> WebChatRealtimeVoiceVisibleState.SPEAKING
                WebChatRealtimeVoiceTurn.UNKNOWN,
                WebChatRealtimeVoiceTurn.IDLE -> WebChatRealtimeVoiceVisibleState.IDLE
            }
        }
}

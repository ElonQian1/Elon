package com.elon.app

internal enum class WebChatRealtimeVoiceLifecycle {
    CONNECTING,
    ACTIVE,
    ENDING,
    HANGUP_UNCONFIRMED,
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
    val paused: Boolean = false,
)

internal enum class WebChatRealtimeVoiceVisibleState(val label: String) {
    CONNECTING("连接中"),
    IDLE("待机中"),
    LISTENING("正在聆听"),
    THINKING("思考中"),
    SPEAKING("回答中"),
    PAUSED("已暂停"),
    ENDING("结束中"),
    HANGUP_UNCONFIRMED("仍在通话"),
    FAILED("连接异常"),
}

internal enum class WebChatRealtimeVoiceExpansionDecision {
    PRESERVE,
    EXPAND,
    COLLAPSE,
}

internal object WebChatRealtimeVoiceStatePolicy {
    fun visibleState(state: WebChatRealtimeVoiceState): WebChatRealtimeVoiceVisibleState =
        when (state.lifecycle) {
            WebChatRealtimeVoiceLifecycle.CONNECTING -> WebChatRealtimeVoiceVisibleState.CONNECTING
            WebChatRealtimeVoiceLifecycle.ENDING -> WebChatRealtimeVoiceVisibleState.ENDING
            WebChatRealtimeVoiceLifecycle.HANGUP_UNCONFIRMED ->
                WebChatRealtimeVoiceVisibleState.HANGUP_UNCONFIRMED
            WebChatRealtimeVoiceLifecycle.FAILED -> WebChatRealtimeVoiceVisibleState.FAILED
            WebChatRealtimeVoiceLifecycle.ACTIVE -> if (state.paused) {
                WebChatRealtimeVoiceVisibleState.PAUSED
            } else when (state.turn) {
                WebChatRealtimeVoiceTurn.LISTENING -> WebChatRealtimeVoiceVisibleState.LISTENING
                WebChatRealtimeVoiceTurn.THINKING -> WebChatRealtimeVoiceVisibleState.THINKING
                WebChatRealtimeVoiceTurn.SPEAKING -> WebChatRealtimeVoiceVisibleState.SPEAKING
                WebChatRealtimeVoiceTurn.UNKNOWN,
                WebChatRealtimeVoiceTurn.IDLE -> WebChatRealtimeVoiceVisibleState.IDLE
            }
        }

    fun expansionDecision(
        state: WebChatRealtimeVoiceVisibleState,
    ): WebChatRealtimeVoiceExpansionDecision = when (state) {
        WebChatRealtimeVoiceVisibleState.FAILED -> WebChatRealtimeVoiceExpansionDecision.EXPAND
        WebChatRealtimeVoiceVisibleState.HANGUP_UNCONFIRMED ->
            WebChatRealtimeVoiceExpansionDecision.COLLAPSE
        else -> WebChatRealtimeVoiceExpansionDecision.PRESERVE
    }
}

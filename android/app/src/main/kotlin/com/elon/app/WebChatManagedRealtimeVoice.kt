package com.elon.app

internal enum class WebChatManagedRealtimeVoicePhase {
    UNAVAILABLE,
    IDLE,
    STARTING,
    ACTIVE,
    OFFICIAL_FALLBACK,
    FAILED,
    CLOSED,
}

internal data class WebChatManagedRealtimeVoiceState(
    val phase: WebChatManagedRealtimeVoicePhase,
    val code: String? = null,
) {
    val managed: Boolean
        get() = phase in setOf(
            WebChatManagedRealtimeVoicePhase.STARTING,
            WebChatManagedRealtimeVoicePhase.ACTIVE,
            WebChatManagedRealtimeVoicePhase.OFFICIAL_FALLBACK,
        )

    companion object {
        val Unavailable = WebChatManagedRealtimeVoiceState(
            WebChatManagedRealtimeVoicePhase.UNAVAILABLE,
        )
    }
}

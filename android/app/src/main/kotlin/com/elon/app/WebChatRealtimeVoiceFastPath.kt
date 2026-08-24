package com.elon.app

internal object WebChatRealtimeVoiceFastPath {
    fun shouldRetryVisibleSurface(lifecycle: WebChatRealtimeVoiceLifecycle?): Boolean =
        lifecycle == WebChatRealtimeVoiceLifecycle.FAILED

    fun canStartAfterPreparation(
        controlsAlreadyCurrent: Boolean,
        state: WebChatConsumerState,
    ): Boolean = controlsAlreadyCurrent && state.controls.any { descriptor ->
        descriptor.control.semantic == REALTIME_VOICE_SEMANTIC && descriptor.control.enabled
    }

    private const val REALTIME_VOICE_SEMANTIC = "voice_mode"
}

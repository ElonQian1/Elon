package com.elon.app

internal sealed interface WebChatRealtimeVoiceDelayedCloseDecision {
    data class Wait(val refreshControls: Boolean) : WebChatRealtimeVoiceDelayedCloseDecision
    data object Complete : WebChatRealtimeVoiceDelayedCloseDecision
    data object Expired : WebChatRealtimeVoiceDelayedCloseDecision
}

/**
 * Observes a late official hang-up without clicking the official control again.
 * The normal close path already owns command retries; this bounded tail only
 * reconciles a delayed page transition so the native voice surface cannot stay stale.
 */
internal class WebChatRealtimeVoiceDelayedCloseReconciler(
    private val maxPolls: Int = DEFAULT_MAX_POLLS,
    private val stableConversationPolls: Int = DEFAULT_STABLE_CONVERSATION_POLLS,
    private val controlRefreshInterval: Int = DEFAULT_CONTROL_REFRESH_INTERVAL,
) {
    private var polls = 0
    private var consecutiveConversationPolls = 0

    init {
        require(maxPolls > 0)
        require(stableConversationPolls > 0)
        require(controlRefreshInterval > 0)
    }

    fun begin() {
        polls = 0
        consecutiveConversationPolls = 0
    }

    fun observe(state: WebChatConsumerState?): WebChatRealtimeVoiceDelayedCloseDecision {
        val officialVoiceEnded = state?.adapterCurrent == true &&
            state.pageKind == CONVERSATION_PAGE_KIND &&
            WebChatRealtimeVoiceEndPolicy.resolve(state.controls) == null
        consecutiveConversationPolls = if (officialVoiceEnded) {
            consecutiveConversationPolls + 1
        } else {
            0
        }
        if (consecutiveConversationPolls >= stableConversationPolls) {
            return WebChatRealtimeVoiceDelayedCloseDecision.Complete
        }
        if (polls >= maxPolls) return WebChatRealtimeVoiceDelayedCloseDecision.Expired
        polls += 1
        return WebChatRealtimeVoiceDelayedCloseDecision.Wait(
            refreshControls = polls == 1 || polls % controlRefreshInterval == 0,
        )
    }

    private companion object {
        const val CONVERSATION_PAGE_KIND = "conversation"
        const val DEFAULT_MAX_POLLS = 120
        const val DEFAULT_STABLE_CONVERSATION_POLLS = 4
        const val DEFAULT_CONTROL_REFRESH_INTERVAL = 5
    }
}

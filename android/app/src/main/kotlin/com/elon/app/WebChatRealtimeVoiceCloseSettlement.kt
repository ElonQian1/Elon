package com.elon.app

internal sealed interface WebChatRealtimeVoiceCloseDecision {
    data class InvokeEnd(val control: WebChatConsumerControl) : WebChatRealtimeVoiceCloseDecision

    data class Wait(val refreshControls: Boolean) : WebChatRealtimeVoiceCloseDecision

    data object CompleteGracefully : WebChatRealtimeVoiceCloseDecision

    data object CompleteInterrupted : WebChatRealtimeVoiceCloseDecision
}

/**
 * Waits for the official voice surface to expose its hangup action, then confirms the
 * conversation has returned before the hidden WebView is released. A missing manifest
 * is treated as pending observation rather than immediate failure.
 */
internal class WebChatRealtimeVoiceCloseSettlement(
    private val maxPolls: Int = DEFAULT_MAX_POLLS,
    private val stableConversationPolls: Int = DEFAULT_STABLE_CONVERSATION_POLLS,
    private val controlRefreshInterval: Int = DEFAULT_CONTROL_REFRESH_INTERVAL,
) {
    private var polls = 0
    private var endInvoked = false
    private var consecutiveConversationPolls = 0

    init {
        require(maxPolls > 0)
        require(stableConversationPolls > 0)
        require(controlRefreshInterval > 0)
    }

    fun begin() {
        polls = 0
        endInvoked = false
        consecutiveConversationPolls = 0
    }

    fun observe(state: WebChatConsumerState?): WebChatRealtimeVoiceCloseDecision {
        val controls = state?.controls.orEmpty()
        val endControl = WebChatRealtimeVoiceEndPolicy.resolve(controls)
        if (!endInvoked && endControl != null) {
            return WebChatRealtimeVoiceCloseDecision.InvokeEnd(endControl)
        }

        if (
            endInvoked && state?.adapterCurrent == true &&
            state.pageKind == CONVERSATION_PAGE_KIND && endControl == null
        ) {
            consecutiveConversationPolls += 1
            if (consecutiveConversationPolls >= stableConversationPolls) {
                return WebChatRealtimeVoiceCloseDecision.CompleteGracefully
            }
        } else {
            consecutiveConversationPolls = 0
        }

        if (polls >= maxPolls) {
            return WebChatRealtimeVoiceCloseDecision.CompleteInterrupted
        }
        polls += 1
        return WebChatRealtimeVoiceCloseDecision.Wait(
            refreshControls = polls == 1 || polls % controlRefreshInterval == 0,
        )
    }

    fun endInvocationAccepted() {
        endInvoked = true
        consecutiveConversationPolls = 0
    }

    fun reset() = begin()

    private companion object {
        const val CONVERSATION_PAGE_KIND = "conversation"
        const val DEFAULT_MAX_POLLS = 40
        const val DEFAULT_STABLE_CONVERSATION_POLLS = 4
        const val DEFAULT_CONTROL_REFRESH_INTERVAL = 3
    }
}

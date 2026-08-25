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
    private val watchdogDelaysMs: LongArray = DEFAULT_WATCHDOG_DELAYS_MS,
    private val stableConversationPolls: Int = DEFAULT_STABLE_CONVERSATION_POLLS,
    private val stableConversationMs: Long = DEFAULT_STABLE_CONVERSATION_MS,
    private val controlRefreshInterval: Int = DEFAULT_CONTROL_REFRESH_INTERVAL,
) {
    private var watchdogChecks = 0
    private var consecutiveConversationPolls = 0
    private var conversationEvidenceStartedAtMs: Long? = null

    init {
        require(watchdogDelaysMs.isNotEmpty())
        require(watchdogDelaysMs.all { it > 0L })
        require(stableConversationPolls > 0)
        require(stableConversationMs >= 0L)
        require(controlRefreshInterval > 0)
    }

    fun begin() {
        watchdogChecks = 0
        consecutiveConversationPolls = 0
        conversationEvidenceStartedAtMs = null
    }

    fun nextWatchdogDelayMs(): Long? = watchdogDelaysMs.getOrNull(watchdogChecks)

    fun observeEvent(
        state: WebChatConsumerState?,
        observedAtMs: Long,
    ): WebChatRealtimeVoiceDelayedCloseDecision = observe(state, observedAtMs, watchdog = false)

    fun observeWatchdog(
        state: WebChatConsumerState?,
        observedAtMs: Long,
    ): WebChatRealtimeVoiceDelayedCloseDecision = observe(state, observedAtMs, watchdog = true)

    private fun observe(
        state: WebChatConsumerState?,
        observedAtMs: Long,
        watchdog: Boolean,
    ): WebChatRealtimeVoiceDelayedCloseDecision {
        val officialVoiceEnded = state?.adapterCurrent == true &&
            state.pageKind == CONVERSATION_PAGE_KIND &&
            WebChatRealtimeVoiceEndPolicy.resolve(state.controls) == null
        consecutiveConversationPolls = if (officialVoiceEnded) {
            if (conversationEvidenceStartedAtMs == null) {
                conversationEvidenceStartedAtMs = observedAtMs
            }
            consecutiveConversationPolls + 1
        } else {
            conversationEvidenceStartedAtMs = null
            0
        }
        val stableForMs = conversationEvidenceStartedAtMs?.let { startedAt ->
            (observedAtMs - startedAt).coerceAtLeast(0L)
        } ?: 0L
        if (
            consecutiveConversationPolls >= stableConversationPolls &&
            stableForMs >= stableConversationMs
        ) {
            return WebChatRealtimeVoiceDelayedCloseDecision.Complete
        }
        if (!watchdog) {
            return WebChatRealtimeVoiceDelayedCloseDecision.Wait(refreshControls = false)
        }
        if (watchdogChecks >= watchdogDelaysMs.size) {
            return WebChatRealtimeVoiceDelayedCloseDecision.Expired
        }
        val currentCheck = watchdogChecks
        watchdogChecks += 1
        if (watchdogChecks >= watchdogDelaysMs.size) {
            return WebChatRealtimeVoiceDelayedCloseDecision.Expired
        }
        return WebChatRealtimeVoiceDelayedCloseDecision.Wait(
            refreshControls = currentCheck == 0 ||
                (currentCheck + 1) % controlRefreshInterval == 0,
        )
    }

    private companion object {
        const val CONVERSATION_PAGE_KIND = "conversation"
        const val DEFAULT_STABLE_CONVERSATION_POLLS = 2
        const val DEFAULT_STABLE_CONVERSATION_MS = 2_000L
        const val DEFAULT_CONTROL_REFRESH_INTERVAL = 2
        val DEFAULT_WATCHDOG_DELAYS_MS = longArrayOf(
            1_000L,
            1_000L,
            2_000L,
            3_000L,
            5_000L,
            8_000L,
            15_000L,
            25_000L,
            30_000L,
            30_000L,
        )
    }
}

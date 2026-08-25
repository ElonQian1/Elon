package com.elon.app

/**
 * Reconciles a late official hang-up from page events first and a sparse watchdog second.
 * Scheduled callbacks are epoch-bound because the host scheduler does not expose cancellation.
 */
internal class WebChatRealtimeVoiceDelayedCloseMonitor(
    private val schedule: (Runnable, Long) -> Unit,
    private val currentState: () -> WebChatConsumerState?,
    private val requestControls: () -> Unit,
    private val nowMs: () -> Long,
    private val onConfirmed: (WebChatConsumerState?) -> Unit,
    private val onExpired: () -> Unit,
    private val reconciler: WebChatRealtimeVoiceDelayedCloseReconciler =
        WebChatRealtimeVoiceDelayedCloseReconciler(),
) {
    private var active = false
    private var watchdogScheduled = false
    private var epoch = 0

    fun begin() {
        active = true
        watchdogScheduled = false
        epoch += 1
        reconciler.begin()
        scheduleNext(epoch)
    }

    fun cancel() {
        active = false
        watchdogScheduled = false
        epoch += 1
        reconciler.begin()
    }

    fun observe(state: WebChatConsumerState) {
        if (!active) return
        advance(state, watchdog = false, expectedEpoch = epoch)
    }

    private fun scheduleNext(expectedEpoch: Int) {
        if (!active || expectedEpoch != epoch || watchdogScheduled) return
        val delayMs = reconciler.nextWatchdogDelayMs() ?: run {
            expire()
            return
        }
        watchdogScheduled = true
        schedule(Runnable {
            if (!active || expectedEpoch != epoch) return@Runnable
            watchdogScheduled = false
            advance(currentState(), watchdog = true, expectedEpoch = expectedEpoch)
        }, delayMs)
    }

    private fun advance(
        state: WebChatConsumerState?,
        watchdog: Boolean,
        expectedEpoch: Int,
    ) {
        if (!active || expectedEpoch != epoch) return
        val decision = if (watchdog) {
            reconciler.observeWatchdog(state, nowMs())
        } else {
            reconciler.observeEvent(state, nowMs())
        }
        when (decision) {
            is WebChatRealtimeVoiceDelayedCloseDecision.Wait -> {
                if (decision.refreshControls) requestControls()
                if (watchdog) scheduleNext(expectedEpoch)
            }
            WebChatRealtimeVoiceDelayedCloseDecision.Complete -> {
                active = false
                watchdogScheduled = false
                epoch += 1
                onConfirmed(state)
            }
            WebChatRealtimeVoiceDelayedCloseDecision.Expired -> expire()
        }
    }

    private fun expire() {
        active = false
        watchdogScheduled = false
        epoch += 1
        onExpired()
    }
}

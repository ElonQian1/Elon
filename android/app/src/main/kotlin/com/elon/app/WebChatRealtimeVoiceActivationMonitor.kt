package com.elon.app

/**
 * Reconciles a voice launch whose official command was accepted but whose page
 * evidence arrived late. Missing DOM evidence is not proof that audio stopped.
 */
internal class WebChatRealtimeVoiceActivationMonitor(
    private val schedule: (Runnable, Long) -> Unit,
    private val observeActivation: () -> WebChatRealtimeVoiceActivationDecision,
    private val requestControls: () -> Unit,
    private val onConfirmed: () -> Unit,
    private val onRejected: (String) -> Unit,
    private val onWatchdogExhausted: () -> Unit,
    private val watchdogDelaysMs: LongArray = DEFAULT_WATCHDOG_DELAYS_MS,
    private val controlRefreshInterval: Int = DEFAULT_CONTROL_REFRESH_INTERVAL,
) {
    private var active = false
    private var watchdogIndex = 0
    private var epoch = 0

    init {
        require(watchdogDelaysMs.isNotEmpty())
        require(watchdogDelaysMs.all { it > 0L })
        require(controlRefreshInterval > 0)
    }

    fun begin() {
        active = true
        watchdogIndex = 0
        epoch += 1
        scheduleNext(epoch)
    }

    fun cancel() {
        active = false
        watchdogIndex = 0
        epoch += 1
    }

    fun observeEvent() {
        if (active) advance(watchdog = false, expectedEpoch = epoch)
    }

    private fun scheduleNext(expectedEpoch: Int) {
        if (!active || expectedEpoch != epoch) return
        val delayMs = watchdogDelaysMs.getOrNull(watchdogIndex) ?: run {
            onWatchdogExhausted()
            return
        }
        schedule(Runnable {
            if (!active || expectedEpoch != epoch) return@Runnable
            advance(watchdog = true, expectedEpoch = expectedEpoch)
        }, delayMs)
    }

    private fun advance(watchdog: Boolean, expectedEpoch: Int) {
        if (!active || expectedEpoch != epoch) return
        when (val decision = observeActivation()) {
            WebChatRealtimeVoiceActivationDecision.Active -> finish(onConfirmed)
            is WebChatRealtimeVoiceActivationDecision.Rejected ->
                finish { onRejected(decision.detail) }
            WebChatRealtimeVoiceActivationDecision.Unconfirmed,
            is WebChatRealtimeVoiceActivationDecision.Wait -> {
                if (!watchdog) return
                val completedIndex = watchdogIndex
                watchdogIndex += 1
                if (completedIndex % controlRefreshInterval == 0) requestControls()
                scheduleNext(expectedEpoch)
            }
        }
    }

    private fun finish(action: () -> Unit) {
        active = false
        epoch += 1
        action()
    }

    private companion object {
        const val DEFAULT_CONTROL_REFRESH_INTERVAL = 3
        val DEFAULT_WATCHDOG_DELAYS_MS = longArrayOf(
            1_000L,
            2_000L,
            4_000L,
            8_000L,
            15_000L,
            30_000L,
            60_000L,
        )
    }
}

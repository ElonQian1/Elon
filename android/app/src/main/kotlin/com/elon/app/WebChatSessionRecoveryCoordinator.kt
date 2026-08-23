package com.elon.app

/**
 * Recovers a foreground web-chat session without turning every slow bridge into a page reload.
 * Navigation progress extends the stall deadline; a finished document gets one bridge repair
 * before the coordinator permits one bounded full-page reload.
 */
internal class WebChatSessionRecoveryCoordinator(
    private val schedule: (Runnable, Long) -> Unit,
    private val cancel: (Runnable) -> Unit,
    private val retry: () -> Boolean,
    private val repair: () -> Boolean = { false },
    private val onExhausted: () -> Unit,
    retryDelaysMs: List<Long> = DEFAULT_RETRY_DELAYS_MS,
    private val navigationStallTimeoutMs: Long = DEFAULT_NAVIGATION_STALL_TIMEOUT_MS,
    private val bridgeReadinessTimeoutMs: Long = DEFAULT_BRIDGE_READINESS_TIMEOUT_MS,
) {
    private enum class Phase { IDLE, NAVIGATING, WAITING_BRIDGE }

    private val retryDelaysMs = retryDelaysMs.toList()
    private var active = false
    private var exhausted = false
    private var retryIndex = 0
    private var repairAttempted = false
    private var lastNavigationProgress = -1
    private var phase = Phase.IDLE
    private var retryTask: Runnable? = null
    private var readinessTask: Runnable? = null

    init {
        require(this.retryDelaysMs.isNotEmpty())
        require(this.retryDelaysMs.all { it >= 0L })
        require(navigationStallTimeoutMs > 0L)
        require(bridgeReadinessTimeoutMs > 0L)
    }

    fun activate() {
        if (!active) resetBudget()
        active = true
    }

    fun isActive(): Boolean = active

    fun deactivate() {
        active = false
        cancelPending()
    }

    fun onNavigationStarted() {
        if (!active) return
        phase = Phase.NAVIGATING
        lastNavigationProgress = -1
        cancelRetry()
        resetReadinessWatchdog(navigationStallTimeoutMs)
    }

    fun onNavigationProgress(progress: Int) {
        if (!active || phase != Phase.NAVIGATING || progress <= lastNavigationProgress) return
        lastNavigationProgress = progress.coerceIn(0, 100)
        resetReadinessWatchdog(navigationStallTimeoutMs)
    }

    fun onPageFinished() {
        if (!active) return
        if (phase == Phase.WAITING_BRIDGE && readinessTask != null) return
        phase = Phase.WAITING_BRIDGE
        resetReadinessWatchdog(bridgeReadinessTimeoutMs)
    }

    fun onReady() {
        cancelPending()
        resetBudget()
    }

    fun onTerminal() {
        cancelPending()
        resetBudget()
    }

    fun onFailure() {
        if (!active || exhausted) return
        cancelReadiness()
        if (phase == Phase.WAITING_BRIDGE && !repairAttempted) {
            repairAttempted = true
            if (repair()) {
                resetReadinessWatchdog(bridgeReadinessTimeoutMs)
                return
            }
        }
        scheduleRetry()
    }

    fun retryNow(): Boolean {
        if (!active) return false
        cancelPending()
        resetBudget()
        return dispatchRetry()
    }

    fun dispose() {
        deactivate()
        resetBudget()
    }

    private fun scheduleRetry() {
        if (retryTask != null) return
        val delayMs = retryDelaysMs.getOrNull(retryIndex) ?: run {
            exhaust()
            return
        }
        lateinit var task: Runnable
        task = Runnable {
            if (retryTask !== task) return@Runnable
            retryTask = null
            if (active) dispatchRetry()
        }
        retryTask = task
        schedule(task, delayMs)
    }

    private fun dispatchRetry(): Boolean {
        if (retryIndex >= retryDelaysMs.size) {
            exhaust()
            return false
        }
        retryIndex += 1
        phase = Phase.NAVIGATING
        lastNavigationProgress = -1
        val dispatched = retry()
        if (dispatched) resetReadinessWatchdog(navigationStallTimeoutMs) else scheduleRetry()
        return dispatched
    }

    private fun resetReadinessWatchdog(delayMs: Long) {
        cancelReadiness()
        lateinit var task: Runnable
        task = Runnable {
            if (readinessTask !== task) return@Runnable
            readinessTask = null
            onFailure()
        }
        readinessTask = task
        schedule(task, delayMs)
    }

    private fun resetBudget() {
        exhausted = false
        retryIndex = 0
        repairAttempted = false
        lastNavigationProgress = -1
        phase = Phase.IDLE
    }

    private fun exhaust() {
        if (exhausted) return
        exhausted = true
        cancelPending()
        onExhausted()
    }

    private fun cancelPending() {
        cancelRetry()
        cancelReadiness()
    }

    private fun cancelRetry() {
        retryTask?.let(cancel)
        retryTask = null
    }

    private fun cancelReadiness() {
        readinessTask?.let(cancel)
        readinessTask = null
    }

    private companion object {
        val DEFAULT_RETRY_DELAYS_MS = listOf(2_000L)
        const val DEFAULT_NAVIGATION_STALL_TIMEOUT_MS = 30_000L
        const val DEFAULT_BRIDGE_READINESS_TIMEOUT_MS = 10_000L
    }
}

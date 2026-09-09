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
    private val onExhausted: (WebChatSessionRecoveryFailure) -> Unit,
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
    private var lastFailure: WebChatSessionRecoveryFailure? = null
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
        lastFailure = null
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

    fun onPageFailure(detail: String) = onFailure(
        WebChatSessionRecoveryFailure(WebChatSessionRecoveryFailure.Kind.PAGE_ERROR, detail),
    )

    fun onFailure(failure: WebChatSessionRecoveryFailure? = null) {
        if (!active || exhausted) return
        if (failure != null) lastFailure = failure
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
        lastFailure = null
        val dispatched = retry()
        if (dispatched) {
            resetReadinessWatchdog(navigationStallTimeoutMs)
        } else {
            lastFailure = WebChatSessionRecoveryFailure(WebChatSessionRecoveryFailure.Kind.RETRY_NOT_STARTED)
            scheduleRetry()
        }
        return dispatched
    }

    private fun resetReadinessWatchdog(delayMs: Long) {
        cancelReadiness()
        val timeoutKind = if (phase == Phase.WAITING_BRIDGE) {
            WebChatSessionRecoveryFailure.Kind.BRIDGE_TIMEOUT
        } else {
            WebChatSessionRecoveryFailure.Kind.NAVIGATION_TIMEOUT
        }
        lateinit var task: Runnable
        task = Runnable {
            if (readinessTask !== task) return@Runnable
            readinessTask = null
            // WebView may finish an error document after reporting its actual HTTP/TLS error.
            val failure = lastFailure?.takeIf { it.kind == WebChatSessionRecoveryFailure.Kind.PAGE_ERROR }
                ?: WebChatSessionRecoveryFailure(timeoutKind)
            onFailure(failure)
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
        lastFailure = null
    }

    private fun exhaust() {
        if (exhausted) return
        exhausted = true
        cancelPending()
        onExhausted(lastFailure ?: WebChatSessionRecoveryFailure(WebChatSessionRecoveryFailure.Kind.UNKNOWN))
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

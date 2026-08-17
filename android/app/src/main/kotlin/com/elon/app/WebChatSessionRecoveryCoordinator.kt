package com.elon.app

/**
 * Bounds recovery work for a foreground web-chat session.
 *
 * The owner supplies the scheduler so this policy stays independent from Android lifecycle types.
 */
internal class WebChatSessionRecoveryCoordinator(
    private val schedule: (Runnable, Long) -> Unit,
    private val cancel: (Runnable) -> Unit,
    private val retry: () -> Boolean,
    private val onExhausted: () -> Unit,
    retryDelaysMs: List<Long> = DEFAULT_RETRY_DELAYS_MS,
    private val readinessTimeoutMs: Long = DEFAULT_READINESS_TIMEOUT_MS,
) {
    private val retryDelaysMs = retryDelaysMs.toList()
    private var active = false
    private var exhausted = false
    private var retryIndex = 0
    private var retryTask: Runnable? = null
    private var readinessTask: Runnable? = null

    init {
        require(this.retryDelaysMs.isNotEmpty())
        require(this.retryDelaysMs.all { it >= 0L })
        require(readinessTimeoutMs > 0L)
    }

    fun activate() {
        if (!active) {
            retryIndex = 0
            exhausted = false
        }
        active = true
    }

    fun isActive(): Boolean = active

    fun deactivate() {
        active = false
        cancelPending()
    }

    fun onNavigationStarted() {
        if (!active) return
        cancelRetry()
        armReadinessWatchdog()
    }

    fun onPageFinished() {
        if (!active) return
        armReadinessWatchdog()
    }

    fun onReady() {
        cancelPending()
        retryIndex = 0
        exhausted = false
    }

    fun onTerminal() {
        cancelPending()
        retryIndex = 0
        exhausted = false
    }

    fun onFailure() {
        if (!active || exhausted) return
        cancelReadiness()
        scheduleRetry()
    }

    fun retryNow(): Boolean {
        if (!active) return false
        cancelPending()
        retryIndex = 0
        exhausted = false
        return dispatchRetry()
    }

    fun dispose() {
        deactivate()
        retryIndex = 0
        exhausted = false
    }

    private fun scheduleRetry() {
        if (retryTask != null) return
        val delayMs = retryDelaysMs.getOrNull(retryIndex) ?: run {
            exhaust()
            return
        }
        retryIndex += 1
        lateinit var task: Runnable
        task = Runnable {
            if (retryTask !== task) return@Runnable
            retryTask = null
            if (!active) return@Runnable
            dispatchRetry()
        }
        retryTask = task
        schedule(task, delayMs)
    }

    private fun dispatchRetry(): Boolean {
        val dispatched = retry()
        if (dispatched) {
            armReadinessWatchdog()
        } else {
            scheduleRetry()
        }
        return dispatched
    }

    private fun armReadinessWatchdog() {
        cancelReadiness()
        lateinit var task: Runnable
        task = Runnable {
            if (readinessTask !== task) return@Runnable
            readinessTask = null
            onFailure()
        }
        readinessTask = task
        schedule(task, readinessTimeoutMs)
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
        val DEFAULT_RETRY_DELAYS_MS = listOf(2_000L, 5_000L, 15_000L)
        const val DEFAULT_READINESS_TIMEOUT_MS = 20_000L
    }
}

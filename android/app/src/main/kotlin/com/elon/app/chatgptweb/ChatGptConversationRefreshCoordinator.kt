package com.elon.app.chatgptweb

internal class ChatGptConversationRefreshCoordinator(
    private val dispatch: () -> Boolean,
    private val schedule: (Runnable, Long) -> Unit,
    private val cancel: (Runnable) -> Unit,
    private val retryDelaysMs: List<Long> = DEFAULT_RETRY_DELAYS_MS,
) {
    private var inFlight = false
    private var retryIndex = 0
    private var scheduledRetry: Runnable? = null

    val isBusy: Boolean
        get() = inFlight || scheduledRetry != null

    fun requestNow(): Boolean {
        if (inFlight) return true
        cancelScheduledRetry()
        retryIndex = 0
        return dispatchIfIdle()
    }

    fun requestIfIdle(): Boolean {
        if (isBusy) return false
        retryIndex = 0
        return dispatchIfIdle()
    }

    fun onSucceeded() {
        inFlight = false
        retryIndex = 0
        cancelScheduledRetry()
    }

    fun onFailed() {
        inFlight = false
        scheduleNextRetry()
    }

    fun reset() {
        inFlight = false
        retryIndex = 0
        cancelScheduledRetry()
    }

    private fun dispatchIfIdle(): Boolean {
        if (inFlight) return false
        return dispatch().also { inFlight = it }
    }

    private fun scheduleNextRetry() {
        cancelScheduledRetry()
        val delayMs = retryDelaysMs.getOrNull(retryIndex++) ?: return
        lateinit var retry: Runnable
        retry = Runnable {
            if (scheduledRetry !== retry) return@Runnable
            scheduledRetry = null
            if (!dispatchIfIdle()) scheduleNextRetry()
        }
        scheduledRetry = retry
        schedule(retry, delayMs)
    }

    private fun cancelScheduledRetry() {
        scheduledRetry?.let(cancel)
        scheduledRetry = null
    }

    private companion object {
        val DEFAULT_RETRY_DELAYS_MS = listOf(2_000L, 5_000L, 15_000L, 30_000L)
    }
}

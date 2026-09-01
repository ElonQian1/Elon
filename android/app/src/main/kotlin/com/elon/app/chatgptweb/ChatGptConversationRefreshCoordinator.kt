package com.elon.app.chatgptweb

internal class ChatGptConversationRefreshCoordinator(
    private val dispatch: () -> Boolean,
    private val schedule: (Runnable, Long) -> Unit,
    private val cancel: (Runnable) -> Unit,
    private val retryDelaysMs: List<Long> = DEFAULT_RETRY_DELAYS_MS,
) {
    private var inFlight = false
    private var refreshAgain = false
    private var retryIndex = 0
    private var scheduledRetry: Runnable? = null
    private var suppressCurrentCompletion = false

    val isBusy: Boolean
        get() = inFlight || scheduledRetry != null

    fun requestNow(): Boolean {
        if (inFlight) return true
        cancelScheduledRetry()
        retryIndex = 0
        return dispatchOrScheduleRetry()
    }

    fun requestIfIdle(): Boolean {
        if (isBusy) return false
        retryIndex = 0
        return dispatchIfIdle()
    }

    fun requestAfterCurrent(): Boolean {
        cancelScheduledRetry()
        retryIndex = 0
        if (inFlight) {
            refreshAgain = true
            return true
        }
        return dispatchOrScheduleRetry()
    }

    fun onSucceeded() {
        if (consumeSuppressedCompletion()) return
        inFlight = false
        retryIndex = 0
        cancelScheduledRetry()
        dispatchQueuedRefresh()
    }

    fun onFailed() {
        if (consumeSuppressedCompletion()) return
        inFlight = false
        if (!dispatchQueuedRefresh()) scheduleNextRetry()
    }

    fun yieldToUserNavigation() {
        refreshAgain = false
        retryIndex = 0
        cancelScheduledRetry()
        suppressCurrentCompletion = inFlight
    }

    fun reset() {
        inFlight = false
        refreshAgain = false
        retryIndex = 0
        suppressCurrentCompletion = false
        cancelScheduledRetry()
    }

    private fun dispatchIfIdle(): Boolean {
        if (inFlight) return false
        return dispatch().also { inFlight = it }
    }

    private fun dispatchOrScheduleRetry(): Boolean {
        if (dispatchIfIdle()) return true
        scheduleNextRetry()
        return scheduledRetry != null
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

    private fun dispatchQueuedRefresh(): Boolean {
        if (!refreshAgain) return false
        refreshAgain = false
        return dispatchIfIdle()
    }

    private fun consumeSuppressedCompletion(): Boolean {
        if (!suppressCurrentCompletion) return false
        suppressCurrentCompletion = false
        inFlight = false
        retryIndex = 0
        cancelScheduledRetry()
        return true
    }

    private fun cancelScheduledRetry() {
        scheduledRetry?.let(cancel)
        scheduledRetry = null
    }

    private companion object {
        val DEFAULT_RETRY_DELAYS_MS = listOf(2_000L, 5_000L, 15_000L, 30_000L)
    }
}

internal object ChatGptConversationRefreshScopePolicy {
    fun select(
        pendingProjectId: String?,
        requestedProjectId: String?,
        refreshBusy: Boolean,
    ): String? = when {
        requestedProjectId != null -> requestedProjectId
        refreshBusy -> pendingProjectId
        else -> null
    }
}

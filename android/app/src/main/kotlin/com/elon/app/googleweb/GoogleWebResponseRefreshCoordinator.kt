package com.elon.app.googleweb

internal class GoogleWebResponseRefreshCoordinator(
    private val requestSnapshot: () -> Unit,
    private val schedule: (Runnable, Long) -> Unit,
    private val cancel: (Runnable) -> Unit,
    private val delaysMs: List<Long> = DEFAULT_DELAYS_MS,
) {
    private var generation = 0L
    private var nextDelayIndex = 0
    private var scheduled: Runnable? = null
    private var expectedPrompt = ""

    val isActive: Boolean
        get() = scheduled != null

    fun onSendStarted(prompt: String) {
        stop()
        expectedPrompt = normalize(prompt)
    }

    fun onSendConfirmed() {
        if (expectedPrompt.isBlank()) return
        cancelScheduled()
        generation += 1
        nextDelayIndex = 0
        scheduleNext(generation)
    }

    fun onSnapshot(
        latestUserPrompt: String?,
        assistantObserved: Boolean,
        streaming: Boolean,
    ) {
        if (
            normalize(latestUserPrompt.orEmpty()) == expectedPrompt &&
            assistantObserved &&
            !streaming
        ) stop()
    }

    fun stop() {
        generation += 1
        cancelScheduled()
        nextDelayIndex = 0
        expectedPrompt = ""
    }

    private fun scheduleNext(expectedGeneration: Long) {
        val delayMs = delaysMs.getOrNull(nextDelayIndex++) ?: run {
            scheduled = null
            return
        }
        lateinit var task: Runnable
        task = Runnable {
            if (scheduled !== task || generation != expectedGeneration) return@Runnable
            scheduled = null
            requestSnapshot()
            scheduleNext(expectedGeneration)
        }
        scheduled = task
        schedule(task, delayMs)
    }

    private fun cancelScheduled() {
        scheduled?.let(cancel)
        scheduled = null
    }

    private fun normalize(value: String): String = value.trim().replace(WHITESPACE, " ")

    private companion object {
        val DEFAULT_DELAYS_MS = listOf(400L, 800L, 1_500L, 2_500L, 4_000L, 6_000L, 8_000L, 10_000L)
        val WHITESPACE = Regex("\\s+")
    }
}

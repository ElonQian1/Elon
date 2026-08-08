package com.elon.app.chatgptweb

internal class ChatGptWebBridgeHandshake(
    private val schedule: (delayMs: Long, action: () -> Unit) -> Unit,
    private val injectAndRequestSnapshot: () -> Unit,
    private val maxAttempts: Int = DEFAULT_MAX_ATTEMPTS,
    private val retryDelayMs: Long = DEFAULT_RETRY_DELAY_MS,
) {
    private var generation = 0
    private var attemptCount = 0

    init {
        require(maxAttempts > 0)
        require(retryDelayMs >= 0)
    }

    fun start() {
        val activeGeneration = ++generation
        attemptCount = 0
        attempt(activeGeneration)
    }

    fun acknowledge() {
        generation++
    }

    fun cancel() {
        generation++
    }

    private fun attempt(activeGeneration: Int) {
        if (activeGeneration != generation || attemptCount >= maxAttempts) return

        attemptCount++
        injectAndRequestSnapshot()
        if (attemptCount < maxAttempts) {
            schedule(retryDelayMs) { attempt(activeGeneration) }
        }
    }

    private companion object {
        const val DEFAULT_MAX_ATTEMPTS = 12
        const val DEFAULT_RETRY_DELAY_MS = 1_250L
    }
}

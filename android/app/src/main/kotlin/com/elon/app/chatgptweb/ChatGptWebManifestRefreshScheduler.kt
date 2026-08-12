package com.elon.app.chatgptweb

internal class ChatGptWebManifestRefreshScheduler(
    private val schedule: (Long, () -> Unit) -> Unit,
    private val refresh: () -> Unit,
) {
    private var generation = 0

    fun afterAdaptiveTouch() {
        val requestGeneration = ++generation
        SETTLE_DELAYS_MS.forEach { delayMs ->
            schedule(delayMs) {
                if (generation == requestGeneration) refresh()
            }
        }
    }

    fun dispose() {
        generation += 1
    }

    internal companion object {
        val SETTLE_DELAYS_MS = listOf(360L, 900L, 1_800L, 3_200L)
    }
}

package com.elon.app.chatgptweb

internal class ChatGptWebOfficialOverlayController(
    private val dispatchEscape: () -> Unit,
    private val schedule: (Long, () -> Unit) -> Unit,
    private val refreshManifest: () -> Unit,
) {
    private var generation = 0

    fun dismissTop() {
        val requestGeneration = ++generation
        dispatchEscape()
        schedule(TOP_DISMISS_SETTLE_MS) {
            if (generation == requestGeneration) refreshManifest()
        }
    }

    fun dismissAllThen(action: () -> Unit) {
        val requestGeneration = ++generation
        dismissNext(requestGeneration, MAX_DISMISS_ATTEMPTS, action)
    }

    fun dispose() {
        generation += 1
    }

    private fun dismissNext(requestGeneration: Int, remaining: Int, action: () -> Unit) {
        if (generation != requestGeneration) return
        if (remaining <= 0) {
            refreshManifest()
            schedule(ALL_DISMISSED_SETTLE_MS) {
                if (generation == requestGeneration) action()
            }
            return
        }
        dispatchEscape()
        schedule(DISMISS_INTERVAL_MS) {
            dismissNext(requestGeneration, remaining - 1, action)
        }
    }

    private companion object {
        const val MAX_DISMISS_ATTEMPTS = 3
        const val DISMISS_INTERVAL_MS = 120L
        const val TOP_DISMISS_SETTLE_MS = 250L
        const val ALL_DISMISSED_SETTLE_MS = 180L
    }
}

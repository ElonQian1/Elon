package com.elon.app.chatgptweb

/** Shared bounded hydration reads for the group picker and the send-time configuration. */
internal class GroupWebAiModelCatalogRetry(
    private val schedule: (Long, () -> Unit) -> Unit,
    private val shouldRead: () -> Boolean,
    private val read: () -> Unit,
    private val exhausted: () -> Unit,
) {
    private var attempts = 0
    private var pending = false

    fun onEmpty() {
        if (pending || !shouldRead()) return
        if (attempts == DELAYS.size) { exhausted(); return }
        pending = true
        schedule(DELAYS[attempts++]) {
            pending = false
            if (shouldRead()) read()
        }
    }

    private companion object {
        val DELAYS = longArrayOf(500L, 1_000L, 2_000L, 4_000L)
    }
}

package com.elon.app

/** Background discovery budget only; explicit consumer commands do not use it. */
internal class WebChatProductionPrewarmAttempts(
    private val nowMs: () -> Long,
) {
    private val attempts = linkedMapOf<Key, Attempt>()

    fun eligible(key: Key): Boolean {
        val previous = attempts[key] ?: return true
        val now = nowMs()
        return now < previous.startedAtMs || now - previous.startedAtMs >= previous.delayMs
    }

    fun started(key: Key) {
        val previous = attempts.remove(key)
        val now = nowMs()
        val failures = if (previous != null && now >= previous.startedAtMs) previous.failures else 0
        attempts[key] = Attempt(
            startedAtMs = now,
            delayMs = RETRY_DELAYS_MS[failures.coerceAtMost(RETRY_DELAYS_MS.lastIndex)],
            failures = (failures + 1).coerceAtMost(RETRY_DELAYS_MS.size),
        )
        while (attempts.size > MAX_KEYS) {
            // Keep provider catalogs when paging through many conversation controls.
            val oldest = attempts.keys.firstOrNull { it.pageKey != null } ?: attempts.keys.first()
            attempts.remove(oldest)
        }
    }

    fun confirmed(key: Key) {
        attempts.remove(key)
    }

    internal data class Key(
        val providerId: WebChatProviderId,
        val capability: String,
        val pageKey: String? = null,
    )

    private data class Attempt(val startedAtMs: Long, val delayMs: Long, val failures: Int)

    private companion object {
        const val MAX_KEYS = 128
        val RETRY_DELAYS_MS = longArrayOf(60_000L, 5 * 60_000L, 15 * 60_000L)
    }
}

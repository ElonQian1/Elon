package com.elon.app.chatgptweb

internal object WebChatSideMenuRefreshPolicy {
    fun shouldRefreshOnOpen(
        collection: ChatGptWebConversationCollection,
        nowMs: Long,
        lastRequestedAtMs: Long,
    ): Boolean {
        if (collection.officialLoadState == ChatGptWebConversationCollection.LOAD_LOADING) {
            return false
        }
        if (lastRequestedAtMs > 0L && elapsed(nowMs, lastRequestedAtMs) < REQUEST_COOLDOWN_MS) {
            return false
        }
        if (collection.officialLoadState == ChatGptWebConversationCollection.LOAD_FAILED) {
            return true
        }
        if (collection.source == ChatGptWebConversationCollection.SOURCE_NONE) return true
        if (collection.cachedAtMs <= 0L) return collection.stale
        return collection.stale || elapsed(nowMs, collection.cachedAtMs) >= CACHE_FRESHNESS_MS
    }

    private fun elapsed(nowMs: Long, earlierMs: Long): Long =
        (nowMs - earlierMs).coerceAtLeast(0L)

    internal const val REQUEST_COOLDOWN_MS = 15_000L
    internal const val CACHE_FRESHNESS_MS = 2 * 60_000L
}

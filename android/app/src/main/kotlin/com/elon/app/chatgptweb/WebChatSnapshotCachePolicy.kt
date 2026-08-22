package com.elon.app.chatgptweb

internal data class WebChatSnapshotCacheEntry(
    val name: String,
    val lastModifiedMs: Long,
    val sizeBytes: Long,
)

internal object WebChatSnapshotCachePolicy {
    fun isUsable(savedAtMs: Long, nowMs: Long): Boolean =
        nowMs - savedAtMs in 0..MAX_AGE_MS

    fun shouldTouch(lastModifiedMs: Long, nowMs: Long): Boolean =
        nowMs - lastModifiedMs > TOUCH_INTERVAL_MS

    fun retainedNames(entries: List<WebChatSnapshotCacheEntry>): Set<String> {
        var totalBytes = 0L
        return entries
            .sortedWith(compareByDescending<WebChatSnapshotCacheEntry> { it.lastModifiedMs }
                .thenBy { it.name })
            .asSequence()
            .take(MAX_CONVERSATION_ITEMS)
            .takeWhile { entry ->
                val nextTotal = totalBytes + entry.sizeBytes.coerceAtLeast(0L)
                (nextTotal <= MAX_CONVERSATION_BYTES).also { keep ->
                    if (keep) totalBytes = nextTotal
                }
            }
            .mapTo(linkedSetOf()) { it.name }
    }

    const val MAX_MEMORY_ITEMS = 16
    const val MAX_CONVERSATION_ITEMS = 128
    const val MAX_FILE_BYTES = 2 * 1024 * 1024
    const val MAX_CONVERSATION_BYTES = 24L * 1024L * 1024L
    const val MAX_AGE_MS = 30L * 24L * 60L * 60L * 1_000L
    const val TOUCH_INTERVAL_MS = 6L * 60L * 60L * 1_000L
}

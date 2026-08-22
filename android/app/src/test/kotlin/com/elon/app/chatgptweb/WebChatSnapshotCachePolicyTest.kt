package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatSnapshotCachePolicyTest {
    @Test
    fun cacheRemainsUsableForThirtyDays() {
        assertTrue(WebChatSnapshotCachePolicy.isUsable(100L, 100L))
        assertTrue(WebChatSnapshotCachePolicy.isUsable(
            100L,
            100L + WebChatSnapshotCachePolicy.MAX_AGE_MS,
        ))
        assertFalse(WebChatSnapshotCachePolicy.isUsable(
            100L,
            101L + WebChatSnapshotCachePolicy.MAX_AGE_MS,
        ))
    }

    @Test
    fun retentionKeepsTheNewestBoundedPrefix() {
        val entries = (0 until WebChatSnapshotCachePolicy.MAX_CONVERSATION_ITEMS + 2).map { index ->
            WebChatSnapshotCacheEntry("cache-$index", index.toLong(), 1L)
        }

        val retained = WebChatSnapshotCachePolicy.retainedNames(entries)

        assertEquals(WebChatSnapshotCachePolicy.MAX_CONVERSATION_ITEMS, retained.size)
        assertFalse(retained.contains("cache-0"))
        assertFalse(retained.contains("cache-1"))
        assertTrue(retained.contains("cache-${entries.lastIndex}"))
    }

    @Test
    fun retentionStopsBeforeTheTotalDiskBudget() {
        val half = WebChatSnapshotCachePolicy.MAX_CONVERSATION_BYTES / 2L
        val retained = WebChatSnapshotCachePolicy.retainedNames(listOf(
            WebChatSnapshotCacheEntry("newest", 3L, half),
            WebChatSnapshotCacheEntry("middle", 2L, half),
            WebChatSnapshotCacheEntry("oldest", 1L, 1L),
        ))

        assertEquals(setOf("newest", "middle"), retained)
    }
}

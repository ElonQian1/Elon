package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebConversationCollectionJsonTest {
    @Test
    fun exportsOfficialCompletenessAndCacheFreshnessWithoutAmbiguity() {
        val official = ChatGptWebConversationCollectionJson.encode(
            ChatGptWebConversationCollection(
                reachedEnd = true,
                observedCount = 3,
                source = ChatGptWebConversationCollection.SOURCE_OFFICIAL,
                officialLoadState = ChatGptWebConversationCollection.LOAD_READY,
                cachedAtMs = 123L,
            ),
        )
        val cached = ChatGptWebConversationCollectionJson.encode(
            ChatGptWebConversationCollection.cached(count = 2, savedAtMs = 99L),
        )

        assertTrue(official.getBoolean("complete"))
        assertEquals(ChatGptWebConversationCollection.SOURCE_OFFICIAL, official.getString("source"))
        assertFalse(official.getBoolean("stale"))
        assertEquals(ChatGptWebConversationCollection.SOURCE_CACHE, cached.getString("source"))
        assertTrue(cached.getBoolean("stale"))
        assertEquals(99L, cached.getLong("cached_at_ms"))
    }
}

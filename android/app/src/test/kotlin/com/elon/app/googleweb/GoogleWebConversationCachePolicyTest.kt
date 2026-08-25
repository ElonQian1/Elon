package com.elon.app.googleweb

import com.elon.app.chatgptweb.ChatGptWebConversationCollection
import com.elon.app.chatgptweb.WebChatSideMenuRefreshPolicy
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class GoogleWebConversationCachePolicyTest {
    @Test
    fun legacyRowsRenderImmediatelyAndRequestAnOfficialRefresh() {
        val collection = GoogleWebConversationCachePolicy.collection(
            recordCount = 4,
            officialCachedAtMs = 0L,
        )

        assertTrue(collection.stale)
        assertTrue(shouldRefresh(collection))
    }

    @Test
    fun anEmptyFirstRunRequestsTheOfficialDirectory() {
        val collection = GoogleWebConversationCachePolicy.collection(
            recordCount = 0,
            officialCachedAtMs = 0L,
        )

        assertTrue(collection.source == ChatGptWebConversationCollection.SOURCE_NONE)
        assertTrue(shouldRefresh(collection))
    }

    @Test
    fun aRecentlyAcceptedOfficialDirectoryDoesNotRefetchOnEveryOpen() {
        val collection = GoogleWebConversationCachePolicy.collection(
            recordCount = 4,
            officialCachedAtMs = NOW_MS - 30_000L,
        )

        assertFalse(collection.stale)
        assertFalse(shouldRefresh(collection))
    }

    private fun shouldRefresh(collection: ChatGptWebConversationCollection): Boolean =
        WebChatSideMenuRefreshPolicy.shouldRefreshOnOpen(
            collection = collection,
            nowMs = NOW_MS,
            lastRequestedAtMs = 0L,
        )

    private companion object {
        const val NOW_MS = 1_000_000L
    }
}

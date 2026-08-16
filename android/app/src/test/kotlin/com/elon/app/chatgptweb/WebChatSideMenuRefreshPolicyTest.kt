package com.elon.app.chatgptweb

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatSideMenuRefreshPolicyTest {
    @Test
    fun firstOpenRefreshesAnEmptyIndex() {
        assertTrue(shouldRefresh(ChatGptWebConversationCollection()))
    }

    @Test
    fun aFreshCacheRendersWithoutAnotherAutomaticRefresh() {
        assertFalse(shouldRefresh(collection(
            source = ChatGptWebConversationCollection.SOURCE_CACHE,
            cachedAtMs = NOW_MS - 30_000L,
        )))
    }

    @Test
    fun anExpiredOrExplicitlyStaleCacheRefreshesInTheBackground() {
        assertTrue(shouldRefresh(collection(
            source = ChatGptWebConversationCollection.SOURCE_CACHE,
            cachedAtMs = NOW_MS - WebChatSideMenuRefreshPolicy.CACHE_FRESHNESS_MS,
        )))
        assertTrue(shouldRefresh(collection(
            source = ChatGptWebConversationCollection.SOURCE_CACHE,
            cachedAtMs = NOW_MS - 10_000L,
            stale = true,
        )))
    }

    @Test
    fun anInflightOrRecentlyRequestedRefreshIsNotDuplicated() {
        assertFalse(shouldRefresh(collection(
            source = ChatGptWebConversationCollection.SOURCE_CACHE,
            cachedAtMs = NOW_MS - 10 * 60_000L,
            loadState = ChatGptWebConversationCollection.LOAD_LOADING,
        )))
        assertFalse(WebChatSideMenuRefreshPolicy.shouldRefreshOnOpen(
            collection = collection(
                source = ChatGptWebConversationCollection.SOURCE_CACHE,
                cachedAtMs = NOW_MS - 10 * 60_000L,
            ),
            nowMs = NOW_MS,
            lastRequestedAtMs = NOW_MS - 2_000L,
        ))
    }

    @Test
    fun aFailedRefreshCanRetryAfterTheCooldownWithoutDiscardingCache() {
        val failed = collection(
            source = ChatGptWebConversationCollection.SOURCE_CACHE,
            cachedAtMs = NOW_MS - 10_000L,
            loadState = ChatGptWebConversationCollection.LOAD_FAILED,
        )

        assertFalse(WebChatSideMenuRefreshPolicy.shouldRefreshOnOpen(
            failed,
            nowMs = NOW_MS,
            lastRequestedAtMs = NOW_MS - 2_000L,
        ))
        assertTrue(WebChatSideMenuRefreshPolicy.shouldRefreshOnOpen(
            failed,
            nowMs = NOW_MS,
            lastRequestedAtMs = NOW_MS - WebChatSideMenuRefreshPolicy.REQUEST_COOLDOWN_MS,
        ))
    }

    private fun shouldRefresh(collection: ChatGptWebConversationCollection): Boolean =
        WebChatSideMenuRefreshPolicy.shouldRefreshOnOpen(collection, NOW_MS, 0L)

    private fun collection(
        source: String,
        cachedAtMs: Long,
        stale: Boolean = false,
        loadState: String = ChatGptWebConversationCollection.LOAD_READY,
    ) = ChatGptWebConversationCollection(
        source = source,
        cachedAtMs = cachedAtMs,
        stale = stale,
        officialLoadState = loadState,
    )

    private companion object {
        const val NOW_MS = 1_000_000L
    }
}

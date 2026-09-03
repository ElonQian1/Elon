package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class WebChatDictationRouterTest {
    @Test
    fun startUsesPrivateThenSharedThenDom() {
        val calls = mutableListOf<String>()
        val selected = WebChatDictationStartChain.start(
            privateReady = true,
            startPrivate = { calls += "private"; false },
            startShared = { calls += "shared"; false },
            startDom = { calls += "dom"; true },
        )

        assertEquals(WebChatDictationTransport.DOM, selected)
        assertEquals(listOf("private", "shared", "dom"), calls)
    }

    @Test
    fun successfulPrivateStartDoesNotTouchFallbacks() {
        val calls = mutableListOf<String>()
        val selected = WebChatDictationStartChain.start(
            privateReady = true,
            startPrivate = { calls += "private"; true },
            startShared = { calls += "shared"; true },
            startDom = { calls += "dom"; true },
        )

        assertEquals(WebChatDictationTransport.PRIVATE, selected)
        assertEquals(listOf("private"), calls)
    }

    @Test
    fun unavailablePrivateStartsSharedImmediately() {
        val calls = mutableListOf<String>()
        val selected = WebChatDictationStartChain.start(
            privateReady = false,
            startPrivate = { calls += "private"; true },
            startShared = { calls += "shared"; true },
            startDom = { calls += "dom"; true },
        )

        assertEquals(WebChatDictationTransport.SHARED, selected)
        assertEquals(listOf("shared"), calls)
    }

    @Test
    fun returnsNullWhenEveryLayerRejectsStart() {
        assertNull(WebChatDictationStartChain.start(false, { false }, { false }, { false }))
    }

    @Test
    fun activeSessionOwnsTheNextTap() {
        assertEquals(
            WebChatProductionDictationTapRoute.SUBMIT_PRIVATE,
            WebChatProductionDictationRoutePolicy.resolve(true, true, true, true),
        )
        assertEquals(
            WebChatProductionDictationTapRoute.SUBMIT_SHARED,
            WebChatProductionDictationRoutePolicy.resolve(false, true, true, true),
        )
        assertEquals(
            WebChatProductionDictationTapRoute.SUBMIT_DOM,
            WebChatProductionDictationRoutePolicy.resolve(false, false, true, true),
        )
    }
}

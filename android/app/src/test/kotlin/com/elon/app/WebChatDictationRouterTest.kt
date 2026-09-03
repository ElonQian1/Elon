package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatDictationRouterTest {
    @Test
    fun modeDefaultsToPrivateAndChangesOnlyOnExplicitToggle() {
        val selector = WebChatDictationModeSelector()

        assertEquals(WebChatDictationMode.PRIVATE, selector.selected)
        assertEquals(WebChatDictationMode.SHARED, selector.toggle())
        assertEquals(WebChatDictationMode.SHARED, selector.selected)
        assertEquals(WebChatDictationMode.PRIVATE, selector.toggle())
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

    @Test
    fun completedSessionCannotImmediatelyRearm() {
        var now = 1_000L
        val gate = WebChatDictationRearmGate(clock = { now }, settleMs = 600L)

        assertTrue(gate.canStart())
        assertFalse(gate.observe(true))
        assertTrue(gate.canStart())
        assertTrue(gate.observe(false))
        assertFalse(gate.canStart())

        now += 599L
        assertFalse(gate.canStart())
        now += 1L
        assertTrue(gate.canStart())
    }
}

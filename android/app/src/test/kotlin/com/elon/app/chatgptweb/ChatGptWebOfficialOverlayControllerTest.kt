package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebOfficialOverlayControllerTest {
    @Test
    fun dismissesStackedOfficialOverlaysBeforeRunningTheRequestedAction() {
        var escapes = 0
        var refreshed = false
        var requested = false
        val scheduled = ArrayDeque<() -> Unit>()
        val controller = ChatGptWebOfficialOverlayController(
            dispatchEscape = { escapes += 1 },
            schedule = { _, action -> scheduled.addLast(action) },
            refreshManifest = { refreshed = true },
        )

        controller.dismissAllThen { requested = true }
        assertEquals(1, escapes)
        assertFalse(requested)

        while (scheduled.isNotEmpty()) scheduled.removeFirst().invoke()

        assertEquals(3, escapes)
        assertTrue(refreshed)
        assertTrue(requested)
    }

    @Test
    fun newerRequestCancelsAnOlderScheduledRequest() {
        val scheduled = ArrayDeque<() -> Unit>()
        var firstRequested = false
        var secondRequested = false
        val controller = ChatGptWebOfficialOverlayController(
            dispatchEscape = {},
            schedule = { _, action -> scheduled.addLast(action) },
            refreshManifest = {},
        )

        controller.dismissAllThen { firstRequested = true }
        controller.dismissAllThen { secondRequested = true }
        while (scheduled.isNotEmpty()) scheduled.removeFirst().invoke()

        assertFalse(firstRequested)
        assertTrue(secondRequested)
    }

    @Test
    fun dismissTopRefreshesWithoutRunningAComposerRequest() {
        val scheduled = ArrayDeque<() -> Unit>()
        var escapes = 0
        var refreshed = false
        val controller = ChatGptWebOfficialOverlayController(
            dispatchEscape = { escapes += 1 },
            schedule = { _, action -> scheduled.addLast(action) },
            refreshManifest = { refreshed = true },
        )

        controller.dismissTop()
        while (scheduled.isNotEmpty()) scheduled.removeFirst().invoke()

        assertEquals(1, escapes)
        assertTrue(refreshed)
    }
}

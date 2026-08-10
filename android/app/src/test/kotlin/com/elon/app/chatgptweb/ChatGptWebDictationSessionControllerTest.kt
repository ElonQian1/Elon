package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebDictationSessionControllerTest {
    @Test
    fun nativeSessionUsesOfficialRecorderThenRestoresNativeMode() {
        var mode = "native"
        val controller = controller(
            isNative = { mode == "native" },
            showOfficial = { mode = "official" },
            restoreNative = { mode = "native" },
        )

        controller.onStartRequested()
        assertEquals("official", mode)

        controller.onSnapshot(false)
        assertEquals("official", mode)

        controller.onSnapshot(true)
        controller.onSnapshot(false)
        assertEquals("native", mode)
    }

    @Test
    fun officialSessionDoesNotForceNativeModeAfterRecording() {
        var mode = "official"
        val controller = controller(
            isNative = { false },
            showOfficial = { mode = "official" },
            restoreNative = { mode = "native" },
        )

        controller.onSnapshot(true)
        controller.onSnapshot(false)

        assertEquals("official", mode)
    }

    @Test
    fun backCancelsOnlyAnActiveDictationSession() {
        var cancellations = 0
        val controller = controller(cancel = { cancellations += 1 })

        assertFalse(controller.handleBack())
        controller.onSnapshot(true)
        assertTrue(controller.handleBack())
        assertEquals(1, cancellations)
        controller.onSnapshot(false)
        assertFalse(controller.handleBack())
    }

    @Test
    fun failedOrTimedOutStartRestoresTheRememberedNativeMode() {
        var mode = "native"
        val controller = controller(
            isNative = { mode == "native" },
            showOfficial = { mode = "official" },
            restoreNative = { mode = "native" },
        )

        val attempt = controller.onStartRequested() ?: error("attempt missing")
        assertEquals("official", mode)
        controller.onStartTimedOut(attempt)
        assertEquals("native", mode)
    }

    @Test
    fun staleTimeoutCannotCancelANewerStartAttempt() {
        var mode = "native"
        val controller = controller(
            isNative = { mode == "native" },
            showOfficial = { mode = "official" },
            restoreNative = { mode = "native" },
        )

        val first = controller.onStartRequested() ?: error("first attempt missing")
        controller.onStartFailed()
        val second = controller.onStartRequested() ?: error("second attempt missing")
        controller.onStartTimedOut(first)

        assertEquals("official", mode)
        controller.onStartTimedOut(second)
        assertEquals("native", mode)
    }

    @Test
    fun backDuringPendingStartRestoresNativeWithoutCancellingAnInactiveRecorder() {
        var mode = "native"
        var cancellations = 0
        val controller = controller(
            isNative = { mode == "native" },
            showOfficial = { mode = "official" },
            restoreNative = { mode = "native" },
            cancel = { cancellations += 1 },
        )

        controller.onStartRequested()

        assertTrue(controller.handleBack())
        assertEquals("native", mode)
        assertEquals(0, cancellations)
    }

    @Test
    fun startWaitsForOfficialViewSettlementAndSkipsAStaleAction() {
        val scheduled = mutableListOf<Pair<Long, () -> Unit>>()
        var starts = 0
        val controller = controller(
            schedule = { delayMs, action -> scheduled += delayMs to action },
        )

        val attempt = controller.onStartRequested { starts += 1 } ?: error("attempt missing")
        assertEquals(0, starts)
        assertEquals(ChatGptWebDictationSessionController.OFFICIAL_SETTLE_MS, scheduled.single().first)
        scheduled.single().second()
        assertEquals(1, starts)

        controller.onStartTimedOut(attempt)
        scheduled.single().second()
        assertEquals(1, starts)
    }

    private fun controller(
        isNative: () -> Boolean = { true },
        showOfficial: () -> Unit = {},
        restoreNative: () -> Unit = {},
        cancel: () -> Unit = {},
        schedule: (Long, () -> Unit) -> Unit = { _, action -> action() },
    ) = ChatGptWebDictationSessionController(
        isNativeSelected = isNative,
        showOfficial = showOfficial,
        restoreNative = restoreNative,
        cancelOfficial = cancel,
        schedule = schedule,
    )
}

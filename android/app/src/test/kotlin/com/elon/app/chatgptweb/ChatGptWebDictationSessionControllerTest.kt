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

        controller.onSnapshot(true)
        assertEquals("official", mode)

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

    private fun controller(
        isNative: () -> Boolean = { true },
        showOfficial: () -> Unit = {},
        restoreNative: () -> Unit = {},
        cancel: () -> Unit = {},
    ) = ChatGptWebDictationSessionController(
        isNativeSelected = isNative,
        showOfficial = showOfficial,
        restoreNative = restoreNative,
        cancelOfficial = cancel,
    )
}

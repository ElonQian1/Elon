package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebUiControlInvocationCoordinatorTest {
    @Test
    fun messageOverflowUsesOfficialLayoutBeforeInvocation() {
        val fixture = Fixture(officialVisible = false)

        fixture.coordinator.invoke(control(semantic = "more"), "message-more", "request-1")

        assertEquals(1, fixture.showOfficialCount)
        assertTrue(fixture.invocations.isEmpty())
        assertEquals(ChatGptWebUiControlInvocationCoordinator.OFFICIAL_LAYOUT_SETTLE_MS, fixture.delayMs)

        fixture.scheduled?.invoke()

        assertEquals(listOf("message-more" to "request-1"), fixture.invocations)
    }

    @Test
    fun messageOverflowAlreadyInOfficialLayoutInvokesImmediately() {
        val fixture = Fixture(officialVisible = true)

        fixture.coordinator.invoke(control(semantic = "more"), "message-more", null)

        assertEquals(0, fixture.showOfficialCount)
        assertEquals(listOf("message-more" to null), fixture.invocations)
        assertEquals(null, fixture.scheduled)
    }

    @Test
    fun ordinaryMessageActionsKeepNativeInvocationPath() {
        val fixture = Fixture(officialVisible = false)

        fixture.coordinator.invoke(control(semantic = "copy"), "message-copy", "request-2")

        assertEquals(0, fixture.showOfficialCount)
        assertEquals(listOf("message-copy" to "request-2"), fixture.invocations)
        assertEquals(null, fixture.scheduled)
    }

    @Test
    fun realtimeVoiceOpensOfficialLayoutBeforeInvocation() {
        val fixture = Fixture(officialVisible = false)

        fixture.coordinator.invoke(
            control(
                semantic = ChatGptRealtimeVoicePolicy.SEMANTIC,
                region = ChatGptWebUiRegion.COMPOSER,
            ),
            "realtime-voice",
            "request-voice",
        )

        assertEquals(1, fixture.showOfficialCount)
        assertTrue(fixture.invocations.isEmpty())
        fixture.scheduled?.invoke()
        assertEquals(listOf("realtime-voice" to "request-voice"), fixture.invocations)
    }

    @Test
    fun nonMessageMoreControlsKeepNativeInvocationPath() {
        val fixture = Fixture(officialVisible = false)

        fixture.coordinator.invoke(
            control(semantic = "more", region = ChatGptWebUiRegion.CONTENT),
            "content-more",
            null,
        )

        assertEquals(0, fixture.showOfficialCount)
        assertEquals(listOf("content-more" to null), fixture.invocations)
    }

    @Test
    fun disposeCancelsDelayedInvocation() {
        val fixture = Fixture(officialVisible = false)

        fixture.coordinator.invoke(control(semantic = "more"), "message-more", "request-3")
        fixture.coordinator.dispose()
        fixture.scheduled?.invoke()

        assertTrue(fixture.invocations.isEmpty())
    }

    @Test
    fun dictationInvocationUsesTheSharedSessionGuardAndTimeout() {
        val fixture = Fixture(officialVisible = false)

        fixture.coordinator.invoke(control(semantic = "dictation"), "dictation", "request-4")

        assertEquals(listOf("dictation" to "request-4"), fixture.invocations)
        assertEquals(ChatGptWebUiControlInvocationCoordinator.DICTATION_START_TIMEOUT_MS, fixture.delayMs)
        fixture.scheduled?.invoke()
        assertEquals(listOf(41L), fixture.timedOutAttempts)
    }

    @Test
    fun rejectedDictationReportsTheOriginalCommand() {
        val fixture = Fixture(officialVisible = false, dictationAttempt = null)

        fixture.coordinator.startDictation("request-5")

        assertEquals(
            listOf(Triple("request-5", "start_dictation", "dictation_start_in_progress")),
            fixture.failures,
        )
        assertTrue(fixture.invocations.isEmpty())
    }

    private fun control(
        semantic: String,
        region: String = ChatGptWebUiRegion.MESSAGE,
    ) = ChatGptWebUiControl(
        id = "control-$semantic",
        semantic = semantic,
        label = semantic,
        region = region,
        role = "button",
        enabled = true,
        selected = false,
        contextId = "conversation-turn-2",
        inViewport = true,
        webXRatio = 0.5,
        webYRatio = 0.5,
    )

    private class Fixture(
        officialVisible: Boolean,
        private val dictationAttempt: Long? = 41L,
    ) {
        var showOfficialCount = 0
        var delayMs: Long? = null
        var scheduled: (() -> Unit)? = null
        val invocations = mutableListOf<Pair<String, String?>>()
        val timedOutAttempts = mutableListOf<Long>()
        val failures = mutableListOf<Triple<String, String, String>>()
        val coordinator = ChatGptWebUiControlInvocationCoordinator(
            isOfficialVisible = { officialVisible },
            showOfficial = { showOfficialCount += 1 },
            schedule = { delay, action ->
                delayMs = delay
                scheduled = action
            },
            beginDictation = { start ->
                if (dictationAttempt != null) start()
                dictationAttempt
            },
            onDictationTimedOut = timedOutAttempts::add,
            startOfficialDictation = { requestId ->
                invocations += "start-dictation" to requestId
            },
            failCommand = { requestId, action, error ->
                failures += Triple(requestId, action, error)
            },
            invokeOfficialControl = { controlId, requestId ->
                invocations += controlId to requestId
            },
        )
    }
}

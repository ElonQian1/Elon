package com.elon.app.googleweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class GoogleWebPendingSendStateTest {
    @Test
    fun unconfirmedSubmissionRestoresPromptAfterTimeout() {
        val state = GoogleWebPendingSendState()
        assertEquals(GoogleWebPendingSendState.Phase.IDLE, state.phase())
        val generation = state.begin("hello")
        assertEquals(GoogleWebPendingSendState.Phase.SUBMITTING, state.phase())

        val result = state.onConfirmationTimeout(generation)

        assertEquals(GoogleWebPendingSendState.TimeoutAction.RESTORE, result.action)
        assertEquals("hello", result.prompt)
        assertNull(state.prompt())
        assertEquals(GoogleWebPendingSendState.Phase.IDLE, state.phase())
    }

    @Test
    fun confirmedSubmissionKeepsWaitingWithoutRestoringPrompt() {
        val state = GoogleWebPendingSendState()
        val generation = state.begin("hello")

        assertTrue(state.confirmSubmission())
        assertEquals(GoogleWebPendingSendState.Phase.AWAITING_RESPONSE, state.phase())
        val result = state.onConfirmationTimeout(generation)

        assertEquals(GoogleWebPendingSendState.TimeoutAction.KEEP_WAITING, result.action)
        assertNull(result.prompt)
        assertEquals("hello", state.prompt())
        assertEquals(
            GoogleWebPendingSendState.TimeoutAction.KEEP_WAITING,
            state.onConfirmationTimeout(generation).action,
        )
        assertEquals("hello", state.prompt())
        assertEquals(
            GoogleWebPendingSendState.TimeoutAction.REQUIRE_OFFICIAL_CONFIRMATION,
            state.onConfirmationTimeout(generation).action,
        )
        assertTrue(state.requiresOfficialConfirmation())
        assertEquals(GoogleWebPendingSendState.Phase.OFFICIAL_CONFIRMATION, state.phase())
        assertEquals(
            GoogleWebPendingSendState.TimeoutAction.IGNORE,
            state.onConfirmationTimeout(generation).action,
        )
        assertEquals("hello", state.prompt())
    }

    @Test
    fun observedUserMessageWaitsForItsAssistantBeforeCompleting() {
        val state = GoogleWebPendingSendState()
        val generation = state.begin("hello")
        state.confirmSubmission()

        assertFalse(state.observeCompletedTurn("different", assistantObserved = true))
        assertFalse(state.observeCompletedTurn(" hello ", assistantObserved = false))
        assertEquals("hello", state.prompt())
        assertTrue(state.observeCompletedTurn(" hello ", assistantObserved = true))
        assertNull(state.prompt())
        assertFalse(state.requiresOfficialConfirmation())
        assertEquals(
            GoogleWebPendingSendState.TimeoutAction.IGNORE,
            state.onConfirmationTimeout(generation).action,
        )
    }

    @Test
    fun staleTimeoutCannotRestoreNewerPrompt() {
        val state = GoogleWebPendingSendState()
        val oldGeneration = state.begin("old")
        val newGeneration = state.begin("new")

        assertEquals(
            GoogleWebPendingSendState.TimeoutAction.IGNORE,
            state.onConfirmationTimeout(oldGeneration).action,
        )
        assertEquals("new", state.prompt())
        assertEquals(
            GoogleWebPendingSendState.TimeoutAction.RESTORE,
            state.onConfirmationTimeout(newGeneration).action,
        )
    }

    @Test
    fun failedSubmissionReturnsPromptOnce() {
        val state = GoogleWebPendingSendState()
        state.begin("retry me")

        assertEquals("retry me", state.failSubmission())
        assertNull(state.failSubmission())
        assertFalse(state.confirmSubmission())
    }

    @Test
    fun consumerStatusExplainsEachPendingSendPhase() {
        assertNull(GoogleWebPendingSendPresentation.status(GoogleWebPendingSendState.Phase.IDLE))
        assertEquals(
            "发送中…",
            GoogleWebPendingSendPresentation.status(GoogleWebPendingSendState.Phase.SUBMITTING),
        )
        assertEquals(
            "已发送 · 等待回复",
            GoogleWebPendingSendPresentation.status(GoogleWebPendingSendState.Phase.AWAITING_RESPONSE),
        )
        assertEquals(
            "已发送 · 回答同步较慢",
            GoogleWebPendingSendPresentation.status(
                GoogleWebPendingSendState.Phase.OFFICIAL_CONFIRMATION,
            ),
        )
    }
}

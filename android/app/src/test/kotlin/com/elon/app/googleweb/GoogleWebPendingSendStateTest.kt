package com.elon.app.googleweb

import com.elon.app.WebChatPendingSendPresentation
import com.elon.app.WebChatPendingSendState
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class GoogleWebPendingSendStateTest {
    @Test
    fun visibleOfficialQueryConfirmsNavigationBasedSubmission() {
        val state = WebChatPendingSendState()
        val generation = state.begin("hello   world")

        assertTrue(state.observeSubmission(" hello world "))
        assertFalse(state.observeSubmission("hello world"))
        assertEquals(WebChatPendingSendState.Phase.AWAITING_RESPONSE, state.phase())
        assertEquals(
            WebChatPendingSendState.TimeoutAction.KEEP_WAITING,
            state.onConfirmationTimeout(generation).action,
        )
    }

    @Test
    fun unrelatedOfficialQueryDoesNotConfirmPendingSubmission() {
        val state = WebChatPendingSendState()
        state.begin("expected")

        assertFalse(state.observeSubmission("different"))
        assertEquals(WebChatPendingSendState.Phase.SUBMITTING, state.phase())
    }

    @Test
    fun unconfirmedSubmissionRestoresPromptAfterTimeout() {
        val state = WebChatPendingSendState()
        assertEquals(WebChatPendingSendState.Phase.IDLE, state.phase())
        val generation = state.begin("hello")
        assertEquals(WebChatPendingSendState.Phase.SUBMITTING, state.phase())

        val result = state.onConfirmationTimeout(generation)

        assertEquals(WebChatPendingSendState.TimeoutAction.RESTORE, result.action)
        assertEquals("hello", result.prompt)
        assertNull(state.prompt())
        assertEquals(WebChatPendingSendState.Phase.IDLE, state.phase())
    }

    @Test
    fun confirmedSubmissionKeepsWaitingWithoutRestoringPrompt() {
        val state = WebChatPendingSendState()
        val generation = state.begin("hello")

        assertTrue(state.confirmSubmission())
        assertEquals(WebChatPendingSendState.Phase.AWAITING_RESPONSE, state.phase())
        val result = state.onConfirmationTimeout(generation)

        assertEquals(WebChatPendingSendState.TimeoutAction.KEEP_WAITING, result.action)
        assertNull(result.prompt)
        assertEquals("hello", state.prompt())
        assertEquals(
            WebChatPendingSendState.TimeoutAction.KEEP_WAITING,
            state.onConfirmationTimeout(generation).action,
        )
        assertEquals("hello", state.prompt())
        assertEquals(
            WebChatPendingSendState.TimeoutAction.REQUIRE_OFFICIAL_CONFIRMATION,
            state.onConfirmationTimeout(generation).action,
        )
        assertTrue(state.requiresOfficialConfirmation())
        assertEquals(WebChatPendingSendState.Phase.OFFICIAL_CONFIRMATION, state.phase())
        assertEquals(
            WebChatPendingSendState.TimeoutAction.IGNORE,
            state.onConfirmationTimeout(generation).action,
        )
        assertEquals("hello", state.prompt())
    }

    @Test
    fun observedUserMessageWaitsForItsAssistantBeforeCompleting() {
        val state = WebChatPendingSendState()
        val generation = state.begin("hello")
        state.confirmSubmission()

        assertFalse(state.observeCompletedTurn("different", assistantObserved = true))
        assertFalse(state.observeCompletedTurn(" hello ", assistantObserved = false))
        assertEquals("hello", state.prompt())
        assertTrue(state.observeCompletedTurn(" hello ", assistantObserved = true))
        assertNull(state.prompt())
        assertFalse(state.requiresOfficialConfirmation())
        assertEquals(
            WebChatPendingSendState.TimeoutAction.IGNORE,
            state.onConfirmationTimeout(generation).action,
        )
    }

    @Test
    fun staleTimeoutCannotRestoreNewerPrompt() {
        val state = WebChatPendingSendState()
        val oldGeneration = state.begin("old")
        val newGeneration = state.begin("new")

        assertEquals(
            WebChatPendingSendState.TimeoutAction.IGNORE,
            state.onConfirmationTimeout(oldGeneration).action,
        )
        assertEquals("new", state.prompt())
        assertEquals(
            WebChatPendingSendState.TimeoutAction.RESTORE,
            state.onConfirmationTimeout(newGeneration).action,
        )
    }

    @Test
    fun failedSubmissionReturnsPromptOnce() {
        val state = WebChatPendingSendState()
        state.begin("retry me")

        assertEquals("retry me", state.failSubmission())
        assertNull(state.failSubmission())
        assertFalse(state.confirmSubmission())
    }

    @Test
    fun consumerStatusExplainsEachPendingSendPhase() {
        assertNull(WebChatPendingSendPresentation.status(WebChatPendingSendState.Phase.IDLE))
        assertEquals(
            "发送中…",
            WebChatPendingSendPresentation.status(WebChatPendingSendState.Phase.SUBMITTING),
        )
        assertEquals(
            "已发送 · 等待回复",
            WebChatPendingSendPresentation.status(WebChatPendingSendState.Phase.AWAITING_RESPONSE),
        )
        assertEquals(
            "已发送 · 回答同步较慢",
            WebChatPendingSendPresentation.status(
                WebChatPendingSendState.Phase.OFFICIAL_CONFIRMATION,
            ),
        )
    }
}

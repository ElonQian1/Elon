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
        val generation = state.begin("hello")

        val result = state.onConfirmationTimeout(generation)

        assertEquals(GoogleWebPendingSendState.TimeoutAction.RESTORE, result.action)
        assertEquals("hello", result.prompt)
        assertNull(state.prompt())
    }

    @Test
    fun confirmedSubmissionKeepsWaitingWithoutRestoringPrompt() {
        val state = GoogleWebPendingSendState()
        val generation = state.begin("hello")

        assertTrue(state.confirmSubmission())
        val result = state.onConfirmationTimeout(generation)

        assertEquals(GoogleWebPendingSendState.TimeoutAction.KEEP_WAITING, result.action)
        assertNull(result.prompt)
        assertEquals("hello", state.prompt())
    }

    @Test
    fun observedUserMessageCompletesPendingSubmission() {
        val state = GoogleWebPendingSendState()
        val generation = state.begin("hello")
        state.confirmSubmission()

        assertFalse(state.observeUserPrompt("different"))
        assertTrue(state.observeUserPrompt(" hello "))
        assertNull(state.prompt())
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
}

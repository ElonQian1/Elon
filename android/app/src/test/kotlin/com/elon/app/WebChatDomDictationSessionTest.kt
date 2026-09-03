package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatDomDictationSessionTest {
    @Test
    fun officialRecorderReleaseKeepsReviewControlsUntilUserAccepts() {
        var now = 100L
        val session = WebChatDomDictationSession(nowMs = { now })

        assertTrue(session.startRequested("existing"))
        session.commandResult(WebChatDomDictationSession.START_ACTION, true)
        assertEquals(WebChatDomDictationPhase.ACTIVE, session.state(true, "existing").phase)

        assertEquals(WebChatDomDictationPhase.REVIEW, session.state(false, "transcript").phase)
        assertTrue(session.acceptReview())
        assertEquals(WebChatDomDictationPhase.IDLE, session.state(false, "transcript").phase)
    }

    @Test
    fun cancelingReviewRestoresTheDraftFromBeforeRecording() {
        val session = WebChatDomDictationSession(nowMs = { 10L })

        session.startRequested("before")
        session.state(true, "before")
        session.state(false, "transcript")

        assertEquals("before", session.cancelReview())
        assertFalse(session.state(false, "before").controlsActive)
    }

    @Test
    fun startFailureAndTimeoutDoNotLeaveControlsStuck() {
        var now = 0L
        val failed = WebChatDomDictationSession(nowMs = { now })
        failed.startRequested("")
        failed.commandResult(WebChatDomDictationSession.START_ACTION, false)
        assertFalse(failed.state(false, "").controlsActive)

        val timedOut = WebChatDomDictationSession(nowMs = { now }, startTimeoutMs = 50L)
        timedOut.startRequested("")
        now = 51L
        assertFalse(timedOut.state(false, "").controlsActive)
    }

    @Test
    fun officialControlsDoNotConfirmAStartUntilMicrophoneCaptureIsLive() {
        var now = 0L
        val session = WebChatDomDictationSession(nowMs = { now }, startTimeoutMs = 50L)

        assertTrue(session.startRequested(""))
        session.commandResult(WebChatDomDictationSession.START_ACTION, true)
        assertEquals(
            WebChatDomDictationPhase.STARTING,
            session.state(officialActive = true, currentDraft = "", captureActive = false).phase,
        )

        assertEquals(
            WebChatDomDictationPhase.ACTIVE,
            session.state(officialActive = true, currentDraft = "", captureActive = true).phase,
        )
    }

    @Test
    fun controlOnlyStartupTimesOutAsRetryableFailure() {
        var now = 0L
        val session = WebChatDomDictationSession(nowMs = { now }, startTimeoutMs = 50L)

        assertTrue(session.startRequested("draft"))
        session.commandResult(WebChatDomDictationSession.START_ACTION, true)
        now = 51L

        val failed = session.state(
            officialActive = true,
            currentDraft = "draft",
            captureActive = false,
        )
        assertEquals(WebChatDomDictationPhase.START_FAILED, failed.phase)
        assertTrue(failed.startFailed)

        assertEquals(
            WebChatDomDictationPhase.IDLE,
            session.state(officialActive = false, currentDraft = "draft", captureActive = false).phase,
        )
    }

    @Test
    fun unrelatedOfficialControlsDoNotAdoptAnUnrequestedSession() {
        val session = WebChatDomDictationSession(nowMs = { 10L })

        val state = session.state(
            officialActive = true,
            currentDraft = "draft",
            captureActive = false,
        )

        assertEquals(WebChatDomDictationPhase.IDLE, state.phase)
        assertFalse(state.controlsActive)
        assertTrue(session.startRequested("draft"))
    }

    @Test
    fun liveCaptureCanAdoptAnOfficialSessionStartedOutsideNativeControls() {
        val session = WebChatDomDictationSession(nowMs = { 10L })

        val state = session.state(
            officialActive = true,
            currentDraft = "draft",
            captureActive = true,
        )

        assertEquals(WebChatDomDictationPhase.ACTIVE, state.phase)
        assertTrue(state.controlsActive)
    }

    @Test
    fun disappearingControlsDoNotEndAStillLiveCapture() {
        val session = WebChatDomDictationSession(nowMs = { 10L })
        session.startRequested("")
        session.state(officialActive = true, currentDraft = "", captureActive = true)

        val state = session.state(
            officialActive = false,
            currentDraft = "",
            captureActive = true,
        )

        assertEquals(WebChatDomDictationPhase.ACTIVE, state.phase)
        assertFalse(state.reviewPending)
    }

    @Test
    fun explicitOfficialSubmitDoesNotCreateASecondReviewState() {
        val session = WebChatDomDictationSession(nowMs = { 10L })
        session.startRequested("")
        session.state(true, "")

        assertTrue(session.finishRequested(WebChatDomDictationSession.SUBMIT_ACTION))
        session.commandResult(WebChatDomDictationSession.SUBMIT_ACTION, true)

        assertEquals(WebChatDomDictationPhase.IDLE, session.state(false, "transcript").phase)
    }

    @Test
    fun acceptedFinishKeepsControlsUntilOfficialRecorderActuallyStops() {
        var now = 10L
        val session = WebChatDomDictationSession(nowMs = { now })
        session.startRequested("")
        session.state(true, "")

        assertTrue(session.finishRequested(WebChatDomDictationSession.CANCEL_ACTION))
        session.commandResult(WebChatDomDictationSession.CANCEL_ACTION, true)

        assertEquals(WebChatDomDictationPhase.CANCELLING, session.state(true, "").phase)
        now += WebChatDomDictationSession.DEFAULT_FINISH_TIMEOUT_MS + 1
        assertEquals(WebChatDomDictationPhase.ACTIVE, session.state(true, "").phase)
    }

    @Test
    fun failedFinishImmediatelyRestoresRetryableControls() {
        val session = WebChatDomDictationSession(nowMs = { 10L })
        session.startRequested("")
        session.state(true, "")

        assertTrue(session.finishRequested(WebChatDomDictationSession.SUBMIT_ACTION))
        session.commandResult(WebChatDomDictationSession.SUBMIT_ACTION, false)

        assertEquals(WebChatDomDictationPhase.ACTIVE, session.state(true, "").phase)
    }
}

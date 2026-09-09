package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class WebChatModelLevelSubmissionTest {
    @Test fun keyboardAndAccessibilityChangesSubmitWithoutTouchCallbacks() {
        val policy = WebChatModelLevelSubmission(5)
        assertEquals(2, policy.progress(2, fromUser = true))
        assertEquals(3, policy.progress(3, fromUser = true))
        assertNull(policy.stopTouch(3))
    }

    @Test fun dragOnlySubmitsItsLastUserPositionOnRelease() {
        val policy = WebChatModelLevelSubmission(5)
        policy.startTouch()
        assertNull(policy.progress(1, fromUser = true))
        assertNull(policy.progress(2, fromUser = true))
        assertNull(policy.progress(4, fromUser = true))
        assertEquals(4, policy.stopTouch(4))
        assertNull(policy.stopTouch(4))
    }

    @Test fun tapsWithoutProgressAndProgrammaticUpdatesDoNotSubmit() {
        val policy = WebChatModelLevelSubmission(5)
        assertNull(policy.progress(3, fromUser = false))
        policy.startTouch()
        assertNull(policy.stopTouch(3))
        policy.startTouch()
        assertNull(policy.progress(2, fromUser = false))
        assertNull(policy.stopTouch(2))
    }

    @Test fun programmaticUpdateDuringDragInvalidatesTheOldUserPosition() {
        val policy = WebChatModelLevelSubmission(5)
        policy.startTouch()
        policy.progress(2, fromUser = true)
        policy.progress(3, fromUser = false)
        assertNull(policy.stopTouch(3))
    }

    @Test fun changedReleaseValueDoesNotCommitAnUnobservedPosition() {
        val policy = WebChatModelLevelSubmission(5)
        policy.startTouch()
        policy.progress(1, fromUser = true)
        assertNull(policy.stopTouch(4))
        assertNull(policy.stopTouch(1))
    }

    @Test fun invalidValuesDoNotWriteOrSurviveAsPendingChanges() {
        val policy = WebChatModelLevelSubmission(5)
        assertNull(policy.progress(-1, fromUser = true))
        assertNull(policy.progress(5, fromUser = true))
        policy.startTouch()
        policy.progress(2, fromUser = true)
        policy.progress(7, fromUser = true)
        assertNull(policy.stopTouch(2))
    }

    @Test fun newTouchReplacesOldPendingIntentAndLaterKeyboardStillWorks() {
        val policy = WebChatModelLevelSubmission(5)
        policy.startTouch()
        policy.progress(4, fromUser = true)
        policy.startTouch()
        assertNull(policy.stopTouch(4))
        assertEquals(1, policy.progress(1, fromUser = true))
    }
}

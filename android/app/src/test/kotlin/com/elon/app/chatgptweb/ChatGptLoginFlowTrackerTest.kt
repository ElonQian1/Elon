package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Test

class ChatGptLoginFlowTrackerTest {
    @Test
    fun tracksOfficialAuthenticationWithoutRetainingSensitivePageData() {
        var now = 1_000L
        val tracker = ChatGptLoginFlowTracker { now }

        assertEquals(ChatGptLoginStage.OPENING_OFFICIAL_AUTH, tracker.begin().stage)

        now = 1_400L
        assertEquals(
            ChatGptLoginStage.WAITING_FOR_USER,
            tracker.onPageReady("https://chatgpt.com/auth/login?state=secret").stage,
        )

        now = 2_100L
        assertEquals(
            ChatGptLoginStage.WAITING_FOR_USER,
            tracker.onPageStarted("https://accounts.google.com/o/oauth2/auth?code=secret").stage,
        )

        now = 3_500L
        assertEquals(
            ChatGptLoginStage.COMPLETING,
            tracker.onPageReady("https://chatgpt.com/").stage,
        )

        now = 4_250L
        val authenticated = tracker.markAuthenticated()
        assertEquals(ChatGptLoginStage.AUTHENTICATED, authenticated.stage)
        assertEquals(3_250L, authenticated.elapsedMillis)
        assertFalse(authenticated.isRunning)
        assertEquals(3, ChatGptLoginFlowSnapshot::class.java.declaredFields.size)
    }

    @Test
    fun reportsChallengeAndFailureOnlyForAnActiveAttempt() {
        var now = 5_000L
        val tracker = ChatGptLoginFlowTracker { now }

        assertEquals(ChatGptLoginStage.READY, tracker.fail().stage)
        tracker.begin()
        now = 5_500L
        assertEquals(
            ChatGptLoginStage.WAITING_FOR_USER,
            tracker.onPageStarted("https://chatgpt.com/cdn-cgi/challenge-platform/test").stage,
        )

        now = 6_000L
        val failed = tracker.fail()
        assertEquals(ChatGptLoginStage.FAILED, failed.stage)
        assertEquals(1_000L, failed.elapsedMillis)
        assertFalse(failed.isRunning)

        assertEquals(0L, tracker.reset().elapsedMillis)
    }
}

package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptComposerRefreshInterlockTest {
    @Test
    fun repeatedForegroundRequestsExtendTheQuietWindowBeforeRefreshResumes() {
        val fixture = Fixture()

        fixture.interlock.acquire()
        fixture.interlock.releaseAfterQuietPeriod()
        val staleRelease = fixture.scheduled.single().task
        fixture.interlock.acquire()

        assertEquals(listOf("suspend"), fixture.events)
        assertTrue(fixture.scheduled.isEmpty())
        staleRelease.run()
        assertTrue(fixture.interlock.isHeld())

        fixture.interlock.releaseAfterQuietPeriod()
        assertEquals(2_000L, fixture.scheduled.single().delayMs)
        fixture.runNext()

        assertEquals(listOf("suspend", "resume"), fixture.events)
        assertFalse(fixture.interlock.isHeld())
    }

    @Test
    fun abandonCancelsWorkWithoutResumingDuringPageNavigation() {
        val fixture = Fixture()

        fixture.interlock.acquire()
        fixture.interlock.releaseAfterQuietPeriod()
        fixture.interlock.abandon()

        assertEquals(listOf("suspend"), fixture.events)
        assertTrue(fixture.scheduled.isEmpty())
        assertFalse(fixture.interlock.isHeld())
    }

    private class Fixture {
        val events = mutableListOf<String>()
        val scheduled = mutableListOf<Scheduled>()
        val interlock = ChatGptComposerRefreshInterlock(
            suspendRefresh = { events += "suspend" },
            resumeRefresh = { events += "resume" },
            schedule = { task, delayMs -> scheduled += Scheduled(task, delayMs) },
            cancel = { task -> scheduled.removeAll { it.task === task } },
        )

        fun runNext() = scheduled.removeAt(0).task.run()
    }

    private data class Scheduled(val task: Runnable, val delayMs: Long)
}
